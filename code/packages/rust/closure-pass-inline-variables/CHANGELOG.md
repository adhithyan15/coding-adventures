# Changelog

All notable changes to the `coding-adventures-closure-pass-inline-variables` crate will be documented in this file.

## [0.5.0] - 2026-06-20

### Added — CLOC22: variable inlining inside `for`-`in`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `ForInStatement`. The for-in `left`, when a declaration, is counted as a
binding (the loop variable), mirroring the for-statement init handling.

## [0.4.0] - 2026-06-20

### Added — CLOC21: handle `DebuggerStatement`

The statement walks (`count_decl_names_stmt`, `count_uses_stmt`,
`propagate_in_stmt`) now cover `DebuggerStatement` (grouped with the other
childless leaf statements) as a no-op. Added to keep the matches exhaustive over
the new AST variant.

## [0.3.0] - 2026-06-20

### Added — CLOC20: variable inlining inside `do`/`while`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `DoWhileStatement` (loop body and test), mirroring the existing `while`
handling so const-literal propagation reaches into do-while loops.

## [0.2.0] - 2026-06-20

### Added — CLOC19: variable inlining inside `try`/`catch`/`finally`

`count_decl_names_stmt`, `count_uses_stmt`, and `propagate_in_stmt` now recurse
through `TryStatement` (protected block, catch handler body, finalizer). The catch
`param` is counted as a declared binding in `count_decl_names_stmt` so a candidate
that shadows it is correctly excluded from propagation — preserving soundness when
a top-level name is also bound by a catch clause.

## [0.1.0] - 2026-06-17

### Added (CLOC13.H — constant propagation)

New crate per CLOC06's canonical pass set — Closure Compiler's `InlineVariables`
in miniature. `InlineVariablesPass::run` propagates a **top-level `const` bound
to a literal** to all of its use sites:

```js
const RATE = 2;
total = base * RATE;
// =>  const RATE = 2;   (now unreferenced — removed by remove-unused-vars)
//     total = base * 2;
```

- `InlinePass`-style metadata: `name = "inline-variables"`,
  `depends_on = ["constant-fold"]` (so a folded initializer `const X = 1 + 2`
  → `const X = 3` is a literal by the time we look), `iteration_policy =
  FixedPoint`, `cost = 3`.
- **Soundness** rests on three restrictions, plus the inline pass's
  self-contained shadow guard (the name must be declared exactly once in the
  whole program):
  - **`const` only** — a `let`/`var` can be reassigned between its declaration
    and a use, so its initializer is not a safe substitute. `const` cannot.
  - **literal values only** — a literal is immutable. `const X = y;` (an
    identifier whose value could later change) and `const X = o.p;` (a member
    read that could trigger a getter) are NOT propagated.
  - **temporal-dead-zone guard** — a `const` read before its declaration line
    runs throws `ReferenceError` (even from a function called early). We only
    propagate when every top-level item *before* the declaration is inert (a
    function declaration, or a variable declaration with only literal
    initializers), so nothing executes — and nothing can read the binding in
    its TDZ — before it initializes. Only single-declarator `const`s are taken.
- **Single-use** → always propagated (the whole `const` declaration becomes
  pure overhead once its one use is gone). **Multi-use** → propagated only when
  the literal's emitted form is short (`<= MAX_MULTIUSE_LITERAL_LEN`, 8 bytes),
  so duplicating it across the uses is outweighed by deleting the declaration.
- The pass only **propagates**; it leaves the emptied `const` declaration for
  `remove-unused-vars` to delete (mirrors how the inline pass leaves dead
  functions for treeshake). Property names (non-computed `.x` / object keys)
  and assignment targets are never substituted; computed `o[X]` is.
- Self-contained name-based analysis over the Phase-1 AST (same philosophy as
  the `inline` and `rename` passes); does not depend on `closure-scope-analyzer`.

### Tests
- 19 tests: metadata/pipeline-ordering contract + source → bridge →
  inline-variables → emit roundtrips covering single/multi-use propagation, the
  multi-use literal-size budget, and every rejection (let/var, non-literal
  value, shadowed name, property name, computed member).
