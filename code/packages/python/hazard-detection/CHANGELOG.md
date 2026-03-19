# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- `PipelineSlot` dataclass for ISA-independent pipeline stage description
- `HazardAction` enum with NONE, FORWARD_FROM_EX, FORWARD_FROM_MEM, STALL, FLUSH
- `HazardResult` dataclass with action, forwarded value, stall cycles, flush count, and reason
- `DataHazardDetector` — detects RAW hazards with forwarding from EX/MEM and load-use stall detection
- `ControlHazardDetector` — detects branch mispredictions and signals pipeline flush
- `StructuralHazardDetector` — detects ALU/FP unit conflicts and memory port conflicts, configurable with num_alus, num_fp_units, and split_caches
- `HazardUnit` — combined detector that runs all three and returns highest-priority action (FLUSH > STALL > FORWARD > NONE)
- History tracking and performance statistics (stall_count, flush_count, forward_count)
- Comprehensive test suite covering all hazard types, priority interactions, and edge cases
