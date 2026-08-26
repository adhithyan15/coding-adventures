# Changelog

## 0.27.2 — Security fix: `sanitize_ident` was not injective

Task #65 (`/security-review`, discovered while auditing `java-to-
semantic-ir`'s own loop-control synthetic-flag naming): this backend's
`sanitize_ident` escaped a leading-uppercase name (which Ruby would
otherwise treat as a constant) with a single underscore prefix
(`"Foo"` -> `"_Foo"`), but a completely ordinary, unrelated SIR local
literally named `_Foo` passed through **unchanged** — so two distinct
raw SIR names collided on the same emitted Ruby identifier, silently
aliasing two variables into one with no error anywhere in the pipeline
(confirmed by actually compiling and running the resulting program).
The same flaw applied to the keyword-suffix rule (`"end"` -> `"end_"`,
colliding with a real `end_`) and the `sir_`-namespace-prefix rule.

Fixed by making the passthrough and escaped output sets disjoint by
construction: every non-passthrough case is prefixed with a reserved
`sir_esc_` marker — deliberately a superset of this backend's own
`sir_`-namespace reservation, so both concerns share one mechanism — and
any raw name that already starts with that marker is itself routed into
the escaped case rather than allowed to pass through. See
`sanitize_ident`'s own doc comment for the full argument.

**A second `/security-review` round found the marker alone still wasn't
enough**: the *previous* version of this function computed the escaped
body first, unconditionally, then only decided whether to add the
marker based on the *result* — so a genuinely invalid input (illegal
character) whose escaped form didn't happen to need a marker was
returned completely unmarked, colliding with an ordinary raw name of
that same spelling (e.g. `sanitize_ident("a$")` and
`sanitize_ident("a_u0024_")` both used to produce `"a_u0024_"`). Fixed
by restructuring the function to check validity *before* escaping, and
tagging the two marker sub-cases with distinct fixed characters (`v`/
`e`) so they can never collide with each other — see `sanitize_ident`'s
own doc comment for the full argument.

**A third `/security-review` round found the escaped sub-case's own
per-character encoding was still not injective**: `_` was both the
escape token's own delimiter and, previously, a character that passed
through verbatim — so a literal `_` in the input was indistinguishable
from the `_` that opens/closes a real escape token (`escape_body("_u007e_~")`
and `escape_body("~~")` both used to produce the identical
`"_u007e__u007e_"`). Fixed by escaping every underscore too and widening
the hex width from a minimum-4 `{:04x}` to a genuinely fixed `{:06x}` —
see `semantic-ir-to-python::escape_body`'s own doc comment for the full
argument this mirrors.

This changes the exact spelling `sanitize_ident` produces for every
uppercase/keyword/`sir_`-namespace/invalid-character case (e.g. `"Foo"`
now sanitizes to `"sir_esc_vFoo"`, not `"_Foo"`) — a deliberate,
disclosed behavior change: the old spellings were exactly the ones
proven to collide.

## 0.27.1 — Security fix: depth cap on SIR23 rule-pattern matching

Follow-up fix to 0.27.0's SIR23 Tier A pattern matcher, discovered by
independent security reviews of this same slice's C, Go, and Python
backend ports: `sir_sym_match_pattern`/`sir_sym_substitute` shipped with
**no depth cap at all**, on an unverified assumption (inherited from the
JS reference this was ported from) that a `SymRule`'s `lhs`/`rhs` is
always author-written and therefore shallow. That premise does not hold
in this backend — `Expr::SymRule`'s operands are ordinary `Expr`s, and
`emit_sym_operand`'s catch-all passes a `VarRef` through unchanged, so a
rule's pattern or template can be a local variable holding a term a
compiled `for`-loop built to unbounded depth at runtime, the identical
CWE-674 hazard `sir_sym_replace_all`/`replace_repeated`'s own
`SIR_SYM_MAX_TERM_DEPTH` guard was already built to catch — just on the
rule side instead of the target-tree side. Notably, this gap could NOT be
caught by `replace_all`/`replace_repeated`'s own target-tree depth
tracking alone: a rule shaped `Blank() -> <deep RHS>` matches a
one-node, perfectly shallow target instantly, but `sir_sym_substitute`
still has to rebuild the entire deep RHS to produce the replacement.

Both functions now thread a `depth` parameter (mirroring `sir_sym_
term_equals`'s existing signature), capped at the same `SIR_SYM_MAX_
TERM_DEPTH = 512`, reset to 0 at each fresh `sir_sym_apply_rule` call so
one rule's match/substitute gets its own independent depth budget. Past
the cap, both **raise** the same `"sir-runtime-symbolic: depth-limit"`
error `sir_sym_unwrap` already raises for the target-tree guards — not a
silent `nil`/truncated fallback, matching this repo's standing
never-trade-loud-for-silent discipline for a safety-relevant path.

New regression test `deep_rule_rhs_reports_depth_limit_error_not_a_
crash_even_with_a_shallow_target` in `tests/sir23_symbolic.rs`, built via
a REAL compiled 600-iteration `for`-loop (not a hand-built static AST),
proves the exploit was real before this fix (Ruby's own uncontrolled
`SystemStackError`) and is now a clean, catchable error after it.

## 0.27.0 — SIR23 Tier A pattern matcher (Phase A Slice 4)

Part of the SIR22/SIR23 backend-expansion initiative (see
`code/specs/SIR23-symbolic-pattern-semantic-ir.md`'s "Backend impact"
section). This backend now implements SIR23's Tier A pattern matcher:
`SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/`SymPatternNamed`/
`SymRule`/`SymReplaceAll`, gated on the new `Feature::SymbolicExpr`/
`Feature::PatternMatching`/`Feature::Rationals` flags. Tier B (the
`evalTerm` arithmetic/calculus/user-function evaluator) is explicitly OUT
OF SCOPE — a `SymApply` builds an inert term tree, nothing more; no
`Add`/`Sin`/`D`/... folding exists here.

**New runtime functions** (`runtime.rs`'s "SIR23 symbolic expressions"
section): term constructors `sir_sym_symbol`/`sir_sym_int`/
`sir_sym_rational`/`sir_sym_float`/`sir_sym_string`/`sir_sym_apply`;
pattern/rule vocabulary `sir_sym_blank`/`sir_sym_blank_typed`/
`sir_sym_named`/`sir_sym_rule`/`sir_sym_rule_delayed`; the matcher itself
`sir_sym_match_pattern`/`sir_sym_substitute`/`sir_sym_apply_rule`; and
`sir_sym_replace_all`/`sir_sym_replace_repeated`/`sir_sym_unwrap`, each
depth-capped at `SIR_SYM_MAX_TERM_DEPTH = 512` (mirroring the JS backend's
own `MAX_TERM_DEPTH`, already cross-validated against the published
`sir-runtime-symbolic` TypeScript package) with `replace_repeated` also
enforcing a separate `max_iterations` (default 100) rewrite-cycle cap —
both guards raise a plain `RuntimeError` via `sir_sym_unwrap` rather than
overflowing the native Ruby stack or looping forever, verified against a
REAL runtime-built 600-level-deep term in `tests/sir23_symbolic.rs`, not
a hand-built static AST. A minimal `sir_sym_to_s` display helper (wired
into `sir_fmt`) lets `print`/`puts` render a term via the generic
`head(args, ...)` form — the Derive-specific precedence-aware pretty-
printer (SIR23 addendum item 4) is separate follow-up work, not part of
this port.

**New `emit.rs` arms**: the seven node kinds each lower to a call into
their matching `sir_sym_*` helper, plus a new `emit_sym_operand` helper
(mirrors the JS/TS backends' identically-named function) that wraps a
bare `IntLit`/`FloatLit`/`StrLit` operand through the matching leaf-term
constructor before it can sit inside a term tree.

**New tests**: `tests/sir23_symbolic.rs` — 5 real-`ruby`-execution tests
(ported from `semantic-ir-to-javascript`'s own `tests/sir23_symbolic.rs`,
Tier A cases only) proving `//.`'s fixed-point behavior, `/.`'s
single-pass behavior, typed-blank head-constraint matching, rational
reduction at construction time, and the depth-limit guard.

## 0.26.0 — SIR22 "APL addendum" (Phase A Slice 3)

Part of the SIR22/SIR23 backend-expansion initiative (see
`code/specs/SIR22-array-matrix-semantic-ir.md`'s "Backend impact"
section). This backend now implements the nine-node SIR22 "APL addendum"
that Slice 2 (0.25.0, merged) deferred: `Reduce`/`Scan`/`OuterProduct`/
`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`. These nine
share `Feature::{NDArrays, MatrixOps, ArrayColumnMajor}` with the base
cut, so no new feature flags are added — only new node coverage.

**New runtime functions** (`runtime.rs`'s "SIR22 addendum" section):
`sir_array_reduce`/`sir_array_scan`/`sir_array_outer`/`sir_array_shape`/
`sir_array_reshape`/`sir_array_index_generator`/`sir_array_index_of`/
`sir_array_ravel`/`sir_array_catenate`, plus the internal
`sir_array_flatten_row_major` helper `reshape`/`ravel` both need. All are
1:1 ports of `semantic-ir-to-javascript`'s own already-proven addendum
functions (themselves 1:1 ports of `apl_runtime::builtins`/
`array_runtime::ops`), reusing this crate's existing `sir_array_apply_op`/
`sir_array_checked_shape_size`/`sir_array_ndarray`/`sir_array_get`/
`sir_array_nrows`/`sir_array_ncols`/`sir_array_to_array_value` helpers from
Slice 2 rather than duplicating any of that logic.

**New `emit.rs` arms**: the nine node kinds each lower to a call into their
`sir_array_*` counterpart. `Reduce`/`Scan`/`OuterProduct` carry an
`ElementwiseOpKind` and reuse `elementwise_op_ruby_name` exactly like
`ElementwiseOp` does; the other six have no `op` field and just recurse
into their operand(s). `Reshape`'s field order (`shape, target`) needs no
reordering at the call site — `sir_array_reshape(shape_arg, target)` takes
the same order.

**Non-obvious ported behaviour, called out explicitly because a naive
reading of this crate's own `Expr::IndexGenerator`/`Expr::IndexOf` doc
comments (and the SIR22 spec prose) would get it backwards**:
- `IndexGenerator`/`IndexOf` are **1-based** (`⍳5` = `[1, 2, 3, 4, 5]`;
  "not found" reports `haystack.length + 1`, never `-1`/`nil`) — a
  deliberate exception to this domain's otherwise-universal 0-based
  indexing. `nodes.rs`'s own doc comment on `Expr::IndexGenerator` and the
  SIR22 spec's prose both describe this as "0-based", but that is stale
  relative to the actual, tested ground truth: `apl-runtime`'s own
  `index_generator_produces_one_based_run`/`index_of_finds_and_reports_not_found`
  tests, and `semantic-ir-to-javascript`'s own shipped `indexGenerator`/
  `indexOf` (`out[i] = i + 1`, `idx === -1 ? haystack.length + 1 : idx + 1`).
  This backend matches the shipped, tested behaviour for cross-backend
  parity rather than the stale doc prose; the doc drift is a pre-existing
  issue upstream of this crate, out of this slice's scope to fix.
- `Reduce`/`Scan` on a rank-2 (matrix) `target` fold **each row
  independently** across its columns — not the whole matrix into one
  value. Column-major storage means `(row, col)` lives at `col * r + row`,
  so the row loop must seed from `d[row]` (column 0) and then walk
  `d[col * r + row]`; swapping `row`/`col` here silently transposes the
  result instead of raising, which the port replicates the JS reference's
  own warning about verbatim in a doc comment.
- `Reshape` fills its target shape in **row-major** order (APL's own
  convention: last axis varies fastest) but this domain's storage is
  **column-major** — so the row-major-filled sequence must be
  **transposed** into column-major storage before the result is
  constructed. Handing the row-major sequence straight to the
  `SirNDArray` constructor would silently reshape column-major instead,
  producing a wrong answer that still looks plausible (right multiset of
  values, wrong positions). `sir_array_flatten_row_major` (the shared
  `reshape`/`ravel` helper) has the mirror-image concern: it must walk a
  column-major matrix "row, then column" via `sir_array_get` to produce
  true row-major order, never return the raw column-major buffer.
- `Shape` of a scalar returns the **empty vector** (`shape=[0]`,
  `data=[]`), never a scalar wrapping `0` — `⍴5` is a length-0 vector, not
  a rank-0 value. The distinction only shows up structurally (a second
  `Shape` call on the result behaves differently for the two), not in any
  single displayed value — see the new test
  `shape_of_a_scalar_is_the_empty_vector_not_a_scalar` for the executable
  proof.

**Security**: every function that computes an output size from operand
data validates it via `sir_array_checked_shape_size` *before* allocating,
matching the JS reference's DoS-safety discipline exactly — `outer`'s
`[m, n]` output (two independent operand lengths whose PRODUCT isn't
bounded by either alone), `index_of`'s `haystack.length * needle.length`
scan-work product, `catenate`'s combined length (checked ONCE up front,
regardless of which of the five rank combinations follows — a script that
repeatedly self-catenates doubles its size every line with no other
ceiling), `index_generator`'s `n`, and `reshape`'s target size. Reused
`/security-review` before push; no CRITICAL/HIGH/MEDIUM findings against
this diff.

**Removed**: the Slice 2 pre-emit rejection check for these nine node
kinds (`ScanHit::Sir22AddendumNode` in `emit.rs`, its `compile()` match arm
in `lib.rs`) — now dead once real codegen exists, so the variant and its
handling are deleted rather than left unreachable. `Scan::expr`'s
addendum arms now recurse into their sub-expression(s) instead of
rejecting, matching every other composite-expr arm (`MatMul`/
`ElementwiseOp`, etc.) — a deferred builtin or injectable name nested
inside one of these nine nodes is still caught by the shared pre-emit
scan.

**`tests/sir22_array.rs`**: 12 new execution tests (up from 8), each
hand-building a `Module`, compiling it, and running the result through a
real `ruby` interpreter (skips gracefully when absent) — `reduce` on both
a vector and a matrix (the row-independent-fold/column-major-indexing
case), `scan` on a vector, `outer` product of two vectors, `shape` of a
scalar (the empty-vector proof above) and of a matrix, `reshape` (the
row-major-fill-then-transpose-to-column-major proof above, picked at a
non-square shape so a transposition bug would produce a wrong-but-
plausible answer rather than crash), `index_generator` (1-based),
`index_of` found and not-found, `ravel` of a matrix, and `catenate` of two
vectors and of two matrices with equal row counts. Since `ArrayLit`
always builds rank-2 (`1xn`) storage and this domain's addendum functions
often require a genuine rank-1 vector operand, most tests route a `1xn`
`ArrayLit` through `Ravel` (or `Shape`, for `Reshape`'s shape argument)
first to obtain one — exactly as a compiled APL program would.

`semantic-ir-to-ruby` 0.25.0 -> 0.26.0.

## 0.25.0 — SIR22 array/matrix base cut (second-wave backend rollout, Phase A Slice 2)

Part of the SIR22/SIR23 backend-expansion initiative (opening
C/Go/Rust/Python/Ruby to array/matrix and symbolic/pattern support — see
`code/specs/SIR22-array-matrix-semantic-ir.md`'s "Backend impact" section).
This backend now accepts `Feature::{NDArrays, MatrixOps, ArrayColumnMajor}`
and implements the SIR22 base cut: `ArrayLit`/`Range`/`MatMul`/
`ElementwiseOp`/`Transpose`/`IndexGet` (+ `Stmt::IndexSet`).

**New runtime** (`runtime.rs`): `sir_array_*` — an inlined port of
`semantic-ir-to-javascript`'s own already-proven `ArrayRt` sub-runtime
(itself a port of the published `@coding-adventures/sir-runtime-array`
package), following this crate's existing inlined-runtime convention
(unlike Python/TypeScript's imported-package model). Column-major dense
storage (`SirNDArray(shape, data)`), same value model as the Rust/JS/TS
references.

**A deliberate divergence from the JS/TS references, not a bug**: `data`
holds whatever Numeric type the source arithmetic naturally produces
(Integer stays Integer through `+`/`-`/`*`; only `Div`/`Pow` force a Float
result) rather than JS's `Float64Array`-forced uniform-double storage —
Ruby distinguishes Integer/Float in its own display convention (this
crate's `sir_fmt_float` already deliberately keeps a trailing `.0` on a
real Float), so an all-integer computation like a 2x2 `matmul` prints its
result without a spurious `.0`, matching this crate's own `div_true`
precedent for exactly the same class of decision.

**SIR22 "APL addendum" nodes** (`Reduce`/`Scan`/`OuterProduct`/`Shape`/
`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) share
`NDArrays`/`MatrixOps`/`ArrayColumnMajor` with the base cut above, so a
bare feature-flag check can't tell a module using one of these nine apart
from a safe base-cut-only module. Added a dedicated pre-emit scan arm
(`ScanHit::Sir22AddendumNode`, folded into this crate's existing single
shared `Scan`/`first_scan_issue` traversal rather than a second,
JS-style, separate walker) that rejects them cleanly — mirrors JS/TS's
own `find_unimplemented_sir22_addendum_node` (since removed there, once
their own addendum shipped in a later PR — see the SIR22 spec's
now-corrected "Backend impact" section). Real codegen for these nine is
Phase A Slice 3, a separate later PR.

**Security**: every NaN-safe AND-form bounds check the JS reference
documents (`sir_array_get`/`sir_array_set`'s `r >= 0 && c >= 0 && ...`,
never the OR-form negation) is replicated exactly — Ruby's `Float::NAN`
follows the same IEEE-754 "every relational comparison is false" rule, so
the identical hazard class applies. Every shape/output size is validated
via `sir_array_checked_shape_size` *before* allocating, matching the JS
reference's own DoS-safety discipline (an attacker-influenced shape must
fail cleanly, not exhaust memory or crash on an unvalidated `Array.new`).

New `tests/sir22_array.rs` (8 tests, ported from
`semantic-ir-to-javascript/tests/sir22_array.rs`'s own worked examples,
adapted for this backend's numeric-type-propagation divergence noted
above): hand-built `Module`s exercising `matmul`/scalar-broadcast
`elementwise`/`Div`'s forced-float behavior/`transpose`/`range`/a `Whole`
selector/`IndexSet` mutation, each compiled and run through a real `ruby`
interpreter (skips gracefully when absent), plus a compile-time rejection
test proving `Reduce` is cleanly rejected rather than reaching an
`emit_expr` panic.

`semantic-ir-to-ruby` 0.24.0 -> 0.25.0.

## 0.24.0 — SIR21 T3b-2 Slice 7: cleanup — remove dead `tdiv`/`utdiv`

Part of the SIR21 T3b-2 arc's final slice. `c-to-semantic-ir` was the only
crate that ever emitted the bare `"tdiv"`/`"utdiv"` builtin names (this
backend accepted them so C-sourced modules could target Ruby); it migrated
to `div_trunc`/`udiv_trunc` in Slice 6 (merged). With that migration in,
`"tdiv"`/`"utdiv"` are provably dead names here too — nothing in this
repository constructs a `BuiltinCall` with either name anymore.

Removed `"tdiv"`/`"utdiv"` from `SUPPORTED_BUILTINS` (the validator
allowlist gate) and their `emit_builtin` match arm. The `sir_tdiv` runtime
helper itself is untouched — `div_trunc`/`udiv_trunc`'s own match arm
(added in Slice 2) still calls it, so it remains live code, just reachable
only under the new canonical names now. `tmod`/`utmod` (modulo) are
untouched — this arc has never touched modulo. Bare `"/"` also stays
exactly as it was (still aliased to `div_floor`'s identical rendering) —
`twig-to-semantic-ir`'s permanent fallback route, per the spec.

Also added a new Twig-sourced regression test
(`e2e_twig_bare_slash_still_floors_after_tdiv_utdiv_removal`, `-7 / 2` →
`-4`): Twig's `/` is variadic with no static int/float distinction, so it
can never migrate to one of the four typed division ops — it stays on
bare `"/"` permanently, and this cleanup deliberately proves that removing
the *other* two names Twig's own emission never touched (`"tdiv"`/
`"utdiv"`) left its own path undisturbed.

Verified via the full `semantic-ir-to-ruby` test suite (0 failures) and the
full `sir-conformance` suite (0 failures).

`semantic-ir-to-ruby` 0.23.0 -> 0.24.0.

## 0.23.0 — SIR21 T3b-2 Slice 2: `div_floor`/`div_trunc`/`udiv_trunc`/`div_true`

Additive-only: adds the four new division builtin names from SIR21 T3b-2
(`code/specs/SIR21-type-system-and-integer-semantics.md` §E3) to
`SUPPORTED_BUILTINS` (the validator allowlist gate) and `emit_builtin`'s
match. Bare `"/"` and `tdiv`/`utdiv` keep working unchanged — no frontend
emits any of the new names yet.

- `div_floor` renders IDENTICALLY to bare `/` (a pure alias, not a new
  code path) — Ruby's own `/` already floors ints and true-divides
  floats. It deliberately inherits `/`'s existing float-zero-divisor
  behavior (native Ruby `1.0 / 0` silently returns `Infinity`, unlike
  `Integer#/0`, which raises) rather than retroactively "fixing" it here
  — this additive slice changes zero pre-existing behavior, matching the
  precedent set by this arc's C backend PR.
- `div_trunc`/`udiv_trunc` reuse the pre-existing `sir_tdiv` helper —
  identical to `tdiv`/`utdiv` today (this pair is slated to replace those
  names in a later cleanup slice; both stay wired to the same helper in
  the meantime, per the spec's absorbs/replaces decision). Zero-divisor
  raises via Ruby's own native `Integer#/0` — no explicit check needed.
- `div_true` is genuinely new: `sir_true_div`, a new runtime function.
  Ruby's native `/` can't be reused directly for two reasons —
  `Integer#/` floors instead of true-dividing, and `Float#/0` silently
  returns `Infinity` rather than raising — so `sir_true_div` explicitly
  checks for a zero divisor before the float divide runs, closing the
  gap a naive `a.to_f / b` would leave open.
- New `tests/division_ops_tests.rs`: real `ruby`-execution proof for all
  four ops, including the §E3 worked example, floor-vs-truncate
  divergence, and a zero-divisor case for every op.

## 0.22.0 — SIR28 §7: remove dead bare `print`/`puts` handling

Every frontend now emits `__sys_write__` instead of bare `print`/`puts`
(SIR28 Slices 4-6, all merged), so this backend's `print`/`puts` handling
is dead code. Removed:

- `"print"`/`"puts"` from `SUPPORTED_BUILTINS` (the validator allowlist
  gate) and from `emit_builtin`'s match.
- `sir_print`, `sir_puts`, and `sir_puts_one` from the embedded Ruby
  runtime source in `runtime.rs` (fully independent of `sir_write`/
  `sir_write_puts_one` — confirmed via grep that nothing else called
  them, unlike the C backend's shared `_sir_puts_one` — so this is a
  straight deletion, not a refactor).
- The `"print"`/`"puts"` `when` arms from `sir_builtin_dispatch`'s
  by-name dispatch.

This is a breaking change for any SIR module that still emits bare
`print`/`puts` — none do, in this monorepo, as of SIR28 Slice 6.

Test suite: the local test helper that hand-built bare `puts`
`BuiltinCall`s purely to observe hand-constructed IR's output (unrelated
to testing print semantics itself) now builds the equivalent
`__sys_write__` envelope (`terminator: "per_value"`, `unpack_arrays:
true` — every helper in this backend's tests was `puts`-shaped, not
`print`-shaped), plus `Feature::ConsoleIO`/`Feature::Strings` added to
every affected manifest across the file's many module-builder helpers.
Two shape assertions that checked emitted `sir_puts(...)` text directly
were updated to the new `sir_write(...)` shape.

## 0.21.0 — implement `__sys_write__`, the SIR28 console-output primitive

Adds a `"__sys_write__"` `emit_builtin` arm and a new runtime helper,
`sir_write`, generalizing the existing `sir_print`/`sir_puts` into one
function parameterized by `stream` (stdout/stderr), `terminator`
(none/per_value/once), and `unpack_arrays` — the policy axes SIR28 §2.1
defines. Declares `Feature::ConsoleIO`.

Unlike the C backend (which must bake `stream`/`terminator` in as
compile-time C int constants, since generated C can't easily branch on a
string at the call site the way this crate's target language can), this
backend just passes every arg — including the `stream`/`terminator`
literals, already-validated by `semantic-ir`'s validator against a closed
set — straight through to `sir_write` as ordinary Ruby string arguments,
which branches on them directly at Ruby runtime. No compile-time literal
extraction needed here.

Purely additive: nothing emits `__sys_write__` yet, so `sir_print`/
`sir_puts` and every existing `print`/`puts`-sourced program are unchanged.

New `tests/sys_write_tests.rs`: hand-builds a `Module` directly per
stream/terminator/unpack_arrays combination (no frontend emits the op
yet), emits Ruby, runs it with a real `ruby` interpreter, and asserts
stdout/stderr — covering all three `terminator` modes, `unpack_arrays`
true/false, the `stderr` stream, and the empty-args `per_value` edge case.

## 0.20.1 — accept `c<<`, the C frontend's own bitwise left shift

`c-to-semantic-ir` and `ruby-to-semantic-ir` both used to lower their
`<<` to the SAME bare `BuiltinCall("<<", ...)` name — one meaning C's raw
bitwise left shift, the other Ruby's polymorphic `<<` (Array push/String
concat/saturating-Integer-shift). `semantic-ir-to-c` had to pick ONE
meaning for the shared name and got it wrong for C-sourced programs (see
its own 0.36.2 CHANGELOG entry). `c-to-semantic-ir` now emits a distinct
`c<<` for its own shift; this backend accepts it in `SUPPORTED_BUILTINS`
and renders it identically to `<<` (Ruby's native `<<` operator already
computes the mathematically correct arbitrary-precision left shift for
any Integer operand, so no new runtime behavior is needed — just
recognizing the renamed builtin so the structural scan doesn't reject a
C-sourced program using it).

`semantic-ir-to-ruby` 0.20.0 -> 0.20.1.

## 0.20.0 — fix: `puts` on an Array bracket-displayed instead of unpacking

The same bug independently discovered and fixed in `semantic-ir-to-c`
0.33.0, found here while verifying that fix against `sir-conformance`:
this backend's hand-rolled `sir_puts` called `sir_fmt(x)` for each
argument, whose `else v.to_s` branch bracket-displays an Array (real
Ruby's `Array#to_s`/`#inspect` both do this) — but real `Kernel#puts`
does NOT use `to_s` for an Array argument; it unpacks one element per
line, recursively flattening nested arrays, printing nothing at all for
an empty array. Despite this backend emitting code that runs under a
REAL Ruby interpreter, its own `sir_puts` reimplementation never
delegated to native `puts`, so it never got this behavior for free.

Fixed with a new `sir_puts_one` helper mirroring the C fix's
`_sir_puts_one`: unpacks an `Array` argument recursively, falls through
to `sir_fmt` + a newline for everything else. No depth cap is needed
(unlike the C fix) — a self-referential array would raise Ruby's own
`SystemStackError` on infinite recursion, which is safe under a real
Ruby VM's stack-overflow protection, unlike raw C recursion.

### Added

- Three new `e2e_*` tests in `tests/emit_tests.rs`: empty array prints
  nothing, recursive flattening across nested levels, `print` still
  bracket-displays (unaffected — only `puts` unpacks).

### Changed

- `e2e_seq_literal_displays_as_an_array` and
  `seq_set_writes_in_bounds_and_returns_the_value` updated to the correct
  unpacked expected output.

## 0.19.0 — `fmt_float`: C-printf-faithful float formatting

One builtin, for the C frontend's faithful `printf` (SIR27 milestone 10).

- `fmt_float(value, precision, kind)` → `sir_fmt_float_c`, which renders a
  `double` exactly as C's `printf` would for the conversion `kind`
  (`'f'`/`'F'`/`'e'`/`'E'`/`'g'`/`'G'`) and precision. Ruby's `sprintf` is
  C-compatible, and the runtime switches on the fixed `kind` character (never
  interpolating a source-derived format string), so `printf("%.2f", 3.14159)`
  and the emitted C both produce `"3.14"`.

This leaves the backend's *default* float display (`sir_fmt_float`, `3.14`)
untouched — `fmt_float` is only reached through an explicit C `printf`.

## 0.18.1 — fix: `Foo.new` runs the `initialize` constructor

Fixes a cross-backend conformance failure (`counter_state`): a `def initialize`
was registered like every method under the reserved `sir_um_` prefix as
`sir_um_initialize`, which Ruby's own `Class#new`/`initialize` never calls — so a
native `Foo.new` allocated an instance whose constructor body (its `@ivar`
initialisers) NEVER ran, leaving every `@ivar` nil. `Counter.new; c.inc` then
raised `undefined method '+' for nil` on `@n + 1`.

- `__new__` now emits `sir_new(Foo, args…)` instead of a native `Foo.new(args…)`.
- New `sir_new` runtime helper mirrors the Go/C/Rust runtimes: `allocate` a bare
  instance, then — if the class or an ancestor defines `sir_um_initialize` —
  invoke it on the new object with the constructor args, so `@ivar` assignments
  land on it. Dispatch stays CLOSED (the method name is the fixed literal
  `sir_um_initialize`, never source-derived — the anti-RCE discipline). A class
  with no constructor is a plain allocation, as before.
- Regression tests: `e2e_initialize_runs_on_construction` and
  `e2e_initialize_with_constructor_argument` (the prior ivar e2e tests used an
  explicit `start`/`set` method, sidestepping the constructor — which is why the
  gap escaped).

## 0.18.0 — numeric conversions: `to_f` / `to_i`

Two numeric-conversion builtins, for the C frontend's floating-point value track
(SIR27 milestone 9b) — the int↔double boundaries a C program creates.

- `to_f` → Ruby's native `(x).to_f` (Integer → Float / usual widening).
- `to_i` → `(x).to_i` (Float → Integer, **truncating toward zero**, matching C's
  `(int)double` cast; the frontend then masks to the target width with a
  `Convert`).

Float arithmetic itself needs no new code: `+`/`-`/`*`/`/` and the comparison
builtins are already native Ruby operators that do the right thing on `Float`
(so `7.0 / 2.0 == 3.5`), and `Feature::Floats` / `Expr::FloatLit` were already
supported.  Verified via the C→SIR→Ruby three-way conformance corpus.

## 0.17.0 — classes slice 7: modules / mixins (OOP arc complete)

Accepts `Feature::Modules` — module definitions and `include`/`extend` mixins.
This is the **last OOP slice**: the Ruby backend now covers the full class/module
surface (classes, constants, instance & class methods, `@ivars`, `@@class vars`,
inheritance + `super`, and now modules).

- `module M; …; end` → `Object.const_set(:M, Module.new)` (reflective, like a
  class — a native `module` block is illegal inside the `main` method).
- `include M` (in a class) → `__include__("Class", "M")` → `(Class).include(M)`.
- `extend M` → `__extend__("Class", "M")` → `(Class).extend(M)`.

**Module methods reuse existing machinery.** A module's methods are hoisted and
registered with the SAME `__def_method__` protocol as class methods (slice 2):
`Module#define_method` installs each as `:sir_um_<m>`, and once a class `include`s
the module they resolve through the ancestry via the existing
`__method__`/`public_send` dispatch — so this slice adds **no new method
machinery**, only the `ModuleDef` declaration and the two native mixin builtins.
`include` adds instance methods; `extend` adds singleton (class) methods.

**Injection safety.** The module name (`const_set`) and both mixin operands (the
class and the module, emitted verbatim as bare constants in `.include`/`.extend`)
are validated as constant paths in the co-total scan. A non-empty module body
(class-level code) is deferred — a method-only module has an empty body.

**OOP arc complete** for the Ruby backend. Remaining not-yet-wired features are
the built-in **collection-method** catalog, `TailCalls`, `Intrinsics`,
`NDArrays`, and array-pattern destructuring.

## 0.16.0 — classes slice 6: class variables (@@x)

Accepts `Feature::ClassVars` — `@@` class variables.

- `@@x = v` → `Stmt::Assign { scope: ClassVar }`; `@@x` → `Expr::VarRef { scope:
  ClassVar }` (the name includes the leading `@@`).
- A class-BODY initializer `@@x = init` — the FIRST accepted non-empty class body.

**Why not a bare `@@x`.** A method body runs in a hoisted top-level function, not
a lexical class scope, so a bare `@@x` is a Ruby error ("class variable access
from toplevel"). Read/write in a method therefore routes through a new runtime
helper: `sir_cvar_owner(self).class_variable_get/set(:"@@x")`, where
`sir_cvar_owner(s) = s.is_a?(Module) ? s : s.class` resolves the owning class in
*both* contexts — an instance method (`self.class`) and a class method (`self`
*is* the class). So an instance method and a class method share the same `@@x`,
matching Ruby.

**The class-body initializer** runs where `self` is `main`, not the class, so it
can't use the `sir_cvar_owner(self)` path; it writes on the class by NAME:
`<Class>.class_variable_set(:"@@x", init)`. This is why a non-empty class body is
now legal — but ONLY for `@@x` initializers; any other class-body content stays
rejected.

**Injection safety.** Every `@@`-name — a `ClassVar` `Assign`/`VarRef` and a
class-body initializer — is validated as `@@<identifier>` (new
`is_valid_classvar_name`) in the co-total scan and emitted as a safely-quoted
symbol, so a crafted name cannot inject. The `emit_var_ref` scope match is now
exhaustive (every `Scope` handled), so a new variant is a compile error rather
than reaching a catch-all `unreachable!`.

**Still rejects** modules (`__include__` / `__extend__`) — the last OOP slice.

## 0.15.0 — classes slice 5: class methods (def self.foo)

Class (singleton) **methods**. No new `Feature` (they lower to builtins).

- `def self.m` → a hoisted top-level function `Class__m_cm` +
  `__def_class_method__("Class", "m", MakeClosure(fn))` →
  `Class.define_singleton_method(:sir_um_m, &closure)`.
- `Class.m(args…)` → `__class_method__("Class", "m", args…)` →
  `(Class).public_send(:sir_um_m, args…)` — the receiver is the class *name* (a
  bare constant), not an instance.

Mirrors instance methods (slice 2) but installs on the class's **singleton**
method table via `define_singleton_method`. The SAME reserved `sir_um_` prefix is
reused: a class's singleton methods and its instance methods live in separate
tables, so the shared prefix cannot collide, and class-method dispatch stays
**closed** (anti-RCE) — `public_send` with a crafted class-method name can only
reach a `sir_um_*` (user) method, never `Class.instance_eval`/`send`/etc.

**Totality / clean rejection.** A SECOND allowlist (collected from
`__def_class_method__`, alongside the instance-method allowlist) gates
`__class_method__`: a dispatch to a name the module never registers as a class
method is a **built-in class method** (`Foo.name`, …) — the Collections batch —
rejected cleanly. The two allowlists are independent, so an instance registration
does not authorise a class dispatch of the same name (and vice-versa). The class
name in both builtins is emitted verbatim as a bare constant and validated as a
constant path (co-total injection guard); a malformed `__def_class_method__`
(missing/non-closure third argument) is rejected.

**Still rejects** class variables (`@@x`, `Feature::ClassVars`) — which also pull
in a non-empty class body — and modules; each a later slice.

## 0.14.0 — classes slice 4: inheritance + super

Class **inheritance** and `super`. No new `Feature` (a superclass rides on
`Stmt::ClassDef`; `super` is a builtin).

- `class Dog < Animal` → `ClassDef { superclass: Some("Animal") }` →
  `Object.const_set(:Dog, Class.new(Animal))`. The subclass inherits Animal's
  ancestry natively — `Dog.new.is_a?(Animal)` holds, and method resolution walks
  up it. The superclass is a bare constant **reference** (a `::` path is allowed
  here — it references, not defines), validated as a constant path.
- `super` (bare or with args — the frontend forwards the method's arguments
  explicitly in both cases) → `__super__("m", "Dog", args…)` →
  `(Dog).superclass.instance_method(:sir_um_m).bind(self).call(args…)`.

**Why an explicit ancestry walk (not native `super`).** A method body lives in a
hoisted top-level function (slice 2), not a real method context, so native bare
`super` is unavailable there. Instead the superclass's method is fetched as an
`UnboundMethod` from `<DefiningClass>.superclass`, bound to `self` (the receiver,
inherited via slice 2's `define_method` binding), and called. This resolves up a
multi-level chain correctly (`A → B → C`, each `super` climbing one level).

**Anti-RCE preserved.** The super'd method name is emitted as a `sir_um_`-prefixed
quoted symbol, so `instance_method` can only fetch a user-defined method — never
a reflection/eval built-in — exactly as `__method__` dispatch (slice 2). The
defining-class name is emitted verbatim as a bare constant and validated as a
constant path (co-total injection guard), as is the superclass in `Class.new`.

**Still rejects** class variables (`@@x`, `Feature::ClassVars`), class methods
(`__class_method__` / `__def_class_method__`), and modules — each a later slice.

## 0.13.1 — `is_ruby_keyword` missing `__ENCODING__` (task #116 audit)

Follow-up to task #110/#112 (`semantic-ir-to-javascript`/`-typescript`'s
`eval`/`arguments` gap): a broader audit of every `semantic-ir-to-*`
backend's reserved-word check for the same class of bug.

`is_ruby_keyword` (`emit.rs`) already listed two of Ruby's three
magic-constant keywords, `__FILE__` and `__LINE__`, but not the third,
`__ENCODING__`. All three are genuine lexical keywords, not plain
identifiers — `__ENCODING__ = 5` is a `SyntaxError` under MRI (verified
against Ruby 3.4.9), exactly like `__FILE__ = 5` or `__LINE__ = 5`. A
SIR identifier named `__ENCODING__` was previously emitted verbatim by
`sanitize_ident` instead of being suffixed.

Fixed by adding `__ENCODING__` to the existing magic-constant group in
`is_ruby_keyword`'s `matches!` list (same style as the existing
`__FILE__`/`__LINE__` entries; no restructuring). New test
`sanitize_ident_flags_encoding_magic_constant`
(`tests/emit_tests.rs`) pins it as reserved and confirms ordinary
look-alike identifiers (`encoding`, `__encoding__`) are untouched.

## 0.13.0 — classes slice 3: instance variables (@ivars) + self

Accepts `Feature::InstanceVars` — an instance variable read and write, plus the
`__self__` builtin. The third OOP slice, and a small one: the slice-2 method
machinery already does the heavy lifting.

- `@v = x` → `Stmt::Assign { scope: Instance }` → native `@v = x`.
- `@v` → `Expr::VarRef { scope: Instance }` → native `@v`.
- `__self__` (a bare `self`) → the native `self` keyword.

The frontend puts the leading `@` in the node's `name`, and the emitter renders
it **verbatim** (not through `sanitize_ident`, which would mangle the `@`). No
runtime support is needed: an instance-method body is installed with
`define_method` (slice 2), which binds `self` to the receiver, so `@v` inside a
method reads/writes **that instance's** own variable, and it persists across
dispatches (a counter mutating `@n` across calls works).

**Injection safety.** Both verbatim-emitted instance-variable positions — a
`Scope::Instance` `Assign` target and `VarRef` — are validated as
`@<identifier>` (a new `is_valid_ivar_name`) in the SAME pre-emit traversal as
the builtin/constant scan (co-total with the emitter), so a crafted name (`@v;
system(...)`, a non-`@` name, `@` + digit) cannot inject source and is rejected
cleanly.

**Still rejects** class variables (`@@x` — `Feature::ClassVars`), inheritance (a
superclass / `__super__`), class methods (`__class_method__` /
`__def_class_method__`), and modules — each a later slice.

## 0.12.0 — classes slice 2: instance methods

Instance-method **definition** and **dispatch** — the second OOP slice. No new
`Feature` (the frontend lowers a method-bearing class to builtins, not a
feature-gated node); this wires two builtins:

- `__def_method__("Class", "method", MakeClosure(fn))` — the frontend's
  registration of a hoisted method — renders as
  `Class.define_method(:sir_um_method, &closure)`. `define_method` binds `self`
  to the receiver at call time, and the closure calls the hoisted top-level
  function, so the method body runs with the instance as `self` (its `@ivars`
  become reachable once slice 3 accepts them).
- `__method__(recv, "method", args…)` — instance dispatch — renders as
  `(recv).public_send(:sir_um_method, args…)`.

**Anti-RCE — the `sir_um_` prefix closes reflection dispatch.** `__method__`
dispatches by a method name taken from the IR, so a naive
`recv.public_send(:name)` would be a remote-code-execution sink: a hand-built
module could pass `"instance_eval"` / `"send"` and reach Ruby's metaprogramming.
Both registration and dispatch instead go through a **reserved `sir_um_`
method-name prefix** — no Ruby built-in is named `sir_um_*`, so `public_send`
with a crafted name can reach *only* a method installed by `__def_method__`,
never `instance_eval`/`send`/`eval`/any reflection sink. This is the codebase's
"explicit dispatch, never reflection" invariant, achieved natively (SIR24 §OOP).
The prefixed name is emitted as a quoted symbol via `emit_symbol` (no injection),
and the class name in `__def_method__` is validated as a constant path like the
slice-1 constant positions.

**Totality / clean rejection.** A `__method__` call to a name the module never
registers via `__def_method__` is a **built-in method call** (`.upcase`, …) — the
separate Collections batch — and is rejected cleanly (a source-positioned
`UnsupportedFeature`) rather than compiling to a runtime `NoMethodError` (the
prefixed `sir_um_upcase` is unbound). The scan collects the module-wide set of
registered method names in a first pass, then the single co-total traversal
validates each dispatch against it. A malformed `__def_method__` (missing its
closure) and the remaining OOP builtins (`__super__`, `__self__`,
`__class_method__`, `__def_class_method__`) stay rejected (later slices). Class
**methods**, **inheritance**, `@ivars`, `@@class vars`, and modules remain
unsupported.

## 0.11.0 — classes (slice 1) + constants

Accepts `Feature::Classes` and `Feature::Constants` — the first slice of the OOP
frontier: an **empty base class** and its **construction**, plus the entangled
**constants** prerequisite.

- **Classes.** `Stmt::ClassDef { name, superclass: None, body: [] }` — an empty
  base class — is accepted, and `Foo.new(args…)` (the frontend's `__new__`
  builtin, whose first argument is the class name) constructs an instance.
- **Constants.** A `Scope::Const` assignment (`PI = 3`) and reference (`PI`,
  `Foo::Bar`) are accepted. Constants ride in with Classes because they are
  **entangled**: a class name IS a Ruby constant, so the frontend records
  `Constants` in the manifest for any `Foo.new` (the receiver `Foo` is a
  constant) — an instantiable class cannot compile without it. Accepting
  Constants also unblocks `raise SomeClass` (a specific exception class is a
  `Const` reference — a form the 0.10 exceptions slice deferred precisely
  because Constants was then unaccepted).

**Reflective definition (why not native `class Foo; end` / `PI = 3`).** The
frontend wraps a program's top-level code in `main`, and Ruby forbids BOTH a
`class` definition and a constant assignment inside a method body ("class
definition in method body" / "dynamic constant assignment"). So a class and a
constant are defined **reflectively**:

- `class Foo; end` → `Object.const_set(:Foo, Class.new)`
- `PI = 3` → `Object.const_set(:PI, 3)`

`const_set` is legal anywhere, executes in place (no fragile hoisting /
reordering), and still names the class (`Foo.name == "Foo"`, so `Foo.new` and
`x.is_a?(Foo)` work). Constant *references* (`Foo.new`, bare `PI`) emit the bare
constant, which resolves at runtime. This dynamic construction also composes
cleanly with the next slice's `define_method` for the frontend's hoisted,
separately-registered methods.

**Injection safety.** Every constant name emitted verbatim — a `ClassDef` name,
a `__new__` class name, a `Const` reference, and a `Const` assignment target —
is validated as a Ruby constant path (`Foo` / `Foo::Bar`) by the SAME single
pre-emit traversal that rejects unlowerable builtins (a unified `ScanHit`,
**co-total with the emitter**), so a hand-built module cannot inject source
through a crafted name.

**Totality — deferred shapes rejected cleanly (never `unreachable!`).** Accepting
`Classes` obligates handling every node it surfaces. This slice supports ONLY an
empty base class; the pre-emit scan rejects, with a source-positioned error,
everything deferred to later slices: a **superclass** (inheritance), a
**non-empty class body** (class-level code / constants), a **namespaced**
(`Foo::Bar`) class or constant *definition* (`const_set` names one namespace), a
**singleton class** (`class << self` — `Stmt::SingletonClassDef`, which also
observes `Feature::Classes`), and every **OOP method builtin** (`__def_method__`,
`__method__`, `__super__`, `__self__`, `__class_method__`, …) — so a
method-bearing, inheriting, or singleton-opening class fails cleanly rather than
mis-emitting. Instance variables (`@x`), class
variables (`@@x`), and modules remain unaccepted features (their own later
slices).

## 0.10.0 — exceptions (SIR17)

Accepts `Feature::Exceptions` — the first of the OOP/exception frontier, and
self-contained (a `rescue` clause matches by exception-class NAME, an advisory
string, so it is separable from `Classes`). Ruby handles exceptions natively, so
this needs no runtime support:

- `Stmt::TryCatch` renders `begin … rescue … ensure … end`. Each `rescue`
  clause lists its exception classes by name, optionally binds the caught
  exception to a local (`rescue Foo => e`), and runs its body; an empty class
  list is a bare catch-all. `ensure`, when present, runs afterwards.
- The `raise` builtin renders the native `raise` — bare (re-raise the exception
  being handled), with a message string (`raise "boom"` → `RuntimeError`), or
  with an exception object. `retry` renders the native `retry`.
- `raise SomeClass` (a specific exception class) lowers to a `Const` reference,
  which observes `Feature::Constants` (not accepted) → such a module is rejected;
  `raise "message"`, a bare re-raise, and `rescue` by a standard class
  (`StandardError`, …) or catch-all are the accepted forms.

**Injection safety**: a `rescue` clause's exception-type name is emitted verbatim
as a Ruby constant reference (it must stay capitalized, so it cannot be routed
through `sanitize_ident`). A `compile`-time gate rejects any module whose rescue
type is not a valid Ruby constant path (`Foo` / `Foo::Bar`) — so a hand-built
module cannot inject source through a crafted type name. Crucially, this check is
folded into the SAME single traversal as the unsupported-builtin pre-check
(a unified `ScanHit`), so it is **co-total with the emitter**: every `TryCatch`
the emitter can reach — including ones nested in a call argument, a function's
trailing value, a `SeqSet`/`MapSet` sub-expression, or any other expression
position — is validated. (Security review caught a first attempt using a
separate, hand-picked walk that missed several of those positions; the unified
walk cannot drift.) The caught exception's binding, being an ordinary local,
goes through `sanitize_ident` as usual; a `raise`d message string is quoted by
`quote_ruby_string`.

Documented limitation: a `rescue` by an advisory class name that is not a live
Ruby constant (a user-defined exception class, which needs the not-yet-accepted
`Classes` feature) raises `NameError` at runtime; standard classes and bare
`rescue` always work.

First of the exceptions parity arc: Go/Rust/Python/JS already accept `Exceptions`
(C is tracked next). Verified through a real `ruby` with hand-built modules: a
bare rescue catching a raised message, `ensure` always running, a rescue binding,
a typed `rescue StandardError`, the native emit shape, and the injectable-type
rejection. Bumps semantic-ir-to-ruby 0.9.0 → 0.10.0.

## 0.9.0 — keyword parameters (SIR19)

Accepts `Feature::KeywordParams`. Ruby has native keyword arguments, so this is
a direct emission — no positional resolution like the Go/C backends' KW6:

- A **keyword parameter** renders `def f(x:)` (required) or `def f(x: <default>)`
  (optional — a keyword default is an optional keyword, riding on
  `KeywordParams`, not `DefaultParams`).
- A **keyword argument** (`Expr::KeywordArg`) renders `x: <value>` in the call's
  argument list; Ruby binds it to the parameter by **name**, so keyword
  arguments are order-independent (`f(b: 2, a: 10)` binds `a`/`b` correctly). The
  label is sanitised identically to the parameter it binds, so the two agree.
- The unsupported-builtin pre-check (`scan_expr`) recurses into a keyword
  argument's value.

While restructuring the parameter loop, made it **total** over every
`ParamKind`: a `Rest` parameter now renders `*rest` and a `KwRest` renders
`**opts` (native Ruby), where both were previously mis-emitted as bare names.
This matters because a `**opts` co-occurs with keyword parameters, so accepting
`KeywordParams` must not leave it broken. (These kinds carry no feature of their
own — a validator matter — but the emitter now spells all four kinds correctly.)

First of the KeywordParams parity arc: Go/Python/JS already accept it; this
brings the Ruby backend up (C is tracked next; the Rust backend is a separate
gap). Verified through a real `ruby` with hand-built modules: a keyword argument
binding by name, order-independent resolution (`f(b: 2, a: 10)` → `8` for `f(a:,
b:) = a - b`), an optional keyword using its default when omitted (`f()` → `7`
for `f(x: 7)`) and overridden when supplied, native `x:` / `x: 5` syntax, and
the `*rest` / `**opts` splat emission. Bumps semantic-ir-to-ruby 0.8.0 → 0.9.0.

## 0.8.0 — default parameters (SIR19)

Accepts `Feature::DefaultParams`. A positional parameter carrying a default
expression renders as Ruby's **native** `def f(a, b = <default>)`. Ruby
evaluates the default at call time when the argument is omitted — exactly the
SIR semantics — so no runtime support is needed; it is a one-line addition to
the function-signature emitter (`name = <emit_expr(default)>` when
`p.default.is_some()`).

- The default may reference an **earlier parameter** (`def f(a, b = a)`): Ruby
  binds parameters left to right, matching the validator, which checks each
  default with the parameters declared before it in scope.
- Only the **positional** case is `DefaultParams`; a keyword default is the
  separate (still-unaccepted) `KeywordParams` feature, so it never reaches here.

Also extends the unsupported-builtin pre-check (`first_unsupported_builtin`) to
scan each parameter default, not just the body — a default is an expression
evaluated at call time, so a deferred builtin hidden in one (`def g(x = foo())`)
must be rejected cleanly rather than slip past the body scan and hit the
emitter's `unreachable!`. This keeps the emitter total for the feature.

Security review additionally caught a pre-existing hole the default scan would
inherit: `scan_expr`'s `IndirectCall` arm scanned only the call arguments, not
the callee `target` — yet the emitter renders the target (`sir_apply(<target>,
…)`), so a deferred builtin in the callee position could reach the
`unreachable!`. The arm now scans the target too.

First of the DefaultParams parity arc: Go/Rust/Python/JS already accept it; this
brings the Ruby backend up (C is the last, tracked next). Verified through a
real `ruby` with hand-built modules (a function with a defaulted parameter and a
`main` that calls it with and without the trailing argument): the default is
used when the argument is omitted (`f(1)` → `6` for `f(a, b = 5) = a + b`) and
overridden when supplied (`f(1, 2)` → `3`), a default referencing an earlier
parameter, and the deferred-builtin-in-default rejection. Bumps
semantic-ir-to-ruby 0.7.0 → 0.8.0.

## 0.7.0 — short-circuit (SIR16)

Accepts `Feature::ShortCircuit`. `Expr::LogicalAnd` / `Expr::LogicalOr`
(`&&` / `||`) render as Ruby's native short-circuit operators, which ARE the
SIR semantics exactly — no runtime helper, no coercion:

- They yield the **deciding operand**, not a coerced boolean: `1 && 2` is `2`,
  `false && 2` is `false`, `nil || 7` is `7`, `1 || 2` is `1`.
- They **skip the right operand** when the left already decides — Ruby `&&`
  does not evaluate its rhs when the lhs is falsy, and `||` does not when the
  lhs is truthy.
- Ruby truthiness is the SIR/Lisp convention (only `nil` and `false` are falsy),
  so the operands need no `sir_truthy` wrapper — unlike the Go/C backends, which
  must lift to an IIFE / hoisted `if` to return the operand value rather than a
  native bool.

These are distinct from the eager `and`/`or` **builtins** (which the emitter
also renders with `&&`/`||`); the `ShortCircuit` feature is specifically the two
short-circuit expression nodes. The unsupported-builtin pre-check
(`scan_expr`) now recurses into both operands, so a deferred builtin nested in a
`&&`/`||` is still reported cleanly. Two nodes, both handled → the emitter stays
total.

First of the ShortCircuit parity arc: Go/Rust/Python/JS already accept it; this
brings the Ruby backend up (C is the last, tracked next). Verified through a
real `ruby` with hand-built modules (the frontend constant-folds a literal
`&&`, so the node is built directly): operand-return for both operators, and a
short-circuit proof where the dead operand is `1 / 0` — a correct lowering skips
it (`false && (1/0)` → `false`, exits clean), a broken eager one would raise.
Bumps semantic-ir-to-ruby 0.6.0 → 0.7.0.

## 0.6.0 — floats (SIR16)

Accepts `Feature::Floats`. Ruby has a native `Float`, so this is a one-arm
addition: `Expr::FloatLit` renders directly as a Ruby float literal. The
feature gates ONLY `FloatLit` (float arithmetic reuses the existing
`+`/`-`/`*`/`/` builtins, which already fold to native Ruby operators), and the
runtime's `sir_fmt_float` already rendered every float — so accepting the
feature plus the one emit arm keeps the emitter total.

The literal is produced by a new `float_to_ruby_literal` helper, which fixes two
ways a naive `value.to_string()` would be wrong:

- **Integral floats must keep their point.** Rust's `f64::to_string` renders
  `7.0` as `"7"` — which Ruby parses as an *Integer* (a different type, with
  floor `/` instead of true divide, and `7` instead of `7.0` on display). The
  helper uses `{:?}` (Debug), whose shortest round-tripping form always carries
  a decimal point or exponent (`7.0`, `-0.0`, `1e300`) — every one a valid Ruby
  *Float* literal.
- **Non-finite values have no numeric token.** Ruby has no `inf`/`nan` literal;
  the values are `Float::INFINITY` / `-Float::INFINITY` / `Float::NAN`. A
  `FloatLit` carrying one (rare — it usually arises at runtime from `1.0 / 0.0`)
  now emits the named constant.

Because display routes through the runtime's `sir_fmt_float` (Ruby's own
`to_s`/`nan?`/`infinite?`), the printed form is native regardless of how the
literal was spelled — the helper only has to preserve the numeric value.
Verified end-to-end through a real `ruby` with hand-built modules (the frontend
masks `FloatLit`): integral floats keep `.0` (`7.0`, not `7`), `-0.0` keeps its
sign, `1.5 + 2.5 == 4.0` and `2.0 * 3.0 == 6.0` (integral results stay Float),
`7.0 / 2 == 3.5` while `7 / 2 == 3` (division frontier preserved — a Float
operand promotes, two Integers floor), `1.0 / 0.0 == Infinity` and `0.0 / 0.0 ==
NaN` (Float division by zero does not raise), and `7.0 == 7` is true.

## 0.5.0 — maps (SIR16)

Accepts `Feature::Maps`. Ruby has a native Hash, so the three map nodes render
directly — no runtime value-boxing like the Go/Rust backends' `_sir_map_*`:

- `Expr::MapLit` (`{k => v, …}`) → a native Hash literal.
- `Expr::MapGet` (`h[k]`) → `(h)[k]`: a missing key yields nil (no raise),
  matching `_sir_map_get`.
- `Stmt::MapSet` (`h[k] = v`) → `(h)[k] = v`: insert-or-update, mutating the
  shared Hash (a write through one binding is visible through every alias). A
  map has no bounds, so — unlike `SeqSet` — no guard helper is needed.

Ruby's Hash preserves insertion order and compares keys with `eql?`/`hash`,
which is STRUCTURAL for composite keys — so `{[1, 2] => x}[[1, 2]]` finds the
entry, matching the reference's `_sir_value_eq` key comparison. (One documented
divergence: `eql?` is type-strict for numbers, so a Ruby `{1 => x}[1.0]` is nil
where the reference's cross-representation `_sir_value_eq` would match; a
mixed int/float map key is rare and not exercised by any conformance case.)

`ForEach` over a Hash needs no new arm — the existing `(iter).each { |x| … }`
works on a Hash (yielding `[k, v]`) as well as an Array — so accepting Maps
keeps the emitter total. Every node verified by hand-built modules (bypassing
the frontend, which does not yet produce these), run against a real `ruby`.

## 0.4.0 — sequences (SIR16)

Accepts `Feature::Sequences`. Ruby has native arrays, so the SIR16 sequence
nodes render directly — no runtime value-boxing like the Go/Rust backends'
`_sir_seq_*`:

- `Expr::SeqLit` (`[1, 2, 3]`) → a native array literal. Structural `Array#==`
  makes `[1, 2] == [1, 2]` true, matching every backend that carries sequences.
- `Expr::SeqIndex` (`a[i]`) → `(a)[i]`. Ruby's `Array#[]` already matches the
  SIR reference exactly: a negative index counts from the end, an out-of-range
  index returns `nil` (never raises — that is `fetch`).
- `Expr::SeqLen` (`len a`) → `(a).length`.
- `Stmt::SeqSet` (`a[i] = v`) → `sir_seq_set(a, i, v)`, a new runtime helper
  that enforces the reference's bounds rule (RAISES on a negative or
  out-of-range index, unlike Ruby's native `[]=` which pads with nils / counts
  from the end) and returns the assigned value.
- `Stmt::ForEach` (`for x in a`) → `(a).each { |x| … }` — reachable once
  `Loops` is also accepted. A BLOCK, so `x` (and any body-local) is
  block-scoped, matching the validator (which rewinds the loop body) and the
  Go reference (`for _, x := range`, block-local via `:=`); a leaking `for …
  in` would instead clobber an enclosing same-named local. `ForRange` is
  block-scoped the same way, via a hoisted `->(x) { … }` body called from the
  `while`. Safe as blocks because SIR loop bodies have no break/next/return.

Also fixes a **pre-existing** panic surfaced while making the emitter total:
`Stmt::ForRange` (`for i in 0...3`) is gated by `Feature::Loops` alone
(accepted since 0.3.0) and is produced by the Ruby frontend, yet was sent to
the same `unreachable!` — so a numeric `for` loop crashed the backend. It now
desugars to a `while` mirroring the Go/Rust backends: bounds evaluated once
into nesting-safe `sir_`-prefixed temporaries, a direction-aware exclusive stop
(`step >= 0 ? i < stop : i > stop`, so a descending loop works), and a
block-scoped loop var (the body runs inside a hoisted `->(i) { … }`, so `i`
does not clobber an enclosing same-named local).

Handling all five sequence nodes plus `ForRange` keeps the emitter TOTAL for
its accepted feature set: no conforming producer (Ruby, C→SIR, Twig→SIR, …) can
reach an `unreachable!`. **This was
caught by security review** — an earlier revision handled only `SeqLit` on the
false premise that it was the only `Sequences`-gated node; in fact `SeqIndex`/
`SeqLen`/`SeqSet` are also gated by `Sequences` (the `NDArrays`-gated
`IndexGet`/`IndexSet` are the different SIR22 nodes), and `ForEach` becomes
reachable once `Loops` is accepted — all four would have panicked the emitter
for a non-Ruby producer. Verified with hand-built modules (bypassing the Ruby
frontend, which masks these nodes) for each of the five.

Array *indexing via `Expr::IndexGet`* and slicing are a DIFFERENT feature
(`NDArrays`, not accepted); array-*pattern* destructuring needs `ShortCircuit`
(not accepted) — so those stay rejected at the feature gate.

The `scan_expr`/`scan_stmt` unsupported-builtin pre-check recurses into the new
nodes' sub-expressions too, so an unsupported builtin nested in `[foo()]`,
`a[foo()]`, or `for x in [foo()]` is reported cleanly. It also gains a `While`
arm — a pre-existing hole (also found by the review): an unsupported builtin in
a `while` body previously escaped the pre-check and hit the emitter, so it now
rejects cleanly instead of panicking.

## 0.3.0 — control flow & mutation (SIR16)

Accepts `Feature::Loops` and `Feature::MutableBindings`, and renders the two
statements the C frontend's milestone-2 `if`/`while`/`for` produce:

- `Stmt::While { cond, body }` → Ruby `while sir_truthy(<cond>) … end` (the
  condition, already a bool, is re-tested each iteration).
- `Stmt::Assign { name, value }` → `name = value` (Ruby locals are mutable).

`Expr::If` and the comparison builtins were already rendered, so a C `for`-loop
now round-trips to running Ruby.

## 0.2.0 — render SIR26 integer conversions

Accepts `Feature::Conversions` (plus the SIR21 type-implied `SizedIntegers`,
`Unsigned`, `WrappingArithmetic`) and renders `Expr::Convert` — the C→SIR→Ruby
payoff.

- A conversion emits an inlined mask helper chosen by target width + signedness:
  `sir_u8`/`sir_u16`/`sir_u32`/`sir_u64`/`sir_u128` (mask) and
  `sir_i8`/`sir_i16`/`sir_i32`/`sir_i64`/`sir_i128` (mask then two's-complement
  sign-fold).  A target width of `Arbitrary` is the identity (a widen into
  Ruby's already-unbounded `Integer`) and emits no wrapper.
- The masking is exact for every width because Ruby's `Integer` is arbitrary
  precision and its bitwise ops use a two's-complement model — so `sir_u8(-1)
  == 255`, `sir_i32(4_000_000_000) == -294_967_296`.
- Verified end-to-end through a real `ruby`: `sir_u8(300)==44`,
  `sir_i32(4e9)==-294_967_296`, `(uint32_t)-1==4_294_967_295`,
  `(int8_t)200==-56`, arbitrary-width identity.

## 0.1.0 — v0 core (SIR25)

First release of the Ruby backend — the seventh SIR backend and the first Ruby
*target* (Ruby was previously only a frontend).

### Added

- `compile(module)` / `RubyBackend` implementing `semantic_ir::Backend`
  (`target_tag() == "ruby"`).
- **Self-contained** emission: a single `.rb` file with a small inlined runtime
  preamble (`SirPair`, a `$sir_globals` store, `sir_truthy`, display helpers
  that honour the display convention, `sir_eq`, `sir_apply`, and a
  builtin-as-value dispatcher).  Runs with `ruby <file>.rb`, no gems.
- **Expression-oriented lowering**: because Ruby's `if`/`begin…end` yield values
  and a method returns its last expression, `Block`/`If` render directly — no
  IIFE or statement-hoisting.  `MakeClosure` renders as a native lambda that
  binds the capture values and splats the call arguments; `IndirectCall` is
  `target.call(*args)`.
- v0 capability set (`Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
  `OptionalTypeAnnotations`, `MutualRecursion`, `Globals`) plus the core
  builtins `+ - * / % neg = == != < > <= >= not and or cons car cdr null? pair?
  number? symbol? print puts global_get global_set` (mostly native Ruby, whose
  semantics are the reference).
- A structural gate rejecting builtins the v0 backend cannot lower (e.g. the
  `__method__`/`case_eq` collection-dispatch protocol), so a module using a
  later feature fails cleanly rather than emitting a call with no lowering.
- Identifier sanitisation (Ruby keywords, the `sir_` runtime namespace, and
  leading-uppercase locals) and string/symbol escaping that neutralises `#{…}`
  interpolation so no source text can inject.
- Display-convention substitution (`__SIR_DISPLAY_RUBY__` → a boolean-selected
  literal, never source text).

### Wiring

- Added to the Rust workspace `members`.
- `sir-conformance` gains a `Target::Ruby` arm (`run_ruby`, `ruby` toolchain,
  skip-if-absent); a program whose feature set v0 does not accept is *skipped*
  (a declared gap), not failed — mirroring the C backend.

### Verified

- `cargo test -p semantic-ir-to-ruby` green (emit-shape + end-to-end via `ruby`).
- `cargo test -p sir-conformance` green: the Ruby cells run every v0-accepted
  corpus program and match the reference oracle byte-for-byte.
