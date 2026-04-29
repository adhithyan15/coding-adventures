//! Vulkan backend scaffold for the Paint VM runtime.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_runtime::{
    PaintAcceleration, PaintBackendDescriptor, PaintBackendFamily, PaintPlatformSupport,
    PaintRenderError, PaintRenderer,
};

pub const VERSION: &str = "0.1.0";

pub struct VulkanPaintBackend;

pub fn descriptor() -> PaintBackendDescriptor {
    PaintBackendDescriptor::scaffold(
        "paint-vm-vulkan",
        "Paint VM Vulkan",
        PaintBackendFamily::Vulkan,
        PaintAcceleration::Gpu,
        PaintPlatformSupport::gpu_portable(),
        60,
    )
}

pub fn renderer() -> VulkanPaintBackend {
    VulkanPaintBackend
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for VulkanPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, _scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-vulkan",
            reason: "Vulkan render-pass pipeline is scaffolded but not implemented yet",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_scaffold_descriptor() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-vulkan");
        assert_eq!(descriptor.family, PaintBackendFamily::Vulkan);
    }
}
