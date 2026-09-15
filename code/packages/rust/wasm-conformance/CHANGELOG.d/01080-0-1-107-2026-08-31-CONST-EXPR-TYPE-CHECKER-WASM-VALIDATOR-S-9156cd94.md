## 0.1.107 — 2026-08-31 — const-expr type-checker (`wasm-validator`'s "third gap")

Regenerated the baseline after `wasm-validator` 0.2.77 added a real
const-expression type-checker (global initializers, element-/data-segment
offsets) — see that crate's own changelog for the implementation, and
`code/specs/W33-wasm-gc-recursive-type-subtyping.md`'s "A newly-discovered,
THIRD gap" addendum for how this was found. Diffed programmatically
against the pre-change baseline for every one of the 257 vendored files,
not just the ones expected to change — **7 files changed, all
`assert_invalid` gains, zero `fail`/`trap` changes anywhere.** Full
per-directive attribution (each flipped directive's exact source text was
independently re-extracted and checked, not just tallied):

- **`global.wast`**: `assert_invalid` 22/40 → **39/40** (+17). The 16
  sequential cases probing illegal non-const opcodes in a global
  initializer (`f32.neg`, `local.get`, `i32.ctz`, `nop`), real type
  mismatches (`f32.const` vs declared `i32`, an `externref` global feeding
  a `funcref` slot, two values left on the stack after `end`), and
  `global.get` misuse (self/forward reference, out-of-range index,
  referencing a mutable global) — plus one more later in the file (the
  "Definition order" section's own forward-reference case, `(global $g1
  i32 (global.get $g2)) (global $g2 i32 (i32.const 0))`). The file's real
  ACCEPTED "Definition order" module (`$g0`..`$g3`, `$gf`, two `elem`s and
  two `data`s all referencing globals via `global.get`) was independently
  re-verified to still validate — every reference in it is to a strictly
  earlier, immutable global. One `assert_invalid` case remains
  `NotYetSupported`: `(table $t 10 funcref (global.get $g))`, a
  TABLE-initializer-expression `assert_invalid` this repo's `TableType`
  has no representation for at all (unrelated to this slice's scope).
- **`data.wast`**: `assert_invalid` 6/20 → **20/20** (+14, 100%) — the
  data-segment-offset mirror of `global.wast`'s own type-mismatch/illegal-
  opcode/mutable-global/out-of-range-global cases.
- **`elem.wast`**: `assert_invalid` 9/26 → **23/26** (+14) — the
  element-segment-offset mirror of the same case family; 3 directives
  elsewhere in the file remain `NotYetSupported` for unrelated,
  pre-existing reasons this slice didn't touch (confirmed unchanged
  before/after by per-directive index).
- **`func_ptrs.wast`**: `assert_invalid` 4/7 → **7/7** (+3, 100%) — the
  same element-offset type-mismatch/illegal-opcode family, smaller file.
- **`type-subtyping.wast`**: `assert_invalid` 34/36 → **36/36** (+2,
  100%) — the exact two `(global (ref $f11) (ref.func $f))`-shaped "type
  mismatch" cases the W33 addendum called out by name: `$f`'s actual
  declared type is in an unrelated `rec` group from the global's declared
  type, with no `sub` relationship either way — correctly rejected via
  real nominal-subtype checking (`is_assignable`'s
  `NonNullConcreteFuncRef` arm), not bare type-index equality.
- **`type-rec.wast`**: `assert_invalid` 8/10 → **9/10** (+1) — the
  file's own `(rec (type (func)) (type $ft (func))) (func $f) (global
  (ref $ft) (ref.func $f))` case, `$f`'s implicit type is NOT `$ft`.
- **`ref_func.wast`**: `assert_invalid` 0/3 → **1/3** (+1) — `(global
  funcref (ref.func 7))` where only 2 functions exist: an out-of-range
  `ref.func` funcidx INSIDE a global initializer, previously unchecked
  anywhere (only a function BODY's own `ref.func` was bounds-checked).
  The other 2 `assert_invalid` cases in this file are the "undeclared
  function reference" rule for `ref.func` used INSIDE a function body — a
  distinct, unrelated, still-open gap this slice doesn't touch.

No other file, and no other directive kind within these 7 files, changed
at all — verified by diffing the full baseline JSON programmatically
(file-by-file, kind-by-kind), not spot-checked.

