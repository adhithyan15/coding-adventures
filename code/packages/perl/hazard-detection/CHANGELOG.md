# Changelog — CodingAdventures::HazardDetection (Perl)

## 0.01 — 2026-03-31

Initial release.

- `PipelineSlot` — ISA-agnostic pipeline stage view
  - `new(%opts)` / `empty()`
- `HazardResult` — detector output
  - `action`, `stall_cycles`, `flush_count`, `forwarded_value`, `forwarded_from`, `reason`
- `DataHazardDetector` — RAW hazard detection
  - `detect($id, $ex, $mem)` — returns HazardResult
  - Detects load-use (stall), EX forwarding, MEM forwarding
  - Priority: stall > forward_ex > forward_mem > none
- `ControlHazardDetector` — branch misprediction
  - `detect($ex)` — flushes 2 stages on misprediction
- `StructuralHazardDetector` — resource conflict detection
  - `new(num_alus, num_fp_units, split_caches)`
  - `detect($id, $ex, %opts)` — if_stage, mem_stage for memory conflict
