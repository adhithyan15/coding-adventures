//! # VER08b runtime capability conformance for `collision2d`.
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
use collision2d::{
    aabb_vs_aabb, circle_vs_aabb, circle_vs_circle, fixed_aabb_vs_aabb, fixed_circle_vs_aabb,
    obb_vs_obb_sat, raycast_aabb, raycast_circle, reflect_velocity, sweep_aabb_aabb,
    sweep_circle_aabb, Circle, FixedCircleI64, FixedRectI64, Obb2D, Point, Ray2D, Rect,
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

        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        let aabb_res = aabb_vs_aabb(a, b);
        checks.push(check("aabb_vs_aabb executes", aabb_res.is_ok()));

        let c1 = Circle::new(Point::new(0.0, 0.0), 5.0).unwrap();
        let c2 = Circle::new(Point::new(6.0, 0.0), 5.0).unwrap();
        let cvc_res = circle_vs_circle(c1, c2);
        checks.push(check("circle_vs_circle executes", cvc_res.is_ok()));

        let cva_res = circle_vs_aabb(c1, a);
        checks.push(check("circle_vs_aabb executes", cva_res.is_ok()));

        let obb1 = Obb2D::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0), 0.5).unwrap();
        let obb2 = Obb2D::new(Point::new(1.0, 1.0), Point::new(2.0, 2.0), 0.2).unwrap();
        let obb_res = obb_vs_obb_sat(obb1, obb2);
        checks.push(check("obb_vs_obb_sat executes", obb_res.is_ok()));

        let ray = Ray2D::new(Point::new(-10.0, 5.0), Point::new(20.0, 0.0), 1.0).unwrap();
        let ray_box = raycast_aabb(ray, a);
        checks.push(check("raycast_aabb executes", ray_box.is_ok()));

        let ray_circ = raycast_circle(ray, c1);
        checks.push(check("raycast_circle executes", ray_circ.is_ok()));

        let sweep_box = sweep_aabb_aabb(a, Point::new(20.0, 0.0), b);
        checks.push(check("sweep_aabb_aabb executes", sweep_box.is_ok()));

        let sweep_circ = sweep_circle_aabb(c1, Point::new(20.0, 0.0), a);
        checks.push(check("sweep_circle_aabb executes", sweep_circ.is_ok()));

        let refl = reflect_velocity(Point::new(10.0, -10.0), Point::new(0.0, 1.0), 0.8);
        checks.push(check("reflect_velocity executes", refl.is_ok()));

        let fr1 = FixedRectI64::new(0, 0, 100, 100).unwrap();
        let fr2 = FixedRectI64::new(50, 50, 100, 100).unwrap();
        let faabb = fixed_aabb_vs_aabb(fr1, fr2);
        checks.push(check("fixed_aabb_vs_aabb executes", faabb.is_ok()));

        let fc = FixedCircleI64::new(0, 0, 50).unwrap();
        let fcaabb = fixed_circle_vs_aabb(fc, fr1);
        checks.push(check("fixed_circle_vs_aabb executes", fcaabb.is_ok()));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/collision2d",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
