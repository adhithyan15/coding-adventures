# Changelog

All notable changes to `cockpit-hud-paint` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-21

### Added
- Initial implementation of `GAME43: cockpit-hud-paint` deterministic zero-allocation PFD and HUD vector overlay renderer lowering to `P2D00` `PaintScene`.
- Self-contained `FlightTelemetry` and `HudConfig` with `telemetry_from_rigid_body` bridge to `rigidbody-6dof::RigidBodyState`.
- Full PFD/HUD instruments: Sutherland-Hodgman sky/ground horizon split, Liang-Barsky clipped pitch ladder (`-90°..+90°`) and bank angle arc (`-60°..+60°`), IAS/Mach airspeed tape, MSL altimeter + VSI + radar altitude AGL, HSI heading tape, clamped Flight-Path Marker (FPM), AoA/G-load readouts, and `STALL` / `OVERSPEED` / `TERRAIN` / `PULL UP` annunciators.
- Zero-allocation hot paths (`build_hud_frame_into` with `HudScratch` and stack formatting; `prepare_paint_scene` + `update_paint_scene_into` with retained `PaintScene` and stable `PaintBase.id`s) verified via `CountingAllocator` (`max_alloc_bytes: Some(0)`).
- Exclusive `trig::{sin, cos, atan2, sqrt, radians, degrees}` usage (zero `std` `libm`), angle range reduction (`|x| <= 2^29`), and canonical `-0.0` / `NaN` FNV-1a frame/scene digests.
- Security controls: `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, zero ambient capabilities (`CAP001`), byte-for-byte atomic rollback on `Err` / `BufferTooSmall`.
- Conformance (`tests/conformance.rs`), `#429` hostile-input CVE proof suite with 2,000-case fuzz campaign (`tests/hostile_inputs.rs`), zero-alloc microbenchmarks (`tests/microbenchmarks.rs`), and runtime capability gate (`tests/runtime_capability.rs`).
- Formal verification artifacts: `code/specs/cockpit-hud-paint.warrant`, `assurance-profile.json`, `assurance_report.json`, and `effect_certificate.json`.
