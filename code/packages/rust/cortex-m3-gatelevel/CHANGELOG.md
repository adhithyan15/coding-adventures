# Changelog

## Unreleased

- Add a narrow gate-to-gate integration boundary with preallocated fallible
  snapshots, non-committing delegated traces, validated candidates, explicit
  commit, deterministic allocation failure, and production-derived DFF
  inventory for the Cortex-M7 gate companion.
- Revalidate the complete candidate at the final fallible commit boundary so
  post-settle mutation cannot clock invalid values into authoritative DFF Q.
- Refresh the combined Rust line-coverage receipt to 92.06% (2,343/2,545)
  after adding the hostile external-consumer commit regression.

## 0.1.0 — 2026-10-25

- Add an independent Armv7-M Cortex-M3 gate transition engine.
- Store the complete architectural state in exactly 524,952 repository D
  flip-flops and derive every production transition through repository gates.
- Add typed atomic lifecycle faults, fallible bounded complete-state traces,
  and exact functional-oracle differential coverage for every implemented
  instruction subfamily.
- Cover IT state, low-power/event latches, BASEPRI/FAULTMASK, wide Thumb data
  and memory operations, table branches, and exclusive-monitor lifecycles.
- Reach 92.12% combined Rust line coverage (2,323 of 2,506 lines) at the
  original 0.1.0 boundary.
