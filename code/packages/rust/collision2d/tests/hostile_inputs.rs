//! Hostile-input and deterministic fuzz test suite for `collision2d`.

#![forbid(unsafe_code)]

use collision2d::{
    aabb_vs_aabb, circle_vs_aabb, circle_vs_circle, fixed_aabb_vs_aabb, fixed_circle_vs_aabb,
    obb_vs_obb_sat, raycast_aabb, raycast_circle, reflect_velocity, sweep_aabb_aabb,
    sweep_circle_aabb, Circle, CollisionError, ContactManifold, FixedCircleI64, FixedRectI64,
    Obb2D, Point, Ray2D, Rect, SweepHit, MAX_ROTATION_RAD,
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

    fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    fn next_hostile_f64(&mut self, idx: usize) -> f64 {
        match idx % 11 {
            0 => f64::NAN,
            1 => f64::from_bits(0x7FF8_0000_0000_0001), // non-canonical NaN
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => -0.0,
            5 => f64::from_bits(1), // subnormal
            6 => -f64::from_bits(1),
            7 => f64::MAX,
            8 => f64::MIN,
            9 => 1.0e9,
            _ => ((self.next_i64()) as f64) / 1.0e14,
        }
    }
}

fn assert_manifold_finite(m: &ContactManifold) {
    assert!(m.normal.x.is_finite() && m.normal.y.is_finite());
    assert!(m.penetration.is_finite());
    assert!(m.contact_point.x.is_finite() && m.contact_point.y.is_finite());
}

fn assert_hit_finite(h: &SweepHit) {
    assert!(h.toi.is_finite());
    assert!(h.normal.x.is_finite() && h.normal.y.is_finite());
    assert!(h.contact_point.x.is_finite() && h.contact_point.y.is_finite());
}

#[test]
fn hostile_fixed_i64_min_max_and_obb_rotation() {
    // FixedRectI64 & FixedCircleI64 with i64::MIN / i64::MAX must return Err, not panic
    for &extreme in &[i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
        assert_eq!(
            FixedRectI64::new(extreme, 0, 10, 10),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedRectI64::new(0, extreme, 10, 10),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedRectI64::new(0, 0, extreme, 10),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedRectI64::new(0, 0, 10, extreme),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedCircleI64::new(extreme, 0, 10),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedCircleI64::new(0, extreme, 10),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
        assert_eq!(
            FixedCircleI64::new(0, 0, extreme),
            Err(CollisionError::FixedCoordinateOutOfBounds)
        );
    }

    // Obb2D rotation_rad outside MAX_ROTATION_RAD (e.g. 1e9 exceeding trig::sin domain) must fail
    for &bad_rot in &[
        1.0e9,
        -1.0e9,
        MAX_ROTATION_RAD + 1.0,
        -(MAX_ROTATION_RAD + 1.0),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert_eq!(
            Obb2D::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0), bad_rot),
            Err(if bad_rot.is_finite() {
                CollisionError::CoordinateOutOfBounds
            } else {
                CollisionError::NonFiniteInput
            })
        );
    }
    assert_eq!(
        Obb2D::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0), f64::NAN),
        Err(CollisionError::NonFiniteInput)
    );

    // Zero-radius raycast_circle must produce unit non-zero normal
    let ray = Ray2D::new(Point::new(-5.0, 0.0), Point::new(1.0, 0.0), 10.0).unwrap();
    let zero_circle = Circle::new(Point::new(0.0, 0.0), 0.0).unwrap();
    let hit = raycast_circle(ray, zero_circle).unwrap().unwrap();
    assert_hit_finite(&hit);
    assert!((hit.normal.magnitude_squared() - 1.0).abs() < 1e-9);

    // Zero-radius sweep_circle_aabb and zero-size Rect
    let zero_rect = Rect::new(0.0, 0.0, 0.0, 0.0);
    if let Ok(Some(h)) = sweep_circle_aabb(zero_circle, Point::new(5.0, 0.0), zero_rect) {
        assert_hit_finite(&h);
    }
}

#[test]
fn hostile_fuzz_campaign_2000_cases() {
    let mut rng = SplitMix64(0xC011_1510_2D00_0001);

    for i in 0..2_000 {
        let x0 = rng.next_hostile_f64(i);
        let y0 = rng.next_hostile_f64(i + 1);
        let x1 = rng.next_hostile_f64(i + 2);
        let y1 = rng.next_hostile_f64(i + 3);
        let s0 = rng.next_hostile_f64(i + 4);
        let s1 = rng.next_hostile_f64(i + 5);
        let rot = rng.next_hostile_f64(i + 6);

        let p0 = Point::new(x0, y0);
        let p1 = Point::new(x1, y1);
        let r0 = Rect::new(x0, y0, s0.abs(), s1.abs());
        let r1 = Rect::new(x1, y1, s1.abs(), s0.abs());
        let c0 = Circle {
            center: p0,
            radius: s0,
        };
        let c1 = Circle {
            center: p1,
            radius: s1,
        };
        let obb0 = Obb2D {
            center: p0,
            half_extents: Point::new(s0, s1),
            rotation_rad: rot,
        };
        let obb1 = Obb2D {
            center: p1,
            half_extents: Point::new(s1, s0),
            rotation_rad: -rot,
        };
        let ray = Ray2D {
            origin: p0,
            dir: p1,
            max_toi: s0,
        };

        if let Ok(Some(m)) = aabb_vs_aabb(r0, r1) {
            assert_manifold_finite(&m);
        }
        if let Ok(Some(m)) = circle_vs_circle(c0, c1) {
            assert_manifold_finite(&m);
        }
        if let Ok(Some(m)) = circle_vs_aabb(c0, r1) {
            assert_manifold_finite(&m);
        }
        if let Ok(Some(m)) = obb_vs_obb_sat(obb0, obb1) {
            assert_manifold_finite(&m);
        }
        if let Ok(Some(h)) = raycast_aabb(ray, r1) {
            assert_hit_finite(&h);
        }
        if let Ok(Some(h)) = raycast_circle(ray, c1) {
            assert_hit_finite(&h);
        }
        if let Ok(Some(h)) = sweep_aabb_aabb(r0, p1, r1) {
            assert_hit_finite(&h);
        }
        if let Ok(Some(h)) = sweep_circle_aabb(c0, p1, r1) {
            assert_hit_finite(&h);
        }
        if let Ok(v) = reflect_velocity(p0, p1, s0) {
            assert!(v.x.is_finite() && v.y.is_finite());
        }

        let fx = if i % 7 == 0 { i64::MIN } else { rng.next_i64() };
        let fy = if i % 9 == 0 { i64::MAX } else { rng.next_i64() };
        let fw = if i % 11 == 0 {
            i64::MIN
        } else {
            rng.next_i64()
        };
        let fh = if i % 13 == 0 {
            i64::MAX
        } else {
            rng.next_i64()
        };
        let fr0 = FixedRectI64 {
            x: fx,
            y: fy,
            width: fw,
            height: fh,
        };
        let fr1 = FixedRectI64 {
            x: fy,
            y: fx,
            width: fh,
            height: fw,
        };
        let fc = FixedCircleI64 {
            cx: fx,
            cy: fy,
            radius: fw,
        };
        let _ = fixed_aabb_vs_aabb(fr0, fr1);
        let _ = fixed_circle_vs_aabb(fc, fr1);
    }
}
