# Changelog — coding-adventures-hazard-detection (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `PipelineSlot` — snapshot of one pipeline stage for hazard detection
- `HazardResult` — detection result with action, forwarded value, reason
- `DataHazardDetector` — detects RAW (Read After Write) hazards
  - EX-to-EX forwarding (`"forward_ex"`)
  - MEM-to-EX forwarding (`"forward_mem"`)
  - Load-use stall (`"stall"`) — when LOAD result needed immediately
  - Priority: stall > forward_ex > forward_mem > none
- `ControlHazardDetector` — detects branch mispredictions
  - Flush on misprediction with correct PC redirect
  - Correctly predicted branches return `"none"`
- `StructuralHazardDetector` — detects resource conflicts
  - Unified cache conflict (IF + MEM both need memory)
  - Register file write-port conflict (two simultaneous writes)
