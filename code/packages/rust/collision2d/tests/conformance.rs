//! # Conformance & Adversarial Verification Suite for `collision2d`
//!
//! Validates:
//! - Exact geometric overlap correctness (AABB, Circle, OBB SAT)
//! - Continuous swept collision queries (anti-tunneling for Pong/Breakout)
//! - Concentric circle division-by-zero resilience (CWE-369)
//! - Axis-parallel raycasting resilience (CWE-369)
//! - Degenerate ray/normal vector rejection (CWE-369)
//! - Float bounds & non-finite input guards (CWE-190, CWE-20)
//! - Fixed-point 64-bit integer fast path with 128-bit squared distance math
//! - Restitution and velocity reflection physics

#![deny(unsafe_code)]

use collision2d::{
    aabb_vs_aabb, circle_vs_aabb, circle_vs_circle, fixed_aabb_vs_aabb, fixed_circle_vs_aabb,
    obb_vs_obb_sat, raycast_aabb, raycast_circle, reflect_velocity, sweep_aabb_aabb,
    sweep_circle_aabb, Affine2D, Circle, CollisionError, FixedCircleI64, FixedRectI64, Obb2D,
    Point, Ray2D, Rect, MAX_COORD, MAX_FIXED_COORD,
};

#[test]
fn test_aabb_vs_aabb_overlapping() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(8.0, 2.0, 10.0, 10.0);
    let res = aabb_vs_aabb(a, b).expect("valid rects");
    assert!(res.is_some());
    let manifold = res.unwrap();
    // Overlap on X is 2.0, on Y is 8.0 -> min penetration is 2.0 on X axis
    assert_eq!(manifold.normal, Point::new(1.0, 0.0));
    assert!((manifold.penetration - 2.0).abs() < 1e-9);
}

#[test]
fn test_aabb_vs_aabb_disjoint() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(15.0, 0.0, 10.0, 10.0);
    let res = aabb_vs_aabb(a, b).expect("valid rects");
    assert!(res.is_none());
}

#[test]
fn test_aabb_vs_aabb_touching_edge() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(10.0, 0.0, 10.0, 10.0);
    let res = aabb_vs_aabb(a, b).expect("valid rects");
    assert!(res.is_none());
}

#[test]
fn test_circle_vs_circle_overlapping() {
    let a = Circle::new(Point::new(0.0, 0.0), 5.0).unwrap();
    let b = Circle::new(Point::new(8.0, 0.0), 5.0).unwrap();
    let res = circle_vs_circle(a, b).expect("valid circles");
    assert!(res.is_some());
    let manifold = res.unwrap();
    assert_eq!(manifold.normal, Point::new(1.0, 0.0));
    assert!((manifold.penetration - 2.0).abs() < 1e-9);
    assert_eq!(manifold.contact_point, Point::new(5.0, 0.0));
}

#[test]
fn test_circle_vs_circle_disjoint() {
    let a = Circle::new(Point::new(0.0, 0.0), 5.0).unwrap();
    let b = Circle::new(Point::new(12.0, 0.0), 5.0).unwrap();
    let res = circle_vs_circle(a, b).expect("valid circles");
    assert!(res.is_none());
}

#[test]
fn test_circle_vs_circle_concentric_cwe_369() {
    let a = Circle::new(Point::new(10.0, 10.0), 5.0).unwrap();
    let b = Circle::new(Point::new(10.0, 10.0), 3.0).unwrap();
    let res = circle_vs_circle(a, b).expect("valid concentric circles");
    assert!(res.is_some());
    let manifold = res.unwrap();
    assert_eq!(manifold.normal, Point::new(1.0, 0.0));
    assert!((manifold.penetration - 8.0).abs() < 1e-9);
    assert_eq!(manifold.contact_point, Point::new(10.0, 10.0));
}

#[test]
fn test_circle_vs_aabb_outside_hit() {
    let circle = Circle::new(Point::new(-2.0, 5.0), 4.0).unwrap();
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let res = circle_vs_aabb(circle, rect).expect("valid inputs");
    assert!(res.is_some());
    let m = res.unwrap();
    assert_eq!(m.normal, Point::new(1.0, 0.0));
    assert!((m.penetration - 2.0).abs() < 1e-9);
    assert_eq!(m.contact_point, Point::new(0.0, 5.0));
}

#[test]
fn test_circle_vs_aabb_inside_hit() {
    let circle = Circle::new(Point::new(2.0, 5.0), 1.0).unwrap();
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let res = circle_vs_aabb(circle, rect).expect("valid inputs");
    assert!(res.is_some());
    let m = res.unwrap();
    // Distance to left face is 2.0, penetration = radius + 2.0 = 3.0
    assert_eq!(m.normal, Point::new(-1.0, 0.0));
    assert!((m.penetration - 3.0).abs() < 1e-9);
    assert_eq!(m.contact_point, Point::new(0.0, 5.0));
}

#[test]
fn test_obb_vs_obb_sat_aligned() {
    let a = Obb2D::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0), 0.0).unwrap();
    let b = Obb2D::new(Point::new(8.0, 0.0), Point::new(5.0, 5.0), 0.0).unwrap();
    let res = obb_vs_obb_sat(a, b).expect("valid obbs");
    assert!(res.is_some());
    let m = res.unwrap();
    assert!((m.penetration - 2.0).abs() < 1e-9);
}

#[test]
fn test_obb_vs_obb_sat_rotated() {
    // 45 degree rotation
    let angle = std::f64::consts::FRAC_PI_4;
    let a = Obb2D::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0), angle).unwrap();
    let b = Obb2D::new(Point::new(3.0, 0.0), Point::new(2.0, 2.0), 0.0).unwrap();
    let res = obb_vs_obb_sat(a, b).expect("valid obbs");
    assert!(res.is_some());

    let c = Obb2D::new(Point::new(10.0, 0.0), Point::new(2.0, 2.0), 0.0).unwrap();
    let res_c = obb_vs_obb_sat(a, c).expect("valid obbs");
    assert!(res_c.is_none());
}

#[test]
fn test_sat_obb_rotation_via_affine2d() {
    let rot = Affine2D::rotate(std::f64::consts::FRAC_PI_2); // 90 degrees CCW
    let p = Point::new(1.0, 0.0);
    let p_rot = rot.apply_to_point(p);
    assert!((p_rot.x - 0.0).abs() < 1e-9);
    assert!((p_rot.y - 1.0).abs() < 1e-9);

    // Transforming an OBB by Affine2D
    let obb = Obb2D::new(Point::new(0.0, 0.0), Point::new(10.0, 2.0), 0.0).unwrap();
    let rotated_center = rot.apply_to_point(obb.center);
    let rotated_obb = Obb2D::new(
        rotated_center,
        obb.half_extents,
        obb.rotation_rad + std::f64::consts::FRAC_PI_2,
    )
    .unwrap();

    let original_corners = obb.corners();
    let rotated_corners = rotated_obb.corners();
    for orig in &original_corners {
        let expected = rot.apply_to_point(*orig);
        let found = rotated_corners
            .iter()
            .any(|c| (c.x - expected.x).abs() < 1e-6 && (c.y - expected.y).abs() < 1e-6);
        assert!(
            found,
            "Affine2D transformed corner must match OBB rotated corner"
        );
    }
}

#[test]
fn test_raycast_aabb_axis_parallel_cwe_369() {
    let ray_parallel_x = Ray2D::new(Point::new(5.0, -10.0), Point::new(0.0, 20.0), 1.0).unwrap();
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let hit = raycast_aabb(ray_parallel_x, rect).expect("valid raycast");
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert!((h.toi - 0.5).abs() < 1e-9);
    assert_eq!(h.normal, Point::new(0.0, -1.0));
    assert_eq!(h.contact_point, Point::new(5.0, 0.0));

    let ray_miss = Ray2D::new(Point::new(15.0, -10.0), Point::new(0.0, 20.0), 1.0).unwrap();
    let hit_miss = raycast_aabb(ray_miss, rect).expect("valid raycast");
    assert!(hit_miss.is_none());
}

#[test]
fn test_raycast_circle() {
    let ray = Ray2D::new(Point::new(-10.0, 0.0), Point::new(20.0, 0.0), 1.0).unwrap();
    let circle = Circle::new(Point::new(0.0, 0.0), 4.0).unwrap();
    let hit = raycast_circle(ray, circle).expect("valid raycast");
    assert!(hit.is_some());
    let h = hit.unwrap();
    // Origin at -10, radius 4 -> contact at -4. Distance is 6. toi = 6 / 20 = 0.3
    assert!((h.toi - 0.3).abs() < 1e-9);
    assert_eq!(h.contact_point, Point::new(-4.0, 0.0));
    assert_eq!(h.normal, Point::new(-1.0, 0.0));
}

#[test]
fn test_anti_tunneling_pong_ball_differential() {
    // High speed ball crossing a thin paddle:
    // Paddle at x = 200..208, y = 80..120
    let paddle = Rect::new(200.0, 80.0, 8.0, 40.0);
    let ball_start = Circle::new(Point::new(0.0, 100.0), 4.0).unwrap();
    let velocity = Point::new(500.0, 0.0);

    // Discrete test at t = 1.0 (ball moved to x = 500)
    let ball_end = Circle::new(Point::new(500.0, 100.0), 4.0).unwrap();
    let discrete_hit = circle_vs_aabb(ball_end, paddle).expect("discrete check");
    assert!(discrete_hit.is_none(), "discrete test misses high-speed ball");

    // Continuous sweep query catches the ball mid-frame!
    let sweep_hit = sweep_circle_aabb(ball_start, velocity, paddle).expect("sweep query");
    assert!(sweep_hit.is_some(), "continuous sweep must detect collision");
    let hit = sweep_hit.unwrap();
    // Impact with left slab: x = 200 - 4 = 196. toi = 196 / 500 = 0.392
    assert!((hit.toi - 0.392).abs() < 1e-6);
    assert_eq!(hit.normal, Point::new(-1.0, 0.0));
    assert_eq!(hit.contact_point, Point::new(200.0, 100.0));
}

#[test]
fn test_sweep_circle_aabb_corner_hit() {
    // Ball moving diagonally towards paddle top-left corner
    let paddle = Rect::new(100.0, 100.0, 50.0, 50.0);
    let ball = Circle::new(Point::new(70.0, 70.0), 10.0).unwrap();
    let velocity = Point::new(50.0, 50.0);

    let hit = sweep_circle_aabb(ball, velocity, paddle).expect("sweep query");
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert!(h.toi > 0.0 && h.toi < 1.0);
    assert_eq!(h.contact_point, Point::new(100.0, 100.0));
}

#[test]
fn test_sweep_aabb_aabb() {
    let moving = Rect::new(0.0, 0.0, 10.0, 10.0);
    let velocity = Point::new(100.0, 0.0);
    let target = Rect::new(50.0, 0.0, 10.0, 10.0);

    let hit = sweep_aabb_aabb(moving, velocity, target).expect("sweep aabb");
    assert!(hit.is_some());
    let h = hit.unwrap();
    // Distance from moving right edge (10) to target left edge (50) is 40. toi = 40/100 = 0.4
    assert!((h.toi - 0.4).abs() < 1e-9);
    assert_eq!(h.normal, Point::new(-1.0, 0.0));
}

#[test]
fn test_reflect_velocity_elastic_and_inelastic() {
    let vel = Point::new(100.0, -50.0);
    let normal = Point::new(-1.0, 0.0);

    // Elastic (restitution = 1.0)
    let reflected = reflect_velocity(vel, normal, 1.0).expect("elastic reflection");
    assert_eq!(reflected, Point::new(-100.0, -50.0));

    // Inelastic (restitution = 0.0)
    let absorbed = reflect_velocity(vel, normal, 0.0).expect("inelastic reflection");
    assert_eq!(absorbed, Point::new(0.0, -50.0));

    // Separating velocity check: if already moving away, velocity unchanged
    let separating_vel = Point::new(-100.0, 0.0);
    let unchanged = reflect_velocity(separating_vel, normal, 1.0).expect("separating vel");
    assert_eq!(unchanged, separating_vel);
}

#[test]
fn test_adversarial_cwe_369_degenerate_directions() {
    let zero_dir = Point::origin();
    let res_ray = Ray2D::new(Point::new(0.0, 0.0), zero_dir, 10.0);
    assert_eq!(res_ray, Err(CollisionError::DegenerateDirection));

    let res_reflect = reflect_velocity(Point::new(10.0, 10.0), zero_dir, 1.0);
    assert_eq!(res_reflect, Err(CollisionError::DegenerateDirection));
}

#[test]
fn test_adversarial_cwe_190_and_cwe_20_bounds() {
    // Non-finite (NaN / Inf)
    assert_eq!(
        Circle::new(Point::new(f64::NAN, 0.0), 5.0),
        Err(CollisionError::NonFiniteInput)
    );
    assert_eq!(
        Circle::new(Point::new(0.0, 0.0), f64::INFINITY),
        Err(CollisionError::NonFiniteInput)
    );

    // Out of bounds (> 1.0e12)
    assert_eq!(
        Circle::new(Point::new(MAX_COORD + 1.0, 0.0), 5.0),
        Err(CollisionError::CoordinateOutOfBounds)
    );

    // Negative dimensions
    assert_eq!(
        Circle::new(Point::new(0.0, 0.0), -1.0),
        Err(CollisionError::NegativeDimension)
    );
    assert!(
        Rect::new(0.0, 0.0, -10.0, 5.0).is_empty()
    );
    assert_eq!(
        aabb_vs_aabb(Rect::new(0.0, 0.0, -5.0, 5.0), Rect::new(0.0, 0.0, 5.0, 5.0)),
        Err(CollisionError::NegativeDimension)
    );

    // Invalid restitution
    assert_eq!(
        reflect_velocity(Point::new(1.0, 1.0), Point::new(1.0, 0.0), 1.5),
        Err(CollisionError::InvalidRestitution)
    );
    assert_eq!(
        reflect_velocity(Point::new(1.0, 1.0), Point::new(1.0, 0.0), -0.1),
        Err(CollisionError::InvalidRestitution)
    );
}

#[test]
fn test_fixed_point_i128_equivalence() {
    let r1 = FixedRectI64::new(0, 0, 100, 100).unwrap();
    let r2 = FixedRectI64::new(80, 20, 100, 100).unwrap();
    let res = fixed_aabb_vs_aabb(r1, r2).expect("valid fixed rects");
    assert!(res.is_some());
    let overlap = res.unwrap();
    assert_eq!(overlap.normal_x, 1);
    assert_eq!(overlap.normal_y, 0);
    assert_eq!(overlap.penetration, 20);

    let c = FixedCircleI64::new(0, 50, 40).unwrap();
    let r = FixedRectI64::new(20, 0, 100, 100).unwrap();
    let c_res = fixed_circle_vs_aabb(c, r).expect("valid fixed circle & rect");
    assert!(c_res.is_some());
    let c_overlap = c_res.unwrap();
    assert_eq!(c_overlap.normal_x, 1);
    assert_eq!(c_overlap.normal_y, 0);
    assert_eq!(c_overlap.penetration, 20);

    // Fixed point bounds
    assert_eq!(
        FixedRectI64::new(MAX_FIXED_COORD + 1, 0, 10, 10),
        Err(CollisionError::FixedCoordinateOutOfBounds)
    );
    assert_eq!(
        FixedCircleI64::new(0, 0, -5),
        Err(CollisionError::FixedCoordinateOutOfBounds)
    );
}
