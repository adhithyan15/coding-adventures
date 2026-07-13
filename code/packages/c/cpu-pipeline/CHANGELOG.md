# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `cpu-pipeline` crate: a configurable N-stage CPU
  instruction pipeline simulator that manages instruction flow through stages
  with callback-injected ISA behavior.
- Full model — pipeline tokens/bubbles, stage categories and definitions,
  classic 5-stage and deep 13-stage presets with validation, and a step engine
  handling normal advancement, stalls (freeze + bubble), flushes (discard
  speculative work + PC redirect), operand forwarding, halt propagation, and
  statistics (IPC/CPI, stall/flush/bubble cycles) with a full snapshot trace.
- `cp_pipeline_new`/`_free`, `cp_pipeline_step`/`_run`, `_set_hazard_fn`/
  `_set_predict_fn`, state accessors, `cp_pipeline_stage_contents`, and
  `cp_pipeline_trace`/`_trace_count`. Callbacks carry a `void *ctx` user-data
  pointer.
- Port shape: `CpToken` is a fixed-size plain value type (no per-token heap), so
  a pipeline is capped at `CP_MAX_STAGES` (16) stages — the deepest preset (13)
  fits — and `cp_config_validate` rejects more. The snapshot-history buffer
  guards `size_t` overflow. Verified clean under ASan + UBSan and the macOS
  `leaks` tool (0 leaks).
- 115 checks mirroring the crate's unit tests (token/config/stats/hazard, single
  and multi-instruction flow, fill timing, halt, stalls and flushes with default
  and clamped counts, forwarding, snapshots/trace, deep/custom/two-stage
  configs, branch prediction) run under every ISO C compiler via the shared
  `iso-harness`.
