//! A headless device for exports and previews outside the GUI.

use crate::wgpu;

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("No graphics adapter is available: {0}")]
    Adapter(String),
    #[error("The graphics device could not be created: {0}")]
    Device(String),
    #[error("Rendering failed: {0}")]
    Render(String),
}

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_name: String,
}

impl Gpu {
    /// Prefers a hardware adapter and falls back to the platform's software
    /// rasterizer (WARP, llvmpipe), so exports work on machines without a GPU.
    pub fn headless() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }))
            .or_else(|_| {
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    force_fallback_adapter: true,
                    ..Default::default()
                }))
            })
            .map_err(|e| GpuError::Adapter(e.to_string()))?;
        Self::from_adapter(&adapter)
    }

    /// Forces the software adapter, for reproducible golden tests.
    pub fn fallback() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(
            instance
                .request_adapter(&wgpu::RequestAdapterOptions { force_fallback_adapter: true, ..Default::default() }),
        )
        .map_err(|e| GpuError::Adapter(e.to_string()))?;
        Self::from_adapter(&adapter)
    }

    fn from_adapter(adapter: &wgpu::Adapter) -> Result<Self, GpuError> {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("brickpress headless"),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| GpuError::Device(e.to_string()))?;
        Ok(Self { device, queue, adapter_name: adapter.get_info().name })
    }
}
