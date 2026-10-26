# Changelog

## 0.1.0

- Add the exact 526,469-DFF gate-level partner for Spec 07v.
- Add independent repository-gate AArch64 decode and execution for the full
  documented integer surface.
- Share the functional simulator's typed atomic lifecycle, complete state,
  traces, results, and faults.
- Add topology, lifecycle, malformed-decode, functional lockstep, and all 836
  reproducible Python full-state differential vectors.
- Make the 32-bit ADD/SUB carry and overflow flags come directly from an exact
  32-bit ripple network instead of reconstructing carry with a host comparison.
- Pass strict formatting, Clippy, and rustdoc with 97.79% package line coverage
  (1,105/1,130); retain the green 256-test historical Python gate consumer.
