# Changelog

All notable changes to the `coding_adventures_cpu_pipeline` gem will be documented here.

## [0.1.0] - 2026-03-19

### Added
- `PipelineToken` class representing instructions flowing through the pipeline
- `PipelineStage` class defining individual pipeline stages with name, description, and category
- `PipelineConfig` class with validation for stage configurations
- `Pipeline` class with full step-by-step simulation:
  - Normal advancement (tokens shift one stage per cycle)
  - Stall handling (freeze earlier stages, insert bubble)
  - Flush handling (replace speculative stages with bubbles, redirect PC)
  - Forwarding support (from EX or MEM stages)
  - Halt detection (stop when halt instruction reaches writeback)
- `PipelineSnapshot` for capturing pipeline state at each cycle
- `PipelineStats` with IPC and CPI calculations
- `HazardResponse` and `HazardAction` types for hazard detection integration
- `StageCategory` module with FETCH, DECODE, EXECUTE, MEMORY, WRITEBACK constants
- Factory methods: `classic_5_stage` (textbook MIPS pipeline), `deep_13_stage` (ARM Cortex-A78 inspired)
- Dependency injection via callback procs (fetch, decode, execute, memory, writeback, hazard, predict)
- Comprehensive test suite ported from the Go reference implementation
