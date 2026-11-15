# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Pure, deterministic discrete lunar landing flight simulation core (GAME02) in Ruby.
- Pure integer fixed-point physics with 64-bit and 128-bit integer arithmetic.
- Exact `trunc_div`, `div_euclid`, and `isqrt_u128` operations with bit-level cross-language parity.
- Second-order Euler-Verlet kinematics and ground touchdown contact time solver.
- Declarative 76x5 `PaintScene` telemetry HUD and `paint-vm-ascii` character-grid rasterizer.
- Strict type validation and 26-CWE weakness closure (CWE-843, CWE-190, CWE-369, CWE-502).
- Zero ambient capability cage (`"capabilities": []`).
- Complete conformance suite covering all 39 cases in `cases.json`.
