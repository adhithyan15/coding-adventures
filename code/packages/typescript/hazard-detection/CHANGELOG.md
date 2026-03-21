# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port from Python implementation
- `PipelineSlot` class — ISA-independent representation of pipeline stage contents
- `HazardAction` enum — NONE, FORWARD_FROM_EX, FORWARD_FROM_MEM, STALL, FLUSH
- `HazardResult` class — structured hazard detection verdict with action, forwarded value, stall cycles, flush count, and reason
- `DataHazardDetector` — detects RAW (Read After Write) data hazards, resolves via forwarding from EX/MEM stages or stalling for load-use hazards
- `ControlHazardDetector` — detects branch mispredictions and signals pipeline flush (2-stage penalty)
- `StructuralHazardDetector` — detects execution unit conflicts (ALU, FP) and memory port conflicts (shared cache), configurable resource counts
- `HazardUnit` — combined hazard detection unit that runs all three detectors with strict priority system (FLUSH > STALL > FORWARD > NONE), tracks history and performance statistics
- `pickHighestPriority` helper function for comparing hazard results
- Full Knuth-style literate programming comments with pipeline diagrams, truth tables, and worked examples
- Comprehensive test suite covering all hazard types, edge cases, priority interactions, and statistics tracking
