# Changelog — hardware-vm

## [0.1.0] — 2026-06-13

### Added

- `HardwareVm::new(hir)` — constructs and bootstraps the simulator from an HIR document
- Sensitivity-map construction from `ContAssign` RHS expressions
- Cascade re-evaluation on input change (event-driven, stops at quiescence)
- `set_input` / `read` — drive inputs and read any signal
- `force` / `release` — debug override for any signal
- `subscribe` — register `Fn(&Event) + Send + 'static` value-change callbacks
- `stats()` — returns `RunResult` with event count, cont-assign run count, final time
- `eval` module: `evaluate()` (full expression evaluator) + `referenced_signals()`
- Supported operators: all arithmetic, bitwise, logical, comparison, reduction, shift, exponentiation
- 11 integration tests covering adder, AND gate, truth tables, force/release, subscribers, stats
- 1 doc-test in `lib.rs`
