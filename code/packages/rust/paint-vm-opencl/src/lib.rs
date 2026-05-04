//! OpenCL compute backend profile and plan adapter for the Paint VM runtime.
//!
//! OpenCL is treated as a compute raster path rather than a native vector API.

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

pub fn profile() -> GpuBackendProfile {
    GpuBackendProfile::tier1_solid(
        "paint-vm-opencl",
        GpuApiFamily::OpenCl,
        GpuRenderPath::ComputeRaster,
        "OpenCL C 1.2 kernels",
        GpuReadbackStrategy::StorageBufferReadback,
    )
}

pub fn renderer() -> OpenClPaintBackend {
    OpenClPaintBackend
}

pub fn plan(scene: &PaintScene) -> Result<GpuPaintPlan, PaintRenderError> {
    let plan = plan_scene(scene);
    reject_unsupported_plan(&plan)?;
    Ok(plan)
}

pub fn render(scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
    renderer().render(scene)
}

impl PaintRenderer for OpenClPaintBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        descriptor()
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        let _plan = plan(scene)?;
        Err(PaintRenderError::BackendUnavailable {
            backend: "paint-vm-opencl",
            reason:
                "OpenCL context, kernels, buffers, and readback execution are not implemented yet",
        })
    }
}

fn reject_unsupported_plan(plan: &GpuPaintPlan) -> Result<(), PaintRenderError> {
    let unsupported = unsupported_plan_features(profile(), plan);
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(PaintRenderError::RenderFailed {
            backend: "paint-vm-opencl",
            message: format!(
                "OpenCL Tier 1 plan adapter does not support: {}",
                unsupported.join(", ")
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paint_instructions::{PaintInstruction, PaintRect, PaintText};
    use paint_vm_runtime::PaintBackendTier;

    #[test]
    fn exposes_scaffold_descriptor_until_compute_raster_lands() {
        let descriptor = descriptor();
        assert_eq!(descriptor.id, "paint-vm-opencl");
        assert_eq!(descriptor.family, PaintBackendFamily::OpenCl);
        assert_eq!(descriptor.tier, PaintBackendTier::Tier0Scaffold);
    }

    #[test]
    fn exposes_opencl_gpu_profile() {
        let profile = profile();
        assert_eq!(profile.id, "paint-vm-opencl");
        assert_eq!(profile.family, GpuApiFamily::OpenCl);
        assert_eq!(profile.render_path, GpuRenderPath::ComputeRaster);
        assert_eq!(profile.readback, GpuReadbackStrategy::StorageBufferReadback);
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
