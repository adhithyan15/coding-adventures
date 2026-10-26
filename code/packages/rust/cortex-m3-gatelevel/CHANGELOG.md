# Changelog

## 0.1.0 — 2026-10-25

- Add an independent Armv7-M Cortex-M3 gate transition engine.
- Store the complete architectural state in exactly 524,952 repository D
  flip-flops and derive every production transition through repository gates.
- Add typed atomic lifecycle faults, fallible bounded complete-state traces,
  and exact functional-oracle differential coverage for every implemented
  instruction subfamily.
- Cover IT state, low-power/event latches, BASEPRI/FAULTMASK, wide Thumb data
  and memory operations, table branches, and exclusive-monitor lifecycles.
- Reach 92.12% combined Rust line coverage (2,323 of 2,506 lines).
