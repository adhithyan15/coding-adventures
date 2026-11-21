//! # Conformance Tests for `math3d` (GAME30)
//!
//! Covers:
//! - `Vec3` & `Vec4`: arithmetic, dot, cross, normalize, reflect, perspective_divide
//! - `Mat3` & `Mat4`: column-major layout, TRS, perspective (OpenGL & `[0, 1]`),
//!   orthographic, look_at, mul, determinant, invert (`M * M^-1 == I`)
//! - `Quat`: axis-angle, Euler XYZ/YXZ, conjugate/invert, slerp (including
//!   antipodal shortest-path & near-parallel fallback), nlerp, `rotate_vec3`
//! - `Plane`, `Aabb3`, `Sphere`, `Frustum`: Gribb-Hartmann plane extraction,
//!   point/sphere/AABB frustum culling
//! - `Ray3`: ray-plane and ray-AABB slab intersections (including parallel rays
//!   and rays originating inside the box)
//! - Security & CWE-20/190/369/400 guards: NaN/Inf, zero-length vectors/quats/rays,
//!   singular matrices, invalid projection/frustum bounds, out-of-range angles/coords
//! - 1,000-case deterministic pseudo-random algebraic invariant checks

use math3d::{
    ray_aabb_intersect, ray_plane_intersect, Aabb3, Frustum, Mat3, Mat4, Math3dError, Plane, Quat,
    Ray3, Sphere, Vec3, Vec4, EPSILON, MAX_ANGLE_RAD, MAX_COORD,
};

const TOL: f64 = 1e-9;

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < TOL
}

fn vec3_approx(a: Vec3, b: Vec3) -> bool {
    approx(a.x, b.x) && approx(a.y, b.y) && approx(a.z, b.z)
}

fn mat4_approx(a: &Mat4, b: &Mat4) -> bool {
    for c in 0..4 {
        for r in 0..4 {
            if !approx(a.cols[c][r], b.cols[c][r]) {
                return false;
            }
        }
    }
    true
}

fn mat3_approx(a: &Mat3, b: &Mat3) -> bool {
    for c in 0..3 {
        for r in 0..3 {
            if !approx(a.cols[c][r], b.cols[c][r]) {
                return false;
            }
        }
    }
    true
}

// Deterministic xorshift64* PRNG for property tests (zero external deps)
fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn next_range(state: &mut u64, lo: f64, hi: f64) -> f64 {
    let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
    lo + (hi - lo) * u
}

// --- Vec3 ------------------------------------------------------------------

#[test]
fn vec3_basics_dot_cross_normalize() {
    let x = Vec3::unit_x();
    let y = Vec3::unit_y();
    let z = Vec3::unit_z();
    assert_eq!(x.dot(y), Ok(0.0));
    assert_eq!(x.cross(y), Ok(z));
    assert_eq!(y.cross(x), Ok(Vec3::new(0.0, 0.0, -1.0)));

    let v = Vec3::new(3.0, 4.0, 12.0);
    assert!(approx(v.length().unwrap(), 13.0));
    let n = v.normalize().unwrap();
    assert!(approx(n.length().unwrap(), 1.0));
    assert!(vec3_approx(
        n,
        Vec3::new(3.0 / 13.0, 4.0 / 13.0, 12.0 / 13.0)
    ));

    // Reflection about +Y normal
    let r = Vec3::new(1.0, -1.0, 0.0)
        .reflect(Vec3::new(0.0, 5.0, 0.0))
        .unwrap();
    assert!(vec3_approx(r, Vec3::new(1.0, 1.0, 0.0)));
}

#[test]
fn vec3_guards_nan_inf_zero_overflow() {
    assert_eq!(Vec3::zero().normalize(), Err(Math3dError::ZeroLength));
    assert_eq!(
        Vec3::new(1e-13, 0.0, 0.0).normalize(),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Vec3::new(f64::NAN, 1.0, 2.0).normalize(),
        Err(Math3dError::NonFiniteInput)
    );
    assert_eq!(
        Vec3::new(f64::INFINITY, 0.0, 0.0).dot(Vec3::unit_x()),
        Err(Math3dError::NonFiniteInput)
    );
    assert_eq!(
        Vec3::unit_x().cross(Vec3::new(0.0, f64::NEG_INFINITY, 0.0)),
        Err(Math3dError::NonFiniteInput)
    );
    assert_eq!(
        Vec3::try_vec3(MAX_COORD + 1.0, 0.0, 0.0),
        Err(Math3dError::ValueOutOfBounds)
    );
    assert_eq!(
        Vec3::new(f64::MAX, 0.0, 0.0).vec3_scale(2.0),
        Err(Math3dError::ValueOutOfBounds)
    );
    assert!(Vec3::try_vec3(-MAX_COORD, MAX_COORD, 0.0).is_ok());
}

// --- Vec4 ------------------------------------------------------------------

#[test]
fn vec4_perspective_divide_and_guards() {
    let v = Vec4::from_xyzw(4.0, -6.0, 8.0, 2.0);
    assert!(vec3_approx(
        v.perspective_divide().unwrap(),
        Vec3::new(2.0, -3.0, 4.0)
    ));
    assert_eq!(
        Vec4::from_xyzw(1.0, 2.0, 3.0, 0.0).perspective_divide(),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Vec4::from_xyzw(1.0, 2.0, 3.0, 1e-15).perspective_divide(),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Vec4::from_xyzw(f64::NAN, 0.0, 0.0, 1.0).vec4_normalize(),
        Err(Math3dError::NonFiniteInput)
    );
}

// --- Mat3 & Mat4 -----------------------------------------------------------

#[test]
fn mat3_invert_and_singular_guard() {
    let q = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 1.1).unwrap();
    let r = Mat3::mat3_from_quat(q).unwrap();
    let s = Mat3::mat3_from_scale(2.0, 3.0, 4.0).unwrap();
    let m = r.mat3_mul(&s).unwrap();
    assert!(approx(m.mat3_determinant().unwrap(), 24.0));
    let inv = m.mat3_invert().unwrap();
    assert!(mat3_approx(
        &m.mat3_mul(&inv).unwrap(),
        &Mat3::mat3_identity()
    ));

    let singular =
        Mat3::mat3_from_cols([[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]]).unwrap();
    assert_eq!(singular.mat3_invert(), Err(Math3dError::SingularMatrix));
}

#[test]
fn mat4_trs_and_invert() {
    let t = Vec3::new(10.0, -5.0, 3.0);
    let r = Quat::from_axis_angle(Vec3::unit_y(), trig::PI / 2.0).unwrap();
    let s = Vec3::new(2.0, 3.0, 4.0);
    let m = Mat4::mat4_from_trs(t, r, s).unwrap();

    // Point (1, 0, 0): scale -> (2, 0, 0), rot +90deg Y -> (0, 0, -2), trans -> (10, -5, 1)
    let p = m.transform_point3(Vec3::new(1.0, 0.0, 0.0)).unwrap();
    assert!(vec3_approx(p, Vec3::new(10.0, -5.0, 1.0)));

    let inv = m.invert().unwrap();
    assert!(mat4_approx(&m.mul(&inv).unwrap(), &Mat4::mat4_identity()));
    assert!(mat4_approx(&inv.mul(&m).unwrap(), &Mat4::mat4_identity()));

    let p_back = inv.transform_point3(p).unwrap();
    assert!(vec3_approx(p_back, Vec3::new(1.0, 0.0, 0.0)));
}

#[test]
fn mat4_perspective_orthographic_look_at_and_guards() {
    let proj = Mat4::perspective(trig::PI / 3.0, 16.0 / 9.0, 0.1, 100.0).unwrap();
    let proj_inv = proj.invert().unwrap();
    assert!(mat4_approx(
        &proj.mul(&proj_inv).unwrap(),
        &Mat4::mat4_identity()
    ));

    // Near plane center (0, 0, -0.1) -> NDC z = -1; Far plane (0, 0, -100) -> NDC z = +1
    let ndc_near = proj.transform_point3(Vec3::new(0.0, 0.0, -0.1)).unwrap();
    let ndc_far = proj.transform_point3(Vec3::new(0.0, 0.0, -100.0)).unwrap();
    assert!(approx(ndc_near.z, -1.0));
    assert!(approx(ndc_far.z, 1.0));

    // Zero-to-one perspective
    let proj_zo = Mat4::perspective_zo(trig::PI / 3.0, 16.0 / 9.0, 0.1, 100.0).unwrap();
    let zo_near = proj_zo.transform_point3(Vec3::new(0.0, 0.0, -0.1)).unwrap();
    let zo_far = proj_zo
        .transform_point3(Vec3::new(0.0, 0.0, -100.0))
        .unwrap();
    assert!(approx(zo_near.z, 0.0));
    assert!(approx(zo_far.z, 1.0));

    // Orthographic
    let ortho = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, 1.0, 50.0).unwrap();
    let ortho_inv = ortho.invert().unwrap();
    assert!(mat4_approx(
        &ortho.mul(&ortho_inv).unwrap(),
        &Mat4::mat4_identity()
    ));

    // LookAt: camera at (0, 0, 5) looking at origin -> origin maps to (0, 0, -5)
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let cam_origin = view.transform_point3(Vec3::zero()).unwrap();
    assert!(vec3_approx(cam_origin, Vec3::new(0.0, 0.0, -5.0)));

    // Projection security guards
    assert_eq!(
        Mat4::perspective(0.0, 1.0, 0.1, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(trig::PI, 1.0, 0.1, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, 0.0, 0.1, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, -1.0, 0.1, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, 1.0, 0.0, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, 1.0, -1.0, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, 1.0, 10.0, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(1.0, 1.0, 20.0, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::perspective(f64::NAN, 1.0, 0.1, 10.0),
        Err(Math3dError::NonFiniteInput)
    );

    assert_eq!(
        Mat4::orthographic(5.0, 5.0, 0.0, 1.0, 0.1, 10.0),
        Err(Math3dError::InvalidProjection)
    );
    assert_eq!(
        Mat4::look_at(Vec3::zero(), Vec3::zero(), Vec3::unit_y()),
        Err(Math3dError::DegenerateBasis)
    );
    assert_eq!(
        Mat4::look_at(Vec3::zero(), Vec3::unit_y(), Vec3::unit_y()),
        Err(Math3dError::DegenerateBasis)
    );
}

#[test]
fn mat4_f32_conversion_and_singular() {
    let id = Mat4::mat4_identity();
    let f32_cols = id.mat4_to_f32_cols().unwrap();
    assert_eq!(f32_cols[0], [1.0, 0.0, 0.0, 0.0]);
    let flat = id.mat4_to_f32_flat().unwrap();
    assert_eq!(flat[15], 1.0);

    let zero_mat = Mat4::mat4_from_cols([[0.0; 4]; 4]).unwrap();
    assert_eq!(zero_mat.invert(), Err(Math3dError::SingularMatrix));

    let huge = Mat4::mat4_from_translation(Vec3::new(1e300, 0.0, 0.0)).unwrap();
    assert_eq!(huge.mat4_to_f32_cols(), Err(Math3dError::ValueOutOfBounds));
}

// --- Quat ------------------------------------------------------------------

#[test]
fn quat_rotations_euler_and_slerp() {
    let qz = Quat::from_axis_angle(Vec3::unit_z(), trig::PI / 2.0).unwrap();
    let rotated = qz.rotate_vec3(Vec3::unit_x()).unwrap();
    assert!(vec3_approx(rotated, Vec3::unit_y()));

    // Euler YXZ: yaw 90deg around Y takes +Z to +X (or -X depending on RH sign: Ry(+pi/2)*(0,0,1) = (1,0,0))
    let q_yaw = Quat::from_euler_yxz(trig::PI / 2.0, 0.0, 0.0).unwrap();
    let v_yaw = q_yaw.rotate_vec3(Vec3::unit_z()).unwrap();
    assert!(vec3_approx(v_yaw, Vec3::unit_x()));

    // Euler XYZ matches Mat3
    let q_xyz = Quat::from_euler_xyz(0.3, -0.5, 0.8).unwrap();
    let m_xyz = Mat3::mat3_from_quat(q_xyz).unwrap();
    let v = Vec3::new(1.0, -2.0, 0.5);
    assert!(vec3_approx(
        q_xyz.rotate_vec3(v).unwrap(),
        m_xyz.mat3_mul_vec3(v).unwrap()
    ));

    // SLERP endpoints, midpoint, and antipodal shortest path (-q represents same rotation)
    let a = Quat::quat_identity();
    let b = Quat::from_axis_angle(Vec3::unit_z(), trig::PI / 2.0).unwrap();
    let b_neg = Quat::quat_from_xyzw(-b.x, -b.y, -b.z, -b.w).unwrap();
    let mid = a.slerp(b_neg, 0.5).unwrap();
    let expected_mid = Quat::from_axis_angle(Vec3::unit_z(), trig::PI / 4.0).unwrap();
    assert!(vec3_approx(
        mid.rotate_vec3(Vec3::unit_x()).unwrap(),
        expected_mid.rotate_vec3(Vec3::unit_x()).unwrap()
    ));

    // Nearly identical quats use safe nlerp fallback
    let near_a = Quat::from_axis_angle(Vec3::unit_y(), 1e-8).unwrap();
    let s_near = a.slerp(near_a, 0.5).unwrap();
    assert!(approx(s_near.quat_norm_sq().unwrap(), 1.0));

    // Guards
    assert_eq!(
        Quat::from_axis_angle(Vec3::zero(), 1.0),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Quat::from_axis_angle(Vec3::unit_x(), MAX_ANGLE_RAD + 10.0),
        Err(Math3dError::ValueOutOfBounds)
    );
    assert_eq!(a.slerp(b, -0.1), Err(Math3dError::InvalidParameter));
    assert_eq!(a.slerp(b, 1.1), Err(Math3dError::InvalidParameter));
    assert_eq!(a.nlerp(b, f64::NAN), Err(Math3dError::NonFiniteInput));
}

// --- Frustum & Culling -----------------------------------------------------

#[test]
fn frustum_extraction_and_culling() {
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(trig::PI / 2.0, 1.0, 1.0, 50.0).unwrap();
    let vp = proj.mul(&view).unwrap();
    let frustum = Frustum::from_view_proj(&vp).unwrap();

    // Origin is at z = -10 in camera space, well inside [1, 50]
    assert!(frustum.contains_point(Vec3::zero()).unwrap());
    // Behind camera (z = 12) and beyond far (z = -45 -> distance 55 > 50)
    assert!(!frustum.contains_point(Vec3::new(0.0, 0.0, 12.0)).unwrap());
    assert!(!frustum.contains_point(Vec3::new(0.0, 0.0, -45.0)).unwrap());

    // AABB inside, straddling, and completely outside left/right/far
    let inside =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let straddle =
        Aabb3::aabb3_from_min_max(Vec3::new(-20.0, -1.0, -1.0), Vec3::new(0.0, 1.0, 1.0)).unwrap();
    let outside_far =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -50.0), Vec3::new(1.0, 1.0, -41.0))
            .unwrap();
    let outside_right =
        Aabb3::aabb3_from_min_max(Vec3::new(20.0, -1.0, -1.0), Vec3::new(25.0, 1.0, 1.0)).unwrap();

    assert!(frustum.intersects_aabb(&inside).unwrap());
    assert!(frustum.intersects_aabb(&straddle).unwrap());
    assert!(!frustum.intersects_aabb(&outside_far).unwrap());
    assert!(!frustum.intersects_aabb(&outside_right).unwrap());

    let sp_in = Sphere::sphere_from_center_radius(Vec3::new(0.0, 0.0, -41.0), 2.0).unwrap();
    let sp_out = Sphere::sphere_from_center_radius(Vec3::new(0.0, 0.0, -43.0), 1.0).unwrap();
    assert!(frustum.intersects_sphere(&sp_in).unwrap());
    assert!(!frustum.intersects_sphere(&sp_out).unwrap());

    // ZO frustum matches at origin too
    let vp_zo = Mat4::perspective_zo(trig::PI / 2.0, 1.0, 1.0, 50.0)
        .unwrap()
        .mul(&view)
        .unwrap();
    let frustum_zo = Frustum::from_view_proj_zo(&vp_zo).unwrap();
    assert!(frustum_zo.intersects_aabb(&inside).unwrap());
    assert!(!frustum_zo.intersects_aabb(&outside_far).unwrap());
}

// --- Ray Intersections -----------------------------------------------------

#[test]
fn ray_plane_and_ray_aabb_intersections() {
    let plane = Plane::plane_from_point_normal(Vec3::new(0.0, 5.0, 0.0), Vec3::unit_y()).unwrap();
    let ray_down =
        Ray3::ray3_from_origin_dir(Vec3::new(2.0, 10.0, -3.0), Vec3::new(0.0, -1.0, 0.0)).unwrap();
    let hit = ray_plane_intersect(&ray_down, &plane).unwrap().unwrap();
    assert!(approx(hit.t, 5.0));
    assert!(vec3_approx(hit.point, Vec3::new(2.0, 5.0, -3.0)));
    assert!(vec3_approx(hit.normal, Vec3::unit_y()));

    // Parallel ray -> None (CWE-369 guard)
    let ray_par = Ray3::ray3_from_origin_dir(Vec3::new(0.0, 10.0, 0.0), Vec3::unit_x()).unwrap();
    assert_eq!(ray_plane_intersect(&ray_par, &plane).unwrap(), None);

    // Ray pointing away -> None
    let ray_away = Ray3::ray3_from_origin_dir(Vec3::new(0.0, 10.0, 0.0), Vec3::unit_y()).unwrap();
    assert_eq!(ray_plane_intersect(&ray_away, &plane).unwrap(), None);

    // Ray vs AABB (axis-aligned parallel on Y and Z to verify CWE-369 slab guard)
    let box3 =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let ray_x = Ray3::ray3_from_origin_dir(Vec3::new(-5.0, 0.0, 0.0), Vec3::unit_x()).unwrap();
    let box_hit = ray_aabb_intersect(&ray_x, &box3).unwrap().unwrap();
    assert!(approx(box_hit.t, 4.0));
    assert!(vec3_approx(box_hit.point, Vec3::new(-1.0, 0.0, 0.0)));
    assert!(vec3_approx(box_hit.normal, Vec3::new(-1.0, 0.0, 0.0)));

    // Parallel ray missing box
    let ray_miss = Ray3::ray3_from_origin_dir(Vec3::new(-5.0, 2.0, 0.0), Vec3::unit_x()).unwrap();
    assert_eq!(ray_aabb_intersect(&ray_miss, &box3).unwrap(), None);

    // Ray starting inside box
    let ray_in = Ray3::ray3_from_origin_dir(Vec3::zero(), Vec3::unit_z()).unwrap();
    let hit_in = ray_aabb_intersect(&ray_in, &box3).unwrap().unwrap();
    assert!(approx(hit_in.t, 0.0));
    assert!(vec3_approx(hit_in.point, Vec3::zero()));
}

#[test]
fn bounds_and_error_guards() {
    assert_eq!(
        Aabb3::aabb3_from_min_max(Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
        Err(Math3dError::InvalidBounds)
    );
    assert_eq!(
        Sphere::sphere_from_center_radius(Vec3::zero(), -0.01),
        Err(Math3dError::InvalidBounds)
    );
    assert_eq!(
        Plane::plane_from_normal_d(Vec3::new(0.0, EPSILON * 0.5, 0.0), 1.0),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Ray3::ray3_from_origin_dir(Vec3::zero(), Vec3::zero()),
        Err(Math3dError::ZeroLength)
    );
}

// --- 1,000-case deterministic algebraic property suite --------------------

#[test]
fn deterministic_1000_case_algebraic_properties() {
    let mut rng = 0x1234_5678_9abc_def0_u64;
    for _ in 0..1000 {
        let axis = Vec3::new(
            next_range(&mut rng, -1.0, 1.0),
            next_range(&mut rng, -1.0, 1.0),
            next_range(&mut rng, -1.0, 1.0),
        );
        if axis.length().unwrap() <= 0.05 {
            continue;
        }
        let angle = next_range(&mut rng, -trig::PI, trig::PI);
        let q = Quat::from_axis_angle(axis, angle).unwrap();
        let m3 = Mat3::mat3_from_quat(q).unwrap();

        let v = Vec3::new(
            next_range(&mut rng, -10.0, 10.0),
            next_range(&mut rng, -10.0, 10.0),
            next_range(&mut rng, -10.0, 10.0),
        );
        let rv_q = q.rotate_vec3(v).unwrap();
        let rv_m = m3.mat3_mul_vec3(v).unwrap();
        assert!(vec3_approx(rv_q, rv_m));
        assert!(approx(rv_q.length().unwrap(), v.length().unwrap()));

        let t = Vec3::new(
            next_range(&mut rng, -20.0, 20.0),
            next_range(&mut rng, -20.0, 20.0),
            next_range(&mut rng, -20.0, 20.0),
        );
        let s = Vec3::new(
            next_range(&mut rng, 0.5, 3.0),
            next_range(&mut rng, 0.5, 3.0),
            next_range(&mut rng, 0.5, 3.0),
        );
        let trs = Mat4::mat4_from_trs(t, q, s).unwrap();
        let trs_inv = trs.invert().unwrap();
        let prod = trs.mul(&trs_inv).unwrap();
        assert!(mat4_approx(&prod, &Mat4::mat4_identity()));
    }
}
