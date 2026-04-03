# Changelog

## [0.2.0] - 2026-04-02

### Changed
- Wrapped all public functions and methods with the Operations system (`StartNew`) for unified observability, capability enforcement, and telemetry tracing.
- `StageCategory.String()`, `PipelineStage.String()` — wrapped with Operations.
- `NewBubble()`, `NewToken()`, `PipelineToken.String()`, `PipelineToken.Clone()` — wrapped with Operations.
- `Classic5Stage()`, `Deep13Stage()` — wrapped with Operations.
- `PipelineConfig.NumStages()`, `PipelineConfig.Validate()` — wrapped with Operations; Validate now uses `rf.Fail` for error cases.
- `PipelineSnapshot.String()` — wrapped with Operations.
- `PipelineStats.IPC()`, `PipelineStats.CPI()`, `PipelineStats.String()` — wrapped with Operations.
- `HazardAction.String()` — wrapped with Operations.
- `NewPipeline()` — wrapped with Operations; returns error via `rf.Fail` on invalid config.
- `Pipeline.SetHazardFunc()`, `Pipeline.SetPredictFunc()`, `Pipeline.SetPC()`, `Pipeline.PC()` — wrapped with Operations.
- `Pipeline.Step()` — wrapped with Operations; internal logic extracted to `stepInternal()` (private).
- `Pipeline.Run()`, `Pipeline.Snapshot()`, `Pipeline.Stats()`, `Pipeline.IsHalted()`, `Pipeline.Cycle()`, `Pipeline.Trace()`, `Pipeline.StageContents()`, `Pipeline.Config()` — wrapped with Operations.

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
