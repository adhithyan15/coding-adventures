# Changelog

All notable changes to the C++ `hazard-detection` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `hazard-detection` crate
  (namespace `ca::hazard_detection`) — data, control, and structural hazard
  detection for a classic 5-stage CPU pipeline.
- `DataHazardDetector`, `ControlHazardDetector`, `StructuralHazardDetector`, and
  the combined `HazardUnit` (with `history` / `stall_count` / `flush_count` /
  `forward_count`); plus `pick_higher_priority` and `priority`.
- Value-semantic `PipelineSlot` / `HazardResult` built from `std::optional` /
  `std::vector` / `std::string`; the unit's `Option<&PipelineSlot>` structural
  arguments become nullable const pointers.
- 43 checks mirroring the Rust crate's own unit tests across every detector and
  the combined unit, run under every available C++ compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer.
