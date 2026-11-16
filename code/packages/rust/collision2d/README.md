# collision2d

Deterministic 2D arcade collision detection and continuous swept primitives for historical game engines.

Part of the **Coding Adventures** polyglot monorepo historical game engine collection (`GAME11`).

---

## Historical Lineage

- **1972 — Pong:** Nolan Bushnell and Allan Alcorn at Atari created the seminal table-tennis simulation where the ball rebounded elastically off paddle segments and arena boundaries. Early discrete collision detection suffered from ball tunneling at high velocities.
- **1976 — Breakout:** Designed by Nolan Bushnell and Steve Wozniak, introducing ball reflection off stationary brick matrices and angled paddle corners.
- **Continuous Swept Collision:** Continuous collision detection (CCD) solves the tunneling failure mode by computing exact time-of-impact (TOI) over the velocity displacement interval $[0, 1]$, treating moving bodies as swept volumes (e.g. Minkowski sum slabs and corner circles).

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Anti-Tunneling Continuous Sweep:** `sweep_circle_aabb` and `sweep_aabb_aabb` compute exact parametric time of impact `toi in [0.0, 1.0]`, guaranteeing high-speed projectiles never pass through thin paddles or barriers.
- **Stack-Only Value Semantics:** All geometric primitives (`Circle`, `Obb2D`, `Ray2D`, `ContactManifold`, `SweepHit`, `FixedRectI64`, `FixedCircleI64`) derive `Copy` and `Clone`.
- **Zero Allocation Budget (`VER07`):** All hot-path queries (`aabb_vs_aabb`, `circle_vs_aabb`, `obb_vs_obb_sat`, `sweep_circle_aabb`) allocate **zero heap memory** (`max_alloc_bytes: Some(0)`).
- **Deterministic 64-bit Integer Fast Path:** `fixed_aabb_vs_aabb` and `fixed_circle_vs_aabb` compute integer overlaps using `i128` squaring and integer square roots, producing bit-exact cross-platform results with zero floating-point drift.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

---

## Core Primitives & API

### 1. Geometric Types

```rust
pub use point2d::{Point, Rect};
pub use affine2d::Affine2D;

pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

pub struct Obb2D {
    pub center: Point,
    pub half_extents: Point,
    pub rotation_rad: f64,
}

pub struct Ray2D {
    pub origin: Point,
    pub dir: Point,
    pub max_toi: f64,
}

pub struct ContactManifold {
    pub normal: Point,          // Unit vector pointing from shape A toward shape B
    pub penetration: f64,       // Overlap depth (>= 0.0)
    pub contact_point: Point,   // World contact position
}

pub struct SweepHit {
    pub toi: f64,               // Time of impact in [0.0, 1.0]
    pub normal: Point,          // Surface outward normal
    pub contact_point: Point,   // Impact point
}
```

### 2. Primary Collision Functions

- `aabb_vs_aabb(a: Rect, b: Rect) -> Result<Option<ContactManifold>, CollisionError>`
- `circle_vs_circle(a: Circle, b: Circle) -> Result<Option<ContactManifold>, CollisionError>`
- `circle_vs_aabb(circle: Circle, rect: Rect) -> Result<Option<ContactManifold>, CollisionError>`
- `obb_vs_obb_sat(a: Obb2D, b: Obb2D) -> Result<Option<ContactManifold>, CollisionError>`
- `raycast_aabb(ray: Ray2D, rect: Rect) -> Result<Option<SweepHit>, CollisionError>`
- `raycast_circle(ray: Ray2D, circle: Circle) -> Result<Option<SweepHit>, CollisionError>`
- `sweep_aabb_aabb(moving: Rect, velocity: Point, target: Rect) -> Result<Option<SweepHit>, CollisionError>`
- `sweep_circle_aabb(circle: Circle, velocity: Point, target: Rect) -> Result<Option<SweepHit>, CollisionError>`
- `reflect_velocity(velocity: Point, normal: Point, restitution: f64) -> Result<Point, CollisionError>`
- `fixed_aabb_vs_aabb(a: FixedRectI64, b: FixedRectI64) -> Result<Option<FixedOverlapI64>, CollisionError>`
- `fixed_circle_vs_aabb(c: FixedCircleI64, r: FixedRectI64) -> Result<Option<FixedOverlapI64>, CollisionError>`

---

## Example Usage

```rust
use collision2d::{
    sweep_circle_aabb, reflect_velocity, Circle, Point, Rect,
};

fn main() {
    let paddle = Rect::new(200.0, 80.0, 8.0, 40.0);
    let ball = Circle::new(Point::new(0.0, 100.0), 4.0).unwrap();
    let velocity = Point::new(500.0, 0.0);

    // Continuous sweep query catches high-speed ball mid-flight
    if let Some(hit) = sweep_circle_aabb(ball, velocity, paddle).unwrap() {
        println!("Hit paddle at TOI = {:.3} normal = {:?}", hit.toi, hit.normal);
        // Elastic rebound (restitution = 1.0)
        let rebound = reflect_velocity(velocity, hit.normal, 1.0).unwrap();
        println!("Rebound velocity: {:?}", rebound);
    }
}
```

---

## Building and Testing

```bash
# Debug profile unit and integration tests
cargo test -p collision2d

# Release profile verification
cargo test -p collision2d --release

# Clippy linter enforcement
cargo clippy -p collision2d --all-targets -- -D warnings

# Static capability verification
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
```
