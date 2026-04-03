# Changelog — CodingAdventures::CpuPipeline (Perl)

## 0.01 — 2026-03-31

Initial release.

- `CpuPipeline::Token` — pipeline token with new() and new_bubble()
- `CpuPipeline::PipelineStage` — stage definition (name, description, category)
- `CpuPipeline::PipelineConfig` — validation + classic_5_stage() + deep_13_stage()
- `CpuPipeline::HazardResponse` — stall/flush/forward control signals
- `CpuPipeline::PipelineStats` — IPC, CPI, stall/flush/bubble counters
- `CpuPipeline::Snapshot` — point-in-time pipeline state
- `CpuPipeline::Pipeline` — configurable N-stage pipeline engine
  - step(), run(), set_hazard_fn(), set_predict_fn()
  - get_trace() for full snapshot history
