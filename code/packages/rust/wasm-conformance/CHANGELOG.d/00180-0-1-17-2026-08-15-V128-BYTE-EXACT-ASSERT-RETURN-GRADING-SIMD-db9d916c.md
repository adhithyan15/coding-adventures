## 0.1.17 — 2026-08-15 — v128 byte-exact assert_return grading (SIMD PR1b-3)

### Added

- `run_action` now returns each result's real, resolved v128 bytes
  alongside its `WasmValue` (via `wasm-runtime` 0.6.0's
  `call_typed_with_v128`, SIMD PR1b-1), and `value_matches_expected`
  compares a `(v128.const ...)` expected value byte-exact against those
  resolved bytes, not just "is this a `V128` result" — proven via a
  TEMP-REVERT-CHECK (stubbing the byte compare out reproduces the exact
  predicted false-pass on a deliberately wrong computed value, confirming
  the check is load-bearing).
- 5 new hand-written regression tests exercising this crate's real 5
  SIMD opcodes end to end (`v128.const` exact/mismatch, `i32x4.add`'s
  actual computation vs. "any v128", `i32x4.eq`'s boolean-mask result
  staying a `v128` not a plain `i32`, a `splat`/`extract_lane` round
  trip) plus one confirming a `v128` invoke ARGUMENT degrades loudly to
  `NotYetSupported` rather than silently substituting the zero vector
  (see "Deferred" below for why arguments can't be resolved the way
  results can).

### Deferred: real corpus vendoring

Investigated vendoring one of the 4 pinned-commit root-level
`simd_*.wast` files (`simd_const.wast`/`simd_splat.wast`/
`simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`) per this task's original
scope. Concretely confirmed **none currently parse**: each exercises SIMD
opcodes well beyond this repo's 5-opcode first slice -- e.g.
`simd_const.wast`'s sole `i64x2.add` use (its `i64x2.inc_smin` test),
`simd_splat.wast`'s `i8x16.add`/`f32x4.min`/`v128.and`/`v128.load`/etc.
across ~20 opcode families. Because `Directive::Module` is built EAGERLY
at `wasm_wast_parser::parse_script` time (see that crate's own module doc
comment for the — separately valid — reason why), a SINGLE unsupported
instruction ANYWHERE in a file, even in a test that would never run,
aborts parsing the WHOLE FILE, not just that one directive — the "partial
opcode coverage, grade the rest `NotYetSupported`" pattern that worked for
every prior WASM epic's first PR doesn't apply here until opcode coverage
is wide enough for a real file to fully parse. Logged as two follow-up
backlog items (widen opcode coverage, or make per-module parse failures
degrade gracefully) rather than either faking a pass or silently dropping
the requirement.

### Why a v128 invoke ARGUMENT can't be resolved the way a RESULT can

`call_function_with_v128`'s resolution trick (SIMD PR1b-1) works because
it runs one statement before `ctx`/`ctx.v128_heap` drop, right after a
call already happened. An ARGUMENT is needed *before* any call starts —
no engine, no `ctx`, no heap exists yet to allocate a handle into. A
`(v128.const ...)` invoke argument now degrades to `NotYetSupported`
(loud, honest) rather than silently becoming the reserved zero-vector
placeholder the legacy `wasm-runtime::call()` i64 path uses for exactly
this situation (which would risk a false pass/fail for the wrong reason
here, where exact bytes are the whole point).

