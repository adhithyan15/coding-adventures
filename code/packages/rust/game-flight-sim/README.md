# `game-flight-sim` (`GAME45`)

Pure Tier 1 deterministic 6-DOF flight simulator integrating the `GAME40`–`GAME44` Era 4 stack (`rigidbody-6dof`, `aero-flight-model`, `terrain-lod`, `cockpit-hud-paint`, `render3d-gpu-bridge`), `game-clock`, and explicit `rng::Pcg32` turbulence over caller-owned state, scratch buffers, and zero-allocation hot paths (`VER07`).

## Architecture

- **Explicit 6-DOF Flight State Machine (`VER04m` `state=explicit`)**:
  `FlightSimState` embeds `RigidBodyState`, `MassProperties`, `AircraftModel` (`aircraft_preset_trainer` / `aircraft_solve_level_trim`), `FlightEnvironment`, `ControlInputs`, `GroundContactParams`, `IntegratorKind`, `game_clock::GameClockState`, explicit `rng::Pcg32`, monotonic non-negative `fuel_kg` (flameout at `0.0`), `gear_down`, `flaps_norm`, `brake_norm`, and `FlightPhase` (`OnRunway`, `TakeoffRoll`, `Airborne`, `Approach`, `Landed`, `Crashed`).
- **Terrain-Coupled Ground Contact**:
  Queries `Heightfield::height_at` / `normal_at` for AGL and local ground planes fed into `step_with_contacts_report_in_place` and `step_aircraft_in_place`, supporting takeoff roll, runway landing, gear-up/hard-impact crashes, and bounded ceiling/world envelopes.
- **Black-Box Flight Data Recorder (FDR) & Bit-Exact Replay**:
  Pre-allocated `BlackBoxRecorder` (`<= MAX_BLACK_BOX_FRAMES = 65_536`), hardened `GFDR` v1 binary codec (`encode_black_box_into` / `decode_black_box_into` / `decode_black_box_from_bytes` with magic, version, exact-length, FNV-1a checksum, and scalar/enum validation), and `replay_black_box_verify` proving bit-exact `state_digest_fnv1a` parity on every tick.
- **Integrated 3D Terrain + Cockpit HUD Pipeline**:
  `render_frame_into` pre-validates every input, target, scratch capacity, and optional retained `PaintScene` before mutating any output buffer (byte-for-byte atomic on `Err`), executes `terrain_lod::refine_into` + `emit_mesh_into` -> `render3d_gpu_bridge::execute_render_pass_into`, and updates `HudScratch` + retained `PaintScene` with zero heap allocations.

## Usage Example

```rust
use cockpit_hud_paint::{prepare_paint_scene, HudConfig};
use game_flight_sim::{
    render_frame_into, replay_black_box_verify, sim_init_on_runway, step_in_place,
    BlackBoxRecorder, CameraMode, FlightInputFrame, FlightRenderScratch, FlightSimConfig,
};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    BackendPreference, ClipSpace, DepthBufferMut, PostProcessMode, RenderTargetSpec,
};
use terrain_lod::{Heightfield, HeightfieldMut, HeightfieldSpec, LodParams, RunwaySpec};

let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).unwrap();
let mut heights = vec![0.0_f64; spec.spec_vertex_count().unwrap()];
let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1400.0, 80.0, 20.0, 0.0).unwrap();
HeightfieldMut::from_heights_mut(spec, &mut heights).unwrap().place_runway(&runway).unwrap();
let hf = Heightfield::from_heights(spec, &heights).unwrap();

let cfg = FlightSimConfig::sim_config_default(42, 42);
let initial = sim_init_on_runway(cfg, &runway).unwrap();
let header = initial.sim_state_black_box_header().unwrap();
let mut recorder = BlackBoxRecorder::recorder_with_capacity(header, 64).unwrap();
let mut sim = initial.clone();

let input = FlightInputFrame {
    dt_ns: cfg.fixed_dt_ns,
    elevator: 0.0,
    aileron: 0.0,
    rudder: 0.0,
    throttle: 1.0,
    flaps: 0.0,
    brake: 0.0,
    gear_down: true,
};
step_in_place(&mut sim, &hf, &runway, &input, Some(&mut recorder)).unwrap();
replay_black_box_verify(&initial, &hf, &runway, &recorder).unwrap();
```
