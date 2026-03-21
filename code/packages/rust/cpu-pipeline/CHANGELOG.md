# Changelog

All notable changes to the `cpu-pipeline` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `PipelineToken` struct representing one instruction flowing through the pipeline, with full ISA-independent fields for operands, control signals, computed values, and metadata.
- `PipelineStage` and `StageCategory` for defining pipeline stage configurations.
- `PipelineConfig` with validation, `classic_5_stage()` (textbook MIPS R2000), and `deep_13_stage()` (ARM Cortex-A78 inspired) presets.
- `Pipeline` struct with `step()` and `run()` methods implementing the core pipeline simulation loop.
- Hazard handling: stall (freeze + bubble insertion), flush (speculative discard + PC redirect), and forwarding (EX/MEM value bypass).
- `HazardAction`, `HazardResponse` types for hazard detection integration.
- Branch prediction integration via `PredictFn` callback.
- `PipelineSnapshot` for capturing pipeline state at each cycle.
- `PipelineStats` with IPC and CPI calculations.
- Full test suite ported from the Go reference implementation covering: token lifecycle, config validation, pipeline fill timing, stalls, flushes, forwarding, halt propagation, deep pipelines, custom configurations, branch prediction, edge cases.
