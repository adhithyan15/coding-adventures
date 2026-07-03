//! Vulkan backend profile and plan adapter for the Paint VM runtime.
//!
//! This crate still needs a native Vulkan instance/device/render-pass
//! implementation before it can render pixels. The shared GPU-plan adapter is
//! present now so Vulkan converges with WGPU, OpenGL, Mesa, and OpenCL on the
//! same PaintScene lowering contract, including image and gradient texture
//! plans.

use paint_instructions::{PaintScene, PixelContainer};
use paint_vm_gpu_core::{
    plan_scene, unsupported_plan_features, GpuApiFamily, GpuBackendProfile, GpuPaintPlan,
    GpuReadbackStrategy, GpuRenderPath,
};
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

pub fn profile() -> GpuBackendProfile {
    GpuBackendProfile::tier1_textured(
        "paint-vm-vulkan",
        GpuApiFamily::Vulkan,
        GpuRenderPath::GraphicsPipeline,
        "SPIR-V 1.0",
        GpuReadbackStrategy::TextureCopyToBuffer,
    )
}

pub fn renderer() -> VulkanPaintBackend {
    VulkanPaintBackend
}

pub fn plan(scene: &PaintScene) -> Result<GpuPaintPlan, PaintRenderError> {
    let plan = plan_scene(scene);
    reject_unsupported_plan(&plan)?;
    Ok(plan)
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for VulkanPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        let _plan = plan(scene)?;
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-vulkan",
            reason: "Vulkan instance, device, render pass, and readback execution are not implemented yet",
        })
    }
}

fn reject_unsupported_plan(plan: &GpuPaintPlan) -> Result<(), PaintRenderError> {
    let unsupported = unsupported_plan_features(profile(), plan);
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-vulkan",
            message: format!(
                "Vulkan Tier 1 plan adapter does not support: {}",
                unsupported.join(", ")
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{
        GradientKind, GradientStop, ImageSrc, PaintBase, PaintGradient, PaintImage,
        PaintInstruction, PaintRect, PaintText, PixelContainer,
    };
    use paint_vm_gpu_core::GpuTextureKind;
    use paint_vm_runtime::PaintBackendTier;

    #[test]
    fn exposes_scaffold_descriptor_until_render_pass_lands() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-vulkan");
        assert_eq!(descriptor.family, PaintBackendFamily::Vulkan);
        assert_eq!(descriptor.tier, PaintBackendTier::Tier0Scaffold);
    }

    #[test]
    fn exposes_vulkan_gpu_profile() {
        let profile = profile();
        assert_eq!(profile.id, "paint-vm-vulkan");
        assert_eq!(profile.family, GpuApiFamily::Vulkan);
        assert_eq!(profile.render_path, GpuRenderPath::GraphicsPipeline);
        assert_eq!(profile.readback, GpuReadbackStrategy::TextureCopyToBuffer);
        assert!(profile.supports_texture_sampling);
        assert!(profile.supports_linear_gradients);
        assert!(profile.supports_radial_gradients);
    }

    #[test]
    fn plans_solid_rects_with_shared_gpu_core() {
        let mut scene = PaintScene::new(16.0, 16.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                2.0, 2.0, 8.0, 8.0, "#ff0000",
            )));

        let plan = plan(&scene).unwrap();

        assert_eq!((plan.width, plan.height), (16, 16));
        assert_eq!(plan.meshes.len(), 1);
        assert!(unsupported_plan_features(profile(), &plan).is_empty());
    }

    #[test]
    fn plans_gradient_textures_with_shared_gpu_core() {
        let mut scene = PaintScene::new(20.0, 10.0);
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("linear".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Linear {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 10.0,
                    y2: 0.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#ff0000".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#0000ff".to_string(),
                    },
                ],
            }));
        scene
            .instructions
            .push(PaintInstruction::Gradient(PaintGradient {
                base: PaintBase {
                    id: Some("radial".to_string()),
                    metadata: None,
                },
                kind: GradientKind::Radial {
                    cx: 15.0,
                    cy: 5.0,
                    r: 5.0,
                },
                stops: vec![
                    GradientStop {
                        offset: 0.0,
                        color: "#ffffff".to_string(),
                    },
                    GradientStop {
                        offset: 1.0,
                        color: "#000000".to_string(),
                    },
                ],
            }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some("url(#linear)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 10.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: Some("url(#radial)".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));

        let plan = plan(&scene).unwrap();

        assert_eq!(plan.images.len(), 2);
        assert_eq!(plan.images[0].kind, GpuTextureKind::LinearGradient);
        assert_eq!(plan.images[1].kind, GpuTextureKind::RadialGradient);
        assert!(unsupported_plan_features(profile(), &plan).is_empty());
    }

    #[test]
    fn plans_pixel_images_with_shared_gpu_core() {
        let mut pixels = PixelContainer::new(2, 1);
        pixels.set_pixel(0, 0, 255, 0, 0, 255);
        pixels.set_pixel(1, 0, 0, 0, 255, 255);
        let mut scene = PaintScene::new(20.0, 10.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 2.0,
            y: 3.0,
            width: 16.0,
            height: 4.0,
            src: ImageSrc::Pixels(pixels),
            opacity: Some(0.5),
        }));

        let plan = plan(&scene).unwrap();

        assert_eq!(plan.images.len(), 1);
        assert_eq!(plan.images[0].kind, GpuTextureKind::Image);
        assert_eq!(plan.meshes[0].texture_id, Some(0));
        assert_eq!(plan.meshes[0].vertices[0].color.a, 0.5);
        assert!(unsupported_plan_features(profile(), &plan).is_empty());
    }

    #[test]
    fn rejects_text_until_glyph_atlas_lands() {
        let mut scene = PaintScene::new(80.0, 40.0);
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: Default::default(),
            x: 4.0,
            y: 20.0,
            text: "glyphs later".to_string(),
            font_ref: None,
            font_size: 16.0,
            fill: Some("#000000".to_string()),
            text_align: None,
        }));

        let err = plan(&scene).unwrap_err();

        assert!(matches!(err, PaintRenderError::RenderFailed { .. }));
    }
}
