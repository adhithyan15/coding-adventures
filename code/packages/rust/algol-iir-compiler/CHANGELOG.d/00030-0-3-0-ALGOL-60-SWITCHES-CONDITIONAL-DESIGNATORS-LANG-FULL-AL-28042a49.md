## 0.3.0 — ALGOL 60 switches + conditional designators (LANG-FULL AL5)

- Lower **switch declarations** (`switch s := a1, a2, a3`) and the **computed
  goto** that uses them (`goto s[i]`). A switch records an ordered list of
  target labels; `goto s[i]` selects the i-th (1-based) target via a linear
  `index == k ? jmp Lk` chain. An out-of-range subscript matches no arm and
  falls through to the next statement (ALGOL leaves this undefined; treated as
  a no-op, the conventional implementation choice).
- Lower **conditional designational expressions** in `goto`
  (`goto if b then L1 else L2`), including nested/parenthesised designators —
  the branch is emitted with the portable `jmp_if_false` / `jmp` / `label`
  subset, recursing on the else-designator.
- **Fixed comparison lowering** — `cmp_*` now carries the **i64 operand width**,
  not the `bool` result width. Emitting `bool` made the LLVM backend compare two
  `i64` operands at 1-bit `i1` (`3 == 1` truncates both to `1` → wrongly equal)
  and produced invalid IR that `clang` rejected outright, so every ALGOL program
  with a comparison (`if`, `for … while`, switch index) was latently broken on
  the code-gen backends — it had simply never been exercised there (no ALGOL
  matrix program used a comparison until the switch's index test). This is the
  same width fix the BASIC BA0 work applied.
- Proven by **running**: `lang-aot`'s `lang_matrix.rs` executes a 3-element
  switch (`goto s[3]` ⇒ exit 49) across native / LLVM / WASM / JVM / CLR / VM /
  JIT — `s[3]` chosen because an i1-truncated compare would mis-select the first
  arm, so the cell guards the cmp fix. Unit tests cover each switch index, the
  out-of-range fall-through, both conditional-designator branches, the rejection
  paths (undeclared switch, non-integer index), and the cmp operand width.
- **Limits (follow-ups):** switch-list elements must be plain labels
  (conditional / nested-subscript elements rejected); switch declarations are
  not block-scope-shadowable (a flat per-compilation map, save/restored across
  procedure boundaries).

