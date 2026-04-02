# Changelog — CodingAdventures::CpuPipeline (Perl)

## 0.01 — 2026-03-31

Initial release.

- `Token` — instruction token (new, new_bubble, to_string, clone)
- `PipelineStage` — stage definition (name, description, category)
- `PipelineConfig` — pipeline configuration
  - `classic_5_stage()` / `deep_13_stage()` — presets
  - `validate($config)` / `num_stages()`
- `HazardResponse` — hazard unit result (action, stall_stages, flush_count, …)
- `PipelineStats` — counters with ipc() and cpi()
- `Snapshot` — pipeline state at one cycle
- `Pipeline` — N-stage engine
  - `new($config, $fetch, $decode, $execute, $memory, $writeback)`
  - `step()` / `run($max_cycles)` / `get_trace()` / `get_stats()`
  - `set_hazard_fn($fn)` / `set_predict_fn($fn)` / `set_pc($pc)`
