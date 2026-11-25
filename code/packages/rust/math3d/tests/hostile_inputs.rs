//! Hostile-input security regression and fuzz tests for `math3d` (`GAME30`).
//!
//! Covers:
//! - `NaN`, non-canonical `NaN` payloads, `±Inf`, `-0.0`, subnormals, and `±f64::MAX`
//! - Hand-constructed degenerate `Quat`, `Plane`, `Aabb3`, `Sphere`, `Ray3`, and `Frustum`
//! - Finite `Mat4` row addition/subtraction overflow (`1e308` -> `ValueOutOfBounds`)
//! - Unnormalized hand-constructed `Frustum` plane normalization in `intersects_aabb`
//! - Near-singular `Mat3` / `Mat4` inversion guards
//! - Antipodal `Quat::slerp` / `Quat::nlerp` shortest-path invariance
//! - Seeded 2,000-case hostile fuzz campaign across all public entry points

#![forbid(unsafe_code)]

use math3d::{
    ray_aabb_intersect, ray_plane_intersect, Aabb3, Frustum, Mat3, Mat4, Math3dError, Plane, Quat,
    Ray3, Sphere, Vec3, Vec4, EPSILON, MAX_ANGLE_RAD, MAX_COORD,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_hostile_f64(&mut self) -> f64 {
        let bucket = self.next_u64() % 16;
        match bucket {
            0 => f64::NAN,
            1 => f64::from_bits(0x7ff8_0000_0000_0001),
            2 => f64::from_bits(0xfff8_1234_5678_9abc),
            3 => f64::INFINITY,
            4 => f64::NEG_INFINITY,
            5 => -0.0,
            6 => 0.0,
            7 => f64::from_bits(1),
            8 => f64::MIN_POSITIVE * 0.5,
            9 => f64::MAX,
            10 => -f64::MAX,
            11 => MAX_COORD + 1.0,
            12 => -(MAX_COORD + 1.0),
            13 => EPSILON * 0.5,
            14 => {
                let u = ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64);
                (u * 2.0 - 1.0) * 100.0
            }
            _ => f64::from_bits(self.next_u64()),
        }
    }

    fn next_vec3(&mut self) -> Vec3 {
        Vec3::new(
            self.next_hostile_f64(),
            self.next_hostile_f64(),
            self.next_hostile_f64(),
        )
    }

    fn next_vec4(&mut self) -> Vec4 {
        Vec4::from_xyzw(
            self.next_hostile_f64(),
            self.next_hostile_f64(),
            self.next_hostile_f64(),
            self.next_hostile_f64(),
        )
    }
}

#[test]
fn hostile_scalars_nan_subnormals_extremes() {
    let non_canonical_nan = f64::from_bits(0x7ff8_0000_0000_0001);
    let neg_nan = f64::from_bits(0xfff8_0000_0000_0042);
    let tiny_subnormal = f64::from_bits(1);
    let half_min_pos = f64::MIN_POSITIVE * 0.5;

    let hostile_scalars = [
        f64::NAN,
        non_canonical_nan,
        neg_nan,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        tiny_subnormal,
        -tiny_subnormal,
        half_min_pos,
        f64::MAX,
        -f64::MAX,
        MAX_COORD + 1.0,
        MAX_ANGLE_RAD + 1.0,
    ];

    for &s in &hostile_scalars {
        let v = Vec3::new(s, 1.0, 2.0);
        let _ = Vec3::try_vec3(s, 0.0, 0.0);
        let _ = v.normalize();
        let _ = v.length();
        let _ = v.vec3_scale(s);
        let _ = v.dot(Vec3::unit_x());
        let _ = v.cross(Vec3::unit_y());
        let _ = Vec3::unit_x().vec3_lerp(Vec3::unit_y(), s);

        let v4 = Vec4::from_xyzw(1.0, 2.0, 3.0, s);
        let _ = Vec4::try_vec4(s, 0.0, 0.0, 1.0);
        let _ = v4.perspective_divide();
        let _ = v4.vec4_normalize();

        let _ = Quat::from_axis_angle(Vec3::unit_y(), s);
        let _ = Quat::from_euler_xyz(s, 0.0, 0.0);
        let _ = Quat::from_euler_yxz(0.0, s, 0.0);
        let _ = Mat3::mat3_from_scale(s, 1.0, 1.0);
        let _ = Mat4::perspective(s, 1.0, 0.1, 100.0);
        let _ = Mat4::perspective_zo(s, 1.0, 0.1, 100.0);
        let _ = Mat4::orthographic(s, 1.0, -1.0, 1.0, 0.1, 100.0);
    }

    assert_eq!(
        Vec3::try_vec3(non_canonical_nan, 0.0, 0.0),
        Err(Math3dError::NonFiniteInput)
    );
    assert_eq!(
        Vec3::try_vec3(-0.0, 0.0, 0.0),
        Ok(Vec3::new(-0.0, 0.0, 0.0))
    );
    assert_eq!(
        Vec3::new(tiny_subnormal, 0.0, 0.0).normalize(),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Vec3::new(half_min_pos, 0.0, 0.0).normalize(),
        Err(Math3dError::ZeroLength)
    );
    assert_eq!(
        Vec3::new(f64::MAX, f64::MAX, 0.0).length(),
        Err(Math3dError::ValueOutOfBounds)
    );
}

#[test]
fn hostile_hand_constructed_degenerate_primitives() {
    let bad_quat = Quat {
        x: f64::NAN,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
    assert_eq!(bad_quat.quat_normalize(), Err(Math3dError::NonFiniteInput));
    assert_eq!(
        bad_quat.rotate_vec3(Vec3::unit_x()),
        Err(Math3dError::NonFiniteInput)
    );
    assert_eq!(
        Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0
        }
        .quat_invert(),
        Err(Math3dError::ZeroLength)
    );

    let bad_plane_nan = Plane {
        normal: Vec3::new(f64::NAN, 1.0, 0.0),
        d: 0.0,
    };
    assert_eq!(
        bad_plane_nan.signed_distance(Vec3::zero()),
        Err(Math3dError::NonFiniteInput)
    );

    let bad_plane_zero = Plane {
        normal: Vec3::zero(),
        d: 1.0,
    };
    assert_eq!(
        bad_plane_zero.signed_distance(Vec3::zero()),
        Err(Math3dError::ZeroLength)
    );

    let bad_aabb = Aabb3 {
        min: Vec3::new(5.0, 0.0, 0.0),
        max: Vec3::new(1.0, 1.0, 1.0),
    };
    assert_eq!(bad_aabb.aabb3_center(), Err(Math3dError::InvalidBounds));
    assert_eq!(
        bad_aabb.aabb3_contains_point(Vec3::zero()),
        Err(Math3dError::InvalidBounds)
    );

    let bad_sphere = Sphere {
        center: Vec3::zero(),
        radius: -1.0,
    };
    let valid_frustum = Frustum::from_view_proj(&Mat4::mat4_identity()).unwrap();
    assert_eq!(
        valid_frustum.intersects_sphere(&bad_sphere),
        Err(Math3dError::InvalidBounds)
    );

    let bad_ray = Ray3 {
        origin: Vec3::zero(),
        dir: Vec3::new(f64::INFINITY, 0.0, 0.0),
    };
    let unit_box =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)).unwrap();
    assert_eq!(
        ray_aabb_intersect(&bad_ray, &unit_box),
        Err(Math3dError::NonFiniteInput)
    );
    let unit_plane = Plane::plane_from_normal_d(Vec3::unit_y(), 0.0).unwrap();
    assert_eq!(
        ray_plane_intersect(&bad_ray, &unit_plane),
        Err(Math3dError::NonFiniteInput)
    );

    // Unnormalized hand-constructed frustum planes must be normalized in `intersects_aabb`
    // just like in `contains_point` and `intersects_sphere`, and reject degenerate planes.
    let unnorm_plane = Plane {
        normal: Vec3::new(5.0, 0.0, 0.0),
        d: 5.0,
    };
    let unnorm_frustum = Frustum {
        planes: [unnorm_plane; 6],
    };
    assert_eq!(unnorm_frustum.intersects_aabb(&unit_box), Ok(true));
    assert_eq!(
        unnorm_frustum.contains_point(Vec3::new(-2.0, 0.0, 0.0)),
        Ok(false)
    );
    assert_eq!(
        unnorm_frustum.intersects_aabb(
            &Aabb3::aabb3_from_min_max(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(-2.0, 1.0, 1.0))
                .unwrap()
        ),
        Ok(false)
    );

    let deg_frustum = Frustum {
        planes: [Plane {
            normal: Vec3::zero(),
            d: 0.0,
        }; 6],
    };
    assert_eq!(
        deg_frustum.intersects_aabb(&unit_box),
        Err(Math3dError::ZeroLength)
    );
}

#[test]
fn hostile_mat4_plane_row_overflow_and_near_singular_matrices() {
    let huge_mat = Mat4::mat4_from_cols([
        [1e308, 0.0, 0.0, 1e308],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
    .unwrap();
    assert_eq!(
        Frustum::from_view_proj(&huge_mat),
        Err(Math3dError::ValueOutOfBounds)
    );
    assert_eq!(
        Frustum::from_view_proj_zo(&huge_mat),
        Err(Math3dError::ValueOutOfBounds)
    );

    let near_singular_m3 =
        Mat3::mat3_from_cols([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, EPSILON * 0.1]])
            .unwrap();
    assert_eq!(
        near_singular_m3.invert_or_err(),
        Err(Math3dError::SingularMatrix)
    );

    let near_singular_m4 = Mat4::mat4_from_cols([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, EPSILON * 0.1],
    ])
    .unwrap();
    assert_eq!(near_singular_m4.invert(), Err(Math3dError::SingularMatrix));
}

trait Mat3InvertHelper {
    fn invert_or_err(self) -> Result<Mat3, Math3dError>;
}

impl Mat3InvertHelper for Mat3 {
    fn invert_or_err(self) -> Result<Mat3, Math3dError> {
        self.mat3_invert()
    }
}

#[test]
fn hostile_antipodal_quat_slerp_and_nlerp() {
    let q = Quat::quat_identity();
    let neg_q = Quat::quat_from_xyzw(0.0, 0.0, 0.0, -1.0).unwrap();

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let s = q.slerp(neg_q, t).unwrap();
        assert!((s.quat_norm_sq().unwrap() - 1.0).abs() < 1e-12);
        let n = q.nlerp(neg_q, t).unwrap();
        assert!((n.quat_norm_sq().unwrap() - 1.0).abs() < 1e-12);
    }

    assert_eq!(q.slerp(neg_q, -0.01), Err(Math3dError::InvalidParameter));
    assert_eq!(q.slerp(neg_q, 1.01), Err(Math3dError::InvalidParameter));
    assert_eq!(q.slerp(neg_q, f64::NAN), Err(Math3dError::NonFiniteInput));
    assert_eq!(q.nlerp(neg_q, -0.01), Err(Math3dError::InvalidParameter));
    assert_eq!(q.nlerp(neg_q, 1.01), Err(Math3dError::InvalidParameter));
    assert_eq!(q.nlerp(neg_q, f64::NAN), Err(Math3dError::NonFiniteInput));
}

#[test]
fn hostile_fuzz_2000_cases_zero_panics() {
    let mut rng = SplitMix64(0xDEAD_BEEF_CAFE_3030);
    for _ in 0..2_000 {
        let v0 = rng.next_vec3();
        let v1 = rng.next_vec3();
        let s = rng.next_hostile_f64();

        let _ = v0.vec3_add(v1);
        let _ = v0.vec3_sub(v1);
        let _ = v0.vec3_scale(s);
        let _ = v0.vec3_neg();
        let _ = v0.dot(v1);
        let _ = v0.cross(v1);
        let _ = v0.normalize();
        let _ = v0.distance(v1);
        let _ = v0.vec3_lerp(v1, s);
        let _ = v0.reflect(v1);

        let v4 = rng.next_vec4();
        let _ = v4.vec4_normalize();
        let _ = v4.perspective_divide();

        let q0 = Quat {
            x: rng.next_hostile_f64(),
            y: rng.next_hostile_f64(),
            z: rng.next_hostile_f64(),
            w: rng.next_hostile_f64(),
        };
        let q1 = Quat {
            x: rng.next_hostile_f64(),
            y: rng.next_hostile_f64(),
            z: rng.next_hostile_f64(),
            w: rng.next_hostile_f64(),
        };
        let _ = q0.quat_normalize();
        let _ = q0.quat_invert();
        let _ = q0.quat_mul(q1);
        let _ = q0.slerp(q1, s);
        let _ = q0.nlerp(q1, s);
        let _ = q0.rotate_vec3(v0);

        let m3 = Mat3 {
            cols: [
                [
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                ],
                [
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                ],
                [
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                    rng.next_hostile_f64(),
                ],
            ],
        };
        let _ = m3.mat3_determinant();
        let _ = m3.mat3_invert();
        let _ = m3.mat3_mul_vec3(v0);

        let m4 = Mat4 {
            cols: [
                rng.next_vec4().vec4_to_array(),
                rng.next_vec4().vec4_to_array(),
                rng.next_vec4().vec4_to_array(),
                rng.next_vec4().vec4_to_array(),
            ],
        };
        let _ = m4.mat4_determinant();
        let _ = m4.invert();
        let _ = m4.transform_point3(v0);
        let _ = m4.mat4_to_f32_cols();
        let _ = Frustum::from_view_proj(&m4);
        let _ = Frustum::from_view_proj_zo(&m4);

        let p = Plane { normal: v0, d: s };
        let _ = p.signed_distance(v1);
        let aabb = Aabb3 { min: v0, max: v1 };
        let _ = aabb.aabb3_center();
        let _ = aabb.aabb3_contains_point(v1);
        let sp = Sphere {
            center: v0,
            radius: s,
        };
        let fr = Frustum { planes: [p; 6] };
        let _ = fr.contains_point(v1);
        let _ = fr.intersects_sphere(&sp);
        let _ = fr.intersects_aabb(&aabb);

        let ray = Ray3 {
            origin: v0,
            dir: v1,
        };
        let _ = ray.ray3_at(s);
        let _ = ray_plane_intersect(&ray, &p);
        let _ = ray_aabb_intersect(&ray, &aabb);
    }
}
