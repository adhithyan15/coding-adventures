//! # VER08b runtime capability conformance for `sim-crt-vector-beam`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use sim_crt_vector_beam::{
    accumulate_intensity, clear_phosphor_buffer, decay_phosphor_buffer, decay_phosphor_sample,
    validate_raster_timing, validate_slew_rate, CrtRasterBeam, PhosphorType, PixelContainer,
    RasterTiming, VectorBeamDisplay, VectorStroke,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        // 1. Timing and validators
        let ntsc = RasterTiming::standard_ntsc();
        checks.push(check(
            "validate_raster_timing succeeds",
            validate_raster_timing(ntsc).is_ok(),
        ));
        checks.push(check(
            "validate_slew_rate succeeds",
            validate_slew_rate(50.0).is_ok(),
        ));

        // 2. Phosphor decay utilities
        let mut phosphor = [30000u16; 64];
        let decay_res = decay_phosphor_buffer(&mut phosphor, 32768);
        checks.push(check("decay_phosphor_buffer succeeds", decay_res.is_ok()));
        checks.push(check(
            "decay_phosphor_sample matches",
            decay_phosphor_sample(1000, 32768) == 500,
        ));
        checks.push(check(
            "accumulate_intensity matches",
            accumulate_intensity(10, 20) == 30,
        ));
        clear_phosphor_buffer(&mut phosphor);
        checks.push(check("clear_phosphor_buffer clears", phosphor[0] == 0));

        // 3. Raster beam deflection
        let timing = RasterTiming {
            visible_width: 2,
            visible_height: 2,
            h_front_porch: 1,
            h_sync: 1,
            h_back_porch: 1,
            v_front_porch: 1,
            v_sync: 1,
            v_back_porch: 1,
        };
        let crt_res = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P4White);
        checks.push(check("new_raster_beam succeeds", crt_res.is_ok()));

        if let Ok(mut crt) = crt_res {
            let mut raster_phosphor = [0u16; 4];
            let step_res = crt.step_raster_beam(255, &mut raster_phosphor);
            checks.push(check("step_raster_beam succeeds", step_res.is_ok()));

            let mut pixels = PixelContainer::new(2, 2);
            let lower_pixel_res = crt.lower_raster_to_pixel_buffer(&raster_phosphor, &mut pixels);
            checks.push(check(
                "lower_raster_to_pixel_buffer succeeds",
                lower_pixel_res.is_ok(),
            ));

            let lower_scene_res = crt.lower_raster_to_paint_scene(&raster_phosphor, 100);
            checks.push(check(
                "lower_raster_to_paint_scene succeeds",
                lower_scene_res.is_ok(),
            ));
        }

        // 4. Vector beam display
        let disp_res =
            VectorBeamDisplay::new_vector_display(100.0, 100.0, 20.0, PhosphorType::P31Green);
        checks.push(check("new_vector_display succeeds", disp_res.is_ok()));

        if let Ok(mut disp) = disp_res {
            let vstep_res = disp.step_vector_beam(50.0, 50.0, false, 200);
            checks.push(check("step_vector_beam succeeds", vstep_res.is_ok()));

            let mut vphosphor = [0u16; 100];
            let stroke = VectorStroke::new_vector_stroke(0.0, 0.0, 9.0, 9.0, 150);
            let rast_res = disp.rasterize_vector_stroke_into(&stroke, &mut vphosphor, 10, 10);
            checks.push(check(
                "rasterize_vector_stroke_into succeeds",
                rast_res.is_ok(),
            ));

            let vstrokes = [stroke];
            let vscene_res = VectorBeamDisplay::lower_vector_strokes_to_paint_scene(
                &vstrokes,
                100.0,
                100.0,
                PhosphorType::P31Green,
            );
            checks.push(check(
                "lower_vector_strokes_to_paint_scene succeeds",
                vscene_res.is_ok(),
            ));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sim-crt-vector-beam",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
