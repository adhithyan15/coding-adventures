# Changelog — coding-adventures-cpu-pipeline (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `PipelineToken` — instruction token with all ISA-agnostic fields
  - `new()` — fresh real instruction token
  - `new_bubble()` — NOP/bubble token for stalls and flushes
  - `clone(tok)` — deep copy for snapshot history
- `PipelineConfig` — pipeline configuration
  - `classic_5_stage()` — IF → ID → EX → MEM → WB preset
  - `deep_13_stage()` — 13-stage ARM Cortex-A78-inspired preset
  - `validate(config)` — returns ok, err
  - `num_stages(config)` — stage count
- `HazardResponse` — hazard unit result (action, stall_stages, flush_count, …)
- `PipelineStats` — counters with ipc() and cpi()
- `Snapshot` — complete pipeline state at one cycle
- `Pipeline` — configurable N-stage engine
  - `new(config, fetch, decode, execute, memory, writeback)` — returns {ok, pipeline}
  - `step()` — one clock cycle, returns Snapshot
  - `run(max_cycles)` — run until halt or max_cycles
  - `set_hazard_fn(fn)` / `set_predict_fn(fn)` — optional callbacks
  - `get_trace()` — chronological snapshot history
  - `get_stats()` — current PipelineStats
