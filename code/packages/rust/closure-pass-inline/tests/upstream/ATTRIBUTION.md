# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `inline_functions_test.rs`
    - upstream: `test/com/google/javascript/jscomp/InlineFunctionsTest.java`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

Fifth port under CLOC12 (after constant-fold, dce, the emitter/source-map
ports, and remove-unused-vars). Per CLOC12 §6, each upstream Java test file
maps to one Rust file in the matching pass crate's `tests/upstream/` directory.

- Upstream tests run against a JS source-string surface
  (`test("function f(){return 1} f()", "1")`). This crate already exposes a
  `source → bridge → inline → emit` round-trip in its unit tests, so the port
  reuses that surface directly (an `inline_source(&str) -> String` helper)
  rather than hand-building ASTs.

- Two intrinsic differences from the Java oracle, both because the port runs
  **only** the inline pass:
  1. the dead callee declaration is retained (`remove-unused-vars` / `treeshake`
     delete it downstream), so outputs carry a `function …{…};` prefix; and
  2. no constant-folding runs after inlining, so `d(2)` inlines to `2*2`, not
     `4` (folding is `constant-fold`'s job).

- Our `InlinePass` implements the sound core: substitute a `return <expr>;`
  body at its call site(s) — single-use always, multi-use under a size budget —
  when every argument is a simple leaf (identifier/literal) and the body has no
  free identifiers beyond the parameters. Behaviors upstream supports that this
  slice does not are `#[ignore = "blocked on gap-NNN"]` placeholders pinned to
  `code/specs/CLOC12-gaps.md` (gap-127 … gap-132). Notably **gap-132**, a
  conservative miss surfaced by this port: a compound (non-leaf) argument
  expression like `d(a + b)` is declined rather than inlined with
  precedence-preserving parens.
