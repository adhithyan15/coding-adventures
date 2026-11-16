//! # Deterministic 2D Arcade Collision Detection & Continuous Sweep Primitives
//!
//! Reusable pure-Rust 2D collision algorithms for historical game engine simulations
//! (`hamurabi`, `lunar-lander`, `pong`, `breakout`).
//!
//! Provides:
//! - Stack-only geometric primitives (`Circle`, `Obb2D`, `Ray2D`, `ContactManifold`, `SweepHit`)
//! - Bounded float coordinates (`±1.0e12`) preventing floating-point overflow (`CWE-190`)
//! - Strict rejection of degenerate rays (`CWE-369`) and non-finite numbers (`CWE-20`)
//! - Continuous swept collision queries (`sweep_circle_aabb`, `sweep_aabb_aabb`) to eliminate tunneling
//! - Deterministic 64-bit fixed-point integer queries (`FixedRectI64`, `FixedCircleI64`) with 128-bit squaring
//! - Elastic/inelastic velocity reflection (`reflect_velocity`) with separating velocity guards
//! - Zero heap allocation on all hot paths (`VER07`)

#![forbid(unsafe_code)]

pub mod capabilities;
pub use capabilities::PureCapabilities;

pub use affine2d::Affine2D;
pub use point2d::{Point, Rect};

/// Alias for `Rect` as an Axis-Aligned Bounding Box (AABB).
pub type Aabb = Rect;

/// Alias for `Obb2D` as an Oriented Bounding Box (OBB).
pub type Obb = Obb2D;

/// Maximum coordinate value for float geometric components (`±1.0e12`).
pub const MAX_COORD: f64 = 1.0e12;

/// Maximum coordinate and dimension value for fixed-point integer structures.
pub const MAX_FIXED_COORD: i64 = 1_000_000_000;

/// Numerical epsilon for degenerate vector and division-by-zero checks.
pub const EPSILON: f64 = 1e-12;

/// Collision errors raised during geometry validation and query execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionError {
    /// Coordinate, dimension, angle, or velocity contains NaN or Infinity.
    NonFiniteInput,
    /// Absolute coordinate value exceeds `MAX_COORD` (`1.0e12`).
    CoordinateOutOfBounds,
    /// Negative width, height, radius, or time of impact.
    NegativeDimension,
    /// Degenerate zero-length ray direction or normal vector (`CWE-369`).
    DegenerateDirection,
    /// Restitution coefficient outside `[0.0, 1.0]`.
    InvalidRestitution,
    /// Fixed-point coordinate outside `[-1_000_000_000, 1_000_000_000]` or negative dimension.
    FixedCoordinateOutOfBounds,
}

impl std::fmt::Display for CollisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "geometry input is non-finite (NaN or Inf)"),
            Self::CoordinateOutOfBounds => write!(f, "coordinate exceeds allowable maximum bound"),
            Self::NegativeDimension => write!(f, "dimension or radius cannot be negative"),
            Self::DegenerateDirection => write!(f, "ray or normal vector has zero or degenerate magnitude"),
            Self::InvalidRestitution => write!(f, "restitution must be between 0.0 and 1.0"),
            Self::FixedCoordinateOutOfBounds => write!(f, "fixed coordinate exceeds allowable integer bound"),
        }
    }
}

impl std::error::Error for CollisionError {}

/// Validate an individual coordinate or scalar dimension.
#[inline]
pub fn validate_coord(val: f64) -> Result<(), CollisionError> {
    if !val.is_finite() {
        return Err(CollisionError::NonFiniteInput);
    }
    if val.abs() > MAX_COORD {
        return Err(CollisionError::CoordinateOutOfBounds);
    }
    Ok(())
}

/// Validate a 2D point / vector.
#[inline]
pub fn validate_point(p: Point) -> Result<(), CollisionError> {
    validate_coord(p.x)?;
    validate_coord(p.y)?;
    Ok(())
}

/// Validate an axis-aligned bounding box.
#[inline]
pub fn validate_rect(r: Rect) -> Result<(), CollisionError> {
    validate_coord(r.x)?;
    validate_coord(r.y)?;
    validate_coord(r.width)?;
    validate_coord(r.height)?;
    if r.width < 0.0 || r.height < 0.0 {
        return Err(CollisionError::NegativeDimension);
    }
    Ok(())
}

/// A 2D circle with center and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    /// Construct and validate a 2D circle.
    pub fn new(center: Point, radius: f64) -> Result<Self, CollisionError> {
        let c = Self { center, radius };
        c.validate()?;
        Ok(c)
    }

    /// Validate the circle invariants.
    pub fn validate(&self) -> Result<(), CollisionError> {
        validate_point(self.center)?;
        validate_coord(self.radius)?;
        if self.radius < 0.0 {
            return Err(CollisionError::NegativeDimension);
        }
        Ok(())
    }
}

/// An oriented bounding box (OBB) defined by center, half-extents, and rotation in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb2D {
    pub center: Point,
    pub half_extents: Point,
    pub rotation_rad: f64,
}

impl Obb2D {
    /// Construct and validate a 2D OBB.
    pub fn new(center: Point, half_extents: Point, rotation_rad: f64) -> Result<Self, CollisionError> {
        let obb = Self {
            center,
            half_extents,
            rotation_rad,
        };
        obb.validate()?;
        Ok(obb)
    }

    /// Validate OBB invariants.
    pub fn validate(&self) -> Result<(), CollisionError> {
        validate_point(self.center)?;
        validate_point(self.half_extents)?;
        if self.half_extents.x < 0.0 || self.half_extents.y < 0.0 {
            return Err(CollisionError::NegativeDimension);
        }
        validate_coord(self.rotation_rad)?;
        Ok(())
    }

    /// Compute unit orientation axes (local X and local Y in world space).
    pub fn axes(&self) -> (Point, Point) {
        let c = trig::cos(self.rotation_rad);
        let s = trig::sin(self.rotation_rad);
        (Point::new(c, s), Point::new(-s, c))
    }

    /// Return the 4 corners of the OBB in counter-clockwise world space.
    pub fn corners(&self) -> [Point; 4] {
        let (ax, ay) = self.axes();
        let hx = self.half_extents.x;
        let hy = self.half_extents.y;
        let cx = self.center.x;
        let cy = self.center.y;
        [
            Point::new(cx + ax.x * hx + ay.x * hy, cy + ax.y * hx + ay.y * hy),
            Point::new(cx - ax.x * hx + ay.x * hy, cy - ax.y * hx + ay.y * hy),
            Point::new(cx - ax.x * hx - ay.x * hy, cy - ax.y * hx - ay.y * hy),
            Point::new(cx + ax.x * hx - ay.x * hy, cy + ax.y * hx - ay.y * hy),
        ]
    }
}

/// A 2D ray with an origin, direction vector, and maximum time of impact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray2D {
    pub origin: Point,
    pub dir: Point,
    pub max_toi: f64,
}

impl Ray2D {
    /// Construct and validate a 2D ray.
    pub fn new(origin: Point, dir: Point, max_toi: f64) -> Result<Self, CollisionError> {
        let r = Self { origin, dir, max_toi };
        r.validate()?;
        Ok(r)
    }

    /// Validate ray invariants.
    pub fn validate(&self) -> Result<(), CollisionError> {
        validate_point(self.origin)?;
        validate_point(self.dir)?;
        validate_coord(self.max_toi)?;
        if self.max_toi < 0.0 {
            return Err(CollisionError::NegativeDimension);
        }
        if self.dir.magnitude_squared() <= EPSILON * EPSILON {
            return Err(CollisionError::DegenerateDirection);
        }
        Ok(())
    }

    /// Evaluate point along ray at parametric time `t`.
    pub fn point_at(&self, t: f64) -> Point {
        self.origin.add(self.dir.scale(t))
    }
}

/// Contact manifold resulting from a discrete overlap test.
///
/// Convention: `normal` points from shape A toward shape B.
/// Translating shape A by `normal.scale(-penetration)` separates A from B.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactManifold {
    pub normal: Point,
    pub penetration: f64,
    pub contact_point: Point,
}

/// Continuous swept collision hit record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepHit {
    /// Parametric time of impact in `[0.0, 1.0]` (or `[0.0, ray.max_toi]`).
    pub toi: f64,
    /// Outward surface normal of the obstacle hit by the moving body.
    pub normal: Point,
    /// World contact point where collision occurs.
    pub contact_point: Point,
}

/// Deterministic 64-bit integer rectangle for fixed-point arcade physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRectI64 {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl FixedRectI64 {
    /// Construct and validate a fixed-point rectangle.
    pub fn new(x: i64, y: i64, width: i64, height: i64) -> Result<Self, CollisionError> {
        let r = Self { x, y, width, height };
        r.validate()?;
        Ok(r)
    }

    /// Validate fixed rectangle boundaries.
    pub fn validate(&self) -> Result<(), CollisionError> {
        if self.x.abs() > MAX_FIXED_COORD
            || self.y.abs() > MAX_FIXED_COORD
            || self.width.abs() > MAX_FIXED_COORD
            || self.height.abs() > MAX_FIXED_COORD
            || self.width < 0
            || self.height < 0
        {
            return Err(CollisionError::FixedCoordinateOutOfBounds);
        }
        Ok(())
    }
}

/// Deterministic 64-bit integer circle for fixed-point arcade physics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedCircleI64 {
    pub cx: i64,
    pub cy: i64,
    pub radius: i64,
}

impl FixedCircleI64 {
    /// Construct and validate a fixed-point circle.
    pub fn new(cx: i64, cy: i64, radius: i64) -> Result<Self, CollisionError> {
        let c = Self { cx, cy, radius };
        c.validate()?;
        Ok(c)
    }

    /// Validate fixed circle boundaries.
    pub fn validate(&self) -> Result<(), CollisionError> {
        if self.cx.abs() > MAX_FIXED_COORD
            || self.cy.abs() > MAX_FIXED_COORD
            || self.radius.abs() > MAX_FIXED_COORD
            || self.radius < 0
        {
            return Err(CollisionError::FixedCoordinateOutOfBounds);
        }
        Ok(())
    }
}

/// Discrete overlap manifold for fixed-point integer shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedOverlapI64 {
    pub normal_x: i64,
    pub normal_y: i64,
    pub penetration: i64,
}

/// Integer square root for 128-bit unsigned integers.
fn isqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x0 = 1u128 << (128 - n.leading_zeros()).div_ceil(2);
    loop {
        let x1 = (x0 + n / x0) / 2;
        if x1 >= x0 {
            return x0;
        }
        x0 = x1;
    }
}

/// Discrete collision test between two axis-aligned bounding boxes (AABB vs AABB).
pub fn aabb_vs_aabb(a: Rect, b: Rect) -> Result<Option<ContactManifold>, CollisionError> {
    validate_rect(a)?;
    validate_rect(b)?;

    let a_min = a.min_point();
    let a_max = a.max_point();
    let b_min = b.min_point();
    let b_max = b.max_point();

    if a_min.x >= b_max.x || b_min.x >= a_max.x || a_min.y >= b_max.y || b_min.y >= a_max.y {
        return Ok(None);
    }

    let ox = a_max.x.min(b_max.x) - a_min.x.max(b_min.x);
    let oy = a_max.y.min(b_max.y) - a_min.y.max(b_min.y);

    if ox <= 0.0 || oy <= 0.0 {
        return Ok(None);
    }

    let a_c = a.center();
    let b_c = b.center();

    let (normal, penetration) = if ox <= oy {
        let norm_x = if a_c.x <= b_c.x { 1.0 } else { -1.0 };
        (Point::new(norm_x, 0.0), ox)
    } else {
        let norm_y = if a_c.y <= b_c.y { 1.0 } else { -1.0 };
        (Point::new(0.0, norm_y), oy)
    };

    let cp_x = (a_min.x.max(b_min.x) + a_max.x.min(b_max.x)) * 0.5;
    let cp_y = (a_min.y.max(b_min.y) + a_max.y.min(b_max.y)) * 0.5;

    Ok(Some(ContactManifold {
        normal,
        penetration,
        contact_point: Point::new(cp_x, cp_y),
    }))
}

/// Discrete collision test between two circles.
pub fn circle_vs_circle(a: Circle, b: Circle) -> Result<Option<ContactManifold>, CollisionError> {
    a.validate()?;
    b.validate()?;

    let delta = b.center.subtract(a.center);
    let dist_sq = delta.magnitude_squared();
    let r_sum = a.radius + b.radius;

    if dist_sq > r_sum * r_sum {
        return Ok(None);
    }

    // Concentric circle guard (CWE-369)
    if dist_sq <= 1e-24 {
        return Ok(Some(ContactManifold {
            normal: Point::new(1.0, 0.0),
            penetration: r_sum,
            contact_point: a.center,
        }));
    }

    let dist = trig::sqrt(dist_sq);
    let normal = delta.scale(1.0 / dist);
    let penetration = r_sum - dist;
    let contact_point = a.center.add(normal.scale(a.radius));

    Ok(Some(ContactManifold {
        normal,
        penetration,
        contact_point,
    }))
}

/// Discrete collision test between a circle and an axis-aligned bounding box.
pub fn circle_vs_aabb(circle: Circle, rect: Rect) -> Result<Option<ContactManifold>, CollisionError> {
    circle.validate()?;
    validate_rect(rect)?;

    let r_min = rect.min_point();
    let r_max = rect.max_point();

    let closest_x = circle.center.x.clamp(r_min.x, r_max.x);
    let closest_y = circle.center.y.clamp(r_min.y, r_max.y);
    let closest = Point::new(closest_x, closest_y);

    if closest_x == circle.center.x && closest_y == circle.center.y {
        // Center is inside AABB: find nearest face
        let d_left = circle.center.x - r_min.x;
        let d_right = r_max.x - circle.center.x;
        let d_top = circle.center.y - r_min.y;
        let d_bottom = r_max.y - circle.center.y;

        if d_left <= d_right && d_left <= d_top && d_left <= d_bottom {
            Ok(Some(ContactManifold {
                normal: Point::new(-1.0, 0.0),
                penetration: circle.radius + d_left,
                contact_point: Point::new(r_min.x, circle.center.y),
            }))
        } else if d_right <= d_top && d_right <= d_bottom {
            Ok(Some(ContactManifold {
                normal: Point::new(1.0, 0.0),
                penetration: circle.radius + d_right,
                contact_point: Point::new(r_max.x, circle.center.y),
            }))
        } else if d_top <= d_bottom {
            Ok(Some(ContactManifold {
                normal: Point::new(0.0, -1.0),
                penetration: circle.radius + d_top,
                contact_point: Point::new(circle.center.x, r_min.y),
            }))
        } else {
            Ok(Some(ContactManifold {
                normal: Point::new(0.0, 1.0),
                penetration: circle.radius + d_bottom,
                contact_point: Point::new(circle.center.x, r_max.y),
            }))
        }
    } else {
        // Center is outside AABB
        let delta = closest.subtract(circle.center);
        let dist_sq = delta.magnitude_squared();

        if dist_sq > circle.radius * circle.radius {
            return Ok(None);
        }

        if dist_sq <= 1e-24 {
            return Ok(Some(ContactManifold {
                normal: Point::new(1.0, 0.0),
                penetration: circle.radius,
                contact_point: closest,
            }));
        }

        let dist = trig::sqrt(dist_sq);
        let normal = delta.scale(1.0 / dist);
        let penetration = circle.radius - dist;

        Ok(Some(ContactManifold {
            normal,
            penetration,
            contact_point: closest,
        }))
    }
}

/// Project an array of 4 points onto an axis and return `(min_proj, max_proj)`.
fn project_points(pts: &[Point; 4], axis: Point) -> (f64, f64) {
    let mut min_p = pts[0].dot(axis);
    let mut max_p = min_p;
    for &p in &pts[1..] {
        let dot = p.dot(axis);
        if dot < min_p {
            min_p = dot;
        }
        if dot > max_p {
            max_p = dot;
        }
    }
    (min_p, max_p)
}

/// Oriented Bounding Box (OBB) collision using the Separating Axis Theorem (SAT).
pub fn obb_vs_obb_sat(a: Obb2D, b: Obb2D) -> Result<Option<ContactManifold>, CollisionError> {
    a.validate()?;
    b.validate()?;

    let (a_ax, a_ay) = a.axes();
    let (b_ax, b_ay) = b.axes();
    let axes = [a_ax, a_ay, b_ax, b_ay];

    let corners_a = a.corners();
    let corners_b = b.corners();

    let mut min_overlap = f64::INFINITY;
    let mut min_axis = axes[0];

    for axis in axes {
        let (min_a, max_a) = project_points(&corners_a, axis);
        let (min_b, max_b) = project_points(&corners_b, axis);

        if min_a >= max_b || min_b >= max_a {
            return Ok(None);
        }

        let overlap = max_a.min(max_b) - min_a.max(min_b);
        if overlap <= 0.0 {
            return Ok(None);
        }

        if overlap < min_overlap {
            min_overlap = overlap;
            min_axis = axis;
        }
    }

    let delta = b.center.subtract(a.center);
    let normal = if delta.dot(min_axis) < 0.0 {
        min_axis.negate()
    } else {
        min_axis
    };

    let mut best_proj_a = -f64::INFINITY;
    let mut best_vert_a = corners_a[0];
    for v in corners_a {
        let proj = v.dot(normal);
        if proj > best_proj_a {
            best_proj_a = proj;
            best_vert_a = v;
        }
    }

    let mut best_proj_b = f64::INFINITY;
    let mut best_vert_b = corners_b[0];
    for v in corners_b {
        let proj = v.dot(normal);
        if proj < best_proj_b {
            best_proj_b = proj;
            best_vert_b = v;
        }
    }

    let contact_point = best_vert_a.add(best_vert_b).scale(0.5);

    Ok(Some(ContactManifold {
        normal,
        penetration: min_overlap,
        contact_point,
    }))
}

/// Raycast against an axis-aligned bounding box using the slab method.
pub fn raycast_aabb(ray: Ray2D, rect: Rect) -> Result<Option<SweepHit>, CollisionError> {
    ray.validate()?;
    validate_rect(rect)?;

    let r_min = rect.min_point();
    let r_max = rect.max_point();

    let (t_near_x, t_far_x, normal_near_x) = if ray.dir.x.abs() <= 1e-12 {
        if ray.origin.x < r_min.x || ray.origin.x > r_max.x {
            return Ok(None);
        }
        (-f64::INFINITY, f64::INFINITY, Point::origin())
    } else {
        let inv_d = 1.0 / ray.dir.x;
        let t1 = (r_min.x - ray.origin.x) * inv_d;
        let t2 = (r_max.x - ray.origin.x) * inv_d;
        if t1 > t2 {
            (t2, t1, Point::new(1.0, 0.0))
        } else {
            (t1, t2, Point::new(-1.0, 0.0))
        }
    };

    let (t_near_y, t_far_y, normal_near_y) = if ray.dir.y.abs() <= 1e-12 {
        if ray.origin.y < r_min.y || ray.origin.y > r_max.y {
            return Ok(None);
        }
        (-f64::INFINITY, f64::INFINITY, Point::origin())
    } else {
        let inv_d = 1.0 / ray.dir.y;
        let t1 = (r_min.y - ray.origin.y) * inv_d;
        let t2 = (r_max.y - ray.origin.y) * inv_d;
        if t1 > t2 {
            (t2, t1, Point::new(0.0, 1.0))
        } else {
            (t1, t2, Point::new(0.0, -1.0))
        }
    };

    let t_enter = t_near_x.max(t_near_y);
    let t_exit = t_far_x.min(t_far_y);

    if t_enter > t_exit || t_exit < 0.0 || t_enter > ray.max_toi {
        return Ok(None);
    }

    if t_enter < 0.0 {
        // Origin is inside AABB
        let normal = ray.dir.normalize().negate();
        Ok(Some(SweepHit {
            toi: 0.0,
            normal,
            contact_point: ray.origin,
        }))
    } else {
        let normal = if t_near_x >= t_near_y {
            normal_near_x
        } else {
            normal_near_y
        };
        Ok(Some(SweepHit {
            toi: t_enter,
            normal,
            contact_point: ray.point_at(t_enter),
        }))
    }
}

/// Raycast against a circle using quadratic intersection.
pub fn raycast_circle(ray: Ray2D, circle: Circle) -> Result<Option<SweepHit>, CollisionError> {
    ray.validate()?;
    circle.validate()?;

    let d = ray.dir;
    let m = ray.origin.subtract(circle.center);
    let a = d.magnitude_squared();
    let b = 2.0 * m.dot(d);
    let c = m.magnitude_squared() - circle.radius * circle.radius;

    if c <= 0.0 {
        // Ray starts inside circle
        let normal = ray.dir.normalize().negate();
        return Ok(Some(SweepHit {
            toi: 0.0,
            normal,
            contact_point: ray.origin,
        }));
    }

    if b > 0.0 {
        return Ok(None);
    }

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Ok(None);
    }

    let sqrt_disc = trig::sqrt(disc);
    let t = (-b - sqrt_disc) / (2.0 * a);

    if t < 0.0 || t > ray.max_toi {
        return Ok(None);
    }

    let contact_point = ray.point_at(t);
    let normal = contact_point.subtract(circle.center).normalize();

    Ok(Some(SweepHit {
        toi: t,
        normal,
        contact_point,
    }))
}

/// Continuous swept collision test between two axis-aligned bounding boxes.
pub fn sweep_aabb_aabb(
    moving: Rect,
    velocity: Point,
    target: Rect,
) -> Result<Option<SweepHit>, CollisionError> {
    validate_rect(moving)?;
    validate_point(velocity)?;
    validate_rect(target)?;

    if let Some(m) = aabb_vs_aabb(moving, target)? {
        return Ok(Some(SweepHit {
            toi: 0.0,
            normal: m.normal.negate(),
            contact_point: m.contact_point,
        }));
    }

    if velocity.magnitude_squared() <= 1e-24 {
        return Ok(None);
    }

    let minkowski = Rect::new(
        target.x - moving.width,
        target.y - moving.height,
        target.width + moving.width,
        target.height + moving.height,
    );
    let ray = Ray2D::new(moving.min_point(), velocity, 1.0)?;
    if let Some(hit) = raycast_aabb(ray, minkowski)? {
        if hit.toi <= 1.0 {
            let moving_center_at_hit = Point::new(
                moving.center().x + velocity.x * hit.toi,
                moving.center().y + velocity.y * hit.toi,
            );
            let cp_x = moving_center_at_hit.x.clamp(target.x, target.x + target.width);
            let cp_y = moving_center_at_hit.y.clamp(target.y, target.y + target.height);
            return Ok(Some(SweepHit {
                toi: hit.toi,
                normal: hit.normal,
                contact_point: Point::new(cp_x, cp_y),
            }));
        }
    }

    Ok(None)
}

/// Continuous swept collision test between a moving circle and an AABB (anti-tunneling).
pub fn sweep_circle_aabb(
    circle: Circle,
    velocity: Point,
    target: Rect,
) -> Result<Option<SweepHit>, CollisionError> {
    circle.validate()?;
    validate_point(velocity)?;
    validate_rect(target)?;

    if let Some(m) = circle_vs_aabb(circle, target)? {
        return Ok(Some(SweepHit {
            toi: 0.0,
            normal: m.normal.negate(),
            contact_point: m.contact_point,
        }));
    }

    if velocity.magnitude_squared() <= 1e-24 {
        return Ok(None);
    }

    let ray = Ray2D::new(circle.center, velocity, 1.0)?;
    let mut best_hit: Option<SweepHit> = None;

    // 1. Left slab: x = target.x - circle.radius
    if ray.dir.x > 1e-12 {
        let t = (target.x - circle.radius - ray.origin.x) / ray.dir.x;
        if (0.0..=1.0).contains(&t) {
            let y = ray.origin.y + t * ray.dir.y;
            if y >= target.y && y <= target.y + target.height {
                let hit = SweepHit {
                    toi: t,
                    normal: Point::new(-1.0, 0.0),
                    contact_point: Point::new(target.x, y),
                };
                if best_hit.as_ref().is_none_or(|b| hit.toi < b.toi) {
                    best_hit = Some(hit);
                }
            }
        }
    }

    // 2. Right slab: x = target.x + target.width + circle.radius
    if ray.dir.x < -1e-12 {
        let t = (target.x + target.width + circle.radius - ray.origin.x) / ray.dir.x;
        if (0.0..=1.0).contains(&t) {
            let y = ray.origin.y + t * ray.dir.y;
            if y >= target.y && y <= target.y + target.height {
                let hit = SweepHit {
                    toi: t,
                    normal: Point::new(1.0, 0.0),
                    contact_point: Point::new(target.x + target.width, y),
                };
                if best_hit.as_ref().is_none_or(|b| hit.toi < b.toi) {
                    best_hit = Some(hit);
                }
            }
        }
    }

    // 3. Top slab: y = target.y - circle.radius
    if ray.dir.y > 1e-12 {
        let t = (target.y - circle.radius - ray.origin.y) / ray.dir.y;
        if (0.0..=1.0).contains(&t) {
            let x = ray.origin.x + t * ray.dir.x;
            if x >= target.x && x <= target.x + target.width {
                let hit = SweepHit {
                    toi: t,
                    normal: Point::new(0.0, -1.0),
                    contact_point: Point::new(x, target.y),
                };
                if best_hit.as_ref().is_none_or(|b| hit.toi < b.toi) {
                    best_hit = Some(hit);
                }
            }
        }
    }

    // 4. Bottom slab: y = target.y + target.height + circle.radius
    if ray.dir.y < -1e-12 {
        let t = (target.y + target.height + circle.radius - ray.origin.y) / ray.dir.y;
        if (0.0..=1.0).contains(&t) {
            let x = ray.origin.x + t * ray.dir.x;
            if x >= target.x && x <= target.x + target.width {
                let hit = SweepHit {
                    toi: t,
                    normal: Point::new(0.0, 1.0),
                    contact_point: Point::new(x, target.y + target.height),
                };
                if best_hit.as_ref().is_none_or(|b| hit.toi < b.toi) {
                    best_hit = Some(hit);
                }
            }
        }
    }

    // Corner circles
    let corners = [
        Point::new(target.x, target.y),
        Point::new(target.x + target.width, target.y),
        Point::new(target.x, target.y + target.height),
        Point::new(target.x + target.width, target.y + target.height),
    ];
    for corner in corners {
        let corner_circle = Circle::new(corner, circle.radius)?;
        if let Some(hit) = raycast_circle(ray, corner_circle)? {
            if (0.0..=1.0).contains(&hit.toi) {
                let sweep_hit = SweepHit {
                    toi: hit.toi,
                    normal: hit.normal,
                    contact_point: corner,
                };
                if best_hit.as_ref().is_none_or(|b| sweep_hit.toi < b.toi) {
                    best_hit = Some(sweep_hit);
                }
            }
        }
    }

    Ok(best_hit)
}

/// Compute elastic or inelastic velocity reflection off a surface normal.
pub fn reflect_velocity(
    velocity: Point,
    normal: Point,
    restitution: f64,
) -> Result<Point, CollisionError> {
    validate_point(velocity)?;
    validate_point(normal)?;
    if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
        return Err(CollisionError::InvalidRestitution);
    }
    let n_sq = normal.magnitude_squared();
    if n_sq <= 1e-12 {
        return Err(CollisionError::DegenerateDirection);
    }
    let n = normal.normalize();
    let vn = velocity.dot(n);
    if vn >= 0.0 {
        return Ok(velocity);
    }
    Ok(velocity.subtract(n.scale((1.0 + restitution) * vn)))
}

/// Fast-path discrete collision between integer AABBs.
pub fn fixed_aabb_vs_aabb(
    a: FixedRectI64,
    b: FixedRectI64,
) -> Result<Option<FixedOverlapI64>, CollisionError> {
    a.validate()?;
    b.validate()?;

    let a_min_x = a.x;
    let a_max_x = a.x + a.width;
    let b_min_x = b.x;
    let b_max_x = b.x + b.width;
    let a_min_y = a.y;
    let a_max_y = a.y + a.height;
    let b_min_y = b.y;
    let b_max_y = b.y + b.height;

    if a_min_x >= b_max_x || b_min_x >= a_max_x || a_min_y >= b_max_y || b_min_y >= a_max_y {
        return Ok(None);
    }

    let ox = a_max_x.min(b_max_x) - a_min_x.max(b_min_x);
    let oy = a_max_y.min(b_max_y) - a_min_y.max(b_min_y);

    if ox <= 0 || oy <= 0 {
        return Ok(None);
    }

    if ox <= oy {
        let normal_x = if (2 * a.x + a.width) <= (2 * b.x + b.width) {
            1
        } else {
            -1
        };
        Ok(Some(FixedOverlapI64 {
            normal_x,
            normal_y: 0,
            penetration: ox,
        }))
    } else {
        let normal_y = if (2 * a.y + a.height) <= (2 * b.y + b.height) {
            1
        } else {
            -1
        };
        Ok(Some(FixedOverlapI64 {
            normal_x: 0,
            normal_y,
            penetration: oy,
        }))
    }
}

/// Fast-path discrete collision between an integer circle and integer AABB using 128-bit squaring.
pub fn fixed_circle_vs_aabb(
    c: FixedCircleI64,
    r: FixedRectI64,
) -> Result<Option<FixedOverlapI64>, CollisionError> {
    c.validate()?;
    r.validate()?;

    let r_min_x = r.x;
    let r_max_x = r.x + r.width;
    let r_min_y = r.y;
    let r_max_y = r.y + r.height;

    let closest_x = c.cx.clamp(r_min_x, r_max_x);
    let closest_y = c.cy.clamp(r_min_y, r_max_y);

    if closest_x == c.cx && closest_y == c.cy {
        let d_left = c.cx - r_min_x;
        let d_right = r_max_x - c.cx;
        let d_top = c.cy - r_min_y;
        let d_bottom = r_max_y - c.cy;

        if d_left <= d_right && d_left <= d_top && d_left <= d_bottom {
            Ok(Some(FixedOverlapI64 {
                normal_x: -1,
                normal_y: 0,
                penetration: c.radius + d_left,
            }))
        } else if d_right <= d_top && d_right <= d_bottom {
            Ok(Some(FixedOverlapI64 {
                normal_x: 1,
                normal_y: 0,
                penetration: c.radius + d_right,
            }))
        } else if d_top <= d_bottom {
            Ok(Some(FixedOverlapI64 {
                normal_x: 0,
                normal_y: -1,
                penetration: c.radius + d_top,
            }))
        } else {
            Ok(Some(FixedOverlapI64 {
                normal_x: 0,
                normal_y: 1,
                penetration: c.radius + d_bottom,
            }))
        }
    } else {
        let dx = (closest_x - c.cx) as i128;
        let dy = (closest_y - c.cy) as i128;
        let dist_sq = (dx * dx + dy * dy) as u128;
        let rad = c.radius as u128;
        let rad_sq = rad * rad;

        if dist_sq > rad_sq {
            return Ok(None);
        }

        let dist = isqrt_u128(dist_sq) as i64;
        let penetration = c.radius - dist;

        let (normal_x, normal_y) = if dx.abs() >= dy.abs() {
            (if dx >= 0 { 1 } else { -1 }, 0)
        } else {
            (0, if dy >= 0 { 1 } else { -1 })
        };

        Ok(Some(FixedOverlapI64 {
            normal_x,
            normal_y,
            penetration,
        }))
    }
}
