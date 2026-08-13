# Changelog — wasm-wast-parser

## 0.1.2 — 2026-08-13 — a local-index bug found investigating real assert_return failures (WASM14)

`build_func` assigned local indices by re-walking a function's own literal
`(param ...)` forms, incrementing a counter as it went. That undercounts
the moment a function references its signature purely via `(type $sig)`
(no `(param ...)` forms of its own at all — the official testsuite's
`func.wast` has several such cases: `"type-use-1"` through `"type-use-5"`)
and *also* declares a `(local ...)`: the counter never advances past 0
for the (invisible-to-this-function) params from the referenced type, so
the first declared local silently gets assigned parameter index 0 again
instead of the index right after the real params. `local.get` on that
local then read the PARAM's value instead of the local's own
zero-initialized default — a real, wrong computed VALUE, not a trap
(`func.wast`'s `"f"`/`"g"` cases expected 0, got 42, the argument passed
in).

Fixed by seeding the local-index counter from `ctx.module.types[type_idx]
.params.len()` — the function's REAL resolved param count — rather than
from a count built by re-walking this function's own literal `(param
...)` forms, which can legitimately be empty. Uses `.get()`, not direct
indexing: an already-regression-tested case
(`func_with_out_of_range_numeric_type_reference_does_not_panic`) exercises
a numeric `(type N)` reference with no matching `(type ...)` section entry
at all, which this text-level parser deliberately does not reject (that's
`wasm-validator`'s job) — falls back to a param count of 0 rather than
panicking on the out-of-range index.

1 new regression test
(`local_declared_after_a_type_only_referenced_param_gets_the_next_free_index`)
reproducing `func.wast`'s exact shape in isolation. Baseline: `assert_return`
12169/12238 (99.4%) → 12171/12238 (99.5%).

**A security review of this fix found a residual edge case**: it split
one shared counter into two independent ones (literal `(param ...)`
forms counted as written, vs. the referenced type's real param count),
which only agree when a function's literal params match its `(type
$sig)` reference exactly — not something `resolve_func_signature_ref`
itself enforces. A syntactically-valid but semantically-inconsistent
module (literal params disagreeing in count with a same-function `(type
$sig)` reference — deliberately adversarial input, not something a real
`.wat` file produces) could make a declared local alias whichever count
was smaller. Fixed by seeding the local-index counter from
`max(literal param count, the type's real param count)` the first time a
`(local ...)` form is actually reached, so a declared local can never
collide with a position either count considers a parameter. 1 more
regression test
(`local_index_never_collides_with_a_param_even_if_literal_params_and_the_type_disagree`).
No conformance-baseline change (the real testsuite never disagrees with
its own type references).

**A second round of security review found that round 1's fix wasn't
actually closed by round 2's `max()` patch — it moved the failure mode.**
Since the compiled `FunctionBody` and the function's real type only ever
account for the type's real param count, an "extra" literal param (in
the same mismatched-arity scenario round 2 was defending against) still
encoded a `local.get`/`.set`/`.tee` index past the function's real local
array. Confirmed empirically: `wasm-execution`'s raw, unchecked
`ctx.typed_locals[index]` panics once such a module actually runs — not
memory-unsafe (checked Rust indexing), but a real crash/DoS surface
reachable through this repo's own pipeline
(`wasm-conformance`/`wasm-runtime`/`wasm-execution`), since the only
validation currently wired up (`WasmRuntime::validate`) is structural
only and doesn't check instruction operand bounds. The real fix is
upstream of both prior patches: a new `WastParseError::TypeUseParamCountMismatch`
now REJECTS at parse time when a func's literal `(param ...)` forms
disagree in arity with an explicit `(type $sig)` reference, instead of
silently accepting the inconsistency and hoping every later index
computation stays safe. This is also the spec-correct behavior — a real
`.wat` file's literal params, when given alongside a type reference,
always already match it exactly. The `max()`-based local-index seeding
from round 2 stays as defense in depth (harmless: once this new check
passes, the two counts are always equal whenever literal params were
given), but this rejection is what actually makes the invariant hold. 2
more regression tests: the mismatched case now asserts a clean `Err`
instead of successfully (and unsoundly) parsing, plus a new positive
case confirming the legitimate "type reference + matching literal
params" pattern (`func.wast`'s own `"type-use-6"` shape) still parses
and indexes correctly.

**A third round of security review found round 3's own rejection check
could itself be bypassed.** Its pre-scan stopped at the first field that
wasn't `param`/`result`/`type` — but a `(local ...)` form placed BEFORE
some of a func's trailing `(param ...)` forms (this parser doesn't
enforce that params all precede locals; that's `wasm-validator`'s job
too) made the pre-scan stop before ever counting those later params,
silently skipping the mismatch check while the main assignment loop
still processed them — reproducing round 2's exact out-of-bounds local
index, just via reordering instead of an outright count mismatch. Fixed
by giving the pre-scan the identical leading-region membership test
(`is_leading_field`: `param`/`result`/`type`/`local` are ALL "still in
the prefix," only a real instruction ends it) the main loop already
uses, so the two passes can no longer silently disagree on where the
leading region ends. 1 more regression test reproducing the reordered
bypass directly.

**A fourth (final) round of security review, after re-verifying the
round-3 fix genuinely closed the OOB class, found a functional
regression the mismatch check itself had introduced**: it compared
against `param_count`'s `0` fallback for an out-of-range numeric `(type
N)` reference, silently violating this file's own documented contract
(`func_with_out_of_range_numeric_type_reference_does_not_panic`) that an
unresolvable type reference must NOT be rejected here — that's
`wasm-validator`'s job. `(func (type 0) (param i32))` — ordinary,
spec-legal literal params alongside an unresolvable type index — got
hard-rejected instead of passed through. Fixed by gating the check on
the type reference actually resolving to a real type first. Also
extracted `count_literal_param` (the named-vs-unnamed param-counting
arithmetic) as a single function shared by the pre-scan and the main
loop, the same way `is_leading_field` already is — the review flagged
two independently-maintained copies of that arithmetic as exactly the
drift pattern that produced rounds 2 and 3's findings, even though the
two copies were still identical today. 2 more regression tests: the
false-positive case now confirmed fixed, plus the legitimate
"out-of-range type, no literal params" case re-confirmed unaffected.

## 0.1.1 — 2026-08-13 — 4 grammar bugs found running the real testsuite (W05 PR-4)

`wasm-conformance` (W05 PR-4) is this crate's first real workout: running
every vendored file from the official `WebAssembly/testsuite` corpus, not
just this crate's own hand-written unit tests. That surfaced four genuine
parsing bugs, each fixed with its own regression test:

- **Folded `br_table`'s label/operand split was backwards.** WAT's grammar
  lists all label targets FIRST, then an OPTIONAL folded index operand
  LAST — `(br_table $a $b (i32.const 0))` — the opposite of every other
  instruction's own "immediates trail operands" convention. The original
  code searched from the END of the argument list for the first non-atom
  element (assuming trailing atoms were the labels), found the folded
  operand's own position instead, and silently produced a zero-label
  `br_table` while dropping the real label references. Affected any file
  using a folded `br_table` with more than one label — a majority of the
  corpus's control-flow files.
- **`(table reftype (elem e*))` — a table with its size implied by an
  inline element list instead of explicit numeric limits — was completely
  unhandled.** `funcref` isn't a digit atom, so `parse_limits` always hit
  its "expected 1 or 2 limit numbers" error path. Now recognized as its
  own form: `min`/`max` are set to the element count, and the elem
  segment referenced by those functions is synthesized directly (`i32.const
  0` offset), matching the shorthand's defined meaning.
- **A bare hex integer (no `.` fraction, no `p`/`P` exponent) wasn't a
  valid float literal.** `f32.const 0xf32` means the plain number
  `3890.0`, not a bit reinterpretation and not a hex *float* (which
  requires an exponent) — but the parser required a `p`/`P` exponent
  unconditionally for anything hex-prefixed.
- **A hex float's `p`/`P` exponent is optional even WITH a fractional
  part**, not just on a bare integer — `0xa0_ff.f141_a59a` (no exponent
  at all) defaults to exponent 0. The mantissa parsing was previously
  reachable only via the exponent-bearing branch.

A security review of this same PR found one more, related bug in the
`(table reftype (elem e*))` fix above: `(table funcref ())` — a
syntactically valid but EMPTY inline list, with no `"elem"` head atom at
all — indexed `elem_form[1..]` without first confirming the list was
both non-empty and actually headed by `"elem"`, panicking with a
slice-range-out-of-bounds. Fixed by validating the head atom before
slicing, with its own regression tests (`(table funcref ())` and
`(table funcref (notelem)))`, both now clean `Err`s). None of the 48
currently-vendored files trigger this — every real table-elem shorthand
names at least one function — but it's exactly the shape of input a
future `assert_malformed`/`assert_invalid` fixture (or wider corpus
vendoring) could hit.

Net effect on the vendored corpus: file-level parse failures dropped from
33/48 to 16/48. The remaining 16 are legitimate, out-of-scope gaps
(multi-value block signatures, reference-types `externref` and the
generalized `elem` syntax, post-MVP saturating-truncation/sign-extension
opcodes, and the `func`/`global` inline-import shorthand — the last is
linking-adjacent and shares this phase's already-documented `spectest`
deferral) — see `code/specs/W05-wasm-conformance-harness.md` section 6 and
`wasm-conformance`'s own report output for the exact breakdown.

## 0.1.0 — 2026-08-12 — initial release (W05 PR-2)

New crate. Parses the WebAssembly text format — both plain `.wat` modules
and the official spec testsuite's `.wast` script dialect — into
`wasm-types::WasmModule` and a sequence of test directives. Phase A of the
`wasm-execution` conformance-harness arc; see
`code/specs/W05-wasm-conformance-harness.md`.

- **`tokenizer`**: S-expression tokenizer — atoms, parens, quoted strings
  (with standard, `\u{XXXX}`, and raw `\XX` hex-byte escapes for embedding
  intentionally-invalid bytes), line comments, and **nestable** block
  comments (`(; a (; b ;) c ;)` is one comment, not two).
- **`sexpr`**: generic S-expression tree the tokenizer's flat stream is
  grouped into — folded instruction syntax (`(i32.add (i32.const 1) ...)`)
  is structurally identical to any other nested list, so there is no
  separate "folded vs. flat" parsing code path at this layer.
- **`numeric`**: WAT numeric literal parsing beyond what `str::parse`
  offers — hex integers, hex floats (`0x1.8p3`, computed bit-exact via
  digit-by-digit mantissa accumulation scaled by an exact power of two,
  not an approximate float parse), `inf`/`nan`, and `nan:0x<payload>` (an
  *exact* NaN bit pattern). `i32`/`i64` literals accept the WAT-permitted
  range union of both the signed and unsigned spelling of the same bit
  pattern (`-1` and `0xffffffff` both denote the identical i32 bits).
- **`module`**: the core — two-pass `(module ...)` parsing (pass 1 collects
  every symbolic name in every index space, imports always occupying the
  lowest indices regardless of textual interleaving with non-import
  definitions per the WAT spec; pass 2 encodes function bodies, globals'
  init expressions, and element/data segments straight to raw WASM
  bytecode). Supports both **folded** and **flat** instruction syntax for
  every MVP opcode with immediates (control flow, local/global access,
  calls, memory load/store, the four `*.const`s) via two structurally
  distinct encoders (`encode_flat_instr` for folded-list operand/immediate
  splitting, `encode_stream_instr`/`encode_stream_structured_instr` for a
  bare-atom instruction consuming however many *following* stream elements
  its own immediates need) — the two forms have different immediate
  ordering rules (folded: instruction's own index/label leads, operand
  sub-expressions trail; flat: operands were already pushed by whatever
  came before in the stream, so only trailing immediate atoms belong to
  this instruction) that a single shared code path could not represent
  correctly; the crate's own tests were written specifically to catch this
  after development first got it backwards for `br`/`call`/`local.set`/
  `local.tee`/`global.set` (immediate-first, not immediate-last).
- **`script`**: `.wast` script-directive parsing — `module`, `register`,
  `invoke`/`get`, `assert_return` (including `nan:canonical`/
  `nan:arithmetic` NaN-class result forms), `assert_trap`,
  `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
  `assert_unlinkable`. A plain `module` directive is built eagerly
  (propagating a real syntax error immediately, since `assert_return`/
  `assert_trap` need an already-valid module to invoke against);
  `assert_invalid`/`assert_malformed`'s module is captured as a **raw,
  unparsed S-expression** instead, since failing to build it is exactly
  what those two directives test for — eagerly building it here would turn
  every legitimate fixture into a hard error aborting the whole script.
  Also supports the `(module binary "...")`/`(module quote "...")` module
  variants, concatenating their string-literal bytes for the caller.
- **Hardening pass** (pre-merge security review): this crate will eventually
  process the official testsuite's `assert_malformed`/`assert_invalid`
  fixtures, which are deliberately adversarial — so every reachable panic
  on malformed-but-syntactically-parseable input was replaced with a clean
  `Result::Err`. Fixed: `parse_i32` overflow on an extreme-magnitude
  negative literal (unary negation panicking in debug builds — switched to
  `wrapping_neg`); `parse_limits` panicking via `.unwrap()` on a
  non-numeric or out-of-`u32`-range limit; folded `br_table` underflowing
  `labels.len() - 1` on an empty label list; multiple `script.rs` directive
  parsers and `module.rs`'s `build()`/`build_elem`/`build_data` indexing
  past the end of a too-short field list (`(register)`, `(export "e")`,
  an empty `elem`/`data` segment, etc.) — all now go through a shared
  `sexpr::expect_get` helper instead of `items[N]`; and unbounded `(...)`
  nesting recursion, now capped by `sexpr::MAX_NESTING_DEPTH` (512) with a
  new `WastParseError::TooDeeplyNested` variant instead of a stack
  overflow. Each fix has a dedicated regression test proving the old code
  path would have panicked.
- **Hardening pass, round 2** (same pre-merge security review, second
  pass over `module.rs` specifically): five more `items[N]`-style panics
  of the exact same class, all in spots round 1's sweep missed —
  `build_import_shell`'s error-path indexing an empty import description
  (`(import "m" "n" ())`); `parse_global_type` indexing a `(mut)` form
  with no trailing value type; the `"start"` directive with no function
  reference (`(module (start))`); `handle_inline_export`'s `(export)`
  shorthand with no name string; and a bare `(type)` reference with no
  index/name, reachable from three call sites (`func` import
  descriptions, and both the flat and folded forms of `call_indirect`).
  All converted to `sexpr::expect_get`, each with its own regression
  test.
- **Hardening pass, round 3**: `build_func` indexed `ctx.module.types` by
  an unvalidated numeric `(type N)` reference (`(module (func (type 0)))`
  with no `(type ...)` declared anywhere panics indexing an empty `Vec`)
  while fetching a value that was, on inspection, entirely dead code
  (immediately discarded, never used) — fixed by deleting the dead fetch
  rather than adding unused bounds-checking; bounds-checking a type index
  is `wasm-validator`'s job (structural "index bounds" validation), not
  this text-parser's, so this module now correctly parses to a
  (structurally invalid) `WasmModule` instead of panicking or duplicating
  validation this crate doesn't own.
- **Hardening pass, round 4**: `sexpr::MAX_NESTING_DEPTH` only bounds
  `(...)` parenthesis nesting — but WAT's **flat** instruction syntax lets
  `block`/`loop`/`if` nest with NO parentheses at all (`block block
  block ... end end end`, all sibling atoms in one unnested list), driving
  unbounded `encode_one` <-> `encode_stream_structured_instr` recursion
  the S-expression-level guard never sees. Empirically confirmed as a real
  stack-overflow abort (not a catchable panic) before this fix. Added a
  second, independent `InstrCtx::depth` counter (`enter_block`/
  `exit_block`, covering both the flat and folded structured-instruction
  encoders uniformly) capped by a NEW, deliberately lower
  `MAX_INSTR_NESTING_DEPTH` (100, not 512) — this recursion carries more
  per-frame state than the lightweight S-expression tree-builder, so 512
  levels of it measurably overflows a real thread's stack around depth
  ~487, well before the counter would ever stop it.
- **Hardening pass, round 5**: round 4's guard only covered `block`/
  `loop`/`if` recursion — a deeply nested FOLDED operand of an ordinary
  instruction (`(i32.add (i32.add (i32.add ...) ...) ...)`, no control
  flow involved at all) recurses through `encode_flat_instr` ->
  `encode_instr_list` -> `encode_one` with no depth guard whatsoever, and
  empirically aborted with a real stack overflow around depth ~165 — well
  under `sexpr::MAX_NESTING_DEPTH` (512), so that guard never tripped
  first either. Fixed by consolidating the depth guard into `encode_one`
  itself — the single point every form of instruction nesting (folded
  operands, folded `block`/`loop`/`if`, and flat `block`/`loop`/`if`)
  funnels through — instead of gating only the `block`/`loop`/`if`-
  specific encoders. The now-redundant per-block guards were removed to
  keep exactly one depth-accounting mechanism. A regression test confirms
  a long FLAT (sibling, not nested) instruction sequence well past
  `MAX_INSTR_NESTING_DEPTH` in length still parses fine — the guard tracks
  nesting depth, not instruction count.
- 82 unit tests across all five modules, ~95%+ line coverage.
