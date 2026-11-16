# Changelog

All notable changes to the `sim-teletype-asr33` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `sim-teletype-asr33`: cycle-accurate Teletype Model 33 ASR simulator with both a pure zero-alloc functional model and a gate-level digital-logic model, plus 72-column paper-roll rendering via `paint-vm-ascii` (GAME03a).
- Pure Rust implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures:
  - `Asr33FrameState`: Copy struct tracking 11-bit transmit/receive shift registers, logical wire levels, KRB keyboard buffer, TTO teleprinter buffer, flags (`kb_flag`, `tp_flag`), platen carriage column (`0..=71`), line row counter, bell ring counter (`bel_count`), and last printed character.
  - `Asr33Functional`: complete zero-alloc functional simulator managing an in-memory 64-line circular paper roll buffer with declarative `PaintScene` generation.
  - `Asr33GateLevel`: cycle-accurate gate-level digital logic model constructed from `logic_gates` primitives (`shift_register`, `d_flip_flop`, `FlipFlopState`, and a 7-XOR balanced even-parity generator tree).
- 110-Baud 11-Bit Serial Frame Engine:
  - Start bit (0, Space), 7-bit ASCII data (`b0..b6`, LSB first), even parity bit (bit 7/8), and 2 Stop bits (1, 1, Mark).
  - Idle wire holds Mark (1).
- 72-Column Platen Mechanics:
  - Physical 72-column carriage advancement (`0..=71`) with automatic wrap to column 0 on the next line when column 72 is reached.
  - Control characters: Carriage Return (`0x0D` CR) carriage reset, Line Feed (`0x0A` LF) line advancement, and Bell (`0x07` BEL) electromechanical ring counter.
  - Teletype uppercase typewheel folding: lowercase letters `'a'..='z'` automatically fold to `'A'..='Z'`.
- Declarative Paper-Roll Telemetry (`paint-instructions` & `paint-vm-ascii` / `P2D02`):
  - `to_paint_scene`: generates a 72-column 2D paint scene containing full uppercase text and a filled 1x1 block carriage print-head cursor (`█`).
  - `render_paper_roll_ascii`: renders the paper roll and carriage cursor into terminal-friendly ASCII.
- Verification and Assurance:
  - Exhaustive 256-byte differential equivalence test suite between `Asr33Functional` and `Asr33GateLevel` across all 11 serial bit times, wire levels, and flags.
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) using `CountingAllocator` and sabotage detection.
  - `VER08b` runtime capability confinement tests (`tests/runtime_capability.rs`).
  - Zero ambient capability declaration in `required_capabilities.json` (`capabilities: []`) verified by `ca-capability-analyzer`.
  - Effect certificate (`effect_certificate.json`) proved zero side effects across 45 entry points.
  - Full 26-CWE weakness closure matrix documented in `assurance_report.json` per `VER15`.
