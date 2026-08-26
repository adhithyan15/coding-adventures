# Changelog

## 0.42.1 — Security fix: `sanitize_ident` was not injective

Task #65 (`/security-review`, discovered while auditing `java-to-
semantic-ir`'s own loop-control synthetic-flag naming): this backend's
`sanitize_ident` escaped a Go-keyword/predeclared-identifier collision
with a bare trailing underscore (`"func"` -> `"func_"`), but a
completely ordinary, unrelated SIR local literally named `func_` passed
through **unchanged** — so two distinct raw SIR names collided on the
same emitted Go identifier, silently aliasing two variables into one
with no error anywhere in the pipeline.

Fixed by making the passthrough and marked output sets disjoint by
construction: every non-passthrough case is prefixed with a reserved
`sir_esc_` marker, and any raw name that already starts with that
marker is itself routed into the marked case rather than allowed to
pass through — see `sanitize_ident`'s own doc comment for the full
argument. Also switched the illegal-character escape from unpadded
`_{hex}` to fixed-width `_u{4-hex}_`, closing a separate hex-digit-count
ambiguity the same review round found.

**A second `/security-review` round found the marker alone still wasn't
enough**: a *valid*, marker-prefixed name kept verbatim after the marker
could still collide with a *different*, illegal-character name that
happens to escape to that exact same text. Fixed by tagging the two
marker sub-cases with distinct fixed characters (`v`/`e`) immediately
after the marker, so they can never collide with each other regardless
of content — see `sanitize_ident`'s own doc comment for the full
argument.

This changes the exact spelling `sanitize_ident` produces for every
keyword/predeclared-identifier/invalid-character case (e.g. `"func"` now
sanitizes to `"sir_esc_vfunc"`, not `"func_"`) — a deliberate, disclosed
behavior change: the old spellings were exactly the ones proven to
collide.

## 0.42.0 — SIR23 Tier A pattern matcher (second-wave backend rollout, Phase A Slice 4)

Implements the 7-node SIR23 Tier A slice — the symbolic-expression
pattern matcher and rewrite engine — as one of five parallel per-backend
PRs for this rollout (Ruby merged first as precedent; C/Rust/Python are
sibling PRs): `Expr::SymSymbol`/`SymRational`/`SymApply`/
`SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`. New
`Feature` flags accepted: `SymbolicExpr`, `PatternMatching`, `Rationals`
(the last shared with the SIR22 array/matrix domain rather than a flag
of its own, per the SIR23 spec). Tier B (`evalTerm` — arithmetic/
calculus/user-function folding: `Add`/`Sin`/`D`/...) is explicitly OUT
OF SCOPE for this slice; a `SymApply` builds an inert term tree, nothing
more.

**New `_sir_cas_*` runtime functions + `*SirCasTerm` type (`runtime.rs`)**:
an inlined port of `semantic-ir-to-javascript`'s already-proven
`Symbolic` sub-runtime's Tier A slice — term construction (`symbol`/
`int`/`rational`/`float`/`string`/`apply`), `matchPattern`/
`substituteTerm`/`applyRuleTerm` (the five-case structural matcher:
`Blank()`, `Blank(T)`, `Pattern(name, inner)`, compound-vs-compound,
plain structural equality), and `replaceAll`/`replaceRepeated` (`/.` /
`//.`) with their DoS guards. Cross-checked against the sibling Ruby
backend's own port (`sir23-ruby-matcher`, merged as PR #12128).

**Naming**: `_sir_cas_*` / `SirCasTerm`, NOT `_sir_sym_*` — this backend
already owns that prefix for Ruby `Symbol#to_proc`/`Symbol` method
dispatch (`_sir_sym_to_proc`, `_sir_symbol_*`), an unrelated domain. The
same class of landmine Slice 2 hit with `_sir_array_*` (forced the
rename to `_sir_ndarray_*`); this slice checked first and picked a
distinct prefix (`_sir_cas_*`, for "Computer Algebra System", matching
the SIR23 spec's own `cas-pattern-matching` crate name) up front.

**Value model**: `*SirCasTerm`, one kind-discriminated Go struct
(`sirCasSymbol`/`sirCasInteger`/`sirCasRational`/`sirCasFloat`/
`sirCasString`/`sirCasApply`) — the same established pattern this
backend already uses for SIR22's `*NDArray`, rather than one Go type per
kind. `Head`/`Args` are typed `Value`/`[]Value` (not `*SirCasTerm`/
`[]*SirCasTerm`) so every `_sir_cas_*` function accepts the same boxed
interface every other runtime value flows through, asserting the
concrete type once internally (`_sir_cas_as_term`, mirroring
`_sir_ndarray_as_ndarray`'s identical discipline).

**Bindings**: `map[string]Value`, copy-on-write via a manual
full-copy-plus-insert in `_sir_cas_bindings_bind` (no persistent-map
library exists anywhere in this repo's Go-facing dependency graph, and
Go has neither Ruby's cheap `Hash#merge` nor a built-in structural-
sharing map) — a failed match attempt never mutates a binding set an
earlier attempt still holds a reference to.

**DoS guards (CWE-674)**: `sirCasMaxTermDepth = 512` caps every tree
WALK (`_sir_cas_walk_once`/`_sir_cas_replace_repeated`'s recursive
descent into `head`/`args`, term equality, and display) — NOT the
matcher functions (`_sir_cas_match_pattern`/`_sir_cas_substitute`/
`_sir_cas_apply_rule`), which recurse only as deep as one rule's own
author-written shape and need no cap. `_sir_cas_replace_repeated` also
enforces `sirCasMaxIterationsDefault = 100`, a GLOBAL fixed-point
iteration cap shared across the whole walk (a `for` loop at each call
frame, never a recursive call per firing, so a rule firing repeatedly at
one tree position costs O(1) native stack frames). Both caps `panic`
directly with a `fmt.Sprintf`-formatted message the moment they are
exceeded — a DELIBERATE departure from the Ruby/JS references (which
thread a sentinel value back up through every recursive return,
unwrapping/raising only once the walk fully returns): this backend's own
`_sir_ndarray_checked_shape_size` already established "panic immediately
with a controlled message" as the house convention for this exact class
of guard, and Go's `panic`/`recover` gives a clean, catchable unwind
through however many frames are on the stack — the compiled `TryCatch`
rescue path recovers ANY panic value, not just `*SirError` — so there is
no need for hand-rolled sentinel propagation here.

**Display**: `_sir_cas_to_s` renders a term in generic `head(args, ...)`
form (no Wolfram infix/precedence pretty-printing — separate follow-up
work), wired into `_sir_format_d` (the existing `print`/`puts` dispatch)
via a `*SirCasTerm` case. SIR22's own `*NDArray` deliberately has no
display path, but this slice's own reference tests (ported from the JS
backend's suite) print terms directly and assert on stdout, so a
display path is load-bearing here.

**Emit (`emit.rs`)**: replaces the combined SIR23 deferred-node `panic!`
arm with seven real per-variant codegen arms, plus an `emit_sym_operand`
helper (ported from the Ruby/JS backends' identically-named helper) that
wraps a bare `IntLit`/`FloatLit`/`StrLit` operand through the matching
`_sir_cas_*` leaf constructor before it sits inside a term tree. The
SIR23 arm was already cleanly split from the unrelated SIR26 `Convert`
deferred-node arm (a prior slice's own split) — no further separation
needed this time.

**Tests (`tests/sir23_symbolic.rs`)**: ported from `semantic-ir-to-
javascript`'s own proven suite, Tier A cases only —
`replace_repeated_reduces_nested_add_zero_to_bare_symbol`,
`replace_all_single_pass_does_not_retry_at_same_position`,
`typed_blank_matches_only_constrained_head`, `a_rational_term_prints_
reduced`, plus `depth_limit_guard_panics_instead_of_crashing_the_go_
stack` — a REAL compiled `Stmt::ForRange` loop builds 600 levels of
`Wrap(...)` nesting at runtime (not a hand-built static AST), then
`replaceAll` over it must panic cleanly instead of overflowing the Go
stack. All five hand-build a `Module` directly (no frontend targets this
backend for SIR23 yet), emit Go, and run it with a real `go run`,
skipping (not failing) when no `go` is on `PATH` — mirrors
`sir22_array.rs`'s established pattern.

## 0.41.0 — SIR22 "APL addendum" (second-wave backend rollout, Phase A Slice 3)

Implements the 9-node SIR22 "APL addendum" this backend's own Slice 2
entry (below) explicitly deferred: `Expr::Reduce`/`Scan`/`OuterProduct`/
`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`. No new
`Feature` flags — the addendum shares `NDArrays`/`MatrixOps`/
`ArrayColumnMajor` with the base cut, per
`code/specs/SIR22-array-matrix-semantic-ir.md`.

**New `_sir_ndarray_*` runtime functions (`runtime.rs`)**: `reduce`/
`scan`/`outer` (which reuse `_sir_ndarray_apply_op`, the same 13-op
dispatch table `_sir_ndarray_elementwise` uses), `flatten_row_major`
(an internal helper both `reshape` and `ravel` need — not itself one of
the nine node-backing functions), `shape`, `reshape`,
`index_generator`, `index_of`, `ravel`, `catenate`. Ported 1:1 from
`semantic-ir-to-javascript`'s own already-proven addendum functions
(`runtime.rs`'s FIRST "SIR22 addendum: APL primitive operators" section
— that file has a second, unrelated occurrence of the same section
header inside its `#[cfg(test)]` module), which are themselves 1:1
ports of `array_runtime::ops::{reduce,scan,outer}` and
`apl_runtime::builtins::{shape,reshape,index_generator,index_of,ravel,
catenate}`.

Three subtleties carried over faithfully from the reference:

- **`reduce`/`scan` on a rank-2 matrix fold EACH ROW independently**,
  not the whole matrix into one value — column-major storage means
  `(row, col)` lives at `col*r+row`, so the row loop reads `d[row]` as
  the seed (column 0) then walks `d[col*r+row]` for the rest; swapping
  `row`/`col` here would silently TRANSPOSE the result instead of
  crashing. `tests/sir22_array.rs`'s
  `reduce_folds_each_row_of_a_matrix_independently` pins the correct
  per-row result against a naive whole-matrix-fold regression.
- **`reshape` fills row-major, but must TRANSPOSE into column-major
  storage.** APL's reshape fills the last axis fastest (row-major,
  same convention as ravel); this domain stores column-major. Handing
  the row-major-filled sequence straight to the constructor would
  silently reshape column-major instead — a wrong answer that still
  LOOKS plausible (right multiset of values, wrong positions).
  `tests/sir22_array.rs`'s
  `reshape_fills_row_major_then_transposes_into_column_major_storage`
  reshapes a non-square `2x3` to `3x2` and asserts on element positions
  that differ between the correct and the un-transposed-bug answer.
- **`IndexGenerator`/`IndexOf` are 1-based**, unlike every other index
  in this domain (`IndexGet`/`IndexSet` are 0-based) — this is
  deliberate: it is genuinely what APL's monadic/dyadic `⍳` mean at the
  surface-syntax level, confirmed against `apl_runtime::builtins::
  index_generator`'s own tests (`index_generator_produces_one_based_run`)
  and `index_of`'s doc comment. `IndexOf`'s "not found" case returns
  `len(haystack) + 1` — always a valid, in-range position, never `-1`.
  **Note**: `semantic-ir`'s own `Expr::IndexGenerator` doc comment
  (`nodes.rs`) currently claims 0-based, which is stale/incorrect —
  `apl_runtime::builtins::index_generator`'s source and tests are the
  ground truth this port follows; fixing that doc comment is flagged as
  a small follow-up, out of scope for this PR.

**DoS discipline preserved exactly**: every function validates its
output size via `_sir_ndarray_checked_shape_size` (the Slice-2
overflow-safe validator) BEFORE allocating — `outer`'s `[m, n]` output
(two independent operand lengths, neither bounds their product alone),
`index_of`'s `haystack.length * needle.length` product (an
`O(len(haystack)*len(needle))` scan, capped before scanning, not after),
`catenate`'s combined length (checked ONCE up front regardless of which
of its five rank combinations follows — a script that repeatedly
catenates a value with itself doubles the size every line with no other
ceiling), `index_generator`'s `n`, `reshape`'s target size. `reshape`'s
and `index_generator`'s scalar-argument validation additionally guards
`+Inf`/`NaN` explicitly (`math.IsNaN`/`math.IsInf`), not just `x !=
math.Trunc(x)` — `math.Trunc(+Inf) == +Inf` would otherwise slip an
infinite dimension past a naive integer check.

**`emit.rs`**: the nine addendum nodes previously shared ONE combined
`panic!` match arm with SIR26's `Expr::Convert` (a landmine Slice 2's
own entry flagged: "these 7 [base-cut] nodes previously shared ONE
combined panic arm with the 9 SIR22-addendum nodes AND `Expr::Convert`").
This PR splits that arm: the nine addendum nodes get real
`_sir_ndarray_*` call-emission (mirroring `ElementwiseOp`/`Transpose`'s
existing style — `Reduce`/`Scan`/`OuterProduct` reuse
`elementwise_op_go_name` exactly like `ElementwiseOp` does), and
`Expr::Convert` moves into its own untouched arm with its original panic
message (now naming only SIR26, not "SIR22-addendum/SIR26"). `cargo
test -p semantic-ir-to-go` stayed 100% green throughout, confirming no
`Expr::Convert` regression.

**`lib.rs`**: `check_exception_soundness`'s nine `unsupported_sir22_
addendum(...)` rejection arms (and that now-dead helper function) are
removed; `check_soundness_expr` instead recurses into each addendum
node's sub-expression(s), the same treatment every other real/supported
composite node already gets (mirrors the base-cut `ArrayLit`/`Range`/
`MatMul`/etc. arms Slice 2 added). `ACCEPTED_FEATURES`'s doc comment and
this function's own doc comment are updated to say the addendum is no
longer deferred.

**`tests/sir22_array.rs`** (12 new tests, all real `go run` execution):
`reduce` on a vector AND a matrix (the row-independent-fold proof
above), `scan` on a vector, `outer` product of two vectors, `shape` of a
scalar (proven via a `shape(shape(5))` double-application trick — reading
index 0 of the double-shape yields `0.0` only if the inner `shape(5)` is
correctly a length-0 vector, not a scalar) and of a matrix, `reshape`
(the transposition-correctness proof above), `index_generator` (1-based),
`index_of` found AND not-found, `ravel` of a matrix, `catenate` of two
vectors and of two equal-row-count matrices, and a DoS-guard test
(`outer_product_output_shape_exceeding_the_element_cap_panics_cleanly`)
mirroring Slice 2's own `matmul_output_shape_exceeding_the_element_cap_
panics_cleanly` pattern. None of the addendum nodes have direct literal
syntax available in a hand-built `Module`, so genuine rank-1 "vector"
test fixtures (as opposed to a `[1, n]` row-vector-shaped MATRIX, which
is what `array_lit` with one row and `Range` both produce) are built via
nested scalar `Catenate` — itself one of the nodes under test. Verified
with a real `go vet` + `go run` pass on a hand-assembled module
exercising all nine nodes together (via a throwaway example, not
committed) as an additional sanity check beyond the crate's own test
harness.

## 0.40.0 — SIR22 array/matrix base cut (second-wave backend rollout, Phase A Slice 2)

Opens this backend to `Feature::NDArrays`/`Feature::MatrixOps`/
`Feature::ArrayColumnMajor` and implements the SIR22 array/matrix "base cut"
— `Expr::ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`
and `Stmt::IndexSet` (the mutating counterpart) — per
`code/specs/SIR22-array-matrix-semantic-ir.md`'s "Backend impact" section,
which named this backend (alongside C/Rust/Ruby) as a planned second wave
following JS/TS's own already-shipped SIR22 codegen. The 9-node SIR22 "APL
addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) remains explicitly OUT of
scope for this PR — a later slice.

**New `_sir_ndarray_*` runtime (`runtime.rs`)**: an inlined port of
`semantic-ir-to-javascript`'s own already-proven `ArrayRt` sub-runtime,
following this backend's existing inlined-runtime convention (no `go.mod`
dependency, pasted verbatim into every artifact, same as every other
runtime helper here). Ported 1:1: `checkedShapeSize`, `ndarray`/`fromRows`,
`toArrayValue`'s bare-scalar coercion, `isScalar`/`nrows`/`ncols`,
`get`/`set` (NaN-safe AND-form bounds checks), `applyOp`'s 13-op elementwise
dispatch table (booleans render `1.0`/`0.0`, never a native Go `bool`),
`elementwise`'s scalar-broadcast rule, `matmul`, `transpose`, `range`
(MATLAB-style `start:step:stop`, ULP-tolerant inclusive-stop boundary),
`resolvePositions`/`assertValidPosition` (index-argument resolution),
`indexGet`/`broadcastValues`/`indexSet`.

**Naming**: every new helper is prefixed `_sir_ndarray_`, NOT `_sir_array_`
— this backend's PRE-EXISTING Ruby-`Array`(`*Seq`)-method-dispatch catalog
(C5's `_sir_array_method`/`_sir_array_responds`/`_sir_array_block_method`)
already owns the `_sir_array_*` prefix for an entirely unrelated domain
(Ruby's `Array` == this runtime's `*Seq`, not a SIR22 numeric matrix).
Reusing that prefix would have collided outright with real existing
functions of the same shape (both take a receiver + do a name-keyed
dispatch) — a footgun worth flagging explicitly since a naive port of the
JS reference's `Array.*` naming would have walked straight into it.

**Value representation**: a `*NDArray{Shape []int; Data []float64}` — shared
+ mutable via a pointer handle, exactly like this runtime's existing
`*Seq`/`*Map` (`Stmt::IndexSet` must mutate the very array a caller's
binding already holds, MATLAB assignment semantics, mirroring why `*Seq`
and `*Map` are pointer-backed too). Every element is stored as a **uniform
`float64`** — a deliberate divergence from the sibling Ruby backend's SIR22
port (`claude/sir22-slice2-ruby`), which preserves native Ruby
Integer/Float propagation (Div/Pow force Float; Add/Sub/Mul don't,
following that crate's own `div_true` precedent). MATLAB's OWN numeric
model is "every array is a matrix of doubles" by default — there is no
separate integer-array type in the MATLAB subset this repo's frontends
target — so uniform `float64` storage is the semantically faithful choice
here, not a Go-specific compromise, and it is exactly what the JS reference
this file ports from already does (`Float64Array` throughout). Go's own
numeric tower (`int64`/`float64` boxed into the existing `Value`
interface{}) has no "array of numbers" precedent of its own worth
preserving the way Ruby's native `Integer` class does, so this port stays
byte-for-byte equivalent to the JS reference's uniform-double model instead
of introducing a new Go-specific split. One consequence worth flagging: an
all-integer computation (e.g. `[1 2; 3 4] * [5 6; 7 8]`) prints WITH a
trailing `.0` on this backend (Go's own `_sir_format_float` convention),
unlike Ruby's bare-integer output for the same computation — see
`tests/sir22_array.rs`'s own module doc.

**DoS discipline**: every constructor validates a shape/output size BEFORE
allocating a `[]float64` from it (`_sir_ndarray_checked_shape_size`,
called by `ndarray`/`fromRows`/`matmul`/the 2-index-argument paths of
`indexGet`/`indexSet`), mirroring the JS reference's `MAX_ELEMENTS =
1 << 26` cap exactly. Go-specific tightening beyond a literal JS port: the
running shape-size product is checked for overflow ONE multiplication at a
time (`acc > cap/d` BEFORE `acc *= d`), not after — a JS `Number` cannot
silently wrap on overflow (only lose precision past 2^53), but Go's `int`
is a fixed-width two's-complement type where `acc *= d` for a sufficiently
large shape CAN wrap around to a small or negative `int`, which would slip
straight past a naive post-hoc `> cap` check. `tests/sir22_array.rs`'s
`matmul_output_shape_exceeding_the_element_cap_panics_cleanly` proves this
end-to-end: two independently-under-cap `1x9000`/`9000x1` operands whose
`9000x9000` matmul OUTPUT would exceed the cap panic cleanly (a readable
message on stderr) instead of attempting the ~650MB allocation.

**`emit.rs`**: added 6 new `Expr` match arms + 1 `Stmt` arm calling into the
new runtime, all following this backend's existing emit style
(`elementwise_op_go_name`/`emit_index_arg`/`emit_index_args` mirror the JS
backend's `elementwise_op_js_name`/`emit_index_arg`/`emit_index_args`
exactly). **Landmine avoided**: these 7 nodes previously shared ONE
combined `panic!` match arm with the 9 SIR22-addendum nodes AND
`Expr::Convert` (SIR26) — split carefully so `Expr::Convert`'s existing
(untouched) panic behavior is preserved byte-for-byte in its own arm,
while the addendum nodes' arm was updated to explain that a NEW soundness
gate (not the capability gate) is what actually protects them now (see
below). `cargo test -p semantic-ir-to-go` stayed 100% green throughout,
confirming no `Expr::Convert` regression.

**Addendum rejection**: the 9-node APL addendum shares
`NDArrays`/`MatrixOps`/`ArrayColumnMajor` with the base cut, so — now that
this PR adds those three flags to `ACCEPTED_FEATURES` — the ordinary
feature-flag capability check alone can no longer distinguish "safe" base-
cut modules from "still unimplemented" addendum ones. Unlike the JS
backend's now-removed `find_unimplemented_sir22_addendum_node`
(`Visitor`-based, a second separate walker) or the Ruby backend's
`ScanHit::Sir22AddendumNode` (folded into that crate's single shared
pre-emit scan), this backend already had its OWN pre-existing "reject a
well-formed-but-unimplemented construct cleanly" mechanism —
`check_exception_soundness`/`check_soundness_stmt`/`check_soundness_expr`
(added for E3, extended for O4/MX5 since) — called once from `compile()`
right beside the manifest gate. This PR extends THAT existing walk rather
than adding a third parallel mechanism: nine new `check_soundness_expr`
arms push a clean `BackendError` naming the offending node
(`unsupported_sir22_addendum`), and the six base-cut node kinds gained
recursion arms so nested addendum/`Const` usage inside e.g. an `ArrayLit`
row is still caught. `Stmt::IndexSet`'s prior `panic!` placeholder arm (in
`check_soundness_stmt`) was replaced with real recursion into its three
sub-positions, mirroring the existing `Stmt::SeqSet`/`Stmt::MapSet` arms —
it is a genuinely supported statement now, not a deferred one.

**New `tests/sir22_array.rs`** (9 tests): real `go run` execution proof,
hand-building `Module`s directly (no frontend targets this backend for
SIR22 yet) — ported from the JS backend's own `tests/sir22_array.rs`
worked examples (matmul of two 2x2 matrices against a known product,
elementwise-mul with a bare-scalar RHS operand, `Div`'s always-true-divide,
transpose, a MATLAB-style range read by linear index, a `Whole` (`:`)
selector reading an entire row, `Stmt::IndexSet` mutating in place), plus
two new tests: the shape-overflow DoS-guard proof described above, and a
compile-time-rejection proof for `Expr::Reduce` (an addendum node) —
`compile()` returns a clean `Err` naming the node, not a panic. Uses a
PID + monotonic-counter temp-filename scheme (not just a per-test tag)
to avoid the concurrent-`cargo-test` filename race this session's C-backend
tests already guard against.

## 0.39.0 — SIR21 T3b-2 Slice 2: `div_floor`/`div_trunc`/`udiv_trunc`/`div_true`

Additive only — no frontend emits these names yet, bare `"/"` keeps working
unchanged. Both dispatch sites (the single unified emit-name table and
`_sir_call_builtin_by_name` in `runtime.rs`) gained entries for the four new
canonical division op names, per
`code/specs/SIR21-type-system-and-integer-semantics.md` §E3:

- `div_floor` → `_sir_divide` (the SAME helper `/` already uses — a rename,
  zero new logic).
- `div_trunc` → new `_sir_trunc_div`: Go's native int64 `/` already
  truncates toward zero, so this is a thin wrapper (unlike `_sir_divide`'s
  int path, which explicitly adjusts for floor semantics).
- `udiv_trunc` → new `_sir_utrunc_div`: `div_trunc`'s unsigned twin —
  reinterprets the `int64` bit pattern as `uint64` before dividing (this
  backend has no typed-unsigned frontend reaching it yet, but the helper is
  implemented faithfully, mirroring `semantic-ir-to-c`'s existing
  `_sir_itdiv`/`_sir_utdiv` split, the first backend to need this
  distinction).
- `div_true` → new `_sir_true_div`: always coerces both operands to
  `float64` and divides, regardless of operand tag. Raises
  `ZeroDivisionError` unconditionally, matching `_sir_divide`'s own
  float-path convention (Python's `/` behaves the same way) — IEEE `+Inf`
  is never a silently-produced result here.

New `tests/compile_and_run_division_ops.rs`: real `go run` compile-and-execute
proof for all four ops (6 tests, mirrors `compile_and_run_floats.rs`'s
pattern) — §E3's own worked example, the floor-vs-truncate divergence on
negative operands, and the zero-divisor panic path (Go's uncaught-panic
default behaviour: nonzero exit + `SirError.Error()`'s formatted message on
stderr) for every one of the four ops — no existing test covered this path
before this PR.

## 0.38.0 — SIR28 §7: remove dead bare `print`/`puts` handling

Every frontend now emits `__sys_write__` instead of bare `print`/`puts`
(SIR28 Slices 4-6, all merged), so this backend's `print`/`puts` emit
arms, runtime helpers, and by-name builtin dispatch entries are dead
code. Removed:

- The `"print" => "_sir_print"` / `"puts" => "_sir_puts"` arms from
  `emit_builtin`'s helper-name match.
- `_sir_print`, `_sir_puts`, and `_sir_puts_one` from `runtime.rs` (these
  were fully independent of `_sir_write`/`_sir_write_one` — confirmed via
  grep that nothing else called them — so this is a straight deletion,
  not a refactor).
- The `case "print":` / `case "puts":` arms from
  `_sir_call_builtin_by_name`.

Also fixed two stale doc-comment references to the deleted functions
(the `flatten` case in `_sir_call_method` and `_sir_flatten_into`'s doc
comment now cite `_sir_write`'s `per_value` terminator instead).

This is a breaking change for any SIR module that still emits bare
`print`/`puts` — none do, in this monorepo, as of SIR28 Slice 6.

Test suite: every local test helper that hand-built bare `print`/`puts`
`BuiltinCall`s purely to observe hand-constructed IR's output (unrelated
to testing print semantics itself) now builds the equivalent
`__sys_write__` envelope instead, plus `Feature::ConsoleIO` (and, where
missing, `Feature::Strings`) added to each affected manifest. A
single-value `print`-shaped helper maps to `terminator: "once"` (not
`"none"`) where the backend's old bare `print` historically always
newline-terminated (true for Go, via the deleted `_sir_print`'s
`fmt.Println`) — this preserves existing `stdout.lines()`-based test
assertions exactly, since these helpers were never asserting on real
Ruby `print` semantics to begin with.

## 0.37.3 — implement `__sys_write__`, the SIR28 console-output primitive

Adds a `"__sys_write__"` emit arm (mirroring `__new__`'s "lift a
compile-time-known `StrLit` to a quoted Go string literal" discipline) and
a new runtime function, `_sir_write`/`_sir_write_one`, generalizing the
existing `_sir_print`/`_sir_puts` into one function parameterized by
`stream` (stdout/stderr), `terminator` (none/per_value/once), and
`unpackArrays` — the policy axes SIR28 §2.1 defines. Declares
`Feature::ConsoleIO`. Adds `"os"` to the always-emitted import list
(needed for `os.Stdout`/`os.Stderr`).

`stream`/`terminator` are lifted to quoted Go string literals at emit
time (same rationale as the OOP envelope's class/method-name lifts:
keeps the runtime's `switch` on a compile-time-known string, closed
dispatch) rather than passed through as runtime values.

Deliberately does NOT replicate `_sir_puts`'s trailing-newline-suppression
nuance (`puts "x\n"` prints `x\n`, not `x\n\n`) — a pre-existing
divergence from the C backend's own `puts`, orthogonal to and not fixed
by SIR28; `__sys_write__`'s `per_value` terminator always appends exactly
one newline per value, matching SIR28 §2.1's table and every other
backend's `__sys_write__` faithfully.

Purely additive: nothing emits `__sys_write__` yet, so `_sir_print`/
`_sir_puts` and every existing `print`/`puts`-sourced program are
unchanged.

New `tests/compile_and_run_sys_write.rs` (mirrors
`compile_and_run_case_eq.rs`'s hand-built-`Module` + real-`go run`
pattern): hand-builds a `Module` directly per
stream/terminator/unpack_arrays combination, emits Go, runs it with `go
run`, and asserts stdout/stderr.

## 0.37.2 — doc-comment reframing: SIR25 §2 is the dispatch authority, not "matches Ruby"

Documentation-only, no behavior change. Per `SIR25-language-agnostic-
object-model.md`'s §6 (which explicitly tracks this as its own follow-up),
this backend's OOP-dispatch-mechanism doc-comments — the `Feature::Modules`
manifest note, mixin/ancestry transitivity, and `extend`'s singleton-first
precedence — now cite SIR25 §2.2/§2.4 as the authority, keeping "matching
Ruby's ..." as a parenthetical (still true, still useful context: this
dispatch model happens to coincide with Ruby's today, it just isn't
*defined as* "whatever Ruby does"). Per-method Collections-catalog
doc-comments are deliberately left untouched — SIR25 §3 explicitly
designates that framing as legitimate naming provenance, not structural
coupling.

## 0.37.1 — `<<` (Ruby's shift operator) as a top-level builtin

Part of "Python/JS/Go/Rust/Ruby backends: implement `<<` runtime
dispatch". `ruby-to-semantic-ir` lowers `<<` to a top-level
`BuiltinCall("<<", [lhs, rhs, ...])` — a SEPARATE protocol from the
`__method__("<<", recv, arg)` Collections dispatch Array#push already
used on this backend (the pre-existing `_sir_array_responds`/`case
"<<":` entries in the method catalog). The operator form reached
`_sir_call_builtin_by_name`'s floor and panicked `unknown builtin: <<`
— every Ruby program using `<<` as an operator failed at runtime on Go.

New `_sir_shift_left(args []Value) Value`, polymorphic like `_sir_plus`:
Array pushes each RHS operand in place (chains left-to-right, since the
frontend lowers a `<<` chain to one variadic call); Integer bitwise-shifts
via `_sir_shift_left_i64`, PORTED from the C backend's helper of the same
name for identical overflow/negative-amount semantics (negative amount
reverses direction; left shift saturates at MaxInt64/MinInt64 rather than
wrapping, since this runtime has no bignum growth); String concatenates
to a new string via the existing `_sir_as_string` (matching this
backend's own `+` String-receiver convention, not C's looser
silently-drop-a-non-string one).

New supporting helpers `_sir_shift_amount_arg`, `_sir_f64_to_i64_saturating`
(a non-finite/out-of-range float64->int64 conversion is
implementation-specific in Go, same UB-avoidance discipline as C), and
`_sir_i64_abs_u` (MinInt64-safe magnitude via uint64 wraparound).

`semantic-ir-to-go` 0.37.0 -> 0.37.1.

## 0.37.0 — operator-spelling comparisons: `==`, `!=`, `<=`, `>=`

The Ruby frontend lowers a comparison chain to `==`/`!=`/`<=`/`>=` builtins,
which the Go backend did not lower — so even `puts(1 == 1)` panicked
`unknown builtin: ==`.

- Runtime gains `_sir_ne` (the exact negation of `_sir_eq`), `_sir_le` and
  `_sir_ge`. A new shared `_sir_cmp` orders two strings LEXICOGRAPHICALLY and
  numbers by float64 value (`1 <= 1.0` holds), and `<`/`>`/`<=`/`>=` all route
  through it.
- Emitter and the `_sir_call_builtin_by_name` dispatch gain `==`/`!=`/`<=`/`>=`.

This also **fixes a pre-existing panic**: `_sir_lt`/`_sir_gt` coerced their
operands through `_sir_as_float`, which panics on a string — so `"a" < "b"`
crashed the program instead of comparing. Both now order strings via
`_sir_cmp`, so Go agrees with the C, Rust, Ruby and Python backends on string
ordering (a deep-uncomparable operand — nil/pair vs number — still panics in
`_sir_as_float`, exactly as before; a total order there is a separate
refinement).

## 0.36.0 — `Exception#message`

`rescue => e; puts e.message` — everyday Ruby — raised `NoMethodError`: the
method simply did not exist. `_sir_object_method` gains a `message` arm
returning the text a `raise Foo, "msg"` carried (`SirError.Msg`), answered by
an exception receiver only so any other receiver still falls through to its own
catalog. `_sir_responds_to` reports it on exceptions and NOT on anything else,
reporting it for an exception receiver while still falling through to the user
method table — so a class that defines its own `message` is not DENIED by
`respond_to?` (the same dishonest-`respond_to?` shape this fixes elsewhere).

Also: a bare `raise Foo` carries no message, and `e.message` returned `nil`
where Ruby (and the Python/Rust/JS backends) return the CLASS NAME. It now
matches.

## 0.35.0 — implement `is_a?` / `kind_of?` / `instance_of?`

These were listed in `_sir_responds_to` but **never implemented**, so
`respond_to?(:is_a?)` answered `true` while an actual call fell through to
`NoMethodError` and killed the program. (`class` was already implemented; the
predicates were not.)

`_sir_object_method` gains the three arms, reusing the existing
`_sir_ruby_class_name`. `is_a?`/`kind_of?` honour ancestry — the built-in
surface (`Integer`/`Float` are `Numeric` and `Comparable`, `String` is
`Comparable`, `Object`/`BasicObject` match everything) plus, for a user
instance, its superclass chain (`_sir_is_ancestor_or_self`) and any module
mixed in along it. `instance_of?` is an exact class match.

Transitive module matching (Ruby MRO: `C` includes `M`, `M` includes `N` ⇒
`c.is_a?(N)`) uses an ITERATIVE worklist rather than recursion, because
include-graph depth is shaped by the source — the same design the JavaScript
and Rust backends use. Cyclic and self-including graphs terminate.

The class argument arrives as a NAME (ruby-to-semantic-ir 0.7.0 lifts a
`Const` to a `StrLit`), so no constant-reference support is needed.

Two adjacent fixes found while reviewing the above:
- `_sir_ruby_class_name` now recognises `*SirError`. A raised/caught exception
  is not a `*SirInstance`, so it fell to the `Object` default — `rescue => e;
  e.class` said `Object` and `e.is_a?(StandardError)` was FALSE for every
  exception, silently skipping a handler guarded that way, even though
  `_sir_ancestry` holds the whole exception hierarchy. `_sir_value_is_a` walks
  ancestry for `*SirError` too.
- The `==` / `!=` object arms indexed `args[0]` with no length check, so a
  zero-argument `x.==()` PANICKED and killed the program. Both are guarded.

## 0.34.0 — Ruby `Integer#/` floors toward −∞ (SIR21 §E3)

The inline `__sir` runtime's `_sir_divide` truncated integer division toward
zero (`acc /= d`), so `-7 / 2` gave `-3` instead of Ruby's floored `-4`. The
integer path now floors toward −∞ — the truncated quotient minus one exactly
when the remainder is non-zero and its sign differs from the divisor's —
matching the SIR21 §E3 oracle `DivOp::Floor` on every sign combination. The float
path (`_sir_any_float`) is unchanged and already true-divides (Ruby `Float#/`);
typed division-by-zero is unchanged.

### Fixed — unary minus (`neg`) was unimplemented (any negative literal crashed)

Closing the division frontier surfaced a second, unrelated gap: the Ruby
frontend lowers unary minus (`-x`) to `BuiltinCall("neg", [x])`, but
`_sir_call_builtin_by_name` had **no `neg` case**, so *every* negative literal
(`-7`, not just division) panicked at runtime with `unknown builtin: neg`. The
JavaScript and Python runtimes already implemented it; the Go backend now does
too (`_sir_neg`), tag-preservingly (a `float64` stays a `float64`, otherwise
negate as `int64`). This is what lets the division frontier's negative cases run
at all.

Together these close the **Go arm** of the division frontier
(`sir-conformance/tests/division.rs`).

## 0.33.0 — Array `cycle(n)`

Mirrors the Python reference (PR #8117) into the Go backend's inline `__sir`
runtime (`_sir_array_block_method` beside the existing `chunk_while`/`slice_when`
arms + the `_sir_array_responds` `respond_to?` arm), continuing the `cycle`
cross-backend cascade.

- `cycle(n) { |x| … }` (block) → iterate the array `n` full passes in order,
  yielding each element on every pass; always returns nil. `[1,2,3].cycle(2)`
  yields `1,2,3,1,2,3`. `n <= 0`, a negative count, an empty receiver, or a nil
  / non-integer count (Ruby's block-less Enumerator and infinite no-`n` forms)
  yields nothing rather than hanging — a boolean count is not an `int64`/`int`
  in Go, so it falls through to the no-yield path.
- The `array_methods_compile_and_run` suite gains `array_cycle_compile_and_run`:
  the block `puts`es each yielded element, so the two passes (`1,2,3,1,2,3`) and
  the `nil` returns for `cycle(2)`, `cycle(0)`, and `[].cycle(5)` are proven
  under a real `go run`.

## 0.32.0 — Array `minmax`

Mirrors the Python reference (PR #8092) into the Go backend's inline `__sir`
runtime (`_sir_array_method` beside the existing `min`/`max` arm + the
`_sir_array_responds` `respond_to?` arm), continuing the `minmax` cross-backend
cascade.

- `minmax` (non-block) → the two-element array `[min, max]` in one pass, via `<`
  (`_sir_value_lt`). `[3,1,2].minmax` → `[1, 3]`; `["b","a","c"].minmax` →
  `["a", "c"]`. An empty array yields `[nil, nil]` (no smallest/largest element),
  matching the Python reference's `[None, None]`.
- The `array_methods_compile_and_run` exec-proof test gains `minmax` (non-empty
  and empty) — the emitted Go compiles + runs with the toolchain and asserts
  `[1, 3]` / `[nil, nil]`.

## 0.31.0 — Array `slice_when`

Mirrors the Python reference (PR #8070) into the Go backend's inline `__sir`
runtime (`_sir_array_block_method` + the `_sir_array_responds` `respond_to?`
arm), continuing the `slice_when` cross-backend cascade.

- `slice_when { |prev, cur| pred }` is the INVERSE of `chunk_while`: it splits
  into runs of consecutive elements, starting a NEW run BETWEEN an adjacent pair
  exactly WHERE the block is truthy (whereas `chunk_while` starts a new run where
  the block is FALSY).
  `[1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 }` → `[[1,2],[4],[9,10,11,12]]`;
  an empty array yields `[]`, a single element `[[x]]`.
- `tests/compile_and_run_array_methods.rs::array_slice_when_compile_and_run`
  emits a program with a `b - a > 1` predicate, compiles + runs it with the Go
  toolchain, and asserts the printed runs.

## 0.30.0 — Array `each_slice` / `each_cons` / `chunk_while`

Mirrors the Python reference (PR #8031) into the Go backend's inline `__sir`
runtime, adding the Array consecutive-grouping family (`_sir_array_method` for the
non-block `each_slice`/`each_cons`, `_sir_array_block_method` for `chunk_while`,
plus the `_sir_array_responds` `respond_to?` arm).

- `each_slice(n)` → consecutive sub-arrays of at most `n` elements, the last
  possibly shorter (`[1,2,3,4,5].each_slice(2)` → `[[1,2],[3,4],[5]]`).
- `each_cons(n)` → every consecutive `n`-element sliding window
  (`[1,2,3,4].each_cons(2)` → `[[1,2],[2,3],[3,4]]`); a window larger than the
  array yields `[]`.
- Both treat `n <= 0` as `[]` (Ruby raises `ArgumentError`; the never-panic floor
  yields empty instead).
- `chunk_while { |prev, cur| pred }` → runs of consecutive elements; the block is
  called on each ADJACENT pair, a truthy result extends the run and a falsy one
  starts a new run (`[1,2,4,5,7].chunk_while { |a,b| b-a==1 }` →
  `[[1,2],[4,5],[7]]`).  Empty → `[]`; single element → `[[x]]`.

Exec-proof: `tests/compile_and_run_array_methods.rs` gains
`array_each_slice_each_cons_chunk_while_compile_and_run`, running each_slice/
each_cons (incl. `n<=0` and oversized-window → `[]`) and chunk_while (adjacent
`b-a==1` predicate; empty → `[]`) under real `go run`, diffed against the Python
reference semantics.

## 0.29.0 — Hash `to_h` (block + no-block) / `each_with_index` / `each_with_object`

Mirrors the Python reference (PR #8009) into the Go backend's inline `__sir`
runtime, rounding out Hash's Enumerable iteration surface (`_sir_hash_method` for
the no-block `to_h`, `_sir_hash_block_method` for the block forms, plus the
`_sir_hash_responds` `respond_to?` arm).

- `to_h` **without** a block → a shallow copy of the hash (a fresh `*Map`, so
  mutating it does not alias the receiver's entries).
- `to_h { |k, v| [new_k, new_v] }` → a NEW hash from the block-returned `[k, v]`
  pairs; the block is yielded the two args `(k, v)`; a non-pair result is skipped
  (never-raise floor — Ruby's TypeError is deferred to the typed-error cascade),
  and a later pair with a duplicate key wins (Ruby's rule, `_sir_map_set`).
- `each_with_index { |(k, v), i| … }` → yields each `[k, v]` pair with its
  0-based index, returns the receiver.
- `each_with_object(memo) { |(k, v), memo| … }` → yields each `[k, v]` pair with
  the memo, returns the (mutated) memo; no-memo arg returns the receiver.

Unlike `each`'s two-arg `(k, v)` yield, `each_with_index`/`each_with_object` pass
the element as a single `[k, v]` `*Seq` (the second block param is the
index/memo), matching Ruby's Enumerable convention.

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_to_h_and_indexed_iteration_compile_and_run`, running to_h (copy + re-map),
each_with_index (observed pair+index yield, returns self), and each_with_object
(observed pair+memo yield, returns memo, and no-memo passthrough) under real
`go run`, diffed against the Python reference semantics.

## 0.28.0 — Hash Enumerable breadth: `group_by` / `partition` / `flat_map` / `reduce` / `inject` / `sum`

Mirrors the Python `sir-runtime-oop` v0.1.20 reference (PR #7978) into the Go
backend's emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`).
The block is yielded `(key, value)` (two arguments) — except `reduce`/`inject`,
which follow Ruby's memo convention and yield `(memo, [key, value])` (the pair
as one second argument).  Every "element" a result carries is the two-element
`[key, value]` Array (`&Seq{key, value}`).

- `group_by { |k, v| … }` — a Hash of block key → Array of `[k, v]` pairs, in
  first-seen key order.
- `partition { |k, v| … }` — `[[matching pairs], [non-matching pairs]]`.
- `flat_map`/`collect_concat { |k, v| … }` — one-level splice of block results.
- `reduce`/`inject(init) { |memo, (k, v)| … }` — fold; a seedless `reduce`
  starts from the first pair, and an empty seedless `reduce` returns `nil`.
- `sum(init = 0) { |k, v| … }` — `init` plus the polymorphic-`+` (`_sir_plus`)
  sum of the block results.

`_sir_hash_responds` now advertises all of the above (the hash block dispatch
already forwards the positional args before the block, so `reduce`/`sum` read
their seed).

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_enumerable_breadth_compile_and_run`, running `group_by` (even-value
predicate ⇒ bool-keyed Hash of pairs), `partition`, `flat_map`, `reduce(0)`, and
`sum(100)` under real `go run`, diffed against the Python reference semantics.

## 0.27.0 — Hash Enumerable aggregates: `find` / `any?` / `all?` / `none?` / `count` / `sort_by` / `min_by` / `max_by`

Mirrors the Python `sir-runtime-oop` v0.1.19 reference (PR #7957) into the Go
backend's emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`).
Ruby's `Hash` mixes in `Enumerable`, so these iterate the hash as a sequence of
`[key, value]` pairs: the block is yielded `(key, value)` (two arguments,
matching `each`), and the "element" an aggregate returns is the two-element
`[key, value]` Array (`&Seq{key, value}`).

- `find`/`detect` — first `[k, v]` pair with a truthy block result; `nil` if none.
- `any?`/`all?`/`none?` — booleans over `block(k, v)`.
- `count { |k, v| … }` — number of pairs with a truthy block result.
- `sort_by` — a NEW Array of `[k, v]` pairs sorted by the block key (stable on
  ties, Schwartzian; the never-panic `_sir_value_lt` comparator).
- `min_by`/`max_by` — the extremal `[k, v]` pair (first-on-tie; `nil` on empty).

`_sir_hash_responds` now advertises all of the above.

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_enumerable_aggregates_compile_and_run`, running `sort_by`/`min_by`/
`max_by` (by value), `find`/`count`/`any?`/`all?`/`none?` (even-value
predicate) under real `go run`, diffed against the Python reference semantics.

## 0.26.0 — Hash transforming block methods: `transform_values` / `transform_keys`

Mirrors the Python `sir-runtime-oop` v0.1.18 reference into the Go backend's
emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`), adding two
non-mutating Ruby `Hash` block methods:

- `transform_values { |v| … }` — builds a **new** hash whose keys are copied
  verbatim (so no collision is possible) and whose values are the block results.
  Original insertion order is preserved via a straight append.
- `transform_keys { |k| … }` — builds a **new** hash whose values are untouched
  and whose keys are the block results.  Two source keys can map to the SAME new
  key; Ruby keeps the **last** colliding entry's value, so every write is routed
  through `_sir_map_put`, which overwrites an existing key in place.

Both yield exactly ONE block argument (the value / the key) and leave the
receiver unmodified.  `_sir_hash_responds` now also advertises the pre-existing
`each_key` / `each_value` block methods (previously reachable but not reported by
`respond_to?`).

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains a `transform_values`
case ({a:1,b:2} → {a: 99, b: 99}) and a `transform_keys` **collision** case
({a:1,b:2} with a constant `:z` key → {z: 2}), compiled and run under real
`go run` with stdout diffed against the Python/TS reference semantics.

## 0.25.0 — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Mirrors the Python `sir-runtime-oop` v0.1.17 reference into the Go backend's
emitted runtime (`_sir_numeric_method` + `_sir_numeric_responds`), adding five
Ruby numeric methods:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds a Float to that many decimals (half **away from zero**, via
  `_sir_ruby_round`); `ndigits <= 0` rounds to a power of ten.  Go's `int64`/
  `float64` are FIXED width, so the Python bignum→float `OverflowError` pitfall
  does not apply — the only guards are a place count past int64's ~18 decimal
  digits (dwarfs the value ⇒ `0`, Ruby parity) and a positive `ndigits` past
  Float precision / an overflowing scale-up (returns the value unchanged).
- `divmod(n)` — `[quotient, remainder]` with a floored quotient (`_sir_floor_div`)
  and the divisor-signed remainder; a zero divisor raises a typed
  `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that never panics: a zero divisor yields
  `±Inf`/`NaN` (Go float division already produces these).
- `clamp(min, max)` / `between?(min, max)` — compared numerically.

Dispatch stays an explicit `switch` on the interned method name (never
reflection).  Exec-proven end-to-end via `go run` (the numeric exec-proof test
now covers `round(2)`/`round(-2)`, `divmod` incl. the divisor-signed remainder,
`fdiv` incl. the divide-by-zero `Infinity`, and `clamp`/`between?`).

## 0.24.0 — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Adds four non-block Ruby String methods to the emitted runtime's
`_sir_string_method` switch and the `_sir_string_responds` catalog, mirroring
the Python `sir-runtime-oop` reference semantics (rune-based, so multibyte
strings are never split mid-codepoint):

- `tr(from, to)` — position-wise rune translation; a shorter `to` repeats its
  last rune, an empty `to` deletes matching runes, and a repeated rune in `from`
  keeps the last mapping.
- `count(*sets)` / `delete(*sets)` / `squeeze(*sets)` — char-set methods:
  `count` tallies runes of the receiver in the set, `delete` removes them, and
  `squeeze` collapses consecutive runs (of set runes, or of *all* runes when no
  set is given). Multiple set arguments intersect (Ruby's rule).

Each `set`/`from`/`to` argument is treated **literally** — the range (`"a-z"`)
and negation (`"^abc"`) forms are a follow-up, matching the literal-only
`sub`/`gsub` precedent. Exec-proven end-to-end via `go run`. Second backend of
the String char-set sweep (Python landed in `sir-runtime-oop` v0.1.16).

## 0.23.0 — slice-selection Array methods: `take` / `drop` / `values_at`

Extends the emitted Go runtime's non-block `Array` catalog (and the
`respond_to?` table):

- `take(n)` — a fresh Array of the first `n` elements; `n` is clamped to
  `[0, len]` (`n <= 0` → `[]`, `n > len` → a full copy). A negative `n` raises
  `ArgumentError` in Ruby; the never-raise floor treats it as `0`.
- `drop(n)` — a fresh Array with the first `n` elements removed (same clamp;
  `n >= len` → `[]`).
- `values_at(*idxs)` — a fresh Array of the element at each index, folding a
  negative index from the end; an out-of-range index yields `nil` (never
  panics).

Verified end-to-end under `go run`.

## 0.22.0 — more String methods: `ljust` / `rjust` / `center` / `swapcase`

Extends the emitted Go runtime's `_sir_string_method` catalog (and its
`respond_to?` table):

- `ljust(width, pad = " ")` / `rjust(...)` / `center(...)` — pad to `width`
  **runes** using `pad` cyclically; `width <= length` returns the string
  unchanged; `center` puts an odd extra pad rune on the RIGHT (Ruby's rule).
  An empty pad degrades to a single space rather than raising (never-raise
  floor). New helper `_sir_str_pad` builds the exact-length cyclic padding.
- `swapcase` — flips the case of each ASCII letter (rune-safe; non-letters and
  non-ASCII runes pass through).

Also **fills a pre-existing `respond_to?` under-report**: `capitalize`,
`chomp`, `bytes`, `index`, `replace`, `sub`, `gsub` already dispatch in
`_sir_string_method` but were unlisted; the table is now faithful.

Verified end-to-end under `go run`.

## 0.21.0 — more Array methods: `zip` / `rotate` / `to_h` / `tally`

Extends the emitted Go runtime's non-block `Array` catalog and the
`respond_to?` table:

- `zip(*others)` — Array of tuples `[a[i], b[i], …]`, length = the receiver's;
  a shorter operand pads with nil; a non-array operand is treated as empty.
- `rotate(n = 1)` — elements rotated left by `n` (a negative `n` rotates
  right); the modulo wraps so any `n` terminates without panicking.
- `to_h` — `[[k, v], …]` → a Hash (2-element-array elements only; others
  skipped, matching the never-raise floor).
- `tally` — a Hash of element → occurrence count, first-seen order, keyed by
  structural value-equality.

Verified end-to-end under `go run`.

## 0.20.0 — Array block-method breadth (sort_by / group_by / partition / …)

Mirrors the Rust backend's array block-method batch to Go: extends
`_sir_array_block_method` with the common block-taking Ruby
`Enumerable`/`Array` methods that were missing, and grows the `respond_to?`
table to match.

- `sort_by { |x| key }` — key-sorted (Schwartzian, stable).
- `min_by` / `max_by { |x| key }` — extremal block key (first-on-tie; `nil` on
  empty).
- `group_by { |x| key }` — a Hash of key → Array of elements.
- `partition { |x| pred }` — `[matching, non_matching]`.
- `flat_map` / `collect_concat { |x| … }` — map then splice one level.
- `take_while` / `drop_while { |x| pred }` — leading truthy run / remainder.
- `count { |x| pred }` — number of truthy results (bare/arg forms unchanged).
- `each_with_object(memo) { |x, memo| … }` — folds into and returns the memo.

Ordering reuses `_sir_value_lt` — the never-panic comparator, so a non-numeric
block key degrades to a stable order rather than raising (unlike a naive
numeric coerce). A block-less call floors to `NoMethodError` (Ruby returns an
Enumerator — a v0 cut-line). Verified end-to-end under `go run`.

## 0.19.0 — source-language display convention: Ruby booleans (`true`/`false`)

Mirrors the Rust backend's first increment of the SIR display-convention spec
(`code/specs/sir-display-convention.md`) to Go. A **Ruby**-sourced module now
renders booleans as `true`/`false` instead of the Twig/Lisp `#t`/`#f`, so a
translated `puts true` prints `true`.

Mechanism: the runtime carries a compile-time `const _sir_display_ruby` (a
`__SIR_DISPLAY_RUBY__` placeholder); the emitter substitutes `true`/`false`
from `Module.metadata.source_language` (`== "ruby"` → `true`, else `false`).
`_sir_format` branches the boolean arm on it. The default is the Lisp form, so
all existing non-Ruby (Twig) output is **byte-for-byte unchanged**. The Go
compiler folds the `const` branch — zero per-call cost.

Scope: booleans only (the flagship divergence); `nil`, symbols, string
`inspect` quoting, and the Ruby hash `=>` element form remain follow-ups per
the spec's rollout. Verified end-to-end under `go run`: Ruby source →
`true\nfalse\n`; Twig source → `#t\n#f\n`.

## 0.18.0 — Numeric + String method-catalog parity

Expands the emitted Go runtime's `_sir_numeric_method` and
`_sir_string_method` catalogs to Ruby parity, and grows the matching
`_sir_numeric_responds` / `_sir_string_responds` predicates so
`respond_to?` stays honest.

**Numeric (`int64` / `float64`):** `to_int`, `positive?`, `negative?`,
`succ` / `next`, `pred`, `floor`, `ceil`, `round` (`_sir_ruby_round`,
round-half-up), `gcd` (`_sir_gcd`, overflow-safe), `pow` / `**`
(`_sir_int_pow`, with a closed-form short-circuit for base ∈ {0, 1, −1}
and a bit-width guard so a large exponent can't spin), `digits`
(`_sir_digits`), and the block-taking walkers `upto` / `downto` / `step`
(counter arithmetic guarded against `int64` boundary overflow).

**String:** `capitalize`, `chomp`, `bytes`, `index`, `replace`, `sub`,
`gsub` (literal, first/all-occurrence; no regex or back-reference
expansion). All arity-guard their optional arguments (`len(args)` checks
before any `args[0]`), returning `nil`/receiver rather than panicking.

Dispatch stays receiver-type routed through explicit `switch` labels — no
reflection on source-derived method names.

(Consolidates the previously-separate Numeric and String catalog PRs into
one crate change to avoid intra-crate version churn.)

## 0.16.0

### Added â€” Ruby Symbol method catalog completion (`capitalize` / `inspect` / `to_proc`)

Parity-fill: the Python + TypeScript `sir-runtime-oop` Symbol catalogs already
expose `inspect`; this ports it into the Go runtime's `_sir_symbol_method`
switch and adds the two task-mandated Ruby Symbol methods `capitalize` and
`to_proc`, so a translated Ruby program's Symbol calls execute on the Go
backend instead of hitting the `NoMethodError` floor.

- **`inspect`** â€” returns the source form `":name"` (a String). Matches the
  Python/TS reference semantics.
- **`capitalize`** â€” returns a NEW interned `*Symbol` whose name has an
  uppercase first char and a lowercase remainder (rune-aware, mirroring the
  existing `upcase`/`downcase` arms).
- **`to_proc`** â€” an explicit `sym.to_proc` call returns a `*Closure` built by
  the SAME `_sir_sym_to_proc` helper the `&:sym` block-pass form uses. The
  resulting proc routes each application through the explicit
  `_sir_call_method` switch â€” NEVER Go `reflect` ([[dynamic-dispatch-rce]]); an
  out-of-catalog method surfaces the ordinary `NoMethodError`. Note: the
  `&:sym` block-pass form is FRONTEND-lowered straight to `_sir_sym_to_proc`
  (see `try_emit_block_pass` in `emit.rs`) and never reaches this catalog arm;
  `to_proc` is added for the explicit-call path and full correctness.
- `_sir_symbol_responds` (`respond_to?`) updated to include `capitalize`,
  `inspect`, and `to_proc`.

Exec-proof: `tests/compile_and_run_symbol_methods.rs` runs the emitted Go under
a real `go run` toolchain and asserts `:hello.to_s`â†’"hello", `:hi.length`â†’"2",
`:abc.upcase`â†’"ABC", `:ABC.downcase`â†’"abc", `:hELLO.capitalize`â†’"Hello",
`:x.inspect`â†’":x", `[1,2,3].map(&:to_s).join`â†’"123" (block-pass form), and
`[4,5,6].map(:to_s.to_proc).join`â†’"456" (explicit catalog `to_proc`).

## 0.16.0

### Added â€” Ruby Symbol method catalog completion (`capitalize` / `inspect` / `to_proc`)

Parity-fill: the Python + TypeScript `sir-runtime-oop` Symbol catalogs already
expose `inspect`; this ports it into the Go runtime's `_sir_symbol_method`
switch and adds the two task-mandated Ruby Symbol methods `capitalize` and
`to_proc`, so a translated Ruby program's Symbol calls execute on the Go
backend instead of hitting the `NoMethodError` floor.

- **`inspect`** â€” returns the source form `":name"` (a String). Matches the
  Python/TS reference semantics.
- **`capitalize`** â€” returns a NEW interned `*Symbol` whose name has an
  uppercase first char and a lowercase remainder (rune-aware, mirroring the
  existing `upcase`/`downcase` arms).
- **`to_proc`** â€” an explicit `sym.to_proc` call returns a `*Closure` built by
  the SAME `_sir_sym_to_proc` helper the `&:sym` block-pass form uses. The
  resulting proc routes each application through the explicit
  `_sir_call_method` switch â€” NEVER Go `reflect` ([[dynamic-dispatch-rce]]); an
  out-of-catalog method surfaces the ordinary `NoMethodError`. Note: the
  `&:sym` block-pass form is FRONTEND-lowered straight to `_sir_sym_to_proc`
  (see `try_emit_block_pass` in `emit.rs`) and never reaches this catalog arm;
  `to_proc` is added for the explicit-call path and full correctness.
- `_sir_symbol_responds` (`respond_to?`) updated to include `capitalize`,
  `inspect`, and `to_proc`.

Exec-proof: `tests/compile_and_run_symbol_methods.rs` runs the emitted Go under
a real `go run` toolchain and asserts `:hello.to_s`â†’"hello", `:hi.length`â†’"2",
`:abc.upcase`â†’"ABC", `:ABC.downcase`â†’"abc", `:hELLO.capitalize`â†’"Hello",
`:x.inspect`â†’":x", `[1,2,3].map(&:to_s).join`â†’"123" (block-pass form), and
`[4,5,6].map(:to_s.to_proc).join`â†’"456" (explicit catalog `to_proc`).

## 0.15.0

### Added â€” Array collection-method parity (min / max / sum / uniq / flatten / compact / each_with_index)

Parity-fill: these Ruby `Array` methods already shipped in the Python + TypeScript
`sir-runtime-oop` backends; this ports the SAME surface into the Go runtime's
array dispatch, so a translated Ruby program now executes them on the Go backend
instead of hitting the `NoMethodError` floor. `to_a` was already present and is
unchanged. Semantics match the Python/TS reference impls exactly.

- **`min` / `max`** (non-block, v0) â€” element-wise extremum via `_sir_value_lt`
  (Ruby's `<`/`>`); empty array â‡’ nil. Dispatched in `_sir_array_method`.
- **`sum`** â€” folds with the polymorphic `_sir_plus` over an initial value
  (default `0`, or the supplied `sum(init)` argument), preserving int/float;
  empty array â‡’ the initial value. Dispatched in `_sir_array_method`.
- **`uniq`** â€” order-preserving de-duplication via structural value-equality
  (`_sir_value_eq`); returns a fresh `*Seq`. Dispatched in `_sir_array_method`.
- **`flatten`** â€” recursively flattens nested `*Seq` into a fresh flat `*Seq`.
  **Cycle-guarded** (CWE-674, uncontrolled recursion): the new
  `_sir_flatten_into` helper threads a `visited` set of `*Seq` handle pointers
  on the active recursion path â€” mirroring `_sir_puts_one` â€” so a self-referential
  array (`a = []; a << a`) terminates instead of overflowing the Go stack.
  Sibling (non-cyclic) occurrences still flatten in full.
- **`compact`** â€” fresh `*Seq` with nil elements removed. Dispatched in
  `_sir_array_method`.
- **`each_with_index`** â€” block-taking; yields `(element, index)` pairs and
  returns the receiver. Dispatched in `_sir_array_block_method`.
- `_sir_array_responds` now advertises all of the above for `respond_to?` parity.

Execution proof: `tests/compile_and_run_array_methods.rs`
(`array_methods_compile_and_run`) hand-builds SIR exercising each method, emits
Go, runs it under `go run`, and diffs stdout against the Python/TS reference
values (`[3,1,2].max` â†’ 3, `[1,2,2,3,1].uniq` â†’ `[1,2,3]`, `[[1,[2]],3].flatten`
â†’ `[1,2,3]`, `[1,nil,2,nil].compact` â†’ `[1,2]`, `[1,2,3].sum` â†’ 6,
`[10,20].each_with_index` â†’ `0:10`/`1:20` then the returned receiver).

## 0.14.0

### Security (review-driven)

- Arity guards on `equal?` and boolean `&`/`|`/`^`: these became reachable with
  ZERO args via the new `send` surface (`obj.send(:equal?)`, `true.send(:&)`),
  where indexing `args[0]` was a raw Go index-out-of-range panic (catchable only
  as `StandardError`, or a native crash if uncaught). They now raise a typed
  `ArgumentError` ("wrong number of arguments (given 0, expected 1)") â€” matching
  Ruby. Regression: `send_zero_arg_method_raises_argument_error_not_native_panic`.

### Added â€” M6 universal Object metaprogramming (send / tap / then / respond_to? / boolean &|^)

Parity-fill: M6 shipped in the Python + TypeScript `sir-runtime-oop` backends;
this ports the SAME surface into the Go runtime's method-dispatch path
(`_sir_call_method`), so a translated Ruby program's `send`/`tap`/`then`/
`respond_to?` and boolean `&`/`|`/`^` now execute on the Go backend instead of
hitting the `NoMethodError` floor.

- **`send`/`__send__`/`public_send`** â€” the first argument names a method; the
  dispatcher re-enters `_sir_call_method` with that name and the remaining args
  (a trailing block survives as a trailing arg). **Security ([[dynamic-dispatch-rce]]):**
  the dynamic name is coerced to a string and used ONLY as the key into the
  SAME explicit catalog/switch a normal call walks â€” an unknown name surfaces
  the ordinary `NoMethodError`. NEVER Go `reflect`/`MethodByName` on the
  source-derived name.
- **`tap`** â€” yields the receiver to the block and returns the RECEIVER; a
  block-less `tap` returns the receiver (v0 Enumerator-less floor).
- **`then`/`yield_self`** â€” yields the receiver and returns the BLOCK RESULT;
  block-less returns the receiver.
- **`respond_to?`** â€” true iff dispatch resolves the name, consulting the same
  reflective / `define_method` / type-specific + universal catalog tiers a real
  call uses (`_sir_responds_to` + per-catalog `_sir_*_responds` predicates kept
  in lockstep with the dispatch switches). Out-of-catalog â†’ honest `false`.
- **Boolean `&`/`|`/`^`** on `true`/`false` â€” Ruby's EAGER (non-short-circuit)
  logical operators, coercing the argument by SIR truthiness (`true & nil` is
  `false`, `false | 0` is `true`, `^` is XOR).
- Also filled the universal `Object` table: `inspect`, `equal?` (identity â€”
  value-equal for interned primitives, pointer-equal for `*Seq`/`*Map`/
  `*SirInstance`), `freeze`/`frozen?`, `dup`/`clone` (shallow copy of the
  mutable handles), and `nil.to_a == []` / `Array#to_a == self`.
- Exec-proof via `go run` (`tests/compile_and_run_m6_meta.rs`):
  `"hello".send(:upcase)` â†’ `HELLO`, `[1,2,3].send(:map,&blk)` â†’ `[2,4,6]`,
  `5.tap{â€¦}` â†’ `5`, `5.then{|x|x*2}` â†’ `10`, `respond_to?` true/false honesty,
  the boolean operators, and an unknown `send(:bogus)` failing cleanly through
  the NoMethodError floor (no reflection).


## 0.13.2

### Fixed â€” `or`/`and` builtins (Ruby `||`/`&&`) were unimplemented

Ruby `&&`/`and` and `||`/`or` lower (in the frontend) to
`BuiltinCall("and"/"or", [lhs, rhs])` â€” the fold covers BOTH the 2-operand
`a || b` form and a multi-value `when 1, 2, 3` chain. Only the Python backend's
emitter handled them; this backend fell through to the eager runtime dispatcher,
which has no `or`/`and` entry, so ANY `||`/`&&` (and every multi-value `when`)
crashed at runtime with `unknown builtin: or` / `and`. A case_eq-style gap: no
compile-time gate catches a frontend-emitted builtin the backend never handled.

- The emitter now special-cases `BuiltinCall("or"/"and", [a, b])`, emitting the
  SAME truthy-guarded short-circuit form as `Expr::LogicalOr`/`LogicalAnd`: rhs
  is not evaluated once lhs decides, SIR truthiness is used, and the deciding
  OPERAND is returned (Ruby semantics â€” `nil || "b"` is `"b"`, `"a" || "b"` is
  `"a"`), never a bare bool.
- Emit-shape regression test; verified end-to-end via the sir-conformance
  `logical_ops` + `multi_when` programs (13 corpus x 4 backends, all agree).


## 0.13.1

### Fixed â€” `case_eq` builtin (Ruby case-equality `===`) was unimplemented

Ruby's `case`/`when` (and `case`/`in`) lowers, in the frontend, to a chain of
`if`s whose conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`. This
backend's runtime never implemented `case_eq`, so **every** `case` program hit
`_sir_call_builtin_by_name`'s `unknown builtin` floor and **panicked at
runtime** â€” `case` was unusable on the Go backend (no compile-time gate catches
a missing builtin; only execution does).

- Added `_sir_case_eq(args) Value` to the inlined runtime and wired it into both
  the emitter's helper table (direct-call path) and `_sir_call_builtin_by_name`
  (reified-closure path). Ruby keys `===` to the *pattern*'s type (Range â†’
  membership, Regexp â†’ match, else `==`); the `when SomeClass` case is lowered to
  `value.is_a?(SomeClass)` at the frontend and never reaches here. This backend's
  Value model has no Range/Regexp variant yet, so `case_eq` is exactly structural
  equality (`_sir_value_eq`), matching the Python reference in `sir-runtime-oop`;
  extend with membership/match arms when those value types land.
- New `compile_and_run_case_eq` exec proof: a `when`-style `if case_eq(â€¦)` chain
  emits Go, runs under `go run`, and matches the expected dispatch output.


## 0.13.0

### Added â€” Ruby mixins: `module` + `include` / `extend` MRO (sir-mixins MX5)

- The Go backend's emitted OOP runtime now EXECUTES Ruby mixins. A method
  defined in a `module` and mixed into a class via `include` is found through
  the class's Method Resolution Order; `extend` exposes a module's methods as
  class methods. Runtime-only change; no core-IR or frontend edit. Dispatch
  stays explicit NAME-keyed map lookup â€” NEVER reflection (the
  [[dynamic-dispatch-rce]] discipline).
- **`Feature::Modules` is now ACCEPTED.** A `Stmt::ModuleDef` (`module M; â€¦;
  end`) is hosted as a method *owner* alongside classes: its body's `def`s
  register via the SAME `__def_method__("M", â€¦)` builtin classes use (keyed by
  the module name), and its body is emitted in order like a `ClassDef` body.
  Previously `ModuleDef` was rejected at the soundness gate; the gate now
  recurses into a module body for the residual `Const` checks instead.
- **`__include__("Owner", "M")` â†’ `_sir_include`** â€” appends `M` to a per-owner
  included-module list (`_sir_included_modules map[string][]string`) in include
  order. Ruby searches the most-recently-included module first, so the
  resolution walk iterates this slice in REVERSE.
- **MRO-extended method resolution** (`_sir_resolve_instance_method`): the walk
  now follows class â†’ its included modules (reverse, recursing so a module that
  itself includes another is honoured) â†’ superclass â†’ its modules â†’ â€¦ â†’ Object.
  A class's own method SHADOWS an included module's; a module method shadows the
  superclass's. A module reached via two paths (a diamond) resolves ONCE, at its
  earliest position, because the `seen` set skips an already-visited owner. The
  walk is cycle-guarded (a self-including module or cyclic class hierarchy
  TERMINATES).
- **`__extend__("Owner", "M")` â†’ `_sir_extend`** â€” copies `M`'s instance
  methods (including those `M` itself includes) into `Owner`'s class-method
  table, so they become callable as `Owner.method`. An entry `Owner` already
  defines is not overwritten (own/class method shadows the extended module's).
- **`__class_method__("C", "m", argsâ€¦)` â†’ `_sir_call_class_method`** â€” a new
  emit arm + runtime helper wiring class-method *calls* (`Foo.bar`) through an
  ancestry-walking lookup in the class-method table (which `extend` populates).
  An unresolved name hits the controlled `NoMethodError` floor.
- Emit arms added for `__include__`, `__extend__`, and `__class_method__`; all
  owner/module/method NAMES ride in as `StrLit`s emitted through
  `quote_go_string` (never interpolated), keeping the runtime side reflection-free.
- Tests: five `go run` execution proofs (`compile_and_run_mixins.rs`) â€” an
  included-module method callable on an instance, a class method shadowing the
  module's, a module method shadowing the superclass's with a diamond include
  resolving once, `extend` making a module method a class method, and a mixed-in
  method reading an including class's `@ivar` through the shared self-stack â€”
  plus emit + runtime unit tests for the new arms and helpers.

## 0.12.0

### Added â€” typed runtime errors: ZeroDivision / Index / Key / NoMethod (sir-typed-runtime-errors T4)

- A faulting emitted runtime operation now raises the CORRECT **typed**
  `SirError` (via the existing `_sir_new_error` + `panic` entry point â€” the same
  one an explicit `raise` uses), so a translated `rescue
  ZeroDivisionError`/`IndexError`/`KeyError`/`NoMethodError` catches it exactly
  as Ruby would, and uniformly with the other backends. Runtime-only change; no
  core-IR or frontend edit. Dispatch stays explicit-string (no reflection â€” the
  [[dynamic-dispatch-rce]] discipline).
- **Division by zero** (`_sir_divide`): both the integer path and the
  float-promoted path now reject a zero divisor with
  `ZeroDivisionError` ("divided by 0"). Previously the int path did a raw
  `panic("division by zero")` (caught only as an over-broad generic
  `StandardError`) and the float path returned IEEE-754 `+Inf` (no error at
  all). This matches the spec's load-bearing rule that `1/0` **and** `1.0/0`
  raise `ZeroDivisionError`.
- **`Array#fetch`** (new entry in `_sir_array_method`): an out-of-bounds index
  raises `IndexError`; a supplied default (`fetch(i, d)`) is returned instead of
  raising; negative indices count from the end. The plain index operator
  `arr[i]` is UNCHANGED â€” `.fetch` is the raising read, `[]` is not.
- **`Hash#fetch`** (new entry in `_sir_hash_method`): a missing key raises
  `KeyError` ("key not found: â€¦"); a supplied default (`fetch(k, d)`) is
  returned instead. Because `KeyError < IndexError` in the ancestry table, a
  `rescue IndexError` also catches it. The plain `hash[k]` (`MapGet`) still
  returns `nil` â€” UNCHANGED (no over-raise).
- **Unknown method** (`_sir_method_unknown`): now raises a typed `NoMethodError`
  with a Ruby-shaped message `undefined method 'x' for <class>`, replacing the
  previous raw `panic(string)` (which was caught only as generic
  `StandardError`). The dispatch catalog remains the allowlist â€” an unknown
  name still surfaces a controlled, typed failure, never arbitrary behaviour.
- `*SirError` now implements Go's `error` interface (`Error() string`), so an
  UNCAUGHT typed panic prints a readable `panic: <Class>: <message>` banner
  instead of Go's default `(*main.SirError) 0xâ€¦` pointer dump. Cosmetic for the
  uncaught path only; `recover`/rescue matching still keys off the `Class` tag.
- Execution proof `compile_and_run_typed_errors.rs` (8 tests) runs each case
  through `go run`: `1/0` caught as `ZeroDivisionError` (and as `StandardError`
  via ancestry); `arr.fetch(oob)` â†’ `IndexError`; `h.fetch(miss)` â†’ `KeyError`
  (and caught as `IndexError` via ancestry); `obj.undefined` â†’ `NoMethodError`;
  regression that `h[miss]` (`MapGet`) still yields `nil`; and that
  `.fetch(k, default)` / an in-bounds `.fetch` do NOT over-raise.

## 0.11.0

### Added â€” polymorphic `+` / `*` for strings and arrays (sir-polymorphic-operators PO4)

- Ruby overloads `+` and `*` by receiver type, and every case lowers to the
  same SIR builtins (`_sir_plus` / `_sir_times`). The Go runtime helpers were
  previously **numeric-only** â€” they ran `_sir_as_int`/`_sir_as_float` on every
  operand â€” so `"a" + "b"` and `[1] + [2]` produced garbage or panicked. Both
  helpers now dispatch on the FIRST operand's runtime tag via a Go **type
  switch** (never reflection â€” the [[dynamic-dispatch-rce]] discipline) and add
  the string/array arms ahead of the unchanged numeric fold:
  - `_sir_plus`: first operand a `string` â†’ concatenate all operands as strings
    (`"a"+"b"` â†’ `"ab"`); first operand a `*Seq` â†’ concatenate element slices
    into a **fresh** backing array with no aliasing of any input (`[1]+[2]` â†’
    `[1, 2]`); otherwise the existing int/float-promoting numeric fold.
  - `_sir_times`: `string Ã— Integer` â†’ repeat via `strings.Repeat` (`"ab"*3` â†’
    `"ababab"`; a non-positive count yields `""`, clamped so `strings.Repeat`
    never panics); `*Seq Ã— Integer` â†’ repeat the element list into a fresh slice
    (`[0]*3` â†’ `[0, 0, 0]`; non-positive â†’ empty array); `*Seq Ã— string` â†’ join
    the elements with the separator using the same value-display helper `puts`
    uses (`_sir_format`), so `[1,2]*", "` â†’ `"1, 2"`; otherwise the numeric fold.
- Numeric `+`/`*` semantics (int64 fast path, intâ†’float promotion, variadic
  fold) are **preserved exactly** â€” the new arms only run when the first operand
  is a string/`*Seq`. Ruby `+`/`*` are binary; the string/array arms fold
  left-associatively over the variadic operand list.
- A controlled-panic helper `_sir_as_string` coerces string-`+` operands (a
  non-string operand â€” e.g. `"a" + 1` â€” panics with a Ruby-shaped "no implicit
  conversion of Integer into String" message rather than emitting garbage; the
  strict `TypeError` is deferred to the typed-runtime-errors cascade).
- Execution proof `compile_and_run_polyops.rs` runs `"a"+"b"`, `"ab"*3`,
  `[1]+[2]`, `[0]*3`, `[1,2]*", "`, and the numeric regressions `1+2` / `2*3`
  under `go run` and asserts stdout is exactly `ab\nababab\n[1, 2]\n[0, 0, 0]\n1, 2\n3\n6\n`.
- **Overflow guard (security):** the `*` repeat arms compute `len Ã— count` in a
  fixed-width host `int`, which on a large count could overflow (wrapping to a
  negative/absurd `make` capacity â†’ opaque panic) or drive a multi-gigabyte
  allocation â†’ OOM. Both arms now short-circuit an empty receiver (also avoiding
  a huge append loop) and guard `count > maxInt/len` with a controlled
  `panic("argument too big")` â€” matching Ruby's `ArgumentError: argument too
  big` â€” before any `strings.Repeat`/`make`. The count is program-controlled, so
  this closes a reachable resource-exhaustion vector.

## 0.10.0

### Added â€” `puts` builtin (Ruby semantics)

- The Go backend now emits and executes Ruby's `puts`, the most common output
  method. `puts` maps to a new variadic runtime helper `_sir_puts([]Value{â€¦})`
  (routed both by the emit helper table and the `_sir_call_builtin_by_name`
  dispatch), reusing `_sir_format` for element rendering.
- Ruby semantics implemented exactly: no-arg â†’ one newline; `puts x` â†’
  `x.to_s` + newline (no double newline when the text already ends in `"\n"`);
  `puts a, b` â†’ one line per arg; `puts []` â†’ a single newline; a `*Seq` is
  flattened recursively, one **element** per line; `puts nil` â†’ a blank line.
- Execution proof `compile_and_run_puts.rs` runs `puts "hello"; puts;
  puts [1,2,3]` under `go run` and asserts stdout is exactly
  `hello\n\n1\n2\n3\n` (the Ruby reference output).

### Security â€” cycle-guard the `puts` array flatten (CWE-674)

- `_sir_puts_one` flattened arrays by recursing per element with **no bound**.
  A `*Seq` is a shared, mutable handle, so a translated program can build a
  self-referential array (`a = []; a << a; puts a`) or a pathologically deep
  one; the unguarded recursion overflowed the Go stack and aborted the process
  â€” a denial of service (uncontrolled recursion). The flatten now threads a
  `visited` set of the `*Seq` pointers on the active path (the same identity
  key `_sir_format` uses): a handle re-encountered within its own subtree is a
  cycle and renders as Ruby's `[...]` placeholder + newline instead of
  recursing, so `puts a` on a self-referential array now **terminates** exactly
  as real Ruby does. Non-cyclic output is byte-for-byte unchanged
  (`puts [1,[2,3]]` â†’ `1\n2\n3\n`); a new regression test
  (`puts_cyclic_array_terminates`) proves the self-referential case exits
  cleanly with `[...]\n`.

## 0.9.0

### Added

- **User-defined class OOP â€” method dispatch, `new`, `self`, `super` (O4).**
  The Go backend now EXECUTES real user-defined classes (the Go analogue of the
  Python/TS `sir-runtime-oop` O1 path), not just exception subclasses.  The
  methodâ†”class association â€” which the Ruby frontend loses when it HOISTS every
  `def` to a detached top-level function â€” is recovered at RUNTIME via explicit
  `(class, method)` map tables.
  - **Inlined Go runtime** (`runtime.rs`, verbatim in every artifact):
    - `SirInstance { Class string; Ivars map[string]Value }` + `_sir_new_instance`.
    - Instance/class method tables `map[string]Value` keyed by a NUL-joined
      `class + "\x00" + method` string (a NUL cannot appear in an identifier, so
      the flattened key is unambiguous) â€” `_sir_def_method` /
      `_sir_def_class_method`.
    - `_sir_call_new(cls, argsâ€¦)` â€” allocate â†’ push self â†’ resolve an inherited
      `initialize` (walking the SHARED `_sir_ancestry` table, seen-guarded) â†’
      apply â†’ pop self via `defer` â†’ return the instance.
    - `_sir_call_method` extended: a `*SirInstance` receiver resolves the user
      method table walking ancestry (push self, apply, pop via `defer`); a miss
      falls through to universal Object methods, else the NoMethodError floor.
      NON-instance receivers reach the existing collection/built-in catalog
      **UNCHANGED**.
    - `_sir_call_super(method, cls, argsâ€¦)` â€” walk from the superclass, apply
      with the CURRENT self still bound (no push/pop â€” `super` re-dispatches on
      the same receiver).
    - `_sir_current_self()` (`__self__`), `_sir_ivar_get`/`_sir_ivar_set` on the
      current self (self-stack top, with a default-self so top-level `@x` never
      panics), and `_sir_cvar_get`/`_sir_cvar_set` for class variables.
  - **Emit arms** (`emit::emit_builtin_call`, mirroring `__method__`):
    `__new__`â†’`_sir_call_new`, `__super__`â†’`_sir_call_super`,
    `__def_method__`/`__def_class_method__`â†’ the table registrations,
    `__self__`â†’`_sir_current_self`.  Class/method names ride in as `StrLit`s and
    are emitted through `quote_go_string` â€” never interpolated.
  - **`@ivar` / `@@cvar`** (`emit::emit_var_ref` + `emit_stmt`):
    `VarRef`/`Assign{scope:Instance}` â†’ `_sir_ivar_get`/`set("@x", â€¦)`;
    `scope:ClassVar` â†’ the `_sir_cvar_*` helpers.
  - **Feature acceptance** (`lib.rs`): `ACCEPTED_FEATURES` now includes
    `InstanceVars` + `ClassVars` (alongside the existing `Classes`/`Constants`),
    so a REAL OO module is accepted and routed through the runtime.  The existing
    soundness gate still cleanly REJECTS genuinely-unsupported constructs â€” a
    general `Const` used as a value, a `Const` assignment, a `ModuleDef`
    (`Feature::Modules` stays unaccepted â€” no mixin/MRO runtime in v0).
  - **SECURITY (the C3 RCE lesson).**  Dispatch is ONLY an explicit map lookup on
    the `(class, method)` key â€” NEVER Go `reflect`/`MethodByName` on a
    source-derived name.  A class/method named `constructor`/`__proto__` is just
    a map key (a miss â†’ the clean NoMethodError floor).  Every ancestry walk
    carries a `seen` set so a cyclic hierarchy TERMINATES; self-stack pops go
    through `defer` so a panic still unwinds correctly.
  - **Tests.**  Emitted-shape unit tests for the five builtins + `@ivar`/`@@cvar`
    refs, plus `tests/compile_and_run_oop.rs` execution proofs through `go run`:
    P1 (`Dog.new("Rex").speak` â†’ `Rex`), P2 (inheritance + `super`, parent-set
    ivar visible â†’ `4`), a security case (class/method named `constructor`
    dispatches the user method; unknown `__proto__` hits the NoMethodError
    floor), and a cyclic-ancestry-terminates case.

## 0.8.0

### Added

- **Exception handling via panic/recover + ancestry (E3).**  The Go backend now
  EXECUTES `begin/rescue/ensure` and `raise` end to end.  Go has NO native
  try/catch, so exceptions are modelled with `panic` + a deferred `recover`:
  - **`Stmt::TryCatch` â†’ an immediately-invoked func** (`emit::emit_try_catch`).
    The func registers up to two deferred closures and then runs the try body:
    ```go
    func() {
      defer func() { <ensure> }()            // only if ensure present
      defer func() {
        if r := recover(); r != nil {
          if _sir_rescue_matches(r, []string{"Foo","Bar"}) { e := _sir_exc_value(r); <body> } else
          if _sir_rescue_matches(r, []string{"Baz"}) { <body> } else { panic(r) }
        }
      }()
      <try body>
    }()
    ```
    Rescue clauses are tried in **source order**; the first whose class list
    matches (per the ancestry table) runs, and if **none** match the recovered
    value is re-`panic`ked so it propagates (Ruby's "propagate when unrescued").
    An empty `exception_types` is a bare `rescue` (catch-all); `=> e` binds the
    caught value via `_sir_exc_value(r)`.
  - **ENSURE ORDERING (LIFO).**  Deferred funcs run last-in-first-out, and Ruby's
    `ensure` must run whether or not a rescue matched â€” i.e. it must run LAST â€” so
    its `defer` is registered **first** (deferred earliest â‡’ runs last).  The
    recover/dispatch `defer` is registered second (runs first): it recovers,
    dispatches, and re-`panic`s unmatched exceptions â€” a re-panic still unwinds
    through the already-registered ensure defer, so `ensure` runs on the
    propagating path too.
  - **`raise` â†’ `panic`** (`emit::emit_builtin_call`).  `raise Foo, "m"` â†’
    `panic(_sir_new_error("Foo", <msg>))` (the `Const` class name is intercepted
    and passed as a string â€” it never reaches `emit_var_ref`); `raise "boom"`
    (non-const first arg) â†’ an implicit `RuntimeError`; bare `raise` â†’ a generic
    `RuntimeError` (SIR v0 does not thread the in-flight exception into a bare
    re-raise â€” Go's `recover()` only works in a deferred func, matching the
    TS/Python backends' documented limitation).
  - **Runtime helpers** (`runtime.rs`, inlined verbatim): a `SirError` struct
    `{ Class string; Msg Value }`; `_sir_new_error(class, msg)`;
    `_sir_exc_value(r)` (the `Value` a `rescue => e` binds â€” a `*SirError`
    verbatim, or a synthesised `StandardError` wrapping a native Go panic);
    `_sir_rescue_matches(r, classNames)` (the ordered, ancestry-aware type test);
    and `_sir_register_ancestry(edges)` for user-defined class edges.  A
    `_sir_format` arm makes a caught exception print as its message (Ruby's
    `exception.message`).
  - **Built-in Ruby ancestry table** (`_sir_ancestry`), **ported from the
    TS/Python `sir-runtime-exceptions` reference for parity**: `StandardError â†’
    Exception`, `ArgumentError`/`TypeError`/`RuntimeError`/`RangeError`/
    `ZeroDivisionError`/`IOError`/`StopIteration`/`NotImplementedError`/
    `NameError`/`IndexError â†’ StandardError`, `NoMethodError â†’ NameError`,
    `KeyError â†’ IndexError`.  User `class MyErr < StandardError` declarations
    contribute one edge each, collected from every `ClassDef{superclass:Some}`
    and registered **once at program init** (`emit::emit_ancestry_init`).
  - **SECURITY â€” no reflection, cycle-guarded.**  Rescue matching is an EXPLICIT
    string-map lookup (`_sir_ancestry`), never reflection on a Go type name; user
    edges enter only via `_sir_register_ancestry` (built-in edges are never
    overwritten).  The ancestry walk carries a `seen` set so a malicious cyclic
    hierarchy (`class A<B; class B<A`) terminates instead of looping.

### Changed

- **`Feature::Exceptions`, `Feature::Classes`, `Feature::Constants` are now
  accepted** â€” but `Classes`/`Constants` ONLY for exception subclasses and the
  `raise Foo`/`rescue Foo` class-name references they carry, NOT general OOP.  A
  new structural gate `check_exception_soundness` (beside `check_no_keyword_rest_mix`)
  keeps the backend's "never mis-emit" promise: a `Const` reference/assignment
  OUTSIDE a `raise ClassName`, or a `module â€¦ end`, is rejected CLEANLY with an
  `UnsupportedFeature` error.  A class carrying instance/class variables observes
  `InstanceVars`/`ClassVars` (still unaccepted) and is rejected at the manifest
  gate; method-bearing classes hoist their `def`s to top-level Functions, so the
  `ClassDef` body reaching emit is ordinary supported statements.

## 0.7.0

### Added

- **Collection-method dispatch + runtime catalog (C5).**  The Go backend now
  EXECUTES `recv.meth(argsâ€¦)` end to end.  A method call reaches the backend as
  `BuiltinCall("__method__", [recv, StrLit("meth"), â€¦args])`; previously it fell
  through to the generic `_sir_call_builtin_by_name` fallback, which has no
  method-dispatch arm â€” so any collection method failed at runtime.  Now:
  - **Emit** (`emit.rs`): a `"__method__"` case in `emit_builtin_call` lowers the
    dispatch to `_sir_call_method(recv, "name", []Value{â€¦args})`.  A trailing
    block (`MakeClosure`) rides in as the last `[]Value` element; a `&:sym` /
    `&proc` block-pass that survives on the dispatch is converted via
    `try_emit_block_pass` (`_sir_sym_to_proc(intern("sym"))` for `&:sym`, the
    proc verbatim otherwise).  A `Const`-scoped class operand on a class
    predicate (`x.is_a?(Integer)`) is passed as its name string.
  - **Runtime** (`runtime.rs`): a new inlined `_sir_call_method(recv, name, args)`
    implements the collection-method catalog by an **explicit type-switch +
    method-name switch** (Array `*Seq` / Hash `*Map` / String / Numeric / Symbol),
    **ported from the Python/TS `sir-runtime-oop` reference for behavioural
    parity** (same method names, same semantics).  Implemented:
    - **Array**: `length`/`size`/`count`, `first`, `last`, `empty?`, `include?`,
      `index`, `push`/`append`, `<<`, `pop`, `shift`, `reverse`, `sort`, `join`,
      `to_a`, plus block methods `each`, `map`/`collect`, `select`/`filter`,
      `reject`, `reduce`/`inject`, `find`/`detect`, `any?`, `all?`, `none?`.
    - **Hash**: `keys`, `values`, `has_key?`/`key?`/`include?`/`member?`,
      `has_value?`/`value?`, `size`/`length`, `empty?`, plus block methods `each`/
      `each_pair`, `map`, `select`/`filter`, `reject`.
    - **String**: `length`/`size`, `upcase`, `downcase`, `reverse`, `strip`/
      `lstrip`/`rstrip`, `empty?`, `include?`, `start_with?`, `end_with?`, `split`,
      `chars`, `to_i`, `to_f`, `to_sym`.
    - **Numeric**: `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`, `positive?`,
      `negative?`, `succ`/`next`, `pred`, plus the block method `times`.
    - **Symbol**: `to_s`, `to_sym`, `length`/`size`, `upcase`, `downcase`,
      `empty?`.
    - **Universal** (every receiver): `nil?`, `==`, `!=`, `class`, `to_s`,
      `itself`.
    - **`Symbol#to_proc`** (`_sir_sym_to_proc`): `&:sym` becomes a `*Closure`
      that re-enters dispatch on its first argument, so `map(&:to_s)` behaves
      exactly like `map { |x| x.to_s }`.
  - **Security (the C3 RCE lesson)**: dispatch is ONLY through the explicit
    catalog switches â€” there is **no reflection** on the raw method name, no
    dynamic Go method/field lookup.  The catalog switch IS the allowlist.  An
    unknown method on a known receiver falls through to `_sir_method_unknown`,
    which panics with a controlled `undefined method '<name>' for <Class>`
    message â€” a surfaced runtime error, never arbitrary behaviour.
  - **Capability gate** (`lib.rs`): a **pure** collection-method module (a
    `__method__` dispatch with NO class features) is now proven accepted.  This
    needs no gate change and no new `Feature` variant (the deferred C1
    `MethodDispatch` is not required): the validator observes no feature for
    `__method__`, so such a module carries only its receiver/argument features
    (`Sequences`/`Strings`/`Closures`/`Symbols`/`Maps`/`DynamicTyping`), all
    already accepted â€” while class-bearing modules stay rejected
    (`Feature::Classes` is not accepted).  The runtime catalog is the real gate.
  - Adds `sort` + `strings` to the emitted import block (the runtime catalog
    always references both).
  - Tests: emitted-shape unit tests (dispatch call shape, block/`&:sym` shapes,
    class-predicate name-string, catalog present in the preamble); acceptance
    tests (pure dispatch accepted, classes still rejected); and an
    **execution-proof** integration test (`compile_and_run_coll_methods.rs`) that
    runs `.map`/`.select`/`.length`/`.reduce`/`.join`/`.sort`/`.reverse`/
    `.upcase`/`.split`/`.even?`/`.abs`/`.keys`/`&:to_s` through real `go run` and
    diffs stdout against the Python/TS reference values, plus a proof that an
    unknown method (`[1].bogus_xyz`) exits non-zero with the controlled
    "undefined method" message.

## Unreleased

### Fixed

- **Reject keyword params mixed with `*rest`/`**kwrest` (unsound static
  resolution).**  KW6 resolves keyword arguments by *static* keywordâ†’positional
  slot mapping, which is only sound for **fixed-arity** callees.  The core
  validator, however, accepts a callee that mixes a `Keyword` param with a
  variadic (its ordering rule is `Required* Rest? Keyword* KwRest?`, so Ruby's
  `def f(a, *rest, x: 1)` is well-formed), and this backend accepts
  `Feature::KeywordParams` â€” so such a module reached `emit_direct_call`, where
  the `*rest` slot has no fixed position for a keyword to resolve against.  The
  result was a **panic** in debug builds (`debug_assert!` in the slot loop) or a
  **silent mis-emit** in release builds (a single `_sir_missing` sentinel landed
  in the variadic slot instead of a collected sequence).  A new capability check
  (`check_no_keyword_rest_mix`, run beside the manifest gate in `compile`) now
  returns a clean `BackendError { kind: UnsupportedFeature }` for any function
  carrying BOTH a `Keyword` param AND a `Rest`/`KwRest` param, naming the
  offending function.  This becomes frontend-reachable once the Ruby frontend
  (KW7) emits keyword+splat methods.  The keyword-params-**without**-rest happy
  path (fixed arity) is unchanged and still passes all existing tests.
  Added unit tests for both the `*rest` and `**kwrest` rejections and for the
  preserved happy path.

## 0.6.0 â€” KW6 keyword parameters & arguments via static positional resolution

Adds `Feature::KeywordParams` to the Go backend's accepted set (see
`code/specs/sir-keyword-params.md`, Â§4 Go row).  Go has **no** native keyword
arguments, so the backend lowers them **directly** â€” no runtime library â€” by
resolving each keyword to a positional slot at *emit time* (a `DirectCall`'s
callee signature is statically known).  This mirrors the Rust backend's
strategy and reuses the SIR19 default-parameter machinery (the `_sir_missing`
sentinel + callee body prologue) unchanged.

### Added

- **Keyword def params are positional-ized.**  A `ParamKind::Keyword` parameter
  emits as an ordinary positional Go parameter in declared order â€” the
  by-name-ness is a source affordance the backend resolves at the call site.
  An *optional* keyword (`Keyword` + `default: Some`) reuses the existing
  default-param prologue: `if _sir_is_missing(name) { name = <default> }`.
- **Static keywordâ†’positional call resolution.**  A `DirectCall` whose `args`
  contain `Expr::KeywordArg{ name, value }` elements is emitted as a plain
  positional Go call, built in the callee's declared param order:
  leading positionals fill leading slots; each `KeywordArg` fills the slot
  whose param **name** matches (source order irrelevant); every omitted
  *optional* slot is padded with `_sir_missing` (the callee prologue supplies
  the default). Worked example â€” `greet(greeting:, name: "world")`:
  `greet(greeting: "hi")` â†’ `greet("hi", _sir_missing)`;
  `greet(name: "ada", greeting: "hi")` â†’ `greet("hi", "ada")`.
- **`FN_PARAMS` signature table.**  A new per-module thread-local mapping each
  function name to its parameter shapes (name, is-keyword, has-default), in
  order, populated by `emit_module` alongside `FN_ARITY`.  The `DirectCall`
  arm consults it to reorder keywords by name â€” `FN_ARITY` alone knows only
  *how many* params, not their names.

### Tests

- Emitted-shape unit tests: positional-ized keyword def with optional-keyword
  default prologue; keyword call reordered to declared order (source order
  scrambled); omitted optional keyword padded with the sentinel; mixed
  positional + keyword call.
- Execution proof (`tests/compile_and_run_keyword_params.rs`): a
  `greet(greeting:, name: "world")` module compiled and run through `go run`,
  asserting `greet(greeting: "hi")` prints `(hi world)` (default filled) and
  `greet(greeting: "hi", name: "ada")` prints `(hi ada)` (supplied). Skips
  gracefully if `go` is absent.

### Deferred (spec Â§Out of scope)

- **Indirect/closure keyword calls.**  An `IndirectCall`/`MakeClosure` cannot
  resolve keywords by name (the callee signature is not statically known); the
  frontends do not emit such calls, so a `KeywordArg` reaching that path
  panics with a documented deferral message rather than mis-emitting.

## 0.5.0 â€” SIR19 default parameters (P2f) via missing-sentinel runtime-mimic

Adds `Feature::DefaultParams` to the Go backend's accepted set.  Go has no
native optional/default parameters and emitted functions are *fixed-arity*
over `Value`, so the backend uses a **runtime-mimic** strategy: a unique
package-level MISSING sentinel flows through the ordinary `Value` channel.

Semantics are **call-time, param-scope**: a default expression is evaluated
each call, in the callee, where the *earlier* parameters are already bound
(so a later default may reference an earlier param â€” `def f(a, b = a + 1)`).

### Added

- **Runtime MISSING sentinel.**  A distinct, otherwise-empty `_missingMarker`
  struct type plus the single shared instance `var _sir_missing Value =
  &_missingMarker{}`.  A program can never construct one itself (no IR node
  lowers to it), so pointer identity makes the new
  `func _sir_is_missing(v Value) bool` predicate exact and total.
- **Caller-side padding.**  A `DirectCall` that omits trailing defaulted
  arguments pads the call up to the callee's full (fixed) param count with
  `_sir_missing`, e.g. `f(5)` for `f(a, b = â€¦)` emits
  `f(Value(int64(5)), _sir_missing)`.  The full param count comes from the
  module's function table (`FN_ARITY`, populated by `emit_module` before any
  body is walked).
- **Callee body prologue.**  Each defaulted parameter gets a guard at the top
  of the function body, in declaration order:
  `if _sir_is_missing(<name>) { <name> = <emitted default expr> }`.  Ordering
  is what makes a later default see an earlier param's already-resolved
  value.  Reassigning a parameter is ordinary Go (parameters are mutable
  locals) and the guard itself "uses" the param, so Go's strict
  unused-variable rule is satisfied even when the body never reads it.

### Changed

- **`_sir_format` / `_sir_value_eq`** defensively handle the sentinel â€” it
  never reaches a print or `=` path in a well-formed program (a defaulted
  param is always replaced before use), but `_sir_format` renders a stray
  sentinel as `<missing>` and `_sir_value_eq` treats two sentinels as equal
  and a sentinel as equal to nothing else, so it can never masquerade as a
  user value.

### Tested

- Unit tests assert the emitted shape: the body prologue (`if
  _sir_is_missing(b) { b = _sir_plus([]Value{a, Value(int64(1))}) }`), that a
  required param emits no guard, and that `DirectCall` padding appends the
  right number of sentinels.  Runtime tests assert the sentinel type, the
  `_sir_is_missing` helper, and the defensive format/eq guards.
- New `go run` integration test (`compile_and_run_default_params.rs`):
  module `f(a, b = a + 1)` returning `b`, `main` prints `f(5)` then
  `f(5, 10)`; the emitted Go is compiled and run under the real Go toolchain
  and stdout is asserted to be `6` then `10` (the default ran and saw
  `a = 5`; a supplied argument suppressed it).  The four existing `go run`
  tests (floats / loops / seq+maps / cyclic) still pass.

### Housekeeping

- Fixed three pre-existing `clippy` lints in `emit.rs` (a `write!`-with-
  trailing-newline, a needless lifetime on `pick_global_set`, and a
  `len() >= 1`) so the crate is clippy-clean under `--all-targets`.

## 0.4.1 â€” harden emitted Go runtime against cyclic Seq/Map

`*Seq`/`*Map` are shared, *mutable* handles, so an emitted Go program can
build a cyclic structure (`xs = [0]; xs[0] = xs`).  Before this release the
emitted runtime walked such values structurally with no cycle protection,
so a cyclic value would make **`_sir_format`** recurse forever and overflow
the stack while printing, and make **`_sir_value_eq`** recurse forever when
comparing two *distinct* cyclic structures (a self-cycle was already short-
circuited by the same-pointer fast path, but distinct cyclic operands were
not).  This mirrors the Rust backend's `0.4.1` cyclic-guard.

This is a robustness fix only â€” the public runtime API and the printed form
of every *non-cyclic* value are byte-identical (all existing tests pass
unchanged).

### Fixed

- **`_sir_format` / `_sir_format_seq` / `_sir_format_map`** now thread a
  visited-pointer set through a new `_sir_format_d(v, visited)` variant.
  The set is a `map[Value]bool` keyed on the Seq/Map **pointer** â€” a
  `*Seq`/`*Map` boxed in the `Value` (`interface{}`) compares by pointer
  identity, the idiomatic Go way to key on handle identity.  A handle is
  inserted on entry and removed on exit, so it is only "seen" along the
  *current* path: a true cycle re-entering a handle within its own subtree
  prints a placeholder (`[...]` for a seq, `{...}` for a map) and returns
  instead of recursing, while a value reached twice by sibling (non-cyclic)
  paths still prints in full.  `_sir_format_pair` threads the set too (a
  pair can hold a cyclic seq/map).  The public `_sir_format(Value) string`
  signature is unchanged â€” it allocates a fresh visited set and delegates.
- **`_sir_value_eq`** keeps the same-pointer (`as == bs`) identity fast
  path and adds a co-inductive `pending` set of handle-pairs currently
  being compared (a `map[[2]Value]bool` keyed on the two boxed pointers)
  via a new `_sir_value_eq_d(a, b, pending)` variant: re-encountering a
  pair already in flight (a cycle matched in lock-step) is treated as
  equal, bounding the deep comparison of two distinct cyclic operands so it
  always terminates.
- **`_sir_map_get` / `_sir_map_set` / `_sir_map_put`** need no
  restructuring: Go has no `RefCell`-style aliasing-borrow check (the Rust
  backend's "already mutably borrowed" panic on a self-referential key has
  no Go analogue), and the remaining hazard â€” a cyclic key making
  `_sir_value_eq` recurse forever â€” is now handled by that function's
  co-inductive guard.  A comment on `_sir_map_put` records this.

### Tests

- New `tests/compile_and_run_cyclic.rs` integration test: hand-builds a
  module that constructs a cyclic seq (`xs = [0]; xs[0] = xs; print(xs)`),
  emits Go, `go run`s it (gated on `go` availability), and asserts the
  program *terminates* and prints the `[[...]]` placeholder.  It also
  checks that `_sir_value_eq` terminates on both a self-cyclic operand (via
  the same-pointer fast path) and two *distinct* cyclic structures (via the
  co-inductive guard), both `#t`.
- Two new runtime unit tests assert the cycle-guard plumbing is present in
  the emitted runtime string (`_sir_format_d` / `_sir_value_eq_d` and the
  placeholder literals).

## 0.4.0 â€” SIR16 Sequences + Maps â€” completes Go v1 parity (A6)

The final two SIR16 (v1) features land in the Go backend.  With them the
Go backend accepts **all six** SIR16 features (Floats, ShortCircuit,
MutableBindings, Loops, Sequences, Maps) â€” reaching **full SIR-v1
parity**.  Go is the **fifth and last backend to reach v1**, completing
the backend fleet (joining TypeScript, Rust, Python, and the others).
Before this release a module using `SeqLit` / `SeqIndex` / `SeqLen` /
`MapLit` / `MapGet` / `SeqSet` / `MapSet` was rejected at the capability
check and those emit arms were unreachable `panic!`s; this release wires
them up end-to-end.

### Added

- `Feature::Sequences` and `Feature::Maps` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Sequences** â€” the inlined Go runtime gains a `*Seq` value (a struct
  `Seq{ Items []Value }` held by pointer).  The pointer is the crux: a
  `SeqSet` (`xs[i] = v`) mutates the very sequence the caller holds, and
  two bindings that alias the same literal observe each other's writes â€”
  the reference semantics of a Python list / JS array.  Copying a `Value`
  that holds a `*Seq` copies the handle, not the backing slice.
  - `SeqLit` â†’ `_sir_seq_lit([]Value{...})` builds a fresh shared seq.
  - `SeqIndex` â†’ `_sir_seq_index(seq, i)` (strict bounds; out-of-range
    panics, like `car`/`cdr`).
  - `SeqLen` â†’ `_sir_seq_len(seq)` returns the element count as `int64`.
  - `SeqSet` â†’ `_ = _sir_seq_set(seq, i, v)` mutates in place (no
    auto-grow; out-of-range panics).
- **Maps** â€” the runtime gains a `*Map` value (a struct
  `Map{ Entries []MapEntry }`, an *insertion-ordered* association list).
  Go's native `map` can't key on an arbitrary `Value` (floats, closures,
  nested seqs/maps aren't usable keys), so â€” mirroring the Rust backend â€”
  keys are compared with the runtime's structural value-equality
  (`_sir_value_eq`, a linear scan).  A missing key reads as `nil`.
  - `MapLit` â†’ `_sir_map_lit([]Value{keys...}, []Value{vals...})` (keys
    and values emitted as two parallel slices since Go has no tuple
    literal); last-write-wins on duplicate keys, first-seen order kept.
  - `MapGet` â†’ `_sir_map_get(map, key)` (missing key â‡’ `nil`).
  - `MapSet` â†’ `_ = _sir_map_set(map, key, v)` inserts (appends, order-
    preserving) or overwrites in place.
- **Structural value-equality** â€” `_sir_eq` now routes through a new
  `_sir_value_eq` that handles the whole value tower (numbers cross-type,
  symbols, pairs, and now seqs/maps element-wise / entry-wise, with
  identical-handle short-circuit).  This is the single source of truth
  shared by `=` and map-key lookup.
- **ForEach reconciliation** â€” `_sir_seq_iter` (the A5 cons-list
  flattener used by `ForEach`) now *also* snapshots a real `*Seq`, so
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while
  `ForEach`-over-cons-list keeps working.  A `*Seq` is copied element-wise
  into a fresh `[]Value` so the loop body sees a stable view even if it
  mutates the underlying sequence.
- **Display** â€” `_sir_format` renders a seq as a bracketed list
  (`[1, 2, 3]`) and a map as a brace-wrapped, insertion-ordered entry
  list (`{a: 1, b: 2}`).
- New integration test `tests/compile_and_run_seq_maps.rs` â€” hand-builds
  a module that exercises a sequence (lit/index/len/set + aliasing), a
  map (lit/get/set + missing-key â‡’ nil), and a `for x in [10,20,30]`
  ForEach accumulation; emits Go, `go run`s it (gated on `go`
  availability), and asserts stdout (`99 / 3 / 99 / 2 / 3 / nil / 60`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- `accepts_features` is now in lockstep with emit for **all six** SIR16
  features: every declared feature has a real (non-panicking) emit path.
  The only remaining `panic!` reject arms cover SIR17/18 nodes
  (classes / module-defs / exceptions / `StrConcat`) whose features stay
  unaccepted, so they remain strictly unreachable.

## 0.3.0 â€” SIR16 MutableBindings + Loops (A5)

The next two SIR16 (v1) features land in the Go backend, mirroring the
merged Rust backend equivalent.  Before this release the Go backend
accepted only `Floats` + `ShortCircuit`, so every `Assign` / `While` /
`ForRange` / `ForEach` IR node hit a `panic!` reject arm.  This release
wires up mutation and the three loop forms end-to-end onto Go's native
`for`.

### Added

- `Feature::MutableBindings` and `Feature::Loops` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **MutableBindings** â€” `Stmt::Assign` to a Local/Param/Capture emits a
  plain `<name> = <value>`.  Go has no const/mut distinction, so unlike
  the Rust backend (which needs a `let mut` pre-pass) reassignment just
  works against the name already declared by the matching `LetBinding`
  (`:=`) or parameter.  A `Global` assignment writes through the runtime
  global store (`_sir_globals[<key>] = <value>`).
- **Loops** â€” `Stmt::While` / `ForRange` / `ForEach` map onto Go's
  native `for`:
  - `While` â†’ `for _sir_truthy(<cond>) { <body> }` (Go's `for` is its
    `while`; the test routes through SIR truthiness, never Go `bool`).
  - `ForRange` â†’ a native three-clause `for` whose `stop`/`step` bounds
    are cached **once** into `int64` temporaries (re-evaluating Python's
    `range` bounds each turn would be wrong).  A direction-aware
    continue test (`_sir_range_cont`) lets a negative `step` count down.
    The loop variable is re-bound each turn as a fresh `Value(int64(â€¦))`
    and guarded with `_ = <var>` so an unused loop var still compiles.
  - `ForEach` â†’ `for _, <var> := range _sir_seq_iter(<iter>)`.  The new
    runtime `_sir_seq_iter` flattens a cons-list (`Pair`-chain ending in
    `nil`) into a `[]Value` (Sequences land in a later PR, so a
    "sequence" is still the classic cons-list).
- Loop bodies emit in statement context: a body's trailing non-`nil`
  value becomes `_ = <value>` (so side effects fire), and introduced
  loop variables get a `_ = <var>` guard â€” satisfying Go's strict
  unused-variable rule even when the body ignores them.
- New runtime helpers `_sir_range_cont` and `_sir_seq_iter`.  (`ForRange`
  reuses the existing `_sir_as_int` from the Floats release for its
  bound extraction.)
- New integration test `tests/compile_and_run_loops.rs` â€” hand-builds a
  module using a mutable accumulator, a `for`-range, and a `while`
  countdown, emits Go, `go run`s it (gated on `go` availability), and
  asserts stdout (`sum 0..5 = 10`, countdown to `0`, reassign to `99`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- Only two SIR16 features remain undeclared (`Sequences`, `Maps`); their
  `SeqLit` / `MapLit` / `SeqSet` / `MapSet` nodes still hit `panic!`
  reject arms, kept strictly unreachable by the capability check until a
  later PR.  `accepts_features` stays in lockstep with emit: every
  declared feature has a real (non-panicking) emit path.

## 0.2.0 â€” SIR16 Floats + ShortCircuit (A4)

First two SIR16 (v1) features land in the Go backend, mirroring the
just-merged Rust backend equivalent.  Before this release the Go backend
declared *none* of the six SIR16 features, so every SIR16 IR node hit a
`panic!` reject arm.  This release wires up two of them end-to-end.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Floats** â€” the inlined Go runtime's `Value` (`interface{}`) now
  accepts a `float64` arm:
  - New helpers `_sir_as_float`, `_sir_any_float`, `_sir_is_number_val`,
    and `_sir_format_float`.
  - Arithmetic (`+ - * /`) keeps the exact int64 fast-path while every
    operand is an integer, and promotes the whole fold to `float64` the
    moment any operand is a float ("int op float â‡’ float").  Integer
    division keeps its divide-by-zero panic; float division follows
    IEEE-754 (`1.0/0.0 â‡’ +Inf`).
  - `=` is cross-type for numbers (`1 == 1.0` is true) and uses IEEE
    equality for floats (`NaN != NaN`).  `<` / `>` compare numerically,
    staying on the int path when both operands are int64.
  - `number?` is true for both integers and floats.
  - `FloatLit` emits `Value(float64(<lit>))`; integral values spell out
    `3.0` (never `3`) so the runtime type-switch hits the float arm.
    Non-finite values route through `math.NaN()` / `math.Inf(Â±1)` since
    Go has no float literal for them.
  - Display: `_sir_format_float` prints integral floats with a trailing
    `.0` (`3.0`, not Go's default `%v`-style `3`), fractional values via
    `strconv.FormatFloat(x, 'g', -1, 64)`, and non-finite values as
    `NaN` / `inf` / `-inf` â€” matching the Rust backend's intent.
- **ShortCircuit** â€” `LogicalAnd` / `LogicalOr` emit a truthy-guarded
  immediately-invoked func literal:
  `func() Value { __l := <lhs>; if _sir_truthy(__l) { return <rhs> } else { return __l } }()`
  (and the mirror for `or`).  The operand value is returned (not a
  coerced bool), `lhs` is evaluated exactly once, and each IIFE scopes
  its own `__l` so nesting never collides.  Pure emit â€” no runtime
  change.
- The emitter now imports `"math"` (alongside `"fmt"` and `"strconv"`);
  the runtime always references it via the float `NaN`/`Inf` checks, so
  Go's unused-import rule stays satisfied.
- Integration test `tests/compile_and_run_floats.rs`: hand-builds a SIR
  module exercising floats, short-circuit, and cross-type equality;
  emits Go, runs it with `go run`, and asserts stdout
  (`4.0 / 4.0 / 5 / 7 / #f / #t`).  Gated on `go version` â€” skips with a
  log line if the Go toolchain is absent.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still **not** declared, so the corresponding emit arms
  (`SeqLit`, `MapLit`, `Assign`, `While`, â€¦) remain reachable only as
  internal-bug `panic!`s â€” the capability check rejects such modules
  before emit.  They land in later Go PRs.

## 0.1.2 â€” SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 â€” SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 â€” initial release (SIR15 v0)

Fourth backend for the narrow-waist Semantic IR.  Emits
self-contained Go source from a `semantic_ir::Module`.

### Added

- `GoBackend` implementing `semantic_ir::Backend` with
  `target_tag = "go"`; accepts the v0 feature set minus
  `TailCalls` and `Intrinsics`.
- Per-node lowering per SIR15.  Notable Go-isms:
  - `If` and non-trivial `Block` lower to immediately-invoked
    function expressions (`func() Value { ... }()`) since Go has
    no expression-position blocks.
  - `MakeClosure` emits an adapter `func([]Value) Value` that
    splats the runtime args into the synthesised lambda's
    positional parameters; the per-function arity table is
    threaded through TLS so the splat is sized correctly.
  - `LetBinding` emits `name := value` followed by a defensive
    `_ = name` so unused bindings don't break Go's strict
    unused-variable rule.
  - `ExprStmt` emits `_ = expr` for the same reason.
- Inlined Go runtime (~280 lines) covering `Value` (`interface{}`),
  `Symbol`, `Pair`, `Closure`, all 15 Twig builtins, symbol
  interning, module globals, `_sir_format` and `_sir_truthy` and
  `_sir_apply` and `_sir_make_closure`, plus a `_sir_call_builtin_by_name`
  dispatch table for `VarRef Builtin`.
- Identifier sanitisation handles Go keywords (`for`, `func`,
  `chan`, etc.) and predeclared builtins (`int`, `string`,
  `print`, `len`, etc.) by appending `_`.  Other invalid chars
  encode as `_<hex>`.  Empty â†’ `_sir_empty`.  SIR's `main` is
  renamed to `_sir_user_main` so the emitter's own `main()`
  doesn't collide.
- `sanitize_comment` strips line terminators from external
  strings written into `//` comments â€” same defence as SIR12 /
  SIR13 / SIR14.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via `Backend::check_module`.

### Notes

- The runtime always imports both `"fmt"` and `"strconv"` â€” both
  are referenced inside the runtime block, so Go's strict
  unused-import rule never fires regardless of what the user
  module uses.
