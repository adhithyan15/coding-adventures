# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial Elixir port of the cpu-pipeline package (from Go D04 implementation)
- `CodingAdventures.CpuPipeline.Token` -- ISA-independent pipeline token struct with bubble support
- `CodingAdventures.CpuPipeline.StageCategory` -- stage classification atoms (`:fetch`, `:decode`, `:execute`, `:memory`, `:writeback`)
- `CodingAdventures.CpuPipeline.PipelineStage` -- stage definition struct with name, description, and category
- `CodingAdventures.CpuPipeline.PipelineConfig` -- pipeline configuration with validation
- `CodingAdventures.CpuPipeline.Pipeline` -- main pipeline module with step/run cycle simulation
  - Configurable N-stage depth (2-stage minimal to 20+ deep)
  - Stall support: freeze earlier stages, insert bubbles
  - Flush support: replace speculative stages with bubbles, redirect PC
  - Forwarding integration: callback-based forwarding path activation
  - Branch predictor integration: callback-based next-PC prediction
  - Snapshot and trace: capture pipeline state at every cycle
- `CodingAdventures.CpuPipeline.PipelineStats` -- IPC, CPI, stall/flush/bubble cycle tracking
- `CodingAdventures.CpuPipeline.Snapshot` -- immutable snapshot of pipeline state per cycle
- `CodingAdventures.CpuPipeline.HazardResponse` -- hazard action struct (none/stall/flush/forward)
- Configuration presets: `classic_5_stage/0` (MIPS R2000) and `deep_13_stage/0` (Cortex-A78)
- Comprehensive ExUnit test suite
