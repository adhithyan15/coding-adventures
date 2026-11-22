# cockpit-hud-paint

Deterministic, zero-allocation Primary Flight Display (PFD) and Heads-Up Display (HUD) vector overlay renderer for Rust (`GAME43`), lowering to `P2D00` [`paint-instructions`](../paint-instructions/README.md) and verified against [`paint-vm-runtime`](../paint-vm-runtime/README.md).

Part of the **Coding Adventures** Era 4 flight simulation stack alongside [`rigidbody-6dof`](../rigidbody-6dof/README.md) (`GAME40`) and [`trig`](../trig/README.md) (`PHY00`).

---

## Overview

1. **PFD & HUD Vector Overlays (`HudDisplayMode::CombinedPfdHud`, `PfdOnly`, `HudOverlayOnly`):**
   - **Attitude Director Indicator (ADI):** Clipped sky/ground half-plane polygons (Sutherland-Hodgman), artificial horizon line, bank angle arc (`-60°..+60°`) + rotating bank pointer, and clipped pitch ladder rungs (`-90°..+90°`, solid above / dashed below).
   - **Airspeed & Altitude Tapes:** Indicated Airspeed (IAS) tape in knots or m/s + Mach readout; barometric Altimeter tape (feet or m) + Vertical Speed Indicator (VSI) bar + Radar Altitude (AGL).
   - **HSI / Heading Tape & Flight-Path Marker (FPM):** Wrapping `000°..359°` heading tape with lubber mark, boresight watermark, and clamped FPM velocity vector.
   - **AoA / G-Load & Annunciators:** `STALL`, `OVERSPEED`, `TERRAIN`, and `PULL UP` warning banners driven by telemetry flags or configurable thresholds.
2. **Deterministic Math & Clipping:**
   - Uses `trig::{sin, cos, atan2, sqrt, radians, degrees}` exclusively (zero `std` `libm` calls).
   - Wraps all input angles (`|x| <= MAX_ANGLE_RAD = 2^29`), clips 2D segments via Liang-Barsky, and canonicalizes `-0.0` and `NaN` in 64-bit FNV-1a frame/scene digests.
3. **Zero-Allocation Hot Loop (`VER07`):**
   - `build_hud_frame_into(&telemetry, &config, &mut scratch)` renders into caller-owned `HudScratch` using stack-formatted numeric labels (zero `format!()`).
   - `prepare_paint_scene(&config)` + `update_paint_scene_into(&telemetry, &config, &mut scene)` updates a retained `PaintScene` with stable `PaintBase.id`s in-place (`max_alloc_bytes: Some(0)`).
4. **Rigid-Body Bridge (`GAME40`):**
   - `FlightTelemetry::telemetry_from_rigid_body(&body, &mass, speed_of_sound, terrain_elevation)` derives HUD telemetry directly from `rigidbody_6dof::RigidBodyState`.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
- **Atomic Error Semantics & CWE Closure:** Validates viewport bounds (`64..=8192`), telemetry domains, angle bounds (`|x| <= 2^29`), resource caps, and buffer capacity before mutating `HudScratch` or `PaintScene`.

---

## Usage

```rust
use cockpit_hud_paint::{
    build_hud_frame_into, prepare_paint_scene, update_paint_scene_into,
    FlightTelemetry, HudConfig, HudScratch,
};

let cfg = HudConfig::hud_config_default(1280.0, 720.0)?;
let tel = FlightTelemetry::telemetry_level_flight();

let mut scratch = HudScratch::hud_scratch_new();
let summary = build_hud_frame_into(&tel, &cfg, &mut scratch)?;
assert!(summary.primitive_count > 0);

let mut scene = prepare_paint_scene(&cfg)?;
let _retained = update_paint_scene_into(&tel, &cfg, &mut scene)?;
# Ok::<(), cockpit_hud_paint::HudError>(())
```

---

## Testing

```text
cargo test -p cockpit-hud-paint -- --nocapture
cargo test -p cockpit-hud-paint --release -- --nocapture
cargo clippy -p cockpit-hud-paint --all-targets -- -D warnings
```
