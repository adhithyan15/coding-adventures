# Changelog

All notable changes to the C `hazard-detection` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `hazard-detection` crate — data,
  control, and structural hazard detection for a classic 5-stage CPU pipeline.
- The three stateless detectors (`hd_data_detect`, `hd_control_detect`,
  `hd_structural_detect`) returning value-type `HdHazardResult`s;
  `hd_pick_higher_priority` / `hd_priority`; and the combined `HdHazardUnit`
  (`hd_unit_init` / `hd_unit_free` / `hd_unit_check`) with history-based
  statistics (`hd_unit_stall_count` / `_flush_count` / `_forward_count`).
- `HdHazardResult` is a plain value type with inline `reason` / `forwarded_from`
  buffers (no per-result heap); Rust `Option` → `has_*` flags, Rust `Vec<u32>`
  source registers → a borrowed `(pointer, count)`. The unit's history is the
  only heap owner, with paired init/free and overflow-guarded growth.
- 44 checks mirroring the Rust crate's own unit tests across every detector and
  the combined unit (forwarding, load-use stalls, branch misprediction flushes,
  execution-unit and memory-port conflicts, priority resolution, and statistics
  tracking), run under every available C compiler via the shared `iso-harness`;
  the suite also passes clean under AddressSanitizer + UndefinedBehaviorSanitizer.
