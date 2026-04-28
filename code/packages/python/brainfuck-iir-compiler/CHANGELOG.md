# Changelog

All notable changes to `brainfuck-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] — BF05: JIT to WebAssembly

### Added

- `BrainfuckVM(jit=True)` is now a real JIT.  When constructed with
  `jit=True`, the wrapper attaches `jit-core` (LANG03) with the
  in-house `WASMBackend`.  Brainfuck functions are FULLY_TYPED, so
  tier-up happens before the first interpreted call.
- `BrainfuckVM.jit_enabled` and `BrainfuckVM.is_jit_compiled` properties
  for tests and tooling to inspect whether a run actually executed via
  the JIT path.

### Changed

- Loop emission now uses an unconditional `jmp` back-edge and the
  label format `loop_N_start` / `loop_N_end` (was `bf_loop_N_*`).
  This canonical shape is what `ir-to-wasm-compiler` recognises as a
  structured loop.  The interpreter path is unaffected.

### Notes

- Programs with `.` / `,` (i.e. `call_builtin "putchar"` /
  `"getchar"`) silently deopt to the interpreter for now — the
  WASI-based lowering of those builtins is BF06.  Output bytes are
  unchanged across `jit=False` / `jit=True` for every program.

## [0.1.0] — initial release

### Added

- `compile_to_iir(ast)` — Brainfuck AST → `IIRModule` (LANG01).
- `compile_source(source)` — convenience: lex + parse + compile in one call.
- `BrainfuckVM` — wrapper around `vm-core` (LANG02) preconfigured for
  Brainfuck:
  - `u8_wrap=True` so `+`/`-` automatically mask to 8 bits
  - host-wired `putchar` and `getchar` builtins backed by per-run input/output
    buffers
  - `run(source, input_bytes=b"")` returns collected stdout as `bytes`
  - `tape_size` and `max_steps` guards against runaway programs
  - `metrics` property exposing the underlying `VMMetrics` snapshot
- `BrainfuckError` exception for tape-bounds, fuel-exhaustion, and
  JIT-not-yet-wired failures.
- `BrainfuckVM(jit=True)` is defined as the seam for BF05 but currently
  raises `NotImplementedError` with a pointer to the BF05 spec.
- BF04 spec (`code/specs/BF04-brainfuck-iir-compiler.md`) describing the
  command → IIR mapping, machine model, and BF05 follow-up.
- ≥95% line coverage across `compiler.py` and `vm.py`.
