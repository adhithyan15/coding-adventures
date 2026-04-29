//! OpenGL backend scaffold for the Paint VM runtime.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct OpenGlPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-opengl",
        "Paint VM OpenGL",
        PaintBackendFamily::OpenGl,
        PaintAcceleration::Gpu,
        PaintPlatformSupport::all_desktop(),
        80,
    )
}

pub fn renderer() -> OpenGlPaintBackend {
    OpenGlPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for OpenGlPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-opengl",
            reason: "OpenGL framebuffer pipeline is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-opengl");
        assert_eq!(descriptor.family, PaintBackendFamily::OpenGl);
    }
}
