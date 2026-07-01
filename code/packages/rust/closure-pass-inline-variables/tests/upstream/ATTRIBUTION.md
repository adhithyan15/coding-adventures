# Attribution

Tests in this directory are ported from the Google Closure Compiler
under the Apache License, Version 2.0:

    https://github.com/google/closure-compiler
    LICENSE: https://www.apache.org/licenses/LICENSE-2.0

## Files ported

- `inline_variables_test.rs`
    - upstream: `test/com/google/javascript/jscomp/InlineVariablesTest.java`
    - tracked commit: see `UPSTREAM_SHA`

## Translation notes

CLOC12 port into `closure-pass-inline-variables` — the **first** upstream
port for this crate. Per CLOC12 §6, each upstream Java test file maps to one
Rust file in the matching pass crate's `tests/upstream/` directory.

The crate carries `javascript-parser` + `closure-emitter` as dev-dependencies,
so — like the `rename` and `constant-fold` ports — each case drives the
**real** source → `grammar_to_program` bridge → `InlineVariablesPass` → `emit`
roundtrip. Each case is `assert_eq!(propagate(src), expected)` on emitted JS,
mirroring upstream's `test("var x = 1; alert(x)", "alert(1)")` shape (modulo
the emitter's minified-but-spaced output and boolean shorthand `true` → `!0`).

### What our pass supports (active `#[test]`s)

closurec's `InlineVariablesPass` implements the **provably-sound const-literal
propagation** core of upstream `InlineVariables`: it replaces uses of a
`const` bound to a **literal** with that literal, under a multi-use size
budget (a long literal is only propagated when it has a single use), with two
soundness guards upstream gets from full flow analysis:

- **TDZ**: a `const` is only propagated when everything sequenced before its
  declaration is *inert* (literal-valued `const`s / hoisted-but-uncalled
  function declarations), so no code can observe the binding in its temporal
  dead zone.
- **shadowing**: a name declared more than once (e.g. a top-level `const` plus
  a function parameter of the same name) is declined.

Property names (`obj.RATE`) are never replaced; computed member indices
(`obj[RATE]`) are. `let`/`var` are never propagated (reassignable), and a
non-literal initializer (identifier alias, call, member) is declined.

**Important divergence:** our pass only *propagates* — it leaves the now-dead
`const X = …;` husk in place for the downstream `remove-unused-vars` pass to
delete. Upstream `InlineVariables` removes the declaration itself once every
reference is inlined. So the active cases assert the husk **remains**.

### What we do NOT do yet (`#[ignore = "blocked on gap-NNN"]`)

- **gap-148** — inline a single-assignment `let`/`var` (assigned once, never
  reassigned). Upstream inlines any effectively-constant variable, not only
  `const`.
- **gap-149** — inline an **identifier alias** initializer (`const A = B;` →
  uses of `A` become `B`) when the alias target is not reassigned.
- **gap-150** — **remove the dead declaration** after all references are
  inlined (upstream deletes `const R = 2;` once `R` has no remaining uses;
  ours leaves the husk for `remove-unused-vars`).

Each ignored placeholder is pinned to a `gap-NNN` entry in
`code/specs/CLOC12-gaps.md`; running with `--include-ignored` measures
progress as those gaps close.
