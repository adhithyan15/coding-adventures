//! OpenCL compute backend scaffold for the Paint VM runtime.
//!
//! OpenCL is treated as a compute raster path rather than a native vector API.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct OpenClPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-opencl",
        "Paint VM OpenCL",
        PaintBackendFamily::OpenCl,
        PaintAcceleration::Compute,
        PaintPlatformSupport::gpu_portable(),
        90,
    )
}

pub fn renderer() -> OpenClPaintBackend {
    OpenClPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for OpenClPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-opencl",
            reason: "OpenCL compute raster pipeline is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-opencl");
        assert_eq!(descriptor.family, PaintBackendFamily::OpenCl);
    }
}
