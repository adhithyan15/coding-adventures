# Changelog

## 0.1.0 — 2026-03-18

### Added
- `PipelineSlot` type representing ISA-independent instruction info per stage
- `HazardAction` enum with ActionNone, ActionForwardFromEX, ActionForwardFromMEM, ActionStall, ActionFlush
- `HazardResult` with action, forwarded value, stall/flush counts, and reason
- `DataHazardDetector` detecting RAW hazards with forwarding and load-use stall
- `ControlHazardDetector` detecting branch mispredictions with 2-stage flush
- `StructuralHazardDetector` with configurable ALUs, FP units, and split caches
- `HazardUnit` combining all detectors with priority-based resolution and statistics
- `IntPtr` helper for creating pointer-to-int values
- 36 tests, 100% statement coverage
