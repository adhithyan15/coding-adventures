//! # VER08b runtime capability conformance for `math3d`.
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
use math3d::{
    ray_aabb_intersect, ray_plane_intersect, Aabb3, Frustum, Mat3, Mat4, Plane, Quat, Ray3, Sphere,
    Vec3, Vec4,
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

        let v1 = Vec3::new(3.0, 4.0, 0.0);
        let v2 = Vec3::unit_z();
        let norm = v1.normalize();
        let cross = v1.cross(v2);
        checks.push(check(
            "vec3 normalize and cross",
            norm.is_ok() && cross.is_ok(),
        ));

        let v4 = Vec4::from_xyzw(2.0, 4.0, 6.0, 2.0);
        checks.push(check(
            "vec4 perspective_divide",
            v4.perspective_divide().is_ok(),
        ));

        let q1 = Quat::from_axis_angle(Vec3::unit_y(), 1.0);
        let q2 = Quat::from_euler_yxz(0.5, 0.2, 0.1);
        let slerp_ok = match (q1, q2) {
            (Ok(a), Ok(b)) => a.slerp(b, 0.5).is_ok() && a.rotate_vec3(v1).is_ok(),
            _ => false,
        };
        checks.push(check("quat slerp and rotate_vec3", slerp_ok));

        let m3 = q1
            .and_then(Mat3::mat3_from_quat)
            .and_then(|m| m.mat3_invert());
        checks.push(check("mat3 from_quat and invert", m3.is_ok()));

        let proj = Mat4::perspective(1.0, 1.5, 0.1, 100.0);
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::zero(), Vec3::unit_y());
        let vp_res = match (proj, view) {
            (Ok(p), Ok(v)) => p.mul(&v).and_then(|vp| vp.invert().map(|_| vp)),
            _ => Err(math3d::Math3dError::InvalidProjection),
        };
        checks.push(check(
            "mat4 perspective, look_at, mul, invert",
            vp_res.is_ok(),
        ));

        let aabb = Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let sphere = Sphere::sphere_from_center_radius(Vec3::zero(), 1.0);
        let frustum_ok = match (vp_res, aabb, sphere) {
            (Ok(vp), Ok(b), Ok(s)) => Frustum::from_view_proj(&vp)
                .map(|f| f.intersects_aabb(&b).is_ok() && f.intersects_sphere(&s).is_ok())
                .unwrap_or(false),
            _ => false,
        };
        checks.push(check("frustum extraction and culling", frustum_ok));

        let ray = Ray3::ray3_from_origin_dir(Vec3::new(-5.0, 0.0, 0.0), Vec3::unit_x());
        let plane = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_x());
        let ray_ok = match (ray, plane, aabb) {
            (Ok(r), Ok(p), Ok(b)) => {
                ray_plane_intersect(&r, &p).is_ok() && ray_aabb_intersect(&r, &b).is_ok()
            }
            _ => false,
        };
        checks.push(check("ray_plane and ray_aabb intersections", ray_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/math3d",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
