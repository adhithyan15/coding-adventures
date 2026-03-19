# Changelog

## 0.1.0 — 2026-03-18

### Added
- `PipelineSlot` type representing ISA-independent instruction info per stage
- `HazardAction` module with NONE, FORWARD_FROM_EX, FORWARD_FROM_MEM, STALL, FLUSH
- `HazardResult` with action, forwarded value, stall/flush counts, and reason
- `DataHazardDetector` detecting RAW hazards with forwarding and load-use stall
- `ControlHazardDetector` detecting branch mispredictions with 2-stage flush
- `StructuralHazardDetector` with configurable ALUs, FP units, and split caches
- `HazardUnit` combining all detectors with priority-based resolution and statistics
- 35 tests, 100% line and branch coverage
