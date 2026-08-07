# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `cpu-pipeline` crate in namespace
  `ca::cpu_pipeline`: a configurable N-stage CPU instruction pipeline simulator
  with callback-injected ISA behavior.
- Full model — `PipelineToken`/bubbles (with `std::unordered_map` stage-entry
  tracking), stage categories/definitions, classic 5-stage and deep 13-stage
  presets with `validate()`, and a `step()` engine handling normal advancement,
  stalls, flushes (with PC redirect), forwarding, halt propagation, and
  statistics (IPC/CPI, stall/flush/bubble cycles) plus a full snapshot trace.
- `Pipeline` with `step`/`run`, `set_hazard_fn`/`set_predict_fn`,
  `stage_contents`, `snapshot`, and `trace`; callbacks are `std::function`;
  pipeline slots are `std::optional<PipelineToken>`. `Pipeline`'s constructor
  throws `std::invalid_argument` where the Rust `new` returns `Result`. Value
  semantics / RAII throughout; verified clean under ASan + UBSan.
- 105 checks mirroring the crate's unit tests (token/config/stats/hazard, flow
  and fill timing, halt, stalls and flushes with default and clamped counts,
  forwarding, snapshots/trace, deep/custom/two-stage configs, branch prediction)
  run under every ISO C++ compiler via the shared `iso-harness`.
