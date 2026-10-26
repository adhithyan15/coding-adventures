# Changelog

## 0.1.0

- Add the checked `Esp32Lx6IntegerV1` functional core from Spec 07az.
- Add exact little-endian mixed 16-/24-bit transport, complete physical-window
  state, deterministic traces, and typed transactional lifecycle APIs.
- Cover the bounded core, Code Density, Loop, and problem-state Windowed
  Register surface while preserving explicit unsupported, privileged, and
  raw-reserved boundaries.
- Preserve the ISA's caller-read/callee-write `ENTRY` register mapping and
  lazy-window priority over `MOVSP` allocation faults.
- Add generated family classification, semantic, lifecycle, atomicity, and
  coverage evidence.
