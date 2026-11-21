//! The 2D board renderer: paper and grid, tracing guide, every piece in one
//! instanced draw, then selection outlines and placement ghosts.
//!
//! [`Renderer2D`] holds pipelines and the shape atlas and is shared;
//! [`Frame2D`] holds per-view buffers (the editor canvas, an export, a
//! thumbnail), so several views can render the same scene independently.

use crate::atlas::{ShapeAtlas, ATLAS_SIZE};
use crate::scene::{BoardParams, Instance, RenderMode};
use crate::wgpu;
use brickpress_core::GridStyle;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

const COMMON: &str = include_str!("shaders/common.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Globals {
    view: [f32; 4],
    frame: [f32; 4],
    board: [f32; 4],
    paper: [f32; 4],
    paper2: [f32; 4],
    print0: [f32; 4],
    print1: [f32; 4],
    print2: [f32; 4],
    grid_color: [f32; 4],
    seeds: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct TraceUniform {
    rect: [f32; 4],
    opacity: [f32; 4],
}

/// Where the board sits in the render target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View2D {
    /// Board origin in target pixels (may be off-target).
    pub origin: [f32; 2],
    /// Target pixels per stud.
    pub stud_px: f32,
    /// Target (viewport) size in pixels.
    pub target: [f32; 2],
}

impl View2D {
    /// Board studs → normalized device coordinates.
    fn transform(&self) -> [f32; 4] {
        let sx = 2.0 * self.stud_px / self.target[0];
        let sy = -2.0 * self.stud_px / self.target[1];
        [sx, sy, 2.0 * self.origin[0] / self.target[0] - 1.0, 1.0 - 2.0 * self.origin[1] / self.target[1]]
    }
}

/// Per-frame display options.
#[derive(Clone, Copy, Debug)]
pub struct FrameOptions {
    pub grid_color: [f32; 4],
    /// Draw the grid (design mode, or exports that ask for it).
    pub grid: bool,
    pub paper: bool,
}

pub struct Renderer2D {
    pub atlas: Arc<ShapeAtlas>,
    format: wgpu::TextureFormat,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    trace_uniform_layout: wgpu::BindGroupLayout,
    atlas_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    board: wgpu::RenderPipeline,
    pieces_over: wgpu::RenderPipeline,
    pieces_multiply: wgpu::RenderPipeline,
    trace: wgpu::RenderPipeline,
}

fn over_blend() -> wgpu::BlendState {
    wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
}

fn multiply_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Dst,
            dst_factor: wgpu::BlendFactor::Zero,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

impl Renderer2D {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        atlas: Arc<ShapeAtlas>,
    ) -> Self {
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brickpress globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brickpress texture"),
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
        let trace_uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brickpress trace uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let atlas_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("brickpress shape atlas"),
                size: wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(&atlas.texels_f16()),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("brickpress linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let atlas_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brickpress atlas"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let module = |name: &str, source: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{source}").into()),
            })
        };
        let board_module = module("board.wgsl", include_str!("shaders/board.wgsl"));
        let pieces_module = module("pieces.wgsl", include_str!("shaders/pieces.wgsl"));
        let trace_module = module("trace.wgsl", include_str!("shaders/trace.wgsl"));

        let layout = |groups: &[&wgpu::BindGroupLayout]| {
            let groups: Vec<Option<&wgpu::BindGroupLayout>> = groups.iter().map(|g| Some(*g)).collect();
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &groups,
                immediate_size: 0,
            })
        };
        let board_layout = layout(&[&globals_layout]);
        let pieces_layout = layout(&[&globals_layout, &texture_layout]);
        let trace_layout = layout(&[&globals_layout, &texture_layout, &trace_uniform_layout]);

        let instance_attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4,
            4 => Float32x4, 5 => Uint32, 6 => Uint32, 7 => Float32
        ];
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &instance_attributes,
        };
        let strip = wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() };
        let pipeline = |label: &str,
                        layout: &wgpu::PipelineLayout,
                        module: &wgpu::ShaderModule,
                        vs: &str,
                        fs: &str,
                        buffers: &[Option<wgpu::VertexBufferLayout>],
                        blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vs),
                    buffers,
                    compilation_options: Default::default(),
                },
                primitive: strip,
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let instanced = [Some(instance_layout)];
        let board = pipeline("board", &board_layout, &board_module, "vs_board", "fs_board", &[], over_blend());
        let pieces_over = pipeline(
            "pieces over",
            &pieces_layout,
            &pieces_module,
            "vs_piece",
            "fs_piece_over",
            &instanced,
            over_blend(),
        );
        let pieces_multiply = pipeline(
            "pieces multiply",
            &pieces_layout,
            &pieces_module,
            "vs_piece",
            "fs_piece_multiply",
            &instanced,
            multiply_blend(),
        );
        let trace = pipeline("trace", &trace_layout, &trace_module, "vs_trace", "fs_trace", &[], over_blend());
        Self {
            atlas,
            format,
            globals_layout,
            texture_layout,
            trace_uniform_layout,
            atlas_group,
            sampler,
            board,
            pieces_over,
            pieces_multiply,
            trace,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn new_frame(&self, device: &wgpu::Device) -> Frame2D {
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brickpress globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brickpress globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        Frame2D {
            globals,
            globals_group,
            pieces: InstanceBuffer::default(),
            overlay: InstanceBuffer::default(),
            pieces_key: None,
            trace: None,
            trace_visible: false,
            printed: false,
            paper: true,
        }
    }

    /// Uploads the tracing image (RGBA8, sRGB-encoded) for a frame.
    pub fn set_trace_image(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &mut Frame2D,
        id: u64,
        size: [u32; 2],
        rgba: &[u8],
    ) {
        if frame.trace.as_ref().is_some_and(|t| t.id == id) {
            return;
        }
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("brickpress tracing guide"),
                size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            rgba,
        );
        let texture_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brickpress trace texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("brickpress trace uniform"),
            contents: bytemuck::bytes_of(&TraceUniform { rect: [0.0; 4], opacity: [0.0; 4] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("brickpress trace uniform"),
            layout: &self.trace_uniform_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
        });
        frame.trace = Some(TraceGpu { id, texture_group, uniform, uniform_group });
    }

    /// Writes uniforms and (when `scene_key` changed) instance data for a frame.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &mut Frame2D,
        view: &View2D,
        params: &BoardParams,
        options: &FrameOptions,
        scene_key: u64,
        pieces: &[Instance],
        overlay: &[Instance],
        trace: Option<([f32; 4], f32)>,
    ) {
        let printed = params.mode == RenderMode::Print;
        let globals = Globals {
            view: view.transform(),
            frame: [view.stud_px, printed as u8 as f32, options.paper as u8 as f32, options.grid as u8 as f32],
            board: [
                params.board[0],
                params.board[1],
                params.major_grid as f32,
                match params.grid {
                    GridStyle::Off => 0.0,
                    GridStyle::Points => 1.0,
                    GridStyle::Lines => 2.0,
                    GridStyle::Squares => 3.0,
                },
            ],
            paper: [params.paper_rgb[0], params.paper_rgb[1], params.paper_rgb[2], params.paper_grain],
            paper2: [params.paper_tooth, params.paper_fibers, params.embossed as u8 as f32, params.gap],
            print0: [params.print[0], params.print[1], params.print[2], params.print[3]],
            print1: [params.print[4], params.print[5], params.print[6], params.print[7]],
            print2: [params.print[8], params.print[9], params.print[10], 0.0],
            grid_color: options.grid_color,
            seeds: [params.paper_seed, 0, 0, 0],
        };
        queue.write_buffer(&frame.globals, 0, bytemuck::bytes_of(&globals));
        if frame.pieces_key != Some(scene_key) {
            frame.pieces.write(device, queue, pieces);
            frame.pieces_key = Some(scene_key);
        }
        frame.overlay.write(device, queue, overlay);
        frame.printed = printed;
        frame.paper = options.paper;
        frame.trace_visible = false;
        if let (Some(gpu), Some((rect, opacity))) = (frame.trace.as_ref(), trace) {
            let uniform = TraceUniform { rect, opacity: [opacity, 0.0, 0.0, 0.0] };
            queue.write_buffer(&gpu.uniform, 0, bytemuck::bytes_of(&uniform));
            frame.trace_visible = true;
        }
    }

    /// Records the draws. The board quad is opaque when paper is on.
    pub fn paint(&self, pass: &mut wgpu::RenderPass<'_>, frame: &Frame2D, draw_board: bool) {
        pass.set_bind_group(0, &frame.globals_group, &[]);
        if draw_board {
            pass.set_pipeline(&self.board);
            pass.draw(0..4, 0..1);
        }
        if let (true, Some(trace)) = (frame.trace_visible, frame.trace.as_ref()) {
            pass.set_pipeline(&self.trace);
            pass.set_bind_group(1, &trace.texture_group, &[]);
            pass.set_bind_group(2, &trace.uniform_group, &[]);
            pass.draw(0..4, 0..1);
        }
        pass.set_bind_group(1, &self.atlas_group, &[]);
        if let Some(buffer) = frame.pieces.buffer.as_ref().filter(|_| frame.pieces.count > 0) {
            let multiply = frame.printed && frame.paper;
            pass.set_pipeline(if multiply { &self.pieces_multiply } else { &self.pieces_over });
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..4, 0..frame.pieces.count);
        }
        if let Some(buffer) = frame.overlay.buffer.as_ref().filter(|_| frame.overlay.count > 0) {
            pass.set_pipeline(&self.pieces_over);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..4, 0..frame.overlay.count);
        }
    }
}

struct TraceGpu {
    id: u64,
    texture_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
}

#[derive(Default)]
struct InstanceBuffer {
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
    count: u32,
}

impl InstanceBuffer {
    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[Instance]) {
        self.count = data.len() as u32;
        if data.is_empty() {
            return;
        }
        let bytes = std::mem::size_of_val(data) as u64;
        if self.capacity < bytes {
            let capacity = bytes.next_power_of_two().max(4096);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("brickpress instances"),
                size: capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity = capacity;
        }
        queue.write_buffer(self.buffer.as_ref().expect("allocated"), 0, bytemuck::cast_slice(data));
    }
}

pub struct Frame2D {
    globals: wgpu::Buffer,
    globals_group: wgpu::BindGroup,
    pieces: InstanceBuffer,
    overlay: InstanceBuffer,
    pieces_key: Option<u64>,
    trace: Option<TraceGpu>,
    trace_visible: bool,
    printed: bool,
    paper: bool,
}

impl Frame2D {
    pub fn invalidate(&mut self) {
        self.pieces_key = None;
    }
    pub fn clear_trace(&mut self) {
        self.trace = None;
    }
}
