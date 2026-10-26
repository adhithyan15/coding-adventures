# Changelog

## 0.1.0 — 2026-10-26

- Add a bounded Cortex-M33 functional simulator that inherits the checked
  Cortex-M3 integer instruction surface.
- Add optional Armv8-M Security Extension state, banked Mainline control and
  stack-limit registers, eight-region SAU attribution, checked security
  transitions, complete traces, and transactional lifecycle operations.
- Keep 4-byte-aligned stack pointers restorable independently of 8-byte-aligned
  stack-limit registers, enforce Secure-bank privilege, and roll back a run if
  its fallible final-state snapshot cannot be allocated.
