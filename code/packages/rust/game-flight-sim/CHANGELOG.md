# Changelog — `game-flight-sim`

## [0.1.0] - 2026-11-21

### Added
- Implemented `GAME45` pure Tier 1 deterministic 6-DOF flight simulator (`capabilities: []`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`) uniting `rigidbody-6dof`, `aero-flight-model`, `terrain-lod`, `cockpit-hud-paint`, `render3d-gpu-bridge` (`gpu-runtime` `default-features = false`), `game-clock`, `rng`, `math3d`, `trig`, `pixel-container`, and `paint-instructions`.
- Explicit 6-DOF flight state machine (`FlightSimState`, `FlightPhase`, `sim_init_on_runway`, `sim_init_level_flight`, `sim_init_from_black_box_header`, `step_in_place`, `step`) with terrain-coupled ground contact, monotonic fuel burn and flameout at `0.0`, deterministic `rng::Pcg32` gusts, and byte-for-byte atomic rollback on `Err`.
- Pre-allocated `BlackBoxRecorder` (`<= MAX_BLACK_BOX_FRAMES = 65_536`), hardened `GFDR` v1 binary codec (`encode_black_box_into`, `decode_black_box_into`, `decode_black_box_from_bytes`), and bit-exact `replay_black_box_verify`.
- Integrated zero-allocation 3D terrain + cockpit HUD pipeline (`FlightRenderScratch`, `render_frame_into`, `telemetry_from_sim_state`, `build_flight_camera`) with upfront atomic validation and `CameraUniforms` matrix entry bounds `<= render3d_gpu_bridge::MAX_MATRIX_ENTRY`.
- Warrant contract `code/specs/game-flight-sim.warrant` and matching boundary validators (`validate_fdr_frame_count`, `validate_fdr_blob_bytes`, `validate_dt_ns`, `validate_altitude_envelope_m`, `validate_fuel_mg`, `validate_render_dim`).
- Integration suites: `tests/conformance.rs`, `tests/hostile_inputs.rs` (`#429` hostile vectors + 2,000-case fuzz campaign), `tests/microbenchmarks.rs` (`CountingAllocator` `VER07` zero-alloc proofs), and `tests/runtime_capability.rs` (`VER08b` seccomp gate).
- Formal artifacts: `required_capabilities.json`, `src/capabilities.rs`, `effect_certificate.json`, `assurance-profile.json`, `assurance_report.json`, `BUILD`, `BUILD_linux`, and `BUILD_windows`.
