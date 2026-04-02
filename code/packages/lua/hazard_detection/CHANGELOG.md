# Changelog — coding-adventures-hazard-detection (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `PipelineSlot` — ISA-agnostic view of one pipeline stage for hazard detectors
  - `new(opts)` — configurable slot
  - `empty()` — represents a bubble/empty stage
- `HazardResult` — result of hazard detection
  - `action` — "none" | "stall" | "flush" | "forward_ex" | "forward_mem"
  - `stall_cycles`, `flush_count`, `forwarded_value`, `forwarded_from`, `reason`
- `DataHazardDetector` — RAW hazard detection with forwarding and stall
  - `detect(id_slot, ex_slot, mem_slot)` — returns HazardResult
  - Detects load-use (stall), EX forwarding, MEM forwarding
  - Priority: stall > forward_ex > forward_mem > none
- `ControlHazardDetector` — branch misprediction detection
  - `detect(ex_slot)` — flushes IF+ID (flush_count=2) on misprediction
- `StructuralHazardDetector` — resource conflict detection
  - `new(opts)` — configure num_alus, num_fp_units, split_caches
  - `detect(id_slot, ex_slot, opts)` — ALU/FP/memory-port conflicts
