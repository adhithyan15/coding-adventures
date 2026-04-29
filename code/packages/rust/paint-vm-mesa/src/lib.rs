//! Mesa backend profile scaffold for the Paint VM runtime.
//!
//! Mesa is not a single drawing API; it supplies software and driver-backed
//! implementations for OpenGL and Vulkan. This crate gives the runtime a
//! first-class place to model Mesa profiles such as llvmpipe and lavapipe.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct MesaPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-mesa",
        "Paint VM Mesa",
        PaintBackendFamily::Mesa,
        PaintAcceleration::Software,
        PaintPlatformSupport::all_desktop(),
        85,
    )
}

pub fn renderer() -> MesaPaintBackend {
    MesaPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for MesaPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-mesa",
            reason: "Mesa profile routing is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-mesa");
        assert_eq!(descriptor.family, PaintBackendFamily::Mesa);
    }
}
