# Changelog — twig-jit-wasm

## 0.1.0 — 2026-04-29

### Added — Twig JIT pipeline with WASM backend

- ``run_with_jit(source, *, backend=None) -> Any`` — the headline
  entry point.  Compiles Twig → ``IIRModule`` via
  ``twig.compile_program``, instantiates a ``vm-core`` ``VMCore``,
  wraps it in ``jit_core.JITCore`` with a ``wasm_backend.WASMBackend``
  by default, and returns the result of evaluating the program's
  entry function.  Hot functions specialise to WebAssembly;
  cold functions stay on the interpreter; deopts fall back to
  interpretation transparently (jit-core's standard policy).
- ``compile_to_iir(source) -> IIRModule`` — the parse + compile
  step exposed standalone for tooling.
- Pure-unit tests (no runtime deps) verifying the entry point
  invokes JITCore with the expected default backend.
- Real-WASM smoke tests verifying that arithmetic Twig programs
  produce the same answer through ``run_with_jit`` as through the
  Twig interpreter — this is the JIT-vs-interpreter equivalence
  property the rest of the repo tests against.

### What's NOT in v1

- Closure-bearing programs.  The WASM backend can't yet compile
  IR that touches the host-side heap.  Those programs fall back
  to interpretation transparently — no errors, just no JIT win.
- Custom JIT thresholds.  Defaults from ``jit_core`` are fine for
  v1; future ``run_with_jit(jit_threshold=...)`` work item.
- Profile feedback inspection.  ``JITCore`` keeps stats
  internally; v1 doesn't surface them.
