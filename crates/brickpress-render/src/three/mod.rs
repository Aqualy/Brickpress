//! The 3D view: the same document as extruded pieces on a studded plate.
//!
//! Rendering happens offscreen (MSAA, depth, shadow map) and is blitted into
//! the UI pass, so the UI framebuffer needs no depth buffer.

pub mod camera;
pub mod mesh;

use crate::scene::srgb;
use crate::wgpu;
use brickpress_core::catalog::catalog;
use brickpress_core::geometry::piece_matrix;
use brickpress_core::{PlacedPiece, PressDocument};
use bytemuck::{Pod, Zeroable};
use camera::CameraFrame;
use glam::{Mat4, Vec3};
use mesh::{Mesh, Vertex};
use std::collections::HashSet;
use wgpu::util::DeviceExt;

pub const FLAG_SELECTED: u32 = 1;
pub const FLAG_GHOST: u32 = 2;
const SAMPLES: u32 = 4;
const SHADOW_SIZE: u32 = 2048;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const PLATE_COLOR: &str = "#e9e5db";

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance3D {
    pub m: [f32; 4],
    pub t: [f32; 2],
    pub _pad: [f32; 2],
    pub color: [f32; 4],
    pub flags: u32,
    pub _pad2: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    light_dir: [f32; 4],
    board: [f32; 4],
    accent: [f32; 4],
}

/// Instances grouped by catalog piece, plus ghosts, for one document state.
pub struct Scene3D {
    pub board: (f32, f32),
    /// `by_piece[i]` holds instances of catalog piece `i`.
    pub by_piece: Vec<Vec<Instance3D>>,
    pub ghosts: Vec<(usize, Instance3D)>,
    pub studs: Vec<Instance3D>,
}

fn instance(p: &PlacedPiece, color: [f32; 3], alpha: f32, flags: u32) -> Option<(usize, Instance3D)> {
    let index = catalog().piece_index(&p.piece_id)?;
    let m = piece_matrix(&catalog().pieces[index], p, 1.0);
    Some((
        index,
        Instance3D {
            m: [m[0] as f32, m[1] as f32, m[2] as f32, m[3] as f32],
            t: [m[4] as f32, m[5] as f32],
            _pad: [0.0; 2],
            color: [color[0], color[1], color[2], alpha],
            flags,
            _pad2: [0; 3],
        },
    ))
}

impl Scene3D {
    pub fn build(
        doc: &PressDocument,
        selected: &HashSet<&str>,
        drag: (i32, i32),
        ghosts: &[(PlacedPiece, [f32; 3])],
        ghosts_valid: bool,
    ) -> Self {
        let mut by_piece = vec![Vec::new(); catalog().pieces.len()];
        let mut covered = HashSet::new();
        for pass in doc.passes.iter().filter(|p| p.visible) {
            let color = srgb(&pass.color);
            for p in pass.pieces.iter() {
                let is_selected = selected.contains(&*p.uid);
                let moved =
                    if is_selected { PlacedPiece { x: p.x + drag.0, y: p.y + drag.1, ..p.clone() } } else { p.clone() };
                for cell in brickpress_core::geometry::occupied_cells(&moved) {
                    covered.insert(cell);
                }
                if let Some((i, inst)) = instance(&moved, color, 1.0, if is_selected { FLAG_SELECTED } else { 0 }) {
                    by_piece[i].push(inst);
                }
            }
        }
        let invalid = [0.706, 0.137, 0.094];
        let ghosts = ghosts
            .iter()
            .filter_map(|(p, color)| instance(p, if ghosts_valid { *color } else { invalid }, 0.5, FLAG_GHOST))
            .collect();
        let plate = srgb(PLATE_COLOR);
        let mut studs = Vec::new();
        for y in 0..doc.board.height {
            for x in 0..doc.board.width {
                if !covered.contains(&(x, y)) {
                    studs.push(Instance3D {
                        m: [1.0, 0.0, 0.0, 1.0],
                        t: [x as f32, y as f32],
                        _pad: [0.0; 2],
                        color: [plate[0], plate[1], plate[2], 1.0],
                        flags: 0,
                        _pad2: [0; 3],
                    });
                }
            }
        }
        Self { board: (doc.board.width as f32, doc.board.height as f32), by_piece, ghosts, studs }
    }
}

struct Targets {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    resolve: wgpu::Texture,
    resolve_view: wgpu::TextureView,
    depth: wgpu::TextureView,
    blit_group: wgpu::BindGroup,
}

struct Range {
    first_index: u32,
    count: u32,
    base_vertex: i32,
}

pub struct Renderer3D {
    uniforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    opaque: wgpu::RenderPipeline,
    ghost: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    blit: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    pieces: Vec<Range>,
    stud: Range,
    plate: Option<((f32, f32), wgpu::Buffer, wgpu::Buffer, u32)>,
    instances: Option<(wgpu::Buffer, u64)>,
    draws: Vec<(DrawKind, usize, std::ops::Range<u32>)>,
    targets: Option<Targets>,
}

#[derive(Clone, Copy, PartialEq)]
enum DrawKind {
    Piece,
    Stud,
    Plate,
    Ghost,
}

impl Renderer3D {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        // Merge all meshes into one vertex and index buffer.
        let meshes = mesh::piece_meshes();
        let stud = mesh::stud_mesh();
        let (mut vertices, mut indices) = (Vec::<Vertex>::new(), Vec::<u32>::new());
        let mut add = |m: &Mesh| {
            let range = Range {
                first_index: indices.len() as u32,
                count: m.indices.len() as u32,
                base_vertex: vertices.len() as i32,
            };
            vertices.extend_from_slice(&m.vertices);
            indices.extend_from_slice(&m.indices);
            range
        };
        let pieces: Vec<Range> = meshes.iter().map(&mut add).collect();
        let stud = add(&stud);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3d indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3d shadow map"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&Default::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d shadow sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
            ],
        });
        // The shadow pass cannot sample the map it writes, so it gets a layout
        // with only the uniforms.
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d shadow"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("three.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/three.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d shadow"),
            bind_group_layouts: &[Some(&shadow_layout)],
            immediate_size: 0,
        });
        let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let instance_attributes = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x2, 4 => Float32x4, 5 => Uint32];
        // Instance3D: m (16) t (8) pad (8) color (16) flags (4) pad (12) → offsets need explicit layout.
        let instance_attributes = [
            wgpu::VertexAttribute { offset: 0, ..instance_attributes[0] },
            wgpu::VertexAttribute { offset: 16, ..instance_attributes[1] },
            wgpu::VertexAttribute { offset: 32, ..instance_attributes[2] },
            wgpu::VertexAttribute { offset: 48, ..instance_attributes[3] },
        ];
        let buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attributes,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance3D>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attributes,
            }),
        ];
        let make = |label: &str, blend: Option<wgpu::BlendState>, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &buffers,
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let opaque = make("3d opaque", None, true);
        let ghost = make("3d ghost", Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), false);
        let shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d shadow"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow"),
                buffers: &buffers,
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        let blit_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blit.wgsl").into()),
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3d blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3d blit"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d blit"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_module,
                entry_point: Some("vs_blit"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &blit_module,
                entry_point: Some("fs_blit"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            uniforms,
            group,
            shadow_view,
            opaque,
            ghost,
            shadow,
            blit,
            blit_layout,
            blit_sampler,
            vertices: vertex_buffer,
            indices: index_buffer,
            pieces,
            stud,
            plate: None,
            instances: None,
            draws: Vec::new(),
            targets: None,
        }
    }

    fn ensure_targets(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let extent = wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 };
        let texture = |label: &str, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let msaa = texture("3d msaa", COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT)
            .create_view(&Default::default());
        let resolve = texture(
            "3d resolve",
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        let resolve_view = resolve.create_view(&Default::default());
        let depth = texture("3d depth", DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT)
            .create_view(&Default::default());
        let blit_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d blit"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&resolve_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.blit_sampler) },
            ],
        });
        self.targets = Some(Targets { size, msaa, resolve, resolve_view, depth, blit_group });
    }

    /// Uploads the scene's instances. Call when the document or overlays change.
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene3D) {
        if self.plate.as_ref().is_none_or(|(b, ..)| *b != scene.board) {
            let plate = mesh::plate_mesh(scene.board.0, scene.board.1);
            let v = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d plate"),
                contents: bytemuck::cast_slice(&plate.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let i = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("3d plate"),
                contents: bytemuck::cast_slice(&plate.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            self.plate = Some((scene.board, v, i, plate.indices.len() as u32));
        }
        let mut all: Vec<Instance3D> = Vec::new();
        self.draws.clear();
        let plate_color = srgb(PLATE_COLOR);
        all.push(Instance3D {
            m: [1.0, 0.0, 0.0, 1.0],
            t: [0.0, 0.0],
            _pad: [0.0; 2],
            color: [plate_color[0], plate_color[1], plate_color[2], 1.0],
            flags: 0,
            _pad2: [0; 3],
        });
        self.draws.push((DrawKind::Plate, 0, 0..1));
        let start = all.len() as u32;
        all.extend_from_slice(&scene.studs);
        self.draws.push((DrawKind::Stud, 0, start..all.len() as u32));
        for (i, list) in scene.by_piece.iter().enumerate() {
            if list.is_empty() {
                continue;
            }
            let start = all.len() as u32;
            all.extend_from_slice(list);
            self.draws.push((DrawKind::Piece, i, start..all.len() as u32));
        }
        for (i, inst) in &scene.ghosts {
            let start = all.len() as u32;
            all.push(*inst);
            self.draws.push((DrawKind::Ghost, *i, start..start + 1));
        }
        let bytes = std::mem::size_of_val(all.as_slice()) as u64;
        if self.instances.as_ref().is_none_or(|(_, cap)| *cap < bytes) {
            let capacity = bytes.next_power_of_two().max(4096);
            self.instances = Some((
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("3d instances"),
                    size: capacity,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                capacity,
            ));
        }
        queue.write_buffer(&self.instances.as_ref().expect("allocated").0, 0, bytemuck::cast_slice(&all));
    }

    fn draw_all(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        include_ghosts: bool,
        ghost_pipeline: Option<&wgpu::RenderPipeline>,
    ) {
        let Some((instances, _)) = self.instances.as_ref() else { return };
        pass.set_vertex_buffer(1, instances.slice(..));
        for (kind, piece, range) in &self.draws {
            match kind {
                DrawKind::Plate => {
                    if let Some((_, v, i, count)) = &self.plate {
                        pass.set_vertex_buffer(0, v.slice(..));
                        pass.set_index_buffer(i.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..*count, 0, range.clone());
                    }
                    pass.set_vertex_buffer(0, self.vertices.slice(..));
                    pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
                }
                DrawKind::Stud | DrawKind::Piece => {
                    let r = if *kind == DrawKind::Stud { &self.stud } else { &self.pieces[*piece] };
                    if r.count > 0 && !range.is_empty() {
                        pass.draw_indexed(r.first_index..r.first_index + r.count, r.base_vertex, range.clone());
                    }
                }
                DrawKind::Ghost => {}
            }
        }
        if include_ghosts {
            if let Some(pipeline) = ghost_pipeline {
                pass.set_pipeline(pipeline);
            }
            for (kind, piece, range) in &self.draws {
                if *kind == DrawKind::Ghost {
                    let r = &self.pieces[*piece];
                    pass.draw_indexed(r.first_index..r.first_index + r.count, r.base_vertex, range.clone());
                }
            }
        }
    }

    /// Renders the scene offscreen into `encoder`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        size: [u32; 2],
        camera: &CameraFrame,
        board: (f32, f32),
        accent: [f32; 3],
        shadows: bool,
    ) {
        let size = [size[0].max(1), size[1].max(1)];
        self.ensure_targets(device, size);
        let light_dir = Vec3::new(-12.0, 30.0, 14.0).normalize();
        let radius = board.0.hypot(board.1) / 2.0 + 2.0;
        let light_view = Mat4::look_at_rh(light_dir * (radius + 10.0), Vec3::ZERO, Vec3::Y);
        let light_proj = Mat4::orthographic_rh(-radius, radius, -radius, radius, 0.1, (radius + 10.0) * 2.0);
        let uniforms = Uniforms {
            view_proj: camera.view_projection().to_cols_array_2d(),
            light_view_proj: (light_proj * light_view).to_cols_array_2d(),
            eye: camera.eye.extend(1.0).to_array(),
            light_dir: light_dir.extend(0.0).to_array(),
            board: [board.0, board.1, shadows as u8 as f32, 0.0],
            accent: [accent[0], accent[1], accent[2], 1.0],
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
        if shadows {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3d shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow);
            pass.set_bind_group(0, &self.shadow_group(device), &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            self.draw_all(&mut pass, false, None);
        }
        let targets = self.targets.as_ref().expect("ensured");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.msaa,
                resolve_target: Some(&targets.resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Discard,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.opaque);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_all(&mut pass, true, Some(&self.ghost));
    }

    fn shadow_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let layout = self.shadow.get_bind_group_layout(0);
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3d shadow"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: self.uniforms.as_entire_binding() }],
        })
    }

    /// Draws the last rendered image into the current (UI) pass.
    pub fn paint(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(t) = &self.targets {
            pass.set_pipeline(&self.blit);
            pass.set_bind_group(0, &t.blit_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// The last rendered image (for tests and snapshots).
    pub fn resolved_texture(&self) -> Option<&wgpu::Texture> {
        self.targets.as_ref().map(|t| &t.resolve)
    }
}
