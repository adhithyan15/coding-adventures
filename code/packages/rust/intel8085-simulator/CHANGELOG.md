# Changelog — intel8085-simulator

## [0.1.0] - Unreleased

### Added

- All 246 Intel-documented 8085 opcode encodings: the complete shared 8080
  surface plus 8085-only RIM and SIM.
- Full vectored interrupt controller with TRAP, RST 7.5, RST 6.5, RST 5.5,
  INTR/RST acknowledge, documented priority, masks, pending latches, stack
  entry, global interrupt disable, and halt release.
- Correct delayed EI and immediate DI semantics.
- SID input, SOD output, mask-set enable, SOD enable, and RST 7.5 reset
  behavior.
- Checked program transport at arbitrary 16-bit origins, exact fetch bounds,
  complete owned state, exact traces, bounded execution, and transactional
  program runs.
- A memory-derived retained-trace budget for bounded runs, with atomic
  rejection of unsafe explicit limits and streaming through `step` for long
  executions.
- Typed atomic failures for oversized images, out-of-image and truncated
  fetches, undocumented opcodes, post-halt steps, short-memory data/stack
  references, and unsupported INTR acknowledge bytes.
- Exhaustive first-byte and focused lifecycle tests, package BUILD integration,
  specification, and README.

### Verified

- 13 release integration suites pass.
- Line coverage is 98.95% (377/381), region coverage is 97.71%, and function
  coverage is 100%.
- Formatting, strict Clippy, and strict rustdoc pass.
