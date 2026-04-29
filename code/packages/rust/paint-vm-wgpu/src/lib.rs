//! WGPU backend scaffold for the Paint VM runtime.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct WgpuPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-wgpu",
        "Paint VM WGPU",
        PaintBackendFamily::Wgpu,
        PaintAcceleration::Gpu,
        PaintPlatformSupport::gpu_portable(),
        50,
    )
}

pub fn renderer() -> WgpuPaintBackend {
    WgpuPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for WgpuPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-wgpu",
            reason: "WGPU texture render pipeline is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-wgpu");
        assert_eq!(descriptor.family, PaintBackendFamily::Wgpu);
    }
}
