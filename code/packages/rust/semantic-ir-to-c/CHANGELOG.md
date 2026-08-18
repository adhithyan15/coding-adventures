# Changelog

## 0.40.0 — SIR22 array/matrix base cut (Phase A Slice 2, second-wave backend rollout)

Opens this backend to `Feature::{NDArrays, MatrixOps, ArrayColumnMajor}` and
implements the SIR22 "base cut": `Expr::ArrayLit`/`Range`/`MatMul`/
`ElementwiseOp`/`Transpose`/`IndexGet` and `Stmt::IndexSet` (the mutating
counterpart — a statement, not an expression, per the SIR22 spec's
"Effects" section). One of five parallel, independent backend PRs for this
slice (C/Go/Rust/Ruby follow JS's inlined-runtime model; Python follows
TS's imported-package model — see the SIR22 spec's "Backend impact"
section, amended for this rollout in #11946).

**New `_sir_array_*` runtime** (`runtime.rs`, appended as a new "SIR22
array/matrix domain" section): an inlined port of
`semantic-ir-to-javascript`'s own already-proven `ArrayRt` sub-runtime,
following this crate's existing inlined-runtime convention (this backend
always inlines its runtime helpers into the emitted `.c` file — it never
imports a package, unlike the TS/Python model). A new `SirNDArray` struct
(`{ rank, rows, cols, data, data_len }`, `rank` always 0 or 2 — a rank-1
shape is never actually produced by any base-cut constructor, see the
runtime's own module doc) and a new `SIR_ARRAY` `SirTag` variant carry the
value; `_sir_fmt`/`_sir_display_str`/`_sir_value_eq_d`/`_sir_object_equal_p`
each gained a minimal `SIR_ARRAY` case (a `#<Array>` placeholder for
display, pointer identity for equality — a full `[1 2; 3 4]`-style
rendering is out of this slice's scope, matching every reference backend's
own test suite, which reads a scalar element back out via `IndexGet`
instead of printing the array itself).

**Value-model divergence from the Ruby port (documented, deliberate)**: the
sibling `semantic-ir-to-ruby` 0.x SIR22 slice (merged the same day) chose
to preserve native Ruby `Integer`/`Float` propagation through `+`/`-`/`*`,
forcing `Float` only for `Div`/`Pow`. This C port instead follows the JS
reference's ACTUAL internal representation: every `SirNDArray` element is
an unconditional `double`, and an element read back out is always
`_sir_float(d)` — not JS's cosmetic integer-display shortcut (`19` instead
of `19.0`), just its real `Float64Array` storage. Given C's `SirValue` is
statically tagged (`SIR_INT` xor `SIR_FLOAT`, no dynamic `Numeric`
hierarchy the way Ruby has), this is the simplest correct choice: it
avoids a second int/float dispatch path through `_sir_array_apply_op`
(which the Ruby port needs precisely because Ruby's numeric tower isn't
statically tracked), at the cost of a cosmetic trailing ".0" that no
reference backend's tests need to match — each backend's own
`tests/sir22_array.rs` asserts against its own emitted output only.

**The C-specific overflow hazard neither JS nor Ruby has**: JS `Number`s
are IEEE doubles (lose precision past 2^53, but never wrap) and Ruby
`Integer` is arbitrary-precision (never overflows) — `rows * cols`
computing an element count is safe in both references with no explicit
overflow check. C's `int64_t` has neither property: `rows * cols` can
silently wrap (undefined behaviour, for a signed overflow) before it is
ever compared against the `SIR_ARRAY_MAX_ELEMENTS` (2^26) cap, turning a
huge requested shape into a small/negative/UB computed size and then
`malloc`ing too little for what the caller believes it got — a classic
heap-overflow setup. `_sir_array_checked_size` checks `rows >
SIR_ARRAY_MAX_ELEMENTS / cols` (rejecting) **before** ever computing
`rows * cols`, not after; every allocation in this file routes through it
(directly, or via a caller — `_sir_array_new_matrix` — that always calls
it) before ever calling `_sir_alloc`. Index-position resolution has its
own, distinct NaN/overflow hazard: casting a NaN or out-of-`int64_t`-range
`double` to `int64_t` is undefined behaviour in C (unlike JS's
`Number.isInteger` guard or Ruby's `Float#to_i`, which both fail cleanly
instead) — `_sir_array_assert_valid_position` bounds-checks the `double`
**before** the `(int64_t)` cast, not after, and every index position in
this domain funnels through that one choke point.

**Two compile-time structural checks, added specifically because C's
static AST shape makes them possible where the JS/Ruby ports could only
check at runtime**: (1) an `ArrayLit` with ragged rows is rejected by
`emit.rs`'s pre-emit scan (raggedness is knowable from the SIR AST alone —
`rows: Vec<Vec<Expr>>`'s row lengths are fixed at parse/lowering time, not
a runtime-computed shape); (2) an `IndexGet`/`IndexSet` with other than 1
or 2 index arguments (the "rank <= 2" scope boundary) is likewise rejected
at compile time (`indices.len()` is static too). The runtime
(`_sir_array_index_get`/`_sir_array_index_set`) repeats the arg-count
check anyway, as defense-in-depth for a hand-built `Module` that bypassed
the scan — consistent with this file's existing "don't rely on every
future caller re-deriving an invariant" discipline.

**SIR22 "APL addendum" rejected cleanly, using this crate's EXISTING
mechanism, not a new one**: `Reduce`/`Scan`/`OuterProduct`/`Shape`/
`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate` share
`NDArrays`/`MatrixOps`/`ArrayColumnMajor` with the base cut above, so the
ordinary feature-flag capability check alone cannot distinguish "safe"
modules from "still unimplemented" ones. Nine new arms were added directly
to `emit.rs`'s existing `scan_expr_for_builtin` — the SAME structural
pre-emit scan `first_unsupported_builtin` already uses for every other
"well-formed but not yet lowered" construct in this crate (malformed
`__new__`/`__def_method__`/etc. shapes) — rather than inventing a parallel
mechanism (contrast the sibling Ruby backend, which added a dedicated
`ScanHit::Sir22AddendumNode` variant to its own differently-shaped single
shared scan; this crate's scan already returns a plain `Option<(String,
Span)>`, so no new enum variant was needed). Without this, a module using
e.g. `Reduce` would pass the capability check and panic inside
`emit_assign`'s `unreachable!` catch-all.

**New `tests/sir22_array.rs`** (13 tests, ported from the JS backend's own
`sir22_array.rs`, cross-checked against the sibling Ruby backend's port):
`matmul` of two 2x2 matrices (verified against a known-correct product),
elementwise `Mul` with a bare-scalar operand (the `_sir_array_coerce`
bare-scalar-operand regression case — MATLAB's `.* / .\` lowering can hand
a raw scalar, not an `ArrayLit`, on one side), elementwise `Div`'s
always-true-divide behaviour (distinguished from this backend's own bare
`/`, which floors on two `SIR_INT`s), `transpose`, `range` (both explicit
and default `step`), an `IndexGet` with a `Whole` selector (whole-row
read), `Stmt::IndexSet` mutating in place (both a `Scalar` position and an
`IndexArg::Range` sub-range overwrite), plus five compile-time-rejection
tests: two APL-addendum nodes (`Reduce`, `Catenate` — proving the
rejection is a real per-variant scan arm, not a fluke that only catches
one), a ragged `ArrayLit`, and a rank-3 `IndexGet`. Every execution test
runs against a real `cc`/`clang`/`gcc` toolchain (`SIR_CC` env var, then
PATH probing, matching `compile_and_run_division_ops.rs`'s exact
`find_cc`/`compile_and_link` pattern, copied verbatim including its
unique per-process-and-atomic-counter temp filenames), skipping gracefully
when none is present.

Verified via the full `semantic-ir-to-c` test suite (all pre-existing
integration tests still green) and `cargo clippy --all-targets -- -D
warnings` (clean).

`semantic-ir-to-c` 0.39.0 -> 0.40.0.

## 0.39.0 — SIR21 T3b-2 Slice 7: cleanup — remove dead `tdiv`/`utdiv`

Part of the SIR21 T3b-2 arc's final slice. `c-to-semantic-ir` — the only
crate that ever emitted the bare `"tdiv"`/`"utdiv"` builtin names — migrated
to `div_trunc`/`udiv_trunc` in Slice 6 (0.3.0, merged). With that migration
in, `"tdiv"`/`"utdiv"` are provably dead names: nothing in this repository
constructs a `BuiltinCall` with either name anymore.

Removed the three dead dispatch entries: the `variadic_helper`/binary-arity
table's `"tdiv" => ("_sir_itdiv", 2)` / `"utdiv" => ("_sir_utdiv", 2)`
(`emit.rs`), and the value-referenced-builtin dispatcher's matching
`strcmp(name, "tdiv")`/`strcmp(name, "utdiv")` branches (`runtime.rs`).

**What did NOT change**: the underlying `_sir_itdiv`/`_sir_utdiv` C runtime
functions themselves — `div_trunc`/`udiv_trunc`'s own dispatch entries
(added in Slice 2, 0.38.0) still call them, so they remain live code, just
reachable only under their new canonical names now. `tmod`/`utmod` (modulo)
are untouched — this arc has never touched modulo, a deliberate,
documented asymmetry (see the spec's own forward pointer to a future,
unnumbered milestone). Bare `"/"` also stays exactly as it was (still
aliased to `div_floor`'s `_sir_divide` implementation) — it is `twig-to-
semantic-ir`'s permanent fallback route (Twig's `/` is variadic and has no
static int/float distinction to route to one of the four typed ops, so it
can never migrate off the bare name — see the spec's own explanation).

Also added a new Twig-sourced regression case to
`tests/compile_and_run.rs`'s corpus (`twig_bare_slash_still_floors_after_
tdiv_utdiv_removal`, `-7 / 2` → `-4`): Twig's `/` is variadic with no
static int/float distinction, so it can never migrate to one of the four
typed division ops — it stays on bare `"/"` permanently, and this arc's
own dedicated cleanup slice for the *other* two names it shares a codebase
with (`tdiv`/`utdiv`) is exactly the kind of change that could silently
regress an unrelated, still-live dispatch path if a shared code region
were touched carelessly. This proves it didn't.

Verified via the full `semantic-ir-to-c` test suite (39 integration test
binaries, all real-`cc`-compile-and-run, 0 failures) and the full
`sir-conformance` suite (0 failures) — both green with these dispatch
entries gone, confirming nothing else in the pipeline still reaches for
`"tdiv"`/`"utdiv"` by name.

`semantic-ir-to-c` 0.38.0 -> 0.39.0.

## 0.38.0 — SIR21 T3b-2 Slice 2: `div_floor`/`div_trunc`/`udiv_trunc`/`div_true`

Additive only — no frontend emits these names yet, bare `"/"`/`"tdiv"`/
`"utdiv"` keep working unchanged. All three dispatch sites (`variadic_helper`,
the binary-arity table, and the value-referenced-builtin `_sir_builtin_dispatch`
in `runtime.rs`) gained entries for the four new canonical division op names,
per `code/specs/SIR21-type-system-and-integer-semantics.md` §E3:

- `div_floor` → `_sir_divide` (the SAME helper `/` already uses — a rename,
  zero new logic; floors ints toward −∞, true-divides floats).
- `div_trunc`/`udiv_trunc` → `_sir_itdiv`/`_sir_utdiv` (the SAME helpers
  `tdiv`/`utdiv` already use — also a rename; `tdiv`/`utdiv` themselves stay
  working for now, removed in a later cleanup slice once every frontend
  emitting them has migrated).
- `div_true` → new `_sir_true_div(SirValue a, SirValue b)`: always coerces
  both operands to `double` and divides, regardless of operand tag (models
  Python's `/`). Fails loudly (`fprintf` + `exit(1)`) on a zero divisor,
  matching every other division builtin in this file — deliberately NOT
  matching the older `_sir_divide_v`'s float path, which silently produces
  IEEE `inf`/`nan` on `x / 0.0`, since Python's `ZeroDivisionError` fires
  unconditionally, not just on the integer path.

New `tests/compile_and_run_division_ops.rs`: real `cc`/`clang`/`gcc`
compile-and-execute proof for all four ops (mirrors
`compile_and_run_floats.rs`'s pattern) — §E3's own worked example, the
floor-vs-truncate divergence on negative operands, `div_floor`/`div_trunc`
emitting the byte-identical helper call `/`/`tdiv`/`utdiv` already did, and
`div_true`'s zero-divisor failure specifically (no existing test covered
this path at all before this PR).

## 0.37.0 — SIR28 §7: remove dead bare `print`/`puts` handling

Every frontend now emits `__sys_write__` instead of bare `print`/`puts`
(SIR28 Slices 4-6, all merged), so this backend's `print`/`puts` handling
is dead code. Removed:

- The `"print"`/`"puts"` entries from `variadic_helper`'s emit-name
  match.
- `_sir_print_v`, `_sir_puts_v`, `_sir_print`, and `_sir_puts` from
  `runtime.rs`.
- The `"print"`/`"puts"` arms from `_sir_builtin_dispatch`'s by-name
  dispatch (the value-position/first-class-builtin path).

**Kept** `_sir_puts_one`: unlike the deleted `_sir_print_v`/`_sir_puts_v`,
this helper is genuinely shared — `_sir_write_v`'s `per_value` terminator
(when `unpack_arrays` is set) calls it directly to get `puts`'s
recursive array-flattening behavior. Confirmed via grep before touching
anything; deleting it would have broken `__sys_write__` itself.

This is a breaking change for any SIR module that still emits bare
`print`/`puts` — none do, in this monorepo, as of SIR28 Slice 6.

Test suite: every local test helper that hand-built bare `print`/`puts`
`BuiltinCall`s purely to observe hand-constructed IR's output (unrelated
to testing print semantics itself) now builds the equivalent
`__sys_write__` envelope (`terminator: "per_value"`, `unpack_arrays:
true` — every one of this backend's test helpers was `puts`-shaped, not
`print`-shaped), plus `Feature::ConsoleIO`/`Feature::Strings` added to
each affected manifest. Also updated `examples/dump_convert.rs` and a
runtime-shape assertion in `compile_and_run_conversions.rs` that
asserted on the now-deleted `_sir_puts(...)` call text directly.

## 0.36.6 — implement `__sys_write__`, the SIR28 console-output primitive

Adds a `"__sys_write__"` emit arm (`emit_builtin_simple` for the common
simple-args case, plus a dedicated `emit_compound_call` arm mirroring the
existing `raise`-with-compound-message pattern for when a value argument
needs statement hoisting) and a new runtime function, `_sir_write`/
`_sir_write_v`, generalizing the existing `_sir_print_v`/`_sir_puts_v` into
one function parameterized by `stream` (stdout/stderr), `terminator`
(none/per_value/once), and `unpack_arrays` — the policy axes SIR28 §2.1
defines. Declares `Feature::ConsoleIO`.

Purely additive: nothing emits `__sys_write__` yet (that's SIR28 Slices
4-6), so `_sir_print`/`_sir_puts` and every existing `print`/`puts`-sourced
program are unchanged. `stream`/`terminator` are always compile-time `StrLit`
literals (already validated by `semantic-ir`'s validator against a closed
set, SIR28 §2.2) baked directly into the generated call as C int constants
— never source-derived text reaching a dynamic file-handle/dispatch lookup.

New `tests/compile_and_run_sys_write.rs`: hand-builds a `Module` directly
per stream/terminator/unpack_arrays combination (no frontend emits the op
yet), emits C, compiles with a real `cc`, runs, and asserts stdout/stderr —
covering all three `terminator` modes, `unpack_arrays` true/false, the
`stderr` stream, the empty-args `per_value` edge case, and the
`emit_compound_call` hoisting path (a value argument that is itself an
`If`).

## 0.36.5 — doc-comment reframing: SIR25 §2 is the dispatch authority, not "matches Ruby"

Documentation-only, no behavior change. Per `SIR25-language-agnostic-
object-model.md`'s §6 (which explicitly tracks this as its own follow-up),
this backend's OOP-dispatch-mechanism doc-comments — method-resolution
precedence (`_sir_resolve_method`) and default no-op construction
(`_sir_call_new`) — now cite SIR25 §2.1/§2.2 as the authority, keeping
"matching Ruby's ..." as a parenthetical (still true, still useful context:
this dispatch model happens to coincide with Ruby's today, it just isn't
*defined as* "whatever Ruby does"). Per-method Collections-catalog
doc-comments ("matching Ruby's `Array#fetch`", etc.) are deliberately left
untouched — SIR25 §3 explicitly designates that framing as legitimate
naming provenance, not structural coupling.

## 0.36.4 — string interpolation (`"a#{x}b"`)

Filed as its own backlog item ("C backend: support string interpolation").
The Ruby frontend already lowers `"a#{x}b"` to `Expr::StrConcat { parts }`
for every backend (Python/TypeScript already had real emit arms; Go/JS/Rust
reject it with a deferred-node message); the C backend rejected
`Feature::StringInterpolation` outright — no emitter arm existed at all.

New runtime helper `_sir_display_str(SirValue) -> char*` (plus
`_sir_display_seq`/`_sir_display_map`/`_sir_display_pair`/
`_sir_display_float`) renders a value Ruby's `to_s` way into a fresh
string — a STRING-RETURNING PARALLEL to the existing `puts`/`print`
`FILE*`-writing `_sir_fmt` family, not a refactor of it, so that
already-tested path is completely untouched (the two are kept in
lockstep by inspection: same tag list, same per-tag text, same recursion
structure and depth cap, so `#{arr}` and `puts arr` always agree).

`Expr::StrConcat`'s emitter arm folds each part through
`_sir_display_str` and pairwise through `_sir_cat` (which takes exactly
two operands) into one `_sir_str(...)` — `emit_str_concat` for the
simple-operand case (inline, mirroring `SeqLit`'s simple path) and
`emit_str_concat_names` for the compound-operand case (over
`hoist_operands`-hoisted temps, mirroring `SeqLit`'s hoisted path).
`Feature::StringInterpolation` added to `ACCEPTED_FEATURES`.

`semantic-ir-to-c` 0.36.3 -> 0.36.4.

## 0.36.3 — fix: `Foo.new` never invoked `initialize`

Filed as a follow-up in 0.36.1 and fixed here. `__new__` used to lower
straight to `_sir_new_instance(cls)` — a bare allocation — and the
emitter's own scan REJECTED any `__new__` with more than the class-name
arg ("`__new__` with constructor arguments or a non-constant class name"),
so `Foo.new(args)` either compiled to an object whose constructor never
ran (every `@ivar` an `initialize` would have set stayed nil) or, once
constructor args were involved, failed to compile at all. This matched
NONE of the other five backends: Go's `_sir_call_new`, Rust's equivalent,
and Ruby's `sir_new` have always allocated THEN explicitly invoked the
registered `initialize` (if any) with the constructor args — a comment on
`semantic-ir-to-ruby`'s own `__new__` arm already described this as
mirroring "the Go/C/Rust runtimes", which was aspirational for C until
now. The `counter_state` program in `sir-conformance`'s corpus (`class
Counter; def initialize; @n = 0; end; ...`) was previously SKIPPED on the
C backend for exactly this reason (a declared v0 gap, not a faithfulness
bug) — it now runs and agrees with every other backend.

Fixed with a new `_sir_call_new(cls, argc, ...)` runtime helper: allocate
via the existing `_sir_new_instance`, resolve `initialize` up the
ancestry with the existing `_sir_resolve_method` (the same walk
`_sir_call_method` uses, so an inherited `initialize` runs correctly),
bind `self` to the new object and invoke it with the constructor args if
found, restore `self`, and always return the object — even with no
`initialize` registered (Ruby's default no-op `Object#initialize`). The
emitter's scan now only requires `__new__`'s first arg to be a `StrLit`
class name; any further args are ordinary constructor arguments emitted
like any other call args, no longer specially rejected.

`semantic-ir-to-c` 0.36.2 -> 0.36.3.

## 0.36.2 — fix: `<<` builtin name collided between C's bitwise shift and Ruby's operator

`variadic_helper`'s `"<<" => "_sir_shift_left"` entry (added when Ruby's
polymorphic `<<` operator landed in 0.36.0) is checked BEFORE
`fixed_helper`'s `"<<" => ("_sir_shl", 2)` entry (C's raw bitwise left
shift, pre-existing from `c-to-semantic-ir`'s own milestone-6 shift
lowering) — so `fixed_helper`'s entry was silently DEAD CODE for the name
`"<<"`. Every C program's `<<` was actually getting Ruby's
saturate-instead-of-grow-a-bignum Integer-shift semantics, not a raw bit
shift. Caught by `c-to-semantic-ir`'s `three_way_conformance` test: a
uint64 value needing the bit pattern `0x8000000000000000` (one past
`INT64_MAX`, unrepresentable without saturating) got silently clamped,
corrupting a later logical-right-shift read.

Fixed at the source: `c-to-semantic-ir` now lowers its OWN `<<` to a
distinct `c<<` builtin name (mirroring the pre-existing `>>`/`u>>`
signed/unsigned split), so `fixed_helper`'s `"<<"` key is renamed to
`"c<<"` here too — `variadic_helper`'s `"<<"` entry is UNCHANGED and
still correctly means Ruby's operator. The rarely-reached
`_sir_builtin_dispatch` closure-lookup table (`Scope::Builtin` VarRef,
e.g. a hypothetical `arr.reduce(:<<)`) gained the same split: `"<<"` now
calls `_sir_shift_left_v` (Ruby semantics, matching `variadic_helper`)
and a new `"c<<"` entry calls `_sir_shl` (C semantics) — previously it
only had the C mapping under the bare name, which would have been wrong
for a Ruby-sourced `<<` closure reference.

`semantic-ir-to-c` 0.36.1 -> 0.36.2.

## 0.36.1 — execution proof for dotted/chained bracket-index writes

No runtime code changed: `_sir_builtin_method_v`'s existing `"[]="` arm
already dispatches on the receiver's ACTUAL runtime tag (Array vs Hash vs
whatever an arbitrary receiver EXPRESSION evaluates to), so it already
handled `obj.data[0] = v` and `a[0][1] = v` correctly the moment
`ruby-to-semantic-ir` 0.9.1 started emitting the right nested `__method__`
calls for those receiver shapes — no C-side change needed. This release
just adds the execution proof: `chained_bracket_write_on_a_nested_array`,
`dotted_receiver_write_through_an_oop_method_call`,
`dotted_receiver_write_through_a_hash_value_persists`, and a regression
check (`single_bracket_write_on_a_bare_name_receiver_is_unaffected`) that
the original v0-scoped shape still runs identically.

Along the way, discovered (and filed as its own follow-up, NOT fixed
here) that `ClassName.new` never invokes `initialize` in this backend —
`_sir_new_instance` only allocates a bare instance; already documented
in-code as a deferred "later slice" (`emit.rs`), just not previously
tracked as an explicit backlog item. Worked around in this PR's own
OOP-dot-call test by using a method that returns a literal rather than an
ivar, so the test proves the RECEIVER-CHAIN dispatch is correct
independent of that separate gap.

## 0.36.0 — `<<`, Ruby's shift operator

`ruby-to-semantic-ir` 0.9.0 lowers `<<` to a top-level `BuiltinCall("<<",
[lhs, rhs])` (the same op-name-keyed protocol `+`/`-`/`*`/`/` use, not the
`__method__` dispatch protocol Collections methods use). This adds the C
runtime side: a new `"<<" => "_sir_shift_left"` entry in `variadic_helper`,
and `_sir_shift_left_v`, polymorphic over three receiver types:

- **Array** — pushes each RHS operand in place (`_sir_array_push_one`,
  the same slice-4 growth helper `Array#push` uses) and returns the
  (mutated) receiver, so `a << 1 << 2` chains and mutation is visible
  through a shared binding, exactly like `push`.
- **Integer** — bitwise shift, matching real Ruby's rules: a NEGATIVE
  shift amount REVERSES direction (`5 << -1 == 5 >> 1 == 2`); since this
  runtime has no bignum (unlike real Ruby, which grows arbitrarily — `1 <<
  63 == 9223372036854775808`, one past this runtime's `INT64_MAX`), an
  out-of-range left shift SATURATES at `INT64_MAX`/`INT64_MIN` rather than
  wrapping or reaching C's shift-amount-exceeds-width UB (shifting by
  `>= 64` on a 64-bit type is UB, so every actual C shift is pre-checked
  into range first). The exact-boundary case (`-1 << 63 == INT64_MIN`,
  which fits with no truncation) is verified to NOT spuriously saturate.
- **String** — concatenates via `_sir_cat`/`_sir_str_of` (the same helper
  `_sir_plus_v` already uses for `+`'s String-receiver case) and returns a
  NEW string. Two documented divergences from real Ruby: (1) true
  `String#<<` mutates in place with shared-reference visibility (like
  Array's push) — this runtime's `SIR_STR` is a bare `const char *` with
  no heap box/pointer identity (unlike `SIR_SEQ`/`SIR_MAP`), so in-place
  mutation isn't representable without a different String representation
  entirely; (2) real Ruby's `"a" << 98` appends the CHARACTER at codepoint
  98 (`"ab"`) — `_sir_str_of` only recognizes String/Symbol, so a
  non-String RHS here is silently dropped instead (matching `_sir_plus_v`'s
  existing behavior for `"a" + 5` in this same runtime, which also drops a
  non-string operand rather than stringifying or raising).

Only the C backend has a `<<` runtime implementation so far — Python/JS/
Go/Rust/Ruby all ACCEPT `<<` at emit time (confirmed via direct probe) but
their shared runtime packages don't register a `"<<"` builtin entry, so
running the emitted program raises a clean runtime error naming the gap
(confirmed for Python: `NameError: SIR builtin '<<' is not implemented in
sir-runtime-core's dispatch table ... a backend coverage gap`) — the SAME
shape of gap as `[]`/`[]=` bracket-index dispatch, tracked as its own
follow-up rather than blocking this PR.

`semantic-ir-to-c` 0.35.0 -> 0.36.0.

## 0.35.0 — deferred-from-slice-8 String methods: char-set + padding

Widens `_sir_builtin_method_v` with the two String method families slice 8
explicitly deferred to keep that slice reviewable, matched against the
Python/TS `sir-runtime-oop` reference catalog:

- **Char-set methods** — `count(charset, ...)`, `delete(charset, ...)`,
  `squeeze(charset=nil)`. Each `charset` argument is treated LITERALLY as
  the set of characters it contains (Ruby's char-range (`"a-z"`) and
  negation (`"^abc"`) forms stay a documented follow-up, the same
  literal-only scope precedent `tr`/`sub`/`gsub` already use). Multiple
  charset arguments INTERSECT. `squeeze` with no charset argument collapses
  every consecutive run; with one or more charset arguments, only runs of
  chars in the (intersected) set collapse — kept as genuinely separate
  cases (not "empty intersection"), since a truly empty intersection must
  squeeze NOTHING while "no charset at all" must squeeze EVERYTHING.
  `count`/`delete` share their method names with Array#count (slice 3) and
  Hash#delete (slice 6) — merged into those EXISTING dispatch arms rather
  than given a second `else if` on the same `strcmp`, since this file's
  dispatcher is one long if/else-if chain where the first name match wins
  regardless of whether its body actually returns.
- **Padding methods** — `ljust(width, pad=" ")`, `rjust`, `center`. Pad to
  `width` BYTES (this runtime is byte-oriented throughout, matching
  `length`/`bytes`) using `pad` repeated cyclically; `center` puts any odd
  leftover pad byte on the RIGHT (Ruby's rule — the opposite of Python's
  single-char-only `str.center`). The deficit is clamped at
  `SIR_MAX_PAD_LEN` (100,000,000, mirroring the Python/TS reference's
  `_MAX_REPEAT_LEN`) so a hostile width (`"".ljust(10**18)`) cannot exhaust
  memory — `_sir_alloc` only guards a FAILED allocation, not a succeeding
  multi-gigabyte one. The width argument is extracted via a new
  `_sir_str_width_arg`, never a bare `(int64_t)v.as.f` cast (UB for a
  non-finite/out-of-range Float), and the `width <= 0` short-circuit is
  load-bearing, not just an optimization: `width` can be `INT64_MIN` (a
  saturated hostile Float argument), and `width - len` on that value would
  be signed-overflow UB — the short-circuit sidesteps computing the
  subtraction at all.

Also discovered, and filed as its own backlog item rather than fixed here
(out of scope): a string literal whose content is `"*"`/`"**"`/`"&"`, when
it appears as a non-first, comma-separated call argument, crashes the Ruby
frontend's PARSER with an internal panic rather than a graceful error —
confirmed independent of these new methods (a bare `foo(1, "*")`
reproduces it). Worked around in this PR's own tests by using `"-"` as the
example pad character instead of Ruby's usual `"*"`.

## 0.34.0 — `Numeric#round(ndigits)`, the multi-digit form

`round` previously only accepted the 0-arg form (Collections slice 9). This
widens the `_sir_builtin_method_v` dispatch arm to also accept a single
`ndigits` argument, matching real Ruby's full dispatch:

- **Integer, `ndigits >= 0`** — receiver unchanged (already exact at any
  decimal place an Integer could round to).
- **Integer, `ndigits < 0`** — rounds to the nearest `10^(-ndigits)`,
  half-away-from-zero (Ruby's tie rule), e.g. `1234.round(-2) == 1200`,
  `1250.round(-2) == 1300`.
- **Float, `ndigits > 0`** — rounds to `ndigits` decimal places, stays a
  Float, e.g. `3.14159.round(2) == 3.14`.
- **Float, `ndigits <= 0`** — rounds to the nearest `10^(-ndigits)` and
  CONVERTS to an Integer, e.g. `1234.5.round(-2) == 1200` (an Integer, not
  a Float) — matching real Ruby's actual return-type split, confirmed
  against a live `ruby -e` interpreter for every case in the test suite,
  not hand-derived.

Three overflow/UB hazards addressed, continuing this backend's established
saturate-rather-than-wrap-or-UB discipline:

- The `ndigits` argument itself is extracted through a new
  `_sir_round_ndigits_arg` rather than the generic `_sir_as_int`, whose
  bare `(int64_t)v.as.f` cast is UB for a non-finite/out-of-range Float
  argument (the same class of hazard `to_i` was fixed for in slice 9).
- The Integer negative-`ndigits` path computes the rounded magnitude in
  `uint64_t` and does an explicit saturating check before narrowing back
  to `int64_t` — a round-up carry can need ONE MORE digit than `int64_t`
  holds (e.g. `9223372036854775807.round(-1)` would need
  `9223372036854775810`), so this saturates at `INT64_MAX`/`INT64_MIN`
  rather than silently wrapping.
- Every path is capped ("dwarfs the value" for Integers past 19 decimal
  digits, "beyond `double`'s ~17 significant digits" for Floats) to a
  bound proven safe by construction, rather than depending on incidental
  floating-point behavior (e.g. `0 * Infinity == NaN`) for correctness.
- (Security review) The Float branch's negative-`ndigits` arm originally
  computed `int64_t k = -ndigits` directly — signed-overflow UB when
  `ndigits == INT64_MIN`, reachable because `_sir_round_ndigits_arg`
  saturates a hostile huge-negative Float ndigits argument to exactly that
  value (e.g. `3.14.round(-1.0e300)`). Fixed to reuse `_sir_i64_abs_u`,
  the same overflow-safe magnitude helper the Integer branch already used.

## 0.33.0 — fix: `puts` on an Array bracket-displayed instead of unpacking

Discovered by the `sir-conformance` cross-backend corpus (0.21.0): real
Ruby's `Kernel#puts` special-cases an Array argument — each element gets
its OWN line, RECURSIVELY flattening nested arrays, and an EMPTY array
prints nothing at all (not even a blank line). `_sir_puts_v` instead
routed every argument through the general `_sir_fmt` display path, which
bracket-displays a Seq (`"[1, 2, 3]\n"`) — the same rendering `print`,
`p`/inspect, and a NESTED array correctly use, just wrongly reused for
`puts`'s TOP-LEVEL arguments too. Python/JS/Go/Rust already unpacked
correctly; only C (and, discovered while fixing this, the Ruby backend —
see `semantic-ir-to-ruby` 0.20.0) had the bug.

Fixed with a new `_sir_puts_one` helper: unpacks a `SIR_SEQ` argument
recursively (each element re-dispatched through itself), falls through to
`_sir_fmt` + newline for everything else (`print`, `Hash`, scalars — all
unaffected). Shares `_sir_fmt`'s existing depth counter/cap
(`SIR_MAX_FMT_DEPTH`) so a self-referential array (`a[0] = a`) terminates
instead of recursing forever, the same safety floor every other display
path in this file already holds.

This is a genuine BEHAVIOR CHANGE for every `puts arr` call site across
the whole test suite — updated ~30 existing assertions across 9 test
files to the correct one-per-line (or, for a previously-bracketed nested
result like `divmod`'s `[q, r]` or `zip`'s array-of-pairs, fully
recursively flattened) output.

### Added

- `tests/compile_and_run_puts_array_unpack.rs` — 8 dedicated execution-proof
  tests: flat unpack, empty array (zero lines), recursive flatten across
  nested levels, a nested-empty-array contributing zero lines, `Hash` NOT
  unpacked, `print` NOT unpacked, a self-referential array terminating
  instead of hanging, and multiple Array arguments to one `puts` call each
  unpacking independently.

## 0.32.0 — Collections slice 10: Symbol + universal Object/Bool methods

New `_sir_builtin_method_v` dispatch arms:

- **Symbol** widens `to_s`/`length`/`size`/`empty?`/`upcase`/`downcase`/
  `to_sym` to accept a `SIR_SYM` receiver, REUSING the same String helpers
  slice 1/8 already built (a Symbol's name is stored the identical way a
  String's is). `upcase`/`downcase` are the one exception: they re-intern
  the result as a fresh SYMBOL, not a String (`:foo.upcase == :FOO`).
  `inspect` (`:name`) is Symbol-specific — no String equivalent.
- **Universal `Object` methods**: `nil?`, `equal?` (Ruby's object-IDENTITY
  comparison — pointer identity for every heap-boxed type, distinct from
  `==`'s structural/value equality, which this slice doesn't touch),
  `itself`, `frozen?` (a fixed, receiver-TYPE-only answer — this v0 runtime
  has no per-object mutability tracking, so it reports the Ruby-always-
  frozen primitives (`nil`/bool/Integer/Float/Symbol) as frozen and
  everything else as not).
- **`TrueClass`/`FalseClass`**: eager (non-short-circuit) `&`/`|`/`^`.
  Distinct from `&&`/`||`, which the frontend lowers to `If` and never
  reaches a method dispatch at all — these are only reached via an
  EXPLICIT dot-call (`true.&(x)`), and coerce their argument by Ruby
  truthiness (`0`/`""` are truthy, unlike Python), not by C's `int` rules.

`code/specs/sir-collection-methods.md`'s "C backend lane" table is updated;
slice 9 marked merged (#9713), this PR is slice 10. Deferred (documented,
tracked as follow-up, same discipline as slices 8/9): `respond_to?`
(needs a full reflective query across the user-method table AND the
entire `is_builtin_method` catalog), `dup`/`clone` (needs a generic copy
helper), generic `to_s`/`inspect`/`==`/`!=` on an arbitrary receiver
(needs `_sir_fmt`'s `FILE*`-writing display machinery refactored to build
a `SirValue` String instead), `freeze`/`tap`/`then`/`yield_self`
(low-value with no mutability-tracking or block-less Enumerator return in
this v0).

### Added

- `tests/compile_and_run_symbol_object_methods.rs` — 11 execution-proof
  tests (real `cc` compile+run): every new Symbol/Object/Bool method, the
  Symbol-vs-String return-type distinction for `upcase`/`downcase`, and
  `equal?`'s pointer-identity-vs-structural-equality distinction (two
  separately-built arrays with equal content are NOT `equal?`; the same
  array through an alias IS).
- `collection_symbol_slice10_methods_route_to_the_builtin_dispatcher` — an
  emit-shape test mirroring the prior slices' dispatcher-routing tests.

## 0.31.0 — Collections slice 9: Numeric methods

### Fixed (CI — Linux link failure)

- **Every emitted C program failed to LINK on Linux** with `undefined
  reference to 'floor'`/`'ceil'` (confirmed on `ubuntu-latest` CI; macOS and
  Windows were unaffected). This slice's `floor`/`ceil`/`round`/`abs` are
  the FIRST `<math.h>` function calls the embedded runtime template makes —
  and the template is pasted into every generated `.c` file regardless of
  whether the source program actually calls a Numeric method, so this broke
  every single test in the crate on Linux, not just this slice's own.
  glibc ships `libm` as a separate archive from `libc` (unlike macOS's
  libSystem, which folds it in), so linking requires an explicit `-lm`.
  Fixed by adding `-lm` to every test file's `cc` invocation (30 call sites
  across 26 files) and documenting the requirement in the README for real
  downstream consumers of the generated C.

New `_sir_builtin_method_v` dispatch arms for `Integer`/`Float`: `abs`,
`even?`/`odd?`/`pred` (Integer-only — true Ruby, unlike the looser dynamic
typing the Python/TS `sir-runtime-oop` reference uses), `zero?`/`positive?`/
`negative?`, `floor`/`ceil`/`round` (0-arg form only — the multi-digit
`round(ndigits)` form is a documented follow-up, deferred the same way
slice 8 deferred `ljust`/`rjust`/`center`), `divmod`, `fdiv`, `clamp`,
`between?`, `gcd`, `digits`, and the BLOCK-taking `times`/`upto`/`downto`/
`step`. `to_i`/`to_f` (previously String-only, slice 8) widen to accept a
numeric receiver via the existing `_sir_to_i`/`_sir_to_f` conversions.

Three points worth calling out:

- **`digits` needs no bignum-DoS cap**, unlike the Python reference: this
  runtime's `SirValue` integer is a fixed `int64_t`, never arbitrary
  precision, so the output is naturally bounded at 19 decimal digits — the
  reference's bit-length-cap guard has nothing to guard against here.
- **`divmod` raises a catchable `ZeroDivisionError`** on a zero divisor
  (the class was already registered in the exception hierarchy for
  `rescue`, just never raised from a division site) rather than the raw
  `exit(1)` the primitive `/`/`%` operators still fall back to — a
  deliberately nicer, more Ruby-faithful failure mode for this new call
  site, not a regression to those pre-existing operators. `fdiv` by
  contrast NEVER raises (Ruby's Float division doesn't), returning
  `Infinity`/`-Infinity`/`NaN` instead, matching the reference.
- **`step` with a zero stride is a documented no-op**, not a hang: a zero
  stride never crosses `limit` in a naive loop, so it is special-cased to
  zero iterations up front — the same DoS-safety floor slice 8's empty-
  pattern `sub`/`gsub` guard holds for string scanning.

### Security (found and fixed by the pre-push review, before ever merging)

- **`gcd`/`digits` negated a possibly-`INT64_MIN` value with a bare unary
  `-`.** `-INT64_MIN` is signed-overflow UB (no positive `int64_t` can hold
  its magnitude, 2^63) — verified to actually misbehave: the same code gave
  `gcd(INT64_MIN, 6) == -2` at `-O0` but `== 2` at `-O1`/`-O2`/`-Os`, and
  under one configuration the UB got compiled into a hardware trap
  (`SIGTRAP`) instead of returning at all. Fixed with a new
  `_sir_i64_abs_u` helper that computes the magnitude in `uint64_t` (whose
  well-defined wraparound covers `INT64_MIN` correctly), mirroring the
  `_sir_itdiv`/`_sir_itmod` `INT64_MIN`/`-1` guard this file already uses
  for the analogous hazard on `/`/`%`.
- **`floor`/`ceil`/`round` cast a `double` to `int64_t` with no range/NaN
  guard.** A `(int64_t)` cast on a non-finite or out-of-int64-range double
  is UB — verified platform-dependent (this machine's arm64 saturates;
  x86's `cvttsd2si` yields a different "integer indefinite" value for the
  same input). Fixed with `_sir_f64_to_i64_saturating`, which clamps to
  `INT64_MAX`/`INT64_MIN`/`0` before the cast, matching this runtime's
  other numeric conversions' never-raise floor (e.g. `_sir_mask_to`).

A second review round on the fixes above surfaced three MORE overflow bugs
in the same neighborhood (all fixed before merge, none ever reached `main`):

- **`gcd`'s own fix had a narrowing-overflow gap**: `0.gcd(x) == x.abs` in
  Ruby, and `|INT64_MIN|` is exactly `2^63` — one past `INT64_MAX`
  (`2^63-1`). Casting that magnitude back to `int64_t` silently wrapped to
  `INT64_MIN`. Since this runtime has no bignum to hold the true value, it
  now saturates to `INT64_MAX` instead — the same "true value doesn't fit,
  clamp rather than wrap" convention `_sir_f64_to_i64_saturating` uses.
- **`divmod` had TWO distinct overflow paths**, not the one round 1 fixed
  for `floor`/`ceil`/`round`: (1) `INT64_MIN / -1` is itself signed-overflow
  UB in C (the pre-existing `_sir_ifloordiv` this called into has no guard
  for it, unlike its siblings `_sir_itdiv`/`_sir_itmod`) — special-cased,
  since dividing by `-1` always divides evenly; (2) computing the floored
  remainder via `a - q * b` re-invites overflow through the back door even
  when the division itself is fine — `INT64_MIN.divmod(3)` floors to
  `q = -3074457345618258603`, and `q * b` ALONE overflows `int64_t` by 1,
  even though the true remainder (`1`) fits trivially. Fixed by adjusting
  the TRUNCATING remainder (`a % b`, then `+ b` if the sign needs fixing)
  directly instead of ever multiplying `q * b`.
- **`upto`/`downto`/`step` could increment their loop counter past int64
  range** on the LAST iteration, before the loop's own continuation check
  had a chance to stop it — e.g. `i++` in `for (; i <= n; i++)` is UB the
  instant `i == INT64_MAX` (`upto`) or `i == INT64_MIN` (`downto`), and
  `INT64_MAX.upto(INT64_MAX) { ... }` is exactly the one-iteration case that
  hits it. Fixed by applying the block FIRST, then checking "was that the
  boundary value" and breaking BEFORE ever advancing past it (`upto`/
  `downto`), and by checking `v + stride` for overflow before performing it
  in `step` (stopping the iteration there instead — there is no next
  in-range value to visit anyway).

A THIRD review round, prompted by round 2's finding that the SAME "bare
negation"/"bare float cast" patterns kept recurring, swept for every
remaining instance and found two more call sites that had gone unfixed
purely because earlier rounds fixed the pattern where they happened to be
looking, not everywhere the pattern appeared:

- **`abs`/`pred` had the identical `-INT64_MIN`/`INT64_MIN - 1` overflow**
  `gcd`/`digits`/`divmod` were already fixed for. `abs(INT64_MIN)` used to
  return a NEGATIVE number (the wrapped result of the UB negation); `pred`
  used to wrap to `INT64_MAX`, the opposite end of the range. Both now
  saturate (via the existing `_sir_i64_abs_u` for `abs`, an explicit
  boundary check for `pred`) instead of wrapping.
- **Widening `to_i` to accept a numeric receiver routed a Float through
  the pre-existing generic `_sir_to_i`**, whose cast is the identical UB
  `floor`/`ceil`/`round` were fixed for in round 1 — just not swept to this
  new call site. Now routes a Float receiver through
  `_sir_f64_to_i64_saturating` directly instead.

### Added

- `tests/compile_and_run_numeric_methods.rs` — 27 execution-proof tests
  (real `cc` compile+run): every new method, `divmod`'s catchable
  `ZeroDivisionError`, `fdiv`'s never-raises floor, the zero-stride `step`
  termination guard, and regressions for all seven overflow fixes above
  (`gcd`/`digits` on a runtime-constructed `INT64_MIN`, `floor`/`ceil`/
  `round` on `Infinity`/`-Infinity`/`NaN`/a huge finite float, `divmod` by
  `-1` and by `3` on `INT64_MIN`, `gcd` with a zero operand, `upto`/`downto`
  at the exact int64 boundary, `step` where the stride would cross
  `INT64_MAX`, `abs`/`pred` on `INT64_MIN`, `to_i` on `Infinity`/
  `-Infinity`/`NaN`). All seven were caught and fixed pre-merge across
  three security review rounds — none ever reached `main`.
- `collection_numeric_slice9_methods_route_to_the_builtin_dispatcher` — an
  emit-shape test mirroring the String/Array/Hash slices' dispatcher-routing
  tests.

### Notes

- Discovered (not fixed here, tracked separately): `puts (-5).abs` — a
  paren-less command call whose argument is a space-separated parenthesized
  expression immediately followed by a dot-chain — mis-parses as `puts(-5)`
  plus a dangling `.abs` that raises `NoMethodError`, instead of one
  statement. `puts((-5).abs)` (explicit call parens) sidesteps it and is
  used throughout the new tests. Same family as the already-fixed
  bare-comparison-statement and bracket-index frontend bugs.

## 0.30.0 — Collections slice 8: remaining String methods

New `_sir_builtin_method_v` dispatch arms (all guarded on `recv.tag ==
SIR_STR`, matching every prior slice's polymorphism discipline): `capitalize`,
`swapcase`, `strip`/`lstrip`/`rstrip`, `chomp` (no-arg and 1-arg literal-
separator forms), `chars`/`bytes`/`each_char` (block), `split` (no-arg
whitespace-run form and 1-arg literal-separator form), `replace`, `sub`/`gsub`
(literal, non-regex), `to_i`/`to_f`, `to_sym`, `tr`. Semantics are matched
against the Python/TS `sir-runtime-oop` reference catalog — this cascade's
cross-backend golden source — not always byte-for-byte true Ruby; see
`code/specs/sir-collection-methods.md`'s new "C backend lane" addendum for
the full slice cascade history and this slice's documented scope cuts
(char-set methods `count`/`delete`/`squeeze`, padding methods `ljust`/
`rjust`/`center`, and the `*`/`+` String operators — the last because Ruby
binary operators have no `__method__` lowering path at all yet, same
pre-existing gap as `<<` for `Array#push`).

Two safety properties worth calling out:

- **`chars`/`bytes`/`each_char` are genuinely distinct**, not the same byte
  loop under two names: `chars`/`each_char` are UTF-8-CHARACTER-aware (a
  multi-byte sequence is one element), `bytes` returns the raw byte values.
  A malformed/truncated UTF-8 lead byte falls back to a 1-byte step rather
  than over-reading past the string's NUL terminator.
- **`sub`/`gsub` treat an empty search pattern as a no-op** (the receiver
  comes back unchanged) rather than Python's convention of inserting the
  replacement between every character. A zero-length match would otherwise
  need explicit forward-progress handling to avoid an infinite scan; this
  keeps the helper provably terminating on any input without it — a
  DoS-safety floor consistent with this backend's other "never hang on
  adversarial input" guards (the flatten depth+budget cap, the
  `_sir_value_eq`/`_sir_fmt` cycle caps).

### Added

- `tests/compile_and_run_string_methods_slice8.rs` — 17 execution-proof
  tests (real `cc` compile+run): every new method, the UTF-8-vs-byte
  `chars`/`bytes` distinction on a multi-byte character, and the
  empty-pattern `sub`/`gsub` no-op guard.
- `collection_string_slice8_methods_route_to_the_builtin_dispatcher` — an
  emit-shape test mirroring slice 1/2's dispatcher-routing tests.

### Changed

- `builtin_method_dispatch_is_rejected` (a pre-existing test asserting an
  unsupported method name is cleanly rejected) used `strip` as its example —
  now supported by this slice, so it would have started passing for the
  wrong reason. Repointed to `ljust`, one of this slice's explicitly
  deferred methods.

## 0.29.0 — bracket-index read/write (`recv[k]` / `recv[k] = v`)

New dispatch arms for `"[]"` (read) and `"[]="` (write) in
`_sir_builtin_method_v`, reached via `ruby-to-semantic-ir` 0.8.0's new
`__method__("[]"/"[]=" ...)` lowering for Ruby's `recv[k]`/`recv[k] = v`
syntax (real method syntax: `recv.[](k)`/`recv.[]=(k, v)`). Both branch on
the RECEIVER's actual `SIR_SEQ`/`SIR_MAP` tag at runtime and delegate to
the existing `_sir_seq_index`/`_sir_map_get` and `_sir_seq_set`/
`_sir_map_set` helpers — no new runtime logic, just a new named entry point
into machinery every other Array/Hash method already goes through.

This replaces an earlier, REJECTED design where the Ruby frontend guessed
Array-vs-Hash from the INDEX's syntactic shape at compile time (a
string-literal key → Hash, anything else → Array). That heuristic mis-types
a Hash with a non-string key — `h[2] = "b"` on an int-keyed Hash routed to
`_sir_seq_set` regardless of `h`'s real type, which `exit()`s on a
non-sequence receiver. Dispatching on the receiver's ACTUAL tag here, at
runtime, can never mis-route: the index's type is irrelevant to which
helper runs. As with every other bracket-index frontend gap, this was only
reachable at all once `ruby-parser` grew a grammar rule for it — see that
crate's 0.7.0 CHANGELOG entry for the parse-side half of this fix.

### Added

- `tests/compile_and_run_index_bracket.rs` — 11 execution-proof tests: Array
  and Hash read/write, out-of-bounds/missing-key → `nil`, chained reads
  (`a[1][0]`), updating an existing Hash key in place, a cyclic write
  (`a[0] = a`), and — the case that motivated the runtime-dispatch design —
  Hash writes with integer and symbol keys, which the rejected heuristic
  would have crashed on.

## 0.28.1 — fix: `Array#sum` ignored a block argument

The 0-arg `sum` dispatch arm (slice 3) never checked `argc`/`args`, so
`arr.sum { |x| .. }` — Ruby's block form, which sums the block's
*transformed* values (`[1, 2].sum { |x| x * 2 }` == `6`) — silently summed
the raw elements instead, ignoring the block entirely. Same latent-shadowing
shape as the slice-3 `count` gap (fixed in slice 5), found while
implementing `Hash#sum`'s block form (slice 7), which correctly guarded on
`argc`/the closure tag from the start. Fixed with a new `_sir_array_sum_by`
helper (snapshotting `len`/`items` before its loop, like every other
block-taking helper here), dispatched only when `argc == 1` and the arg is
a closure; the 0-arg form is now itself gated on `argc == 0`.

## 0.28.0 — Collections slice 7: Hash block methods

No new `Feature`.

- New methods taking a trailing BLOCK argument, called with `(key, value)`
  (matching `Array#each_with_index`'s existing 2-arg calling precedent):
  `each_key`/`each_value` (yield one), `group_by` (a fresh Hash mapping each
  distinct block result to an Array of the `[k, v]` pairs that produced it,
  growing a group's array via the SAME `_sir_array_push_one` `Array#push`
  uses), `partition` (`[matching_pairs, non_matching_pairs]`, each a fresh
  Array). `each`/`map`/`select`/`reject`/`sort_by`/`sum` widen to accept a
  Hash receiver alongside their existing Array-only forms — `Hash#map`
  returns an Array (not a re-keyed Hash) and `Hash#sort_by` returns an Array
  of `[k, v]` pairs, both matching Ruby's `Enumerable` semantics; `select`/
  `reject` return a fresh Hash (unlike Array's, which return an Array).
- **Security, applied from the start this time**: every helper here
  snapshots BOTH `m->len` AND the `entries` pointer into locals once before
  its loop, exactly the discipline slice 4 had to retrofit (twice) for
  Array — since slice 6's `delete`/`clear` mutators already existed before
  these block helpers were added, there was no window where the bug could
  ship unguarded.
- **Security fix (found while implementing `Hash#delete`, and it exposed an
  ALREADY-MERGED bug)**: `delete` and `Array#shift` (slice 4) originally
  compacted their backing buffer IN PLACE (no reallocation) after removing
  an entry. That's unsafe against a block-taking helper's pointer snapshot:
  snapshotting a pointer is only safe against a mutator that REALLOCATES
  (like `push`) — the snapshot and the new buffer are then different memory,
  so the OLD one (still valid; this arena never frees) keeps reading exactly
  what it saw at snapshot time. An in-place compact instead mutates the SAME
  memory the snapshot points into, silently corrupting an in-flight outer
  iteration (elements shift under it — some read twice, some skipped).
  Both `delete` and `Array#shift` now reallocate a fresh, one-smaller buffer
  instead, closing the gap. Pinned by new tests in both this crate's Hash
  and (retroactively) Array test suites: `each` deleting/shifting from its
  own receiver mid-iteration now correctly sees every ORIGINAL element
  exactly once.
- **Security fix, round 2 (found by re-review of the round-1 fix)**: the
  reallocation `delete` gained above introduced a NEW bug — it resized
  `m->entries` without also updating `m->cap`, the field `_sir_map_put`'s
  `if (m->len == m->cap)` amortized-growth check relies on (`SirMap`, unlike
  `SirSeq`, tracks spare capacity). A stale, too-large `cap` after `delete`
  makes a LATER `put` skip growing and write one past the end of the freshly
  tightly-sized buffer — a genuine heap out-of-bounds write reachable by
  entirely ordinary code (`h.delete(k); h[new_key] = v`), not an edge case.
  `delete` now sets `m->cap = new_len` alongside `m->entries`/`m->len`.
  Pinned by a new hand-built-IR regression test (bracket-index assignment,
  `h[k] = v`, has no Ruby source syntax yet — a separate, pre-existing
  frontend gap, tracked on the backlog) that deletes then inserts twice,
  crossing the grow-triggering `len == cap` boundary on the second insert.

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection.

## 0.27.0 — Collections slice 6: Hash non-block methods

No new `Feature`.

- New methods: `keys`/`values` (fresh arrays in INSERTION order, matching how
  a map already prints), `fetch(k)` (like `h[k]` but RAISES `KeyError` out of
  range instead of nil — `Array#fetch` already raises `IndexError` the same
  way), `to_a` (a fresh array of `[k, v]` pairs; `to_a`/`fetch` both widen
  their existing Array-only arms to also accept a Hash receiver), `to_h`
  (identity, mirroring `Array#to_a`'s self-identity), `dig(k0, k1, ...)`
  (looks up `k0` then recurses into the result for each remaining key if it's
  itself a Hash or Array, else nil — lenient rather than Ruby's `TypeError`
  on a non-diggable intermediate; polymorphic over the STARTING receiver too,
  so `Array#dig` comes for free), `merge(other)` (a fresh map, `other`'s
  entries win on a shared key, never mutates the receiver), `invert` (a fresh
  map with keys/values swapped; a later duplicate value overwrites the
  earlier one, matching Ruby).
- `delete(k)`/`clear` are the FIRST Hash methods that mutate the receiver:
  `delete` removes an entry (shifting later entries down by one, mirroring
  `Array#shift`'s in-place style — no reallocation) and returns its value
  (nil if absent); `clear` resets `len` to 0 in place (the backing array is
  never freed, matching `Array#pop`) and returns the (now-empty) receiver.
  Both mutate the EXISTING `SirMap` box, like `MapSet` — every binding
  sharing the map sees the change.
- No block-taking Hash method exists yet (that's slice 7), so none of these
  helpers invoke a closure mid-loop — the length/mutation security discipline
  slice 4 established for Array's block helpers doesn't yet apply here, but
  slice 7 must apply the same len+entries-pointer snapshot pattern once it
  lands (a block-taking Hash helper running while `delete`/`clear` mutates
  the SAME map would face the identical class of bug slice 4 found and fixed).

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection.

## 0.26.0 — Collections slice 4: Array mutation + 1-arg query methods

`push`/`pop`/`shift` are the FIRST Array methods that mutate the receiver
after construction. No new `Feature`.

- `push(v1, v2, ...)` appends one or more values (each call reallocates a
  fresh buffer sized to the exact new length — no `cap` field added to the
  shared `SirSeq` struct, which would have required auditing every existing
  `_sir_alloc(sizeof(SirSeq))` call site for an amortized-growth win this v0
  runtime doesn't need); `pop`/`shift` remove and return the last/first
  element (`nil` on empty, matching Ruby), mutating `len` in place with no
  reallocation (`shift` also shifts the remaining elements down). All three
  mutate the EXISTING `SirSeq` box, like `SeqSet` — every binding sharing the
  array sees the change.
- New 1-arg query methods: `fetch(i)` (like `a[i]` but RAISES `IndexError`
  out of range, instead of returning nil), `values_at(i0, i1, ...)` (a fresh
  array of the elements at each index, nil-on-OOB per index), `rotate(n = 1)`
  (a fresh array shifted left by `n`, negative rotates right, never mutates
  the receiver), `zip(other1, other2, ...)` (a fresh array of arrays pairing
  elements positionally, padding a shorter `other` with nil). `include?` and
  `index` (both already allowlisted for String since slice 2) widen to accept
  an Array receiver too.
- **Security retrofit** (two rounds; see below): `push` is the first
  operation that can change `SirSeq.len`/`.items` AFTER a block-taking helper
  (slice 3/5's `map`/`select`/`reject`/`sort_by`/`each`/`any?`/`all?`/`none?`/
  `each_with_index`/`reduce`/`inject`, plus `count`'s block form) has already
  started iterating — e.g. `arr.map { |x| arr.push(x); x }`. Every such
  helper now snapshots BOTH `s->len` AND `s->items` into locals ONCE before
  its loop/allocation and uses only those locals — never `s->len`/`s->items`
  directly — for the output-buffer size and every element read, so a block
  that mutates the receiver mid-iteration can't run unbounded, read/write
  past a buffer sized at the OLD length, or read past a buffer `push`
  reallocated smaller than an outer snapshot (see below). Since this arena
  never frees, a snapshotted buffer stays valid indefinitely regardless of
  what `s->items` is reassigned to afterward — the same "iterate a snapshot"
  convention `_sir_seq_iter` (`ForEach`) already uses.

  Security review round 1 caught that snapshotting `s->len` ALONE (the
  initial draft) is insufficient in two ways, both now fixed:
  1. The `count` block-form arm (added in slice 5, before `push` existed)
     was missed by the retrofit entirely — a block that pushes to its own
     receiver inside `arr.count { |x| ... }` never terminated (unbounded
     loop + unbounded reallocation, a real DoS/OOM).
  2. `push` reallocates its new buffer sized to the CURRENT (live) `s->len`;
     if a block first shrinks the receiver (`pop`/`shift`, in place, no
     reallocation) and THEN pushes, the fresh buffer is smaller than a `len`
     an outer helper already snapshotted — continuing to read the LIVE
     `s->items` up to that stale, larger count then reads past the new,
     smaller allocation (a genuine heap out-of-bounds read). Fixed by
     snapshotting the ITEMS POINTER too, not just the length (see above).

  Pinned by four tests: `each`/`count` pushing to their own receiver
  terminate instead of looping forever; `map` pushing to its own receiver
  returns output reflecting only the original elements (not a heap
  overflow); and a receiver shrunk-then-regrown mid-`each` still yields the
  ORIGINAL snapshotted elements without over-reading.

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection.

**Out of scope:** `<<` (Ruby's shove/append operator) is NOT implemented
here — `ruby-to-semantic-ir` has no grammar rule for `<<` as a binary infix
operator at all yet (a separate, pre-existing frontend gap, found while
fixing the comparison-in-block-tail-position bug), so `a << x` can't reach
`__method__` to dispatch to `push` regardless. `arr.push(x)` (an ordinary
dot-call) is unaffected and fully supported.

## 0.25.0 — Collections slice 5: Array block methods (closure-calling)

The first Collections slice covering methods that take a trailing **block**
argument. No new `Feature`.

- The Ruby frontend already appends a block as the last `__method__` call arg
  (a `MakeClosure`, RB1), so it reaches this runtime as an ordinary
  `SIR_CLOSURE` value — no new calling convention needed. Every element-wise
  call goes through the EXISTING `_sir_apply` (the same dispatcher a
  first-class `Proc`/`sir_apply` call already uses).
- New Array methods: `each`, `map`, `select`, `reject`, `any?`, `all?`,
  `none?`, `sort_by`, `each_with_index`, `reduce`/`inject` (`argc==1`
  block-only, seeding the accumulator with the first element; `argc==2`
  `(initial, block)`). `sort_by` computes each key once (Schwartzian
  transform) rather than re-invoking the block per comparison.
- **Fix (found while wiring this slice):** slice 3's 0-arg `count` ignored
  `argc`/`args` entirely, so `arr.count { |x| .. }` (Ruby's block form, which
  counts only matching elements) silently returned the total length instead —
  wrong, not just unsupported. `count` now checks `argc` and dispatches to a
  real block-counting loop for the 1-arg closure form.
- Each block-taking arm requires the exact shape the frontend emits
  (`argc == 1` and a closure, except `reduce`/`inject`'s `argc` 1–2); anything
  else (missing block, extra args, a non-closure last arg) falls through to
  the existing `NoMethodError`, matching every other malformed-call case here.

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection. Calling the block itself
goes through `_sir_apply`, which only ever invokes a `SIR_CLOSURE`'s
compiler-emitted function pointer — never a name-based lookup.

## 0.24.0 — Collections slice 3: 0-arg Array query/transform methods

Third Collections slice — the first to cover **Array** (`SIR_SEQ`) receivers
beyond the polymorphic `length`/`size`/`empty?` from slice 1. No new `Feature`.

- New 0-arg methods on `is_builtin_method`/`_sir_builtin_method`: `count`,
  `first`, `last`, `sort`, `min`, `max`, `sum`, `uniq`, `compact`, `flatten`,
  `to_a`. `reverse` (already allowlisted for String in slice 1) gains a
  `SIR_SEQ` arm. `count` is polymorphic over Array/Hash, mirroring slice 1's
  `length`/`size`.
- Each returns a **fresh** sequence (or scalar) — the receiver's backing array
  is never mutated, unlike `SeqSet`. `sort`/`min`/`max` reuse `_sir_lt`/
  `_sir_gt` (the same comparators `<`/`>` use); `uniq` reuses `_sir_value_eq`
  (structural, so nested-array elements dedup correctly); `sum` reuses
  `_sir_plus_v`'s existing int/float promotion, defaulting the accumulator to
  `0` (Ruby's `Array#sum` default). `first`/`last`/`min`/`max` return `nil` on
  an empty array (matching Ruby); `sum` returns `0`.
- `flatten` recursively unwraps nested arrays fully (Ruby's default, no depth
  limit in the *language* semantics). It shares `SIR_MAX_EQ_DEPTH` — the same
  depth cap `_sir_value_eq`/`_sir_fmt` use — to bound recursion depth on a
  self-referential array (`a[0] = a`, constructible via `SeqSet`). Security
  review caught that depth ALONE is not enough here (unlike those two
  functions, `flatten` recurses into every element of every nested array, so
  a self-referential array whose elements ALL point back to itself — e.g.
  `a=[1,2]; a[0]=a; a[1]=a` — fans out ~branching^depth calls, which at a
  500-deep cap is astronomically more work, overflows the `int64_t` element
  count well before the cap, and would under-allocate the output buffer for
  the writes actually performed): both the count and fill passes now ALSO
  thread a shared total-work budget (`SIR_MAX_FLATTEN_NODES`, decremented once
  per node visited, leaf or container), so total calls across the whole
  traversal are bounded regardless of fan-out, not just depth. The two passes
  share the same starting budget and traversal order, so they still agree on
  the exact element count. Two-pass count-then-fill avoids a growable buffer.
- A wrong-type receiver (e.g. `"str".sort`) still raises `NoMethodError` via
  the existing fallthrough — no new guard needed, since every new arm only
  matches on `SIR_SEQ`.

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection.

## 0.23.1 — fix: cyclic-map/seq display + equality no longer overflow the stack on Windows

Fixes `cyclic_map_does_not_stack_overflow` (and the sibling cyclic-sequence
path): a self-referential aggregate (`h[0] = h`, constructible via the mutable
`MapSet`/`SeqSet`) is bounded by depth caps in both the display (`_sir_fmt`) and
equality (`_sir_value_eq_d`) recursions — but both caps were **5000**, and 5000
stack frames overrun Windows' 1 MB default stack (~875 KB on the display path)
**before** the cap can trip. So the guard that was supposed to prevent the
overflow was itself unreachable on Windows; the emitted program crashed with a
stack overflow instead of printing the `[...]` ellipsis / returning the
co-inductive `true`. (Linux/macOS give an 8 MB stack, so the same binary passed
there — the failure was Windows-only.)

- `SIR_MAX_FMT_DEPTH` and `SIR_MAX_EQ_DEPTH` lowered from `5000` to `500`, sized
  to fit the SMALLEST common C stack (Windows' 1 MB) with wide margin — ~90 KB on
  the display path, far under the limit — while staying far beyond any real
  (non-cyclic) nesting. No behaviour change for finite structures shallower than
  500; deeper-than-cap output is pathological (only a cycle reaches it) and the
  cap's observable effect (ellipsis / assumed-equal) is unchanged, just reached
  at a safe depth. No test or cross-backend conformance program pins the old value.

## 0.23.0 — Collections slice 2: 1-arg String query methods

Second Collections slice — the first **argument-taking** built-in methods. No new
`Feature`.

- `_sir_builtin_method` now **collects its varargs** (via `_sir_va_collect`, the
  same pattern as `_sir_plus`), split into a `_sir_builtin_method_v` impl over the
  already-collected args + a thin wrapper that frees them. This unblocks methods
  that read an argument.
- New String queries: `include?(sub)`, `start_with?(prefix)`, `end_with?(suffix)`
  → bool; `index(sub)` → the 0-based Int position or nil. Each guards
  `argc >= 1` and `args[0].tag == SIR_STR`, raising `NoMethodError` on a wrong
  receiver/argument type (matching Ruby) — never an out-of-bounds read.
- The emit already carried `argc` + the arguments through the `__method__`
  routing (slice 1), so only the `is_builtin_method` allowlist grows; the scan
  accepts the new names automatically.

**Anti-RCE preserved:** the method name is a compiler-emitted quoted C literal
used only as a `strcmp` target — never reflection.

## 0.22.0 — Collections slice 1: built-in String methods

The first slice of the **Collections** batch — the built-in method catalog every
OOP slice deferred (`"hi".length` used to be rejected as "the Collections
batch"). No new `Feature` (a built-in method is a `__method__` dispatch).

- A new runtime dispatcher `_sir_builtin_method(recv, "m", argc, …)` implements
  common 0-arity String methods — `length`/`size`, `upcase`, `downcase`,
  `reverse`, `empty?`, `to_s` — by an explicit `strcmp` switch on the method name
  plus a receiver-type check. `length`/`size`/`empty?` are **polymorphic** over
  String/Array/Hash; the rest are String-only. A wrong-type receiver raises
  `NoMethodError`, exactly as Ruby's dynamic dispatch would — never a crash.
- `__method__` emit now **routes** by name: a user-registered method (OOP) still
  goes through the class method table (`_sir_call_method`); a built-in name that
  the module did *not* define routes to `_sir_builtin_method`. The structural
  scan's allowlist widens to accept the built-in names, so `"str".upcase` etc.
  compile instead of being deferred. A built-in method **not** in this slice yet
  (e.g. `strip`) is still rejected cleanly.
- **Anti-RCE preserved.** The method name is a compiler-emitted quoted C literal
  and only ever a `strcmp` target — never reflection.

## 0.21.0 — OOP mirror slice 7: modules / mixins (final OOP slice)

Modules and mixins — the **final** slice of the C OOP mirror. Accepts
`Feature::Modules`. With it the C backend covers the full class/module surface,
giving **6-backend OOP parity** (C now joins Ruby/Go/Rust/Python/JS).

- `module M; def m; …; end; end` — a module's methods are registered exactly
  like a class's (`__def_method__`, keyed on the module NAME), so a mixin needs
  **no new method storage**. The `ModuleDef` declaration itself emits only a
  comment (a non-empty module body is rejected cleanly, as with a class).
- `include M` → `__include__("Class", "M")` → `_sir_register_include`, folding
  M's methods into the class's **instance**-method resolution; `extend M` →
  `__extend__` → `_sir_register_extend`, folding them into the class's **class**-
  method resolution. Both are recorded in `(class, module)` tables.
- `_sir_resolve_method` now checks, at each ancestor class, the class's own
  methods **then its included modules'** (most-recently-included first, matching
  Ruby precedence); `_sir_resolve_class_method` likewise consults **extended**
  modules. Both remain bounded by `SIR_ANCESTRY_MAX`.
- The `__class_method__` compile-time allowlist widens to the **union** of
  registered class methods and instance methods, since `extend` makes a module's
  instance method a valid class-method dispatch target.

**Anti-RCE.** Class and module names emit as **quoted C string literals** used
only as table keys (no injection); dispatch stays an explicit data lookup, never
reflection. This closes the C OOP arc (slices 1–7).

## 0.20.0 — OOP mirror slice 6: class variables (`@@x`)

Class variables — the sixth slice of the C OOP mirror. Accepts
`Feature::ClassVars`.

- A class variable belongs to a **class** and is shared **down its hierarchy**
  (a `@@x` defined in a parent is the same storage in every subclass). Storage
  is a flat `(class, @@name) → value` table with an ancestry-resolved owner
  (`_sir_cvar_owner` walks `_sir_class_super`, bounded by `SIR_ANCESTRY_MAX`), so
  a subclass method shares its parent's `@@x`.
- A **method body**'s `@@x` read/write → `_sir_cvar_get` / `_sir_cvar_set`, which
  resolve the owning class from a new `_sir_current_class` — bound by dispatch to
  the receiver's class (`_sir_call_method`) or the dispatched class
  (`_sir_call_class_method`), and restored after (so it composes with `super`
  and nested calls).
- A **class-body** initializer (`@@x = 0` inside `class C`) runs where `self` is
  the top-level `main`, so it names its class **explicitly**:
  `_sir_cvar_set_in("C", "@@x", …)`. The `ClassDef` emit now admits a body of
  **only** such `@@x` initializers; any other class-level statement is still
  rejected cleanly (it would otherwise be silently dropped).
- `emit_var_ref` is now exhaustive over `Scope` (all eight variants have a real
  emit path), so its catch-all `unreachable!` is removed — the exhaustive match
  is the compile-time totality signal.

**Anti-RCE.** The `@@`-name and class name emit as **quoted C string literals**
used only as table keys (no injection). Modules (mixins) are the final slice
(still rejected cleanly).

## 0.19.0 — OOP mirror slice 5: class methods

Class (singleton) methods — the fifth slice of the C OOP mirror. No new
`Feature` (class methods lower to builtins).

- `def self.m` → a hoisted function + `__def_class_method__("C", "m",
  MakeClosure(fn))` → `_sir_def_class_method`, which registers the closure in a
  **separate** `(class, method) → closure` table from instance methods. A class
  method `m` and an instance method `m` on the same class therefore never
  collide (both are legal and distinct in Ruby).
- `Class.m(args…)` → `__class_method__("C", "m", args…)` →
  `_sir_call_class_method`, an explicit table lookup — never reflection — that
  **walks the ancestry** (`_sir_class_super`), so a subclass inherits its
  parent's class methods (`class A; def self.m; end; end; class B < A; end; B.m`).
- A class method has no instance receiver, so `_sir_current_self` is bound to
  **nil** for its body (and restored after) — a class method called from inside
  an instance method never sees the caller's `self`.

**Anti-RCE / totality.** Class and method names emit as **quoted C string
literals** used only as table keys (no injection). A `__class_method__` dispatch
to a name the module never registers via `__def_class_method__` is a built-in
class method (`Foo.name`, the Collections batch) and is rejected cleanly via a
`DEFINED_CLASS_METHODS` allowlist (collected in the same walk as the instance-
method allowlist); a malformed registration/dispatch, or one carrying a
control-flow argument, is likewise rejected rather than mis-emitted. `@@class`
variables and modules remain the last two slices (still rejected cleanly).

## 0.18.0 — `fmt_float`: C-printf-faithful float formatting

One builtin, mirroring the Ruby backend, for the C frontend's faithful `printf`
(SIR27 milestone 10).

- `fmt_float(value, precision, kind)` → `_sir_fmt_float_c`, which renders a
  `double` with `snprintf` for the conversion `kind` (`'f'`/`'F'`/`'e'`/`'E'`/
  `'g'`/`'G'`) and precision. The format string is chosen by a `switch` over the
  fixed `kind` character — never built from source text, so there is no
  format-string vulnerability. The output is measured first (`snprintf(NULL, 0,
  …)`) then arena-allocated to the exact size, so any precision fits.

Compiles clean on clang + gcc + MSVC; matches reference C and emitted Ruby
byte-for-byte on the faithful-`printf` corpus.

## 0.17.1 — fix: `raise ClassName, "msg"` constructs the exception

Fixes a cross-backend conformance failure (`exception_reflection` / `puts(e)`): a
`raise ArgumentError, "boom"` lowers to `BuiltinCall("raise", [VarRef(Const
"ArgumentError"), StrLit("boom")])`, but the C `raise` emitter used only the first
argument and let the `Const` fall through to `emit_var_ref` as
`_sir_const_get("ArgumentError")` — and the C runtime registers no builtin
exception-class CONSTANTS, so that raised `NameError: uninitialized constant
ArgumentError` (crashing the program) while dropping the `"boom"` message.

- The `raise` emitter now intercepts a `Const` first argument as a CLASS NAME and
  constructs the exception directly: `_sir_raise(_sir_error("ArgumentError",
  <msg or nil>))` (nil message for a bare `raise Foo`, whose `#message` then
  defaults to the class name). Mirrors the Go/Rust/JS/Python backends. Any other
  first argument (`raise "boom"`, `raise some_exc`) keeps the value path
  (`_sir_raise_value`).
- Handled on BOTH emit paths: the simple inline path and the compound path
  (`emit_compound_call`) taken when the message is a non-simple expression — so a
  computed message (`raise ArgumentError, cond && "x"`) does not regress to the
  same `uninitialized constant` failure.
- The class name stays a QUOTED C string literal (no injection); rescue matching
  and `puts(e)` display are unchanged (they already read the exception's class /
  message). Existing `raise "string"` behaviour is untouched.
- Regression tests: `raise_named_class_with_message_is_caught_and_prints_the_message`,
  `raise_bare_named_class_defaults_its_message_to_the_class_name`, and
  `raise_named_class_with_a_compound_message_still_constructs_the_exception` (the
  prior exception tests only raised bare string messages, so the class-name path
  was never exercised).

## 0.17.0 — numeric conversions: `to_f` / `to_i`

Two numeric-conversion builtins mirroring the Ruby backend, for the C frontend's
floating-point value track (SIR27 milestone 9b).

- `to_f` → `_sir_to_f` = `_sir_float(_sir_as_num(v))` (numeric → double).
- `to_i` → `_sir_to_i` = `_sir_int(_sir_as_int(v))` (double → int, **truncating
  toward zero** like C's `(int)double`; the frontend then narrows to the target
  width with a `Convert`, i.e. `_sir_iN`/`_sir_uN`).

Float arithmetic itself needs no new code: `_sir_plus/minus/times/divide_v` and
the comparison helpers already promote to `double` when any operand is a
`SIR_FLOAT` (so `_sir_divide_v` does true division), and `Feature::Floats` /
`Expr::FloatLit` were already supported.  The emitted C compiles clean on
clang + gcc + MSVC and matches the reference / emitted-Ruby legs byte-for-byte.

## 0.16.0 — OOP mirror slice 4: inheritance + `super`

Class inheritance and `super` — the fourth slice of the C OOP mirror. No new
`Feature` (a superclass is a `ClassDef` field; `super` is a builtin).

- `class Dog < Animal` (a `ClassDef` with a `superclass`) emits
  `_sir_register_super("Dog", "Animal")`, recording the `sub → super` edge in a
  mutable user-ancestry table.
- **One ancestry, two consumers.** `_sir_class_super` now consults that user
  table **first**, falling back to the baked-in exception hierarchy — so the same
  `super_of` relation drives BOTH `rescue`-by-class matching (a user class that
  subclasses `StandardError` is caught) AND OOP method resolution.
- **Inherited dispatch.** `_sir_call_method` resolves a method by walking the
  ancestry (`_sir_resolve_method`: look up on the class, else climb `super`),
  so a subclass that doesn't define a method inherits the parent's closure.
- `super` (`__super__(method, definingClass, …args)`) → `_sir_call_super`, which
  resolves `method` from the **superclass of the defining class** (so it doesn't
  re-enter the override) and applies it to the **current** receiver — `super`
  does not rebind `self`, so `@x` and nested calls still see the original object.
  No ancestor defines it ⇒ a (rescuable) `NoMethodError`.
- **DoS guard.** Every ancestry walk (`_sir_class_is_a`, `_sir_resolve_method`)
  is bounded by `SIR_ANCESTRY_MAX` steps, so a hand-built cyclic hierarchy
  (`A<B`, `B<A` — which the Ruby frontend never emits) resolves to a clean "not
  found" instead of looping.

**Anti-RCE by construction.** Class / method / defining-class names emit as
**quoted C string literals** used only as table keys and `strcmp` targets —
never as C source — so no name can inject code. Class methods, `@@class` vars,
and modules remain the next slices (still rejected cleanly).

## 0.15.0 — OOP mirror slice 3: instance variables (`@x`) + `self`

Instance state — the third slice of the C OOP mirror. Accepts
`Feature::InstanceVars`, so a method body can now read and write the receiver's
instance variables and refer to the receiver directly.

- `@v = x` (a `Scope::Instance` `Assign`) → `_sir_ivar_set("@v", x)`, and `@v`
  (a `Scope::Instance` `VarRef`) → `_sir_ivar_get("@v")`. Each instance carries a
  lazily-allocated `@name → value` map (`struct SirInstance` gains an `ivars`
  slot, `NULL` until the first write); an unset `@v` reads **nil**, matching Ruby.
- A bare `self` (`__self__`) → `_sir_self()`, the current receiver — so a method
  can return `self` for chaining (`w.me.size`).
- **How a hoisted method body finds its receiver.** A method lowers to a
  top-level function with no lexical `self`, so dispatch carries the receiver in a
  process-global `_sir_current_self`: `_sir_call_method` saves the caller's
  `self`, binds it to the receiver for the call, and restores it after (nested
  calls stack correctly through these C-local saves). `@x`/`self` read that
  global; the top-level `main` object gets its own ivar bag (`_sir_toplevel_ivars`).
- **Exceptions interaction.** A method that `raise`s inside a `begin` `longjmp`s
  past `_sir_call_method`'s own restore, so an enclosing `TryCatch` snapshots
  `_sir_current_self` at the `begin` and restores it on the rescue/ensure/escape
  paths — so `@x` in a rescue body reads the *catcher's* ivars, not the raiser's.

**Anti-RCE by construction.** The `@`-name (including the leading `@`) emits as a
**quoted C string literal** and is used only as an interned map key — never as C
source — so no `@`-name can inject code, exactly as with class/method/rescue
names. `@@class` variables (`Feature::ClassVars`), inheritance/`super`, class
methods, and modules remain the next slices (still rejected cleanly).

## 0.14.0 — OOP mirror slice 2: instance methods

Instance-method definition and dispatch — the second slice of the C OOP mirror.

- `__def_method__("Class", "m", MakeClosure(fn))` registers a method: it inserts
  the closure into an explicit `(class, method) → closure` table
  (`_sir_def_method`, keyed on the interned class + method).
- `__method__(recv, "m", args…)` dispatches: `_sir_call_method` resolves
  `(recv's class, "m")` in the table and applies the closure to the args; a
  non-instance receiver or an unresolved method is a (rescuable) `NoMethodError`.

**Anti-RCE by construction.** Dispatch is an **explicit data lookup** on the
`(class, method)` key — never reflection on a source-derived string (the SIR24
§Security invariant). A user method literally named `system`/`eval` is only ever
a table KEY; an unknown method is a controlled `NoMethodError`, never a jump.
(Class/method names emit as quoted C string literals, so there is no injection
surface either.)

**Totality / clean rejection.** A `__method__` dispatch to a name the module
never registers via `__def_method__` is a **built-in method call** (`.length`,
`.upcase`, … — the separate Collections batch) and is rejected cleanly, not
compiled to a runtime `NoMethodError`: a first pass collects the registered
method names (a thread-local allowlist), and the scan validates each dispatch. A
malformed `__def_method__` (not `[StrLit, StrLit, MakeClosure]`) or `__method__`,
and a `__def_method__`/`__method__` with a control-flow argument (which the
compound emit path cannot render), are also rejected. `self`/`@ivars` are the
next slice, so method bodies here don't yet reference the instance.

## 0.13.0 — OOP mirror slice 1: instance runtime + empty class + constants

Accepts `Feature::Classes` + `Feature::Constants` — the first slice of the C
backend's OOP mirror (the Ruby backend just finished the full 7-slice arc). This
slice is the **instance-runtime foundation**:

- A new `SIR_INSTANCE` value tag + `struct SirInstance { const char *sir_class; }`
  stored **inline in the `SirValue` union** — unlike the Go/Rust backends (which
  hold an integer id into a side-table because their value type is `Copy`), the
  C pointer IS the handle, so pointer-identity is object identity (no id table).
- `class Foo; end` → `Stmt::ClassDef` → a comment: a class is just a NAME in the
  C runtime (an instance carries its class string; there is no class object).
- `Foo.new` → `_sir_new_instance("Foo")`, printing `#<Foo>` (deterministic — no
  address, so tests can assert on it). `_sir_value_eq` gains a `SIR_INSTANCE` arm
  (pointer identity, Ruby's default `==` on an object).
- **Constants** ride in (entangled: the frontend records `Constants` for any
  `Foo.new`, since the receiver is a constant). `PI = 3` / `PI` →
  `_sir_const_set` / `_sir_const_get` over a tiny runtime name→value table; an
  undefined constant raises a rescuable `NameError`. Class/constant names are
  emitted as **quoted C string literals** (no injection, as with rescue types).

**Deferred, rejected cleanly** (each a later slice): `__new__` with constructor
arguments (needs `initialize`), a `class << self` singleton, the OOP method
builtins (`__def_method__` / `__method__` / …), and — via their still-unaccepted
features — `@ivars`, `@@class vars`, method-resolving inheritance, and modules.

## 0.12.0 — exceptions (SIR17)

Accepts `Feature::Exceptions`. C has no stack unwinding, so `begin … rescue …
ensure … end` (`Stmt::TryCatch`) and `raise` lower to a **`setjmp`/`longjmp`
handler stack** — the C analogue of Go `panic`/`recover`, per the SIR24
exception-model design.

- **Runtime**: a new `SIR_ERROR` value (`struct SirError { const char *sir_class;
  SirValue msg; }`); a static stack of `jmp_buf` (`_sir_push_handler`/`_sir_pop_
  handler`); `_sir_current_error` (the exception being handled); `_sir_raise`
  (records the error and `longjmp`s to the top handler, or prints `class:
  message` to stderr and exits non-zero when uncaught); `_sir_raise_value`
  (re-raises an exception object, or wraps any other value — a message string —
  in a `RuntimeError`); and a **baked-in exception-class ancestry table**
  (`RuntimeError`/`ZeroDivisionError`/… → `StandardError` → `Exception`,
  `KeyError` → `IndexError`, `NoMethodError` → `NameError`) with
  `_sir_class_is_a` / `_sir_rescue_matches` so `rescue StandardError` catches a
  raised `RuntimeError`. A single `#include <setjmp.h>` is added to the preamble.
- **`TryCatch` codegen**: a TWO-handler structure — an OUTER "ensure" handler
  wraps the whole thing so `ensure` runs even when a rescue body itself raises
  (Ruby semantics), and an INNER "body" handler catches an exception from the
  guarded body. The inner handler is popped BEFORE the rescue dispatch, so a
  raise in a rescue clause (or an unmatched exception) unwinds to the outer
  handler; the outer handler is popped before `ensure` runs, and an unmatched
  exception is re-raised (propagated) after `ensure`.
- **`raise`**: bare (`raise`) re-raises `_sir_current_error`; `raise "msg"`
  raises a `RuntimeError`; `raise <exception>` re-raises it.

**Injection safety**: a `rescue` clause's exception-type names are emitted as
**quoted string literals** (`quote_c_string`) passed to `_sir_rescue_matches` —
never as bare identifiers — so no rescue type can inject source, and the SIR24
"dispatch is an explicit name-switch" anti-RCE invariant holds. The
unsupported-builtin pre-check descends into a `TryCatch`'s guarded/rescue/ensure
bodies (co-total with the emitter).

Deferred to a follow-up (each a clean rejection): `raise SomeClass` (a specific
class) lowers to a `Const` reference → observes `Feature::Constants`
(unaccepted) → rejected; `retry` is not yet lowered (rejected by the builtin
gate — it needs loop machinery in the `setjmp` model).

Documented v0 limitation (correctness, not memory-safety): a *bare* `raise`
(re-raise) inside a rescue body reads the global current-error, so if a nested
`begin/rescue` completes between the clause's entry and the bare `raise`, it
re-raises that inner (already-handled) exception rather than the clause's own —
faithful `$!` save/restore around nested handling is deferred. (An `ensure` body
that handles a nested exception does NOT mis-propagate — the escaping exception
is snapshotted before `ensure` runs; regression-tested.)

First of the exceptions parity arc's C half: with the Ruby backend (0.10.0),
`Exceptions` is now accepted on all six backends. Verified with hand-built
modules compiled and run through a real `cc`: a bare rescue catching a message,
`rescue StandardError` matching a `RuntimeError` via the ancestry, the rescue
binding, `ensure` on both the normal and the exception path, an unmatched
exception propagating through an outer handler after the inner `ensure` runs,
and an uncaught exception exiting non-zero. Bumps semantic-ir-to-c 0.11.0 →
0.12.0.

## 0.11.0 — keyword parameters (SIR19)

Accepts `Feature::KeywordParams`, building directly on the `_sir_missing`
default-parameter machinery (0.10.0). C has no native keyword calls, so — like
the Go backend's KW6 — a keyword argument is resolved to its callee's parameter
**slot by name at emit time**, producing a plain positional C call:

- A **keyword parameter** needs NO special signature — it is a positional
  `SirValue` C parameter like any other. Only the call site resolves by name.
- A `DirectCall` carrying any `KeywordArg` routes to a dedicated resolver
  (`emit_keyword_call`) instead of the generic left-to-right hoist. For each
  callee slot, in declared order, the filler is: the leading positional argument
  at that index; else the `KeywordArg` naming that parameter; else
  `_sir_missing()` (an omitted optional — the validator guarantees a required
  keyword is never left out, and the same default prologue as `DefaultParams`
  substitutes the default).
- The thread-local signature map — previously just a per-callee arity for
  default padding — now stores each callee's **parameter names** in order, so
  the resolver can place a keyword argument at its slot. (Renamed `ARITY` →
  `SIGNATURES`; `callee_arity` derives the length, `callee_param_names` the
  names. Still read only by key, so emission stays deterministic.)
- Each filler is hoisted into a temp first (matching the statement-oriented
  emitter), so a compound keyword value (`f(b: g(), a: 10)`) is evaluated
  exactly once; the temps are computed in slot order, matching Go's
  declared-order evaluation. The unsupported-builtin pre-check scans a keyword
  argument's value.

Because a `KeywordArg` argument is non-`is_simple`, a keyword-bearing call is
always compound → routed through `emit_keyword_call`, so a `KeywordArg` node
never reaches the generic arg emit or `emit_expr` (where it has no arm). A
`KeywordArg` outside a call is rejected by the validator before emit.

First of the KeywordParams parity arc's C half: with the Ruby backend (0.9.0),
`KeywordParams` is now accepted on five of six backends (the Rust backend is a
separate gap). Verified with hand-built modules compiled and run through a real
`cc`: a keyword argument binding by name, order-independent resolution (`f(b: 2,
a: 10)` → `8` for `f(a:, b:) = a - b`), an optional keyword using its default
when omitted (`f()` → `7` for `f(x: 7)`) and overridden when supplied, a mixed
positional + keyword call, and a compound keyword value hoisted once. Bumps
semantic-ir-to-c 0.10.0 → 0.11.0.

## 0.10.0 — default parameters (SIR19)

Accepts `Feature::DefaultParams`. C has no native default parameters, so — like
the Go backend — this uses a `_sir_missing` sentinel with call-site padding and
a per-function prologue:

- **Runtime**: a new `SIR_MISSING` tag with `_sir_missing()` / `_sir_is_missing`.
  It is an INTERNAL "argument omitted" sentinel — a `SIR_MISSING` value is
  replaced by its default before the body runs, so user code never observes it.
- **Call site**: a `DirectCall` that leaves trailing defaulted arguments off
  pads the call with `_sir_missing()` up to the callee's declared arity. The
  arity is looked up in a thread-local map (`ARITY`) snapshotted at the top of
  `emit_module` — the same mechanism as the `TEMP_ID` counter, so the deep
  `emit_expr`/`emit_assign` call tree reads it without threading a context.
  (The map is only read by key, so emission stays deterministic.)
- **Prologue**: each function opens with `if (_sir_is_missing(p)) { p =
  <default>; }` for every defaulted parameter, in declaration order — so a later
  default may reference an earlier parameter (whose own default is already
  filled), matching the validator and the Go/Ruby backends. A C parameter is a
  mutable lvalue, so it is reassigned in place; a compound default hoists
  through `emit_assign`.

Only the positional case is `DefaultParams`; a keyword default is the separate
(still-unaccepted) `KeywordParams` feature. An `IndirectCall` (a closure with no
statically-known signature) is not padded — the closure's own arity handling
applies; the DirectCall path is the default-parameter path.

Also extends `first_unsupported_builtin` to scan each parameter default, not
just the body — a default is evaluated (in the prologue) at call time, so a
deferred builtin hidden in one must be rejected cleanly rather than reach the
emitter's `unreachable!`. (The C `scan_expr_for_builtin` already scanned an
`IndirectCall`'s target, so — unlike the Ruby backend — no target-scan fix was
needed here.)

This closes the DefaultParams parity arc: with the Ruby backend's default
parameters (0.8.0), `Feature::DefaultParams` is now accepted on all six
backends. Verified with hand-built modules compiled and run through a real `cc`:
a single default used when omitted (`f(1)` → `6` for `f(a, b = 5) = a + b`) and
overridden when supplied (`f(1, 2)` → `3`), two trailing defaults each filling
independently, a default referencing an earlier parameter, and the prologue /
call-site sentinel shape. Bumps semantic-ir-to-c 0.9.0 → 0.10.0.

## 0.9.0 — short-circuit (SIR16)

Accepts `Feature::ShortCircuit`. `Expr::LogicalAnd` / `Expr::LogicalOr`
(`&&` / `||`) reuse the SAME lowering the emitter already applies to the eager
`and`/`or` builtins — no new machinery:

- assign the LEFT operand into the destination, then conditionally OVERWRITE it
  with the right (`dst = lhs; if (_sir_truthy(dst)) { dst = rhs; }` for `&&`,
  `if (!_sir_truthy(dst))` for `||`).

Because the right operand is emitted only inside the `if` body, it is not
evaluated when the left already decides (true short-circuit), and `dst` holds
the DECIDING OPERAND — not a coerced bool. This is the value-returning semantics
Go models with an IIFE and Ruby gets from native `&&`/`||`: `1 && 2` is `2`,
`false && 2` is `false`, `nil || 7` is `7`. It is deliberately NOT lowered to a
bare C `&&`/`||`, which would collapse to an `int` 0/1 and lose the operand.

The nodes are not `is_simple`, so they route through `emit_assign` — and, in
return position, through the existing "compute a compound value into a temp,
then return it" tail fallback — so no other emit arm is needed and the emitter
stays total. The `scan_expr_for_builtin` pre-check recurses into both operands,
so a deferred builtin nested in a `&&`/`||` is still reported cleanly.

This closes the ShortCircuit parity arc: with the Ruby backend's short-circuit
(0.7.0), `Feature::ShortCircuit` is now accepted on all six backends. Verified
with hand-built modules (the frontend constant-folds a literal `&&`) compiled
and run through a real `cc`: operand-return for both operators, a short-circuit
proof where the dead operand is `1 / 0` (which traps if evaluated — a correct
lowering skips it and the program exits 0), and a `LogicalAnd` in tail position.
Bumps semantic-ir-to-c 0.8.0 → 0.9.0.

## 0.8.0 — floats (SIR16)

Accepts `Feature::Floats`. Unlike the sequences and maps batches, this needed
**no new runtime**: `SirValue` has carried a `SIR_FLOAT` tag since v0, and the
runtime already handled floats throughout — `_sir_float` constructor,
`_sir_is_num`/`_sir_as_num`, int→float promotion in `_sir_plus_v`/`_sir_minus_v`/
`_sir_times_v`, an IEEE float path in `_sir_divide_v`, and `_sir_fmt_float`. The
one missing piece was the emitter: a `FloatLit` had no arm and hit
`unreachable!`. `Feature::Floats` gates ONLY `FloatLit`, so this batch is a
single emit arm plus accepting the feature — the emitter stays total.

- `Expr::FloatLit` → `_sir_float(<literal>)` via a new `emit_float_literal`:
  - a **finite** value is spelled with Rust's `{:?}` (Debug) form, whose
    shortest round-tripping text always carries a decimal point or exponent
    (`7.0`, `-0.0`, `1e300`) — a valid C `double` literal that `strtod` parses
    back to the identical bit pattern;
  - a **non-finite** value (which a literal can only carry when hand-built —
    normal arithmetic produces `inf`/`nan` at runtime) uses the C99 `<math.h>`
    macros `INFINITY` / `-INFINITY` / `NAN`, mirroring the Ruby backend's
    `Float::INFINITY` / `Float::NAN`. A single `#include <math.h>` is added to
    the emitted preamble for these (standard, available on every C99 compiler
    including MSVC).

Float arithmetic reuses the existing `+`/`-`/`*`/`/` variadic helpers: an
integral result of a float operation stays a Float (`1.5 + 2.5 == 4.0`, not
`4`), and the division frontier is preserved — a Float operand promotes to true
division (`7.0 / 2 == 3.5`) while two Integers floor (`7 / 2 == 3`); Float
division by zero yields IEEE `Infinity`/`NaN` (no trap — that is Integer-only).
`_sir_fmt_float` renders integral floats with a trailing `.0`, `-0.0` with its
sign, and non-finite values as `Infinity`/`-Infinity`/`NaN`.

This closes the Floats parity arc: with the Ruby backend's floats (0.6.0),
`Feature::Floats` is now accepted on all six backends. Verified with hand-built
modules (the frontend masks `FloatLit`) compiled and run through a real `cc`:
literal display incl. `-0.0`, native arithmetic staying Float, the division
frontier, non-finite results AND non-finite literals, and value-based equality
(`7.0 == 7`). Bumps semantic-ir-to-c 0.7.0 → 0.8.0.

## 0.7.0 — maps (SIR16)

Accepts `Feature::Maps`. `SirValue` gains a `SIR_MAP` tag — a heap-boxed,
insertion-ordered **assoc-array** (`struct SirMap { struct SirMapEntry
*entries; int64_t len; int64_t cap; }`, arena allocated), a shared mutable
handle exactly like `SIR_SEQ`. It is a linear-scan assoc-array, NOT a hash
table — the same representation as the Go (`[]MapEntry`) and Rust
(`Vec<(Value, Value)>`) reference backends: lookups are O(n), but structural
keys and insertion-ordered iteration/printing come for free, with no `Hash`/`Eq`
requirement on the value type. Every construct the feature can surface is
lowered:

- `MapLit` (`{k => v, …}`) → `_sir_map_lit(n, k0, v0, …)`, boxing `n` key/value
  pairs. A later duplicate key overwrites the earlier entry (`{1 => 1, 1 => 2}`
  is `{1 => 2}`), matching Ruby's Hash literal and the Go/Rust `_sir_map_lit`.
- `MapGet` (`h[k]`) → `_sir_map_get`: a missing key yields nil (it does NOT
  raise — matching Ruby's default-less `Hash#[]` and the reference); keys are
  compared by STRUCTURAL equality, so a composite key like `[1, 2]` matches by
  value.
- `MapSet` (`h[k] = v`) → `_sir_map_set`: insert-or-update, mutating the shared
  box so a write through one binding is visible through every alias. A map has
  no bounds, so — unlike `SeqSet` — there is nothing to trap on; a new key
  APPENDS (growing the backing array, capacity doubling from 4), preserving
  insertion order.

`_sir_value_eq` gains a `SIR_MAP` arm: STRUCTURAL and POSITIONAL — equal length,
then entry-wise in insertion order (`entries[i]` key AND value equal) — exactly
mirroring the Go (`[]MapEntry` zip) and Rust (`iter().zip()`) backends, with an
identical-handle fast path. `_sir_fmt` renders a map as `{k: v, k2: v2}` (brace,
colon-space, insertion order), also matching Go/Rust.

**Documented family-wide divergence from real Ruby (unchanged by this batch):**
Ruby's own `Hash#==` is order-INsensitive and its `Hash#inspect` uses ` => ` for
non-symbol keys (and `key:` only for symbol keys). All three source-emitting
backends (Go, Rust, and now C) are instead positional and print a uniform `: ` —
so the three **agree with each other**, which is the property the cross-backend
conformance corpus checks (no corpus program prints or reorder-compares a whole
map, so the real-Ruby form is unexercised). Aligning all three to Ruby's exact
`Hash` semantics is a separate, family-wide change.

Because `MapSet` mutates in place, a self-referential map (`m[k] = m`) is now
constructible; both the `value_eq` and `fmt` `SIR_MAP` arms reuse the
recursion-depth caps introduced for `SeqSet` in 0.6.0, so a cyclic map
terminates rather than overflowing the C stack (verified adversarially).

`ForEach` over a map is deliberately NOT special-cased: iterating a map is
reference-undefined (Go's `_sir_seq_iter` panics on a non-sequence), and C's
lenient `_sir_seq_iter` else-branch already treats a non-seq/non-cons iterable
as an empty iteration — so the loop body runs zero times and the emitter stays
total (no new `unreachable!`), consistent with its pre-existing handling of any
other non-iterable.

Every node verified by hand-built modules (bypassing the frontend, which does
not yet produce these) compiled and run through a real `cc` — covering present/
missing-key reads, insert/update/alias writes, structural composite keys,
duplicate-key overwrite, positional structural equality, brace-list display, the
zero-iteration `ForEach`-over-map, and the cyclic-map stack-safety guard.

## 0.6.0 — sequences (SIR16)

Accepts `Feature::Sequences`. `SirValue` gains a `SIR_SEQ` tag — a heap-boxed
dynamic array (`struct SirSeq { SirValue *items; int64_t len; }`, arena
allocated like every other heap value) — so a sequence is a shared, mutable
handle: a `SeqSet` through one binding is visible through every alias, matching
the Go/Rust `*Seq`. Every construct the feature can surface is lowered:

- `SeqLit` (`[1, 2, 3]`) → `_sir_seq_lit(n, …)`.
- `SeqIndex` (`a[i]`) → `_sir_seq_index`: a negative index counts from the end,
  an out-of-range index yields nil (it does NOT raise — matching the reference
  and every other backend).
- `SeqLen` (`a.length`) → `_sir_seq_len`.
- `SeqSet` (`a[i] = v`) → `_sir_seq_set`, which TRAPS (`stderr` + `exit(1)`) on
  a negative or out-of-range index, matching the Go/Rust `panic`.
- `ForEach` (`for x in a`) → a `for` loop over `_sir_seq_iter(a)`, which
  snapshots the iterable (a real sequence is copied so a mutating body does not
  disturb iteration; a cons-list is flattened). `x` is declared inside the loop
  body block, so it is block-scoped — matching the validator's rewind and Go's
  `:=` counter. This is why `ForEach` is no longer rejected by the `first_foreach`
  pre-pass added in 0.5.0 (that pre-pass and its clean-rejection are removed).

`_sir_value_eq` gains a structural `SIR_SEQ` arm — equal length, element-wise
equal, with an identical-handle fast path (which also short-circuits the common
self-referential `a == a`). `_sir_fmt` renders a sequence as `[1, 2, 3]`
(bracket, comma-space), matching the Go/Rust backends. With this, the
cross-backend composite-equality conformance (`[1,2] == [1,2]`) now asserts on
**all six** backends — C was the last that skipped it.

Because `SeqSet` is the first MUTABLE heap aggregate (cons pairs are immutable
and so cannot form a cycle), a self-referential sequence (`a[0] = a`) is now
constructible; both `_sir_value_eq` and `_sir_fmt` carry a recursion-depth cap
so a cyclic structure terminates rather than overflowing the C stack — a guard
the immutable pair path never needed. (Found by security review, which also
caught that the earlier "matches the pair arm" claim was wrong.)

Every node is verified by hand-built modules (producer-agnostic), compiled with
a real `cc` under `-Werror=unused-variable` and run: display, structural
equality (positive/negative/nested), index (in-range/negative/OOB), length,
in-bounds set, and block-scoped ForEach.

## 0.5.0 — `ForRange` (numeric for-loop) + a scan hole (SIR16)

Fixes a **pre-existing panic**: `Stmt::ForRange` (`for i in 0...3`) is gated by
`Feature::Loops` alone (accepted since 0.4.0), so a producer emitting a numeric
for-loop reached the emitter — which sent it to `unreachable!`. It now lowers to
a native `int64_t` counter loop mirroring the Go/Rust backends byte-for-byte:

- `start`/`stop`/`step` are evaluated ONCE (they may have side effects) into
  `SirValue` temporaries, then reduced to `int64_t` via the new `_sir_as_int`
  runtime helper (a truncating integer view — a float bound truncates toward
  zero).
- the stop is EXCLUSIVE and the direction follows the step's sign
  (`step >= 0 ? i < stop : i > stop`), so a descending loop with a negative step
  works — matching Go's `_sir_range_cont`.
- the loop `var` is declared INSIDE the loop body block, so it (and any
  body-local) is block-scoped — matching the validator (which rewinds the loop
  body) and Go's `:=` counter, never clobbering an enclosing same-named local.
  The outer `{…}` scopes the counter temporaries (nesting-safe via `fresh_id`).

Also closes a **pre-existing scan hole** (same class): the unsupported-builtin
pre-check (`scan_block_for_builtin`) did not recurse into `While` or `ForRange`
bodies, so an unknown builtin hidden in a loop body escaped the clean rejection
and hit the emitter's `unreachable!`. It now scans both; such input rejects
cleanly with a `BackendError` instead of panicking.

Makes the emitter TOTAL for its accepted feature set. `ForEach` also observes
only `Feature::Loops` (not gated out), so it was likewise a latent
`unreachable!` — `compile` now rejects it CLEANLY via a `first_foreach`
pre-pass (a clear `UnsupportedFeature` error) until the sequences batch gives it
an iterator, rather than panicking. The sequence nodes stay rejected at the
feature gate — a follow-up adds `Feature::Sequences` (a real `SIR_SEQ`
runtime).

## 0.4.0 — control flow, mutation & the rest of the comparisons (SIR16)

Accepts `Feature::Loops` and `Feature::MutableBindings`, and:

- Renders `Stmt::While` as a portable `for (;;) { SirValue c; c = <cond>; if
  (!_sir_truthy(c)) break; <body> }` — the condition is re-evaluated each
  iteration, so it may be compound.
- Renders `Stmt::Assign` (re-binding an already-declared `SirValue`).
- Adds the missing comparison builtins `<=`, `>=`, `==`, `!=` (runtime helpers
  `_sir_le`/`_sir_ge`/`_sir_ne`; previously only `<`/`>`/`=` were lowered, so a
  `<=` reached `_sir_unknown_builtin` and failed).
- **Portability fix:** user functions named `min`/`max` are now escaped (trailing
  `_`).  `<stdlib.h>` on MSVC/UCRT defines `min`/`max` as function-like macros,
  so `SirValue min(SirValue a, SirValue b)` expanded to garbage under clang-cl /
  MSVC — now they compile on all three compilers.

## 0.3.0 — lower unary minus (`neg` builtin) — negative literals no longer skip

Ruby lowers unary minus (`-x`) to `BuiltinCall("neg", [x])`, but the v0 C
emitter had no lowering for `neg`, so `first_unsupported_builtin` rejected it and
the whole program was reported `UnsupportedFeature` (i.e. **skipped**) — meaning
ANY negative literal, not just division, was unrunnable on the C backend.

Unary minus IS single-argument subtraction, and the runtime's `_sir_minus_v`
already negates a single argument tag-preservingly (a `SIR_FLOAT` stays float,
otherwise int). So `neg` now lowers to `_sir_minus(1, x)` via `variadic_helper`
— no new runtime code — matching the Go/Rust/Python runtimes that gained `neg`
in SIR21 §E3. New `unary_minus` exec-proof in `tests/compile_and_run.rs`
(`puts(-7)` → `-7`, `puts(-7 / 2)` → `-4` floored, `puts(-(3 * 2))` → `-6`),
compiled and run through a real C compiler.

This closes the **C arm** of the division frontier: with the runtime already
flooring (`_sir_ifloordiv`), C now reproduces Ruby's floor `/` on negative
dividends too, so `sir-conformance`'s `division_matches_ruby_floor_on_every_backend`
asserts (rather than skips) C's negative cases.

## 0.2.0 — render SIR26 integer conversions

Accepts `Feature::Conversions` (plus the SIR21 type-implied `SizedIntegers`,
`Unsigned`, `WrappingArithmetic`) and renders `Expr::Convert`, so C→SIR→C
round-trips a source language's integer width/wrapping/truncating semantics.

- A conversion emits the portable runtime helper `_sir_convert(v, bits, signed)`
  (with `_sir_mask_to` doing a two's-complement reduction over `int64`/`uint64`
  — mask then sign-fold — no reliance on native fixed-width casts, so it behaves
  identically on MSVC/GCC/Clang).  A target width of `Arbitrary` is the identity
  and emits no wrapper.  `bits >= 64` is the `int64` storage floor (u64 above
  2^63 is the documented bignum frontier, shared with the Go/Rust backends).
- Verified on **clang, gcc, and MSVC**: `(uint8_t)300==44`, `(int8_t)200==-56`,
  `(uint16_t)70000==4464`, `(uint32_t)-1==4294967295`,
  `(int32_t)4e9==-294967296`, arbitrary-width identity.

## 0.1.0 — v0 core (SIR24)

First release of the sixth SIR backend: lowers a `semantic_ir::Module` to a
**self-contained ISO C99 source file** compilable on MSVC (`/std:c11`), GCC, and
Clang.  Gives **Ruby → C** (and Python/JS/Twig → C) through the shared
narrow-waist IR.

### Added

- `compile(&Module) -> Result<Artifact, BackendError>` and `CBackend`
  implementing `semantic_ir::Backend` with `target_tag() == "c"`.
- **Capability set (v0):** `Closures`, `Pairs`, `Symbols`, `Strings`,
  `DynamicTyping`, `OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.
  Rejects `TailCalls`, `Intrinsics`, and every later feature (including
  `Bignum`) cleanly rather than mis-compiling.
- **Inlined C runtime** (`runtime.rs`) — a tagged-union `SirValue`
  (nil/bool/int64/float/str/sym/pair/closure), arena/leak-on-exit memory, symbol
  interning, SIR truthiness (false/nil-only), polymorphic `+ - * / < > =` (string
  concat on `+`, int-floor vs float-true division), structural equality,
  `cons`/`car`/`cdr` and type predicates, closures (`make_closure`/`apply`), a
  string-keyed global store, and Ruby/Lisp-aware `print`/`puts` display.  Runtime
  functions use external linkage so the fully-inlined runtime never trips
  `-Wunused-function` on a small program.
- **Emitter** (`emit.rs`) — statement-oriented lowering (`emit_tail` /
  `emit_assign`) so an `if`/block produces a value without any
  statement-expression; variadic builtins via C variadic functions; closure
  thunks; identifier sanitisation (`sanitize_ident`) and C string/comment
  escaping; deterministic (byte-stable) output.
- **Portability:** `#define _CRT_SECURE_NO_WARNINGS`, `snprintf` (no `sprintf`/
  `strcat`), no compiler-specific extensions — verified building and running on
  MSVC, GCC, and Clang.
- **Injection hardening:** string/symbol literals escape `?` as `\?` so a
  source `??/` cannot expand (via C trigraphs under `-std=c99`) into a `\` that
  breaks out of the emitted C literal; `_sir_builtin_dispatch` reads arguments
  through a bounds-checked `_sir_arg` so an under-applied builtin-as-value reads
  `nil` rather than indexing out of bounds.
- **Tests:** `tests/emit.rs` (emit-shape, determinism, sanitisation, capability
  rejection — no compiler needed) and `tests/compile_and_run.rs` (compiles and
  runs each corpus program through a discovered `cc`/`clang`/`gcc`, skipping when
  none is present).  Corpus covers arithmetic, method calls, tail-`if`,
  sequential assignment, string concat, and Twig closures.
- `examples/dump_c.rs` — dump the emitted C for a Ruby/Twig snippet.
- README documenting the design, portability contract, and roadmap to parity.
