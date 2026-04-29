//! Cairo backend scaffold for the Paint VM runtime.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct CairoPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-cairo",
        "Paint VM Cairo",
        PaintBackendFamily::Cairo,
        PaintAcceleration::Cpu,
        PaintPlatformSupport::all_desktop(),
        40,
    )
}

pub fn renderer() -> CairoPaintBackend {
    CairoPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for CairoPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-cairo",
            reason: "Cairo drawing pipeline is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-cairo");
        assert_eq!(descriptor.family, PaintBackendFamily::Cairo);
    }
}
