# Changelog — coding-adventures-cpu-pipeline (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `Pipeline` — configurable N-stage instruction pipeline engine
  - `step()` — advances all stages by one clock cycle
  - `run(max_cycles)` — runs until halt or max_cycles
  - `set_hazard_fn()` — pluggable hazard detection callback
  - `set_predict_fn()` — pluggable branch predictor callback
  - `stage_contents(name)` — inspect token in named stage
  - `get_trace()` — full history of pipeline snapshots
- `PipelineConfig` — configuration with validation
  - `classic_5_stage()` — textbook RISC pipeline (IF/ID/EX/MEM/WB)
  - `deep_13_stage()` — ARM Cortex-A78 inspired 13-stage design
- `PipelineStage` — individual stage definition (name, description, category)
- `Token` / `Token.new_bubble()` — pipeline token and bubble factory
- `HazardResponse` — stall/flush/forward control signals
- `PipelineStats` — IPC, CPI, stall/flush/bubble counters
- `Snapshot` — point-in-time pipeline state for tracing and debugging
