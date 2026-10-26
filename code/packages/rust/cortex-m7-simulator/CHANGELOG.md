# Changelog

## 0.1.0 — 2026-10-25

- Add a checked Cortex-M7 functional boundary built on the complete Cortex-M3
  state and instruction surface.
- Add QADD, QSUB, SADD8, and UADD8 with exact Armv7E-M encodings, sticky Q,
  GE[3:0], IT-block behavior, and complete traces.
- Add typed transactional load, restore, step, bounded run, and execute
  lifecycle operations.
- Keep caches, TCM, branch prediction, buses, and timing at an explicit later
  SoC/gate-level composition boundary.
- Cover 258 of 262 executable source lines (98.47%) in the completion suite.
