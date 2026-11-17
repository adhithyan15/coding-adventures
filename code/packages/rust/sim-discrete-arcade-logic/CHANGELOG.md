# Changelog

All notable changes to the `sim-discrete-arcade-logic` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `code/specs/sim-discrete-arcade-logic.warrant` was ill-typed Warrant (Nat declared, Int produced by `val + 1`); the three advance models now return Int. Document only.

## 0.1.0 — 2026-11-15

### Added
- Initial release of `sim-discrete-arcade-logic`: gate-level netlist simulator for 7400-series TTL ICs, raster sync generator, and paddle ADC (GAME15a).
- Pure Rust implementation under `#![forbid(unsafe_code)]` with zero dynamic heap allocations.
- Full dual-parity (functional and gate-level via `logic-gates`) implementation for classic 7400-series TTL ICs:
  - `Ttl7400QuadNand`: 4x 2-input NAND gates with channel bounds and input logic-level validation.
  - `Ttl7402QuadNor`: 4x 2-input NOR gates with channel bounds and input logic-level validation.
  - `Ttl7474DualDFlipFlop`: dual edge-triggered D flip-flop with asynchronous active-low preset and clear.
  - `Ttl7490DecadeCounter`: BCD decade counter (divide-by-2 and divide-by-5) with gated zero and nine reset inputs.
  - `Ttl7493BinaryCounter`: 4-bit ripple binary counter (divide-by-2 and divide-by-8) with dual reset inputs.
  - `Ttl74153DualMux4to1`: dual 4-to-1 data multiplexer with active-low strobes.
- `ArcadeSyncGenerator`:
  - Master clock divider chain dividing 14.31818 MHz master clock down to 7.15909 MHz pixel clock.
  - Historical 15.75 kHz horizontal line rate (455 pixel clocks per scanline) and 60 Hz vertical frame rate (262 scanlines per frame).
  - Exact HSync, VSync, HBlank, VBlank intervals and `(dot_x, scanline_y)` raster coordinates.
  - Gate-level parity implementation cascading simulated counter and flip-flop blocks.
- `PaddlePotAdc`:
  - Deterministic fixed-point / cycle-counted R-C charging ramp converting paddle potentiometer resistance / normalized position `[0, 65535]` into raster trigger pulses.
  - Models 555-timer / R-C comparator threshold at 2/3 Vcc using zero-drift integer arithmetic.
- Complete security and robustness defenses:
  - CWE-20: Pin, channel, and parameter bounds validation.
  - CWE-369: Clock, capacitance, and resistance zero-division guards.
  - CWE-190: Saturating cycle counter arithmetic and overflow guards.
- Verification and Assurance:
  - Formal verification warrant specification (`code/specs/sim-discrete-arcade-logic.warrant`).
  - Exhaustive conformance and differential equivalence test suite (`tests/conformance.rs`).
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) with `CountingAllocator` and sabotage detection.
  - `VER08b` runtime capability confinement test (`tests/runtime_capability.rs`).
