# Changelog

## [0.1.0] - 2026-03-19

### Added
- `PipelineToken` -- ISA-independent instruction representation flowing through stages
- `PipelineStage` -- configurable stage definition with name, description, and category
- `StageCategory` -- classification (fetch, decode, execute, memory, writeback)
- `PipelineConfig` -- pipeline configuration with validation
- `Classic5Stage()` -- preset for standard 5-stage RISC pipeline (IF, ID, EX, MEM, WB)
- `Deep13Stage()` -- preset for 13-stage pipeline inspired by ARM Cortex-A78
- `Pipeline` -- main pipeline engine with `Step()` and `Run()` methods
- Stall support: freeze earlier stages and insert bubbles for load-use hazards
- Flush support: replace speculative stages with bubbles for branch mispredictions
- Forwarding integration via `HazardFunc` callback
- Branch predictor integration via `PredictFunc` callback
- `PipelineSnapshot` -- captures complete pipeline state at each cycle
- `PipelineStats` -- tracks IPC, CPI, stall cycles, flush cycles, bubble cycles
- `Trace()` -- returns complete history of pipeline snapshots
- Dependency injection pattern: all stage work done via callbacks (FetchFunc, DecodeFunc, ExecuteFunc, MemoryFunc, WritebackFunc)
- Comprehensive test suite with 98.5% coverage (47 tests)
- Literate programming style with extensive inline documentation
