# Changelog

All notable changes to the `cpu-pipeline` package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial implementation of the CPU instruction pipeline, ported from Go.
- `PipelineToken` dataclass representing an instruction flowing through the pipeline.
- `PipelineStage` and `StageCategory` for defining pipeline stage configurations.
- `PipelineConfig` with validation, plus `classic_5_stage()` and `deep_13_stage()` presets.
- `Pipeline` class with `step()` and `run()` methods for cycle-accurate simulation.
- `PipelineSnapshot` for capturing pipeline state at each cycle.
- `PipelineStats` with IPC and CPI calculations.
- `HazardAction` and `HazardResponse` for hazard detection integration.
- Callback-based architecture: fetch, decode, execute, memory, writeback functions are injected.
- Optional hazard detection and branch prediction callbacks.
- Support for stalls (freeze earlier stages, insert bubble) and flushes (replace speculative instructions).
- Comprehensive test suite with >80% coverage.
