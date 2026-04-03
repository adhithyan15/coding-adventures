# Changelog — CodingAdventures::HazardDetection (Perl)

## 0.01 — 2026-03-31

Initial release.

- `HazardDetection::PipelineSlot` — pipeline stage snapshot
- `HazardDetection::HazardResult` — detection result with action/value/reason
- `HazardDetection::DataHazardDetector` — RAW hazard detection
  - `forward_ex` — EX-to-EX forwarding
  - `forward_mem` — MEM-to-EX forwarding
  - `stall` — load-use hazard (cannot forward)
  - Priority: stall > forward_ex > forward_mem > none
- `HazardDetection::ControlHazardDetector` — branch misprediction detection
  - Returns `flush` with redirect target on misprediction
  - Returns `none` on correct prediction
- `HazardDetection::StructuralHazardDetector` — resource conflict detection
  - Unified cache conflict (IF + MEM)
  - Write-port conflict (MEM and WB both writing same register)
