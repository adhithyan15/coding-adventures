# Changelog

## 0.1.0 — 2026-10-26

- Add a bounded Cortex-M33 functional simulator that inherits the checked
  Cortex-M3 integer instruction surface.
- Add optional Armv8-M Security Extension state, banked Mainline control and
  stack-limit registers, eight-region SAU attribution, checked security
  transitions, complete traces, and transactional lifecycle operations.
- Keep 4-byte-aligned stack pointers restorable independently of 8-byte-aligned
  stack-limit registers, enforce privileged limit writes and both MSP/PSP limit
  pairs in either security bank, and roll back a run if its fallible final-state
  snapshot cannot be allocated.
- Apply ALLNS only while the SAU is disabled, reduce overlapping region matches
  to the most-secure attribute, and distinguish Secure data access from Secure
  instruction fetch so ordinary Non-secure code requires an explicit checked
  transition whose target must also own a complete minimum Thumb fetch range.
