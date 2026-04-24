# Changelog

All notable changes to `tetrad-runtime` will be documented in this file.

## [0.1.0] — 2026-04-23

### Added

- Initial release — Tetrad reimplemented on top of the LANG pipeline.
- `compile_to_iir(source)` translator: Tetrad source → standard-opcode `IIRModule`.
- `code_object_to_iir(code, name)` translator: existing Tetrad `CodeObject`
  → `IIRModule`, allowing pre-compiled programs to be re-targeted to the
  LANG pipeline without reparsing.
- `TetradRuntime` façade: wraps `vm_core.VMCore` with the Tetrad-specific
  builtins (`__io_in`, `__io_out`, `__get_global`, `__set_global`) and a
  small set of Tetrad opcode extensions (`tetrad.move`).
- `TetradRuntime.run(source)` — end-to-end interpreted execution.
- `TetradRuntime.run_with_jit(source)` — JIT path through `jit_core.JITCore`
  with an `Intel4004Backend`.  Functions the 4004 backend cannot compile
  fall back to interpretation transparently.
- `Intel4004Backend` — adapts `intel4004-simulator` to the LANG
  `BackendProtocol`.
- Test coverage: end-to-end runs of the canonical Tetrad demo programs
  (arithmetic, control flow, function calls, globals, I/O).

### Notes

- This package lives **alongside** the legacy `tetrad-vm` and `tetrad-jit`
  packages rather than replacing them.  Both paths share the same
  `tetrad-compiler` front end.  Future work will retire the legacy
  packages once `tetrad-runtime` reaches parity on every metric and
  diagnostic the legacy packages expose.
- The Intel 4004 backend currently reuses `tetrad-jit`'s
  `codegen_4004.py` by translating CIR → tetrad-jit's IR shape.
  A native CIR-aware codegen will follow once the backend protocol
  conventions stabilise.
