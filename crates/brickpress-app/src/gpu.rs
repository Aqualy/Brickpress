//! Paint callbacks that draw the board with `brickpress-render` inside egui.

use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::board2d::{Frame2D, FrameOptions, Renderer2D, View2D};
use brickpress_render::scene::{BoardParams, Instance};
use brickpress_render::three::camera::CameraFrame;
use brickpress_render::three::{Renderer3D, Scene3D};
use brickpress_render::wgpu;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use std::sync::Arc;

/// GPU state kept in egui-wgpu's callback resources.
pub struct GpuResources {
    pub r2d: Renderer2D,
    pub frame: Frame2D,
    pub r3d: Option<Renderer3D>,
    pub scene3d_key: Option<u64>,
}

pub fn install(render_state: &egui_wgpu::RenderState, atlas: Arc<ShapeAtlas>) {
    let r2d = Renderer2D::new(&render_state.device, &render_state.queue, render_state.target_format, atlas);
    let frame = r2d.new_frame(&render_state.device);
    render_state.renderer.write().callback_resources.insert(GpuResources { r2d, frame, r3d: None, scene3d_key: None });
}

#[derive(Clone)]
pub struct TraceUpload {
    pub id: u64,
    pub size: [u32; 2],
    pub rgba: Arc<Vec<u8>>,
}

pub struct Board2DCallback {
    pub view: View2D,
    pub params: BoardParams,
    pub options: FrameOptions,
    pub scene_key: u64,
    pub pieces: Arc<Vec<Instance>>,
    pub overlay: Vec<Instance>,
    pub trace: Option<(TraceUpload, [f32; 4], f32)>,
}

impl CallbackTrait for Board2DCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(gpu) = resources.get_mut::<GpuResources>() else { return vec![] };
        let GpuResources { r2d, frame, .. } = gpu;
        match &self.trace {
            Some((upload, ..)) => r2d.set_trace_image(device, queue, frame, upload.id, upload.size, &upload.rgba),
            None => frame.clear_trace(),
        }
        let trace = self.trace.as_ref().map(|(_, rect, opacity)| (*rect, *opacity));
        r2d.prepare(
            device,
            queue,
            frame,
            &self.view,
            &self.params,
            &self.options,
            self.scene_key,
            &self.pieces,
            &self.overlay,
            trace,
        );
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(gpu) = resources.get::<GpuResources>() {
            gpu.r2d.paint(pass, &gpu.frame, true);
        }
    }
}

pub struct Board3DCallback {
    pub size: [u32; 2],
    pub camera: CameraFrame,
    pub board: (f32, f32),
    pub accent: [f32; 3],
    pub scene_key: u64,
    /// Uploaded by `prepare` only when `scene_key` changed.
    pub scene: Arc<Scene3D>,
}

impl CallbackTrait for Board3DCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(gpu) = resources.get_mut::<GpuResources>() else { return vec![] };
        let format = gpu.r2d.format();
        let r3d = gpu.r3d.get_or_insert_with(|| Renderer3D::new(device, format));
        if gpu.scene3d_key != Some(self.scene_key) {
            r3d.upload(device, queue, &self.scene);
            gpu.scene3d_key = Some(self.scene_key);
        }
        r3d.render(device, queue, encoder, self.size, &self.camera, self.board, self.accent, true);
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(r3d) = resources.get::<GpuResources>().and_then(|g| g.r3d.as_ref()) {
            r3d.paint(pass);
        }
    }
}
