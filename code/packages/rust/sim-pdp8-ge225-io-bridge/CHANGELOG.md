# Changelog

All notable changes to the `sim-pdp8-ge225-io-bridge` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `sim-pdp8-ge225-io-bridge`: Era 0 functional and gate-level peripheral I/O controller connecting PDP-8 and GE-225 to the Teletype Model 33 ASR (GAME03b).
- Pure Rust implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures & decoders:
  - `IotDecoderInput` / `IotDecoderOutput`: zero-alloc interface for 12-bit PDP-8 IOT instruction decoding.
  - `FunctionalIotDecoder`: zero-heap-allocation functional IOT instruction decoder for devices `0o03` (Keyboard) and `0o04` (Teleprinter).
  - `GateLevelIotDecoder`: cycle-accurate gate-level digital logic decoder constructed from `logic_gates` primitives (`and_gate`, `or_gate`, `not_gate`, `mux2`, 6-bit comparator, pulse decoders, and 12-bit AC bus multiplexer).
  - `Pdp8TeletypeBridge`: high-level bridge orchestrating `Pdp8Simulator` with `Asr33Functional`.
  - `Pdp8TeletypeGateLevelBridge`: low-level bridge orchestrating `Pdp8GateLevel` with `Asr33GateLevel`.
  - `Ge225TeletypeBridge`: high-level bridge connecting GE-225 typewriter BCD stream to `Asr33Functional` and `Asr33GateLevel`.
- PDP-8 KL8-E / W076 Teletype Instruction Set:
  - `KSF` (`0o6031`), `KCC` (`0o6032`), `KRS` (`0o6034`), `KRB` (`0o6036`).
  - `TSF` (`0o6041`), `TCF` (`0o6042`), `TPC` (`0o6044`), `TLS` (`0o6046`).
  - Interrupt request assertion on `kb_flag || tp_flag`.
- GE-225 BCD Character Code Translation:
  - Bidirectional conversion between standard 7-bit ASCII and GE-225 6-bit BCD (`0o00..=0o76`), with lowercase folding, space handling, and unknown character fallback.
- Machine-Code & Telemetry Integration:
  - `build_pdp8_teletype_print_program`: generates machine-code routines polling teleprinter flags to print strings from auto-indexed vectors.
  - End-to-end integration tests running authentic `hamurabi` and `lunar-lander` game telemetry through both functional and gate-level PDP-8 and GE-225 simulation stacks.
- Verification and Assurance:
  - Exhaustive differential equivalence test suite comparing functional and gate-level decoders across all device IDs, pulse masks, and AC words.
  - End-to-end differential equivalence tests asserting identical CPU state (`Pdp8State`), cycle counts, and ASCII paper-roll output across `Pdp8Simulator` and `Pdp8GateLevel`.
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) on `CountingAllocator`.
  - `VER08b` runtime capability confinement tests (`tests/runtime_capability.rs`).
  - 26-CWE weakness closure matrix in `assurance_report.json`.
  - Effect certificate (`effect_certificate.json`) verified with zero ambient authority (`"capabilities": []`).
