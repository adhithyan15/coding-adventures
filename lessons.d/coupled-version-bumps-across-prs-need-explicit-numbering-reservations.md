---
category: Compiler / VM / language pipeline
---

# Coupled version bumps across PRs need explicit numbering reservations

When Phase 34 (TS) was being pushed before Phase 29-33 (TS) had merged, the natural `0.5.0 → 0.6.0` bump would have collided with PR #3468's `0.5.0 → 0.6.0`. Fix: bump Phase 34 to `0.7.0` directly (skipping 0.6.0) and call out the reservation in the CHANGELOG note ("leaves 0.6.0 for the in-flight Phase 29-33 port"). Rebase merge order — Phase 29-33 first, then Phase 34 — produces a clean 0.5.0 → 0.6.0 → 0.7.0 history without per-PR conflicts.
