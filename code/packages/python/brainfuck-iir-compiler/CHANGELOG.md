# Changelog

All notable changes to `brainfuck-iir-compiler` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
