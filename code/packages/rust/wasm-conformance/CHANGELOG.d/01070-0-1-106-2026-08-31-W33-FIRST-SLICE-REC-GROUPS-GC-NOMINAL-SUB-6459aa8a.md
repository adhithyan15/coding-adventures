## 0.1.106 — 2026-08-31 — W33 first slice: `rec` groups + GC nominal subtyping, real numbers

Regenerated the baseline after `wasm-wast-parser` 0.1.89 (`rec` group
parsing + `sub`/`final` parsing), `wasm-validator` 0.2.76 (structural
sub/final checking + nominal-subtype-aware `is_assignable`), and
`wasm-runtime` 0.6.17 (`rec`-group-shape + finality cross-module import
guards) landed — `code/specs/W33-wasm-gc-recursive-type-subtyping.md`'s
"first slice" (items 1, 2, 3a; NOT 3b or 4). Diffed programmatically
against the pre-change baseline; every delta is accounted for below —
**this is the first slice in this campaign's W-series that could not
achieve a strictly zero-fail-count diff**, so this entry documents why in
full rather than only listing the good news.

### Real improvements (5 files, all newly-real passes or honest
### reclassifications, zero of these are `fail`)

- **`tag.wast`**: `module` 2/4 → **4/4** (100%), `register` 1/2 → **2/2**
  (100%) — the two `rec`-wrapped link-time-typing modules (pure `func`
  types, no struct/array) now parse and instantiate for real.
- **`type-canon.wast`**: `module` 0/2 → **2/2** (100%) — both modules
  were previously blocked by `rec`; both are pure `sub`/`rec` func-type
  declarations with no cross-module or canonical-equivalence dependency.
- **`type-rec.wast`**: `module` 0/11 → **2/11** — the file's own first
  module (lines 3-19: a plain, non-`rec` type self-referencing itself via
  numeric index, PLUS a `rec` group with a genuine forward reference
  between its two members, all in one module) and its second "Implicit
  function types" module (lines 45-49: a `rec`-wrapped single func type,
  a func with an inferred type, and a global checked against it) both
  parse, validate, and instantiate for real — no struct/array involved.
  The other 9 stay `not_yet_supported`, still blocked by struct/array
  bodies.
- **`type-subtyping.wast`**: `module` 0/46 → **8/46**, `assert_return`
  0/17 → **7/17**, `register` 0/11 → **4/11**, `assert_unlinkable` 8/8
  (100%, unchanged count) → **6/8 pass + 2 fail** (see below) — the
  file's pure-`func`, nominal-`sub`-chain-only sections now validate for
  real: the "Subsumption" 3-cycle and mutually-recursive-pair modules
  (lines 68-113), the "Finality violation" and pure-`func` "Invalid
  subtyping definitions" `assert_invalid` cases (lines 780-811, 944-949,
  now rejected for the RIGHT reason instead of a coincidental parse
  failure), and the "Testing exported functions with non-flat types"
  module (lines 954-976).

### Honest reclassifications (Pass → NotYetSupported, same precedent as
### `wasm-wast-parser` 0.1.88's own three examples — never a `fail`)

- `type-rec.wast` `assert_invalid` 9/10 → **8/10, 2 not_yet_supported**;
  `type-subtyping.wast` `assert_invalid` 36/36 → **34/36, 2
  not_yet_supported**. Both pairs are `"type mismatch"` cases whose
  modules use a `(global (ref $t) (ref.func $f))`-shaped check as their
  ONLY mechanism for probing a type mismatch — investigation found
  **global init-expression values are not type-checked against the
  global's declared type ANYWHERE in this repo** (`wasm-validator`
  has no such check at all, structural or instruction-level; confirmed by
  direct code search, not assumed). This is a real, PRE-EXISTING gap,
  entirely orthogonal to `sub`/`final`/`rec` — it would affect a bare
  `(global i32 (i64.const 0))` mismatch too. Before this slice these
  specific cases were unreachable (blocked by `rec` failing to parse) and
  spuriously "Pass" via that unrelated parse failure; now they parse and
  the module structurally validates (nothing catches the mismatch), an
  honest `NotYetSupported`, never a `fail`. Tracked here as a genuine,
  now newly-relevant gap for a future slice — NOT part of W33's own scope.

### New `fail`s: 10, all independently confirmed to need item (3b) or (4)

Total corpus `fail` count: 198 → **208**. This campaign's prior W-series
slices all achieved a strictly non-increasing `fail` count; this one
could not, because `rec`-group parsing is a GLOBAL parser capability —
every corpus module using `rec` becomes reachable at once, including
ones whose correctness depends on the two pieces this slice's own scope
explicitly excludes. Every one of the 10 new fails was traced to a
specific directive (via a throwaway diagnostic harness dumping
`run_wast_source`'s per-directive outcome, not guessed) and confirmed to
need one of exactly two things, never a bug in this slice's own logic:

- **6 fails need item (3b), canonical type-group equivalence**
  (`type-equivalence.wast`: `module` 4/21 → 12/21 pass but **4/21 → 8/21
  fail** — net +4 fail; `assert_return` 1/4 → **1/4 pass, 3/4 fail** — net
  +2 fail). Every failing module/return is a `TypeMismatch` from
  `is_assignable` correctly rejecting two INDEPENDENTLY-declared,
  `sub`-unrelated (or, after this slice's finality guard, otherwise
  identical) concrete function types that the file's own express purpose
  is to prove are canonically EQUIVALENT despite sharing no name, index,
  or `sub` chain (e.g. `(type $t1 (func (param i32 (ref $t1))))` in one
  module vs. `(type $t2 (func (param i32 (ref $t2))))` in another,
  self-referential, isomorphic, and meant to compare equal). This is
  `type-equivalence.wast`'s entire reason for existing — confirmed
  directly (not assumed) by reading the file: every one of its ~21
  modules is exactly this shape. This slice's own `func_type_is_nominal_
  subtype` deliberately does NOT attempt structural/canonical equivalence
  between unrelated types (`code/specs/
  W33-wasm-gc-recursive-type-subtyping.md`'s own item 3b) — inventing a
  simplified version was explicitly out of scope for this slice, per that
  spec's own "do not attempt to invent a simplified version" instruction,
  since a WRONG accept here is a real soundness risk, unlike a missed
  capability.
- **2 fails need item (3b)** in its CROSS-MODULE form specifically
  (`type-subtyping.wast`'s `assert_unlinkable`, the M10/M11 pair, lines
  746-774): the exporter's and importer's `rec` groups are the SAME SIZE
  and POSITION (so this slice's own `type_group_shape` guard, added
  specifically to prevent this class of false accept, does NOT catch
  it) and have the SAME finality (so the finality guard doesn't either)
  — but their INTERNAL cross-reference TOPOLOGY differs: one group's
  second member's `sub` parent points OUTSIDE its own group (to an
  earlier, separately-declared group), the other's points to its OWN
  sibling. Distinguishing these needs the real course-of-values/de-Bruijn
  canonical numbering algorithm the spec's own item 3b describes — no
  shallow shape/finality/position check can catch it without becoming
  that algorithm. (The `type_group_shape`/finality guards this slice DID
  add are a real, verified improvement over the pre-existing plain
  `FuncType`-equality-only check: they fixed 2 OTHER `assert_unlinkable`
  cases — `tag.wast`'s own position-mismatch case, and `type-
  subtyping.wast` lines 594-617's finality-mismatch pair — that would
  otherwise ALSO have newly regressed to `fail` once `rec` became
  parseable. Verified via the same diagnostic harness: before adding
  these two guards, `assert_unlinkable` fails were 4, not 2.)
- **2 fails need item (4)**, dynamic subtype-aware `call_indirect`
  (`type-subtyping.wast`'s "Finality violation `assert_trap`" pair,
  lines 344-371's `fail1`/`fail2`): `$t1`/`$t2` are declared `(sub
  (func))`/`(sub final (func))` — structurally identical, EMPTY
  `FuncType`s. `wasm-execution`'s runtime `call_indirect` dynamic
  dispatch check (independently confirmed, not assumed) compares the
  table entry's ACTUAL function type against the declared `call_indirect`
  type via plain `FuncType` structural equality — a PRE-EXISTING
  simplification, unrelated to this slice, that was simply UNREACHABLE
  before (no module using `sub`/`rec` could parse at all). Since `$t1`
  and `$t2` are structurally identical, the dynamic check wrongly treats
  them as the same type and never traps, so `assert_trap` fails where a
  real type-identity-aware (or nominal-subtype-aware) dynamic check would
  correctly trap. This is squarely item (4)'s own territory (dynamic
  `call_indirect` against real subtype relationships) — explicitly out of
  scope for this slice regardless of this specific case's single-module
  nature (see `code/specs/W33-wasm-gc-recursive-type-subtyping.md`'s
  addendum for the fuller re-verification of item (4)'s real dependency
  shape, including where it is NOT blocked on 3b).

### Not vendored / re-confirmed unaffected

- `struct.wast`/`array.wast`: unaffected — this slice's parser still
  rejects any `(struct ...)`/`(array ...)` composite-type body
  (`wasm-wast-parser`'s pre-existing "not yet supported" error, unchanged
  wording and unchanged trigger condition). Re-confirmed via the full
  baseline diff: zero tally changes in either file.
- `extern.wast`: re-confirmed still blocked on its own, unrelated gaps
  (struct/array declarations, several GC instructions, `ref.host`) per
  the W32 third addendum's own investigation — untouched by this slice.

wasm-conformance 0.1.105 → 0.1.106.

