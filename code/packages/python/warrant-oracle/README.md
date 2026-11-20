# python/warrant-oracle

The **independent oracle** of Warrant corpus v2
([VER04r](../../../specs/VER04r-warrant-corpus-v2.md) section 10, decision [D3]).

The Rust generator (`rust/warrant-grammar`, later `rust/warrant-corpus-gen`) reads
a `.warrant` file and *lowers* it to a language-neutral model IR plus digests of
the inputs and expected values of every claim. A bug in that lowering (`a - b`
emitted as `b - a`, a refinement dropped, a domain in the wrong order) would be
faithfully copied into the corpus and then faithfully *verified* by every lane.
This package is the second, independent reading that catches it:

```text
code/specs/<pkg>.warrant ──► Rust generator ──► corpus.json (IR + digests)
        │                                              │
        │                                              ▼ compared, never trusted
        └────► this package: lex, parse, check, lower, ───► mismatch? the adopter
               enumerate, evaluate, hash                    cannot merge
```

It is **stdlib-only**, shares no code, data file or table with the generator, and
**never reads the generator's IR as truth**: it parses the `.warrant` source and
lowers it itself, then only *compares*.

## What is here (slice S4, parts 1 and 2, S10a, S10t-1, S10c-1 and S11a)

| Part | Module | Notes |
|---|---|---|
| reader | `lexer`, `parser`, `ast` | layout tokens, v0 + VER04j v1 + VER04k WX-G syntax, fail-closed, admission limits |
| checker + lowering | `lower`, `levels`, `patterns`, `irtree`, `typenames` | typing, contextual literals, **unoptimised** syntax-directed IR, the 35 `ir.fixed` built-ins, tuples and `match` on a `Result` (`ir.tuple`), records, variants, `Option` and `match` on any scrutinee (`ir.data`, lowered and costed, **not evaluated**), `requires` inference, named level and feature refusals |
| evaluator | `ir_eval` | `ir.core`, `ir.tuple` and the 35 `ir.fixed` built-ins, the single wrapping rule (VER04 section 3), `MAX_IR_VALUE`, cost bound, call counting (a `match` arm is a guard) |
| domains | `domains` | every descriptor of VER04r 5.1 (`fx_edge` [D21, D25], `nat_shift` [D26]), `size`/`nth`, budget demotion, the shift-count rule, the consumer-side descriptor check |
| streams | `stream` | `warrant-stream-v1/v2` encoders, chunk and suite framing, FNV-1a slim selection |
| suites | `suites` | product enumeration, admission arithmetic, chunking, per-chunk digests |
| gate | `corpus_v1`, `emit`, `cli`, `lowering_dump`, `parity_dumps`, `eval_dumps` | the ct-compare v1 gate, v2 digest emitter, level census, lowering, evaluation, fault, value and domain dumps |
| v2 gate (part 2) | `corpus_v2`, `faults`, `tfv`, `verify_v2` | derives a whole v2 fixture from the `.warrant` and `corpus.profile`; `verify-v2` compares it with the directory |

Every adopter parses: ct-compare, gf256, wasm-leb128, logic-gates, lzw,
arithmetic, x25519 (and the WX-G `inout`/`ensures` syntax of zeroize).

### Feature levels

The evaluator implements **`ir.core`, `ir.tuple` and `ir.fixed`** (VER04r section 4.1; `ir.tuple` since
slice S10t-1 and `ir.fixed` since slice S10c-1, see below), so `derive`, `emit-v2` and `verify-v2` accept a
document that needs any of the three. **`ir.data`** (records, variants, `Option` as a real type, `match` on any
scrutinee) is *lowered, typed and costed* since slice S11a but **not evaluated**: a unit that needs it is lowered
completely and then refused at evaluation with `level ir.data` (slice S11c-1 opens that). What the oracle refuses at
*lowering*, by name and never as `ir.data`, is what has no level it implements: the reserved `ir.list` (a list type
anywhere, a list literal, `slice to_list to_bytes replicate`) and `ir.seq` (`fold map all any range count`), and the
two feature names `err-value` (an `Err(e)` arm that uses `e`) and `fixed-cast` (`fixed<N>(b)`); none is ever skipped
silently. The built-ins are still refused in a v0 document (VER04j 1.2: the names are version scoped).
Census (the pins of `tests/test_lower.py::CENSUS`: suites whose evaluation is not blocked, `ir.core`, and
the suites blocked at evaluation; `check --spec` also prints `suites_refused_at_lowering`):

| Adopter | suites | live (`ir.core`) | `ir.data` | before S10c-1 (core / fixed / data) |
|---|---:|---:|---:|---|
| ct-compare | 9 | 9 | 0 | 9 / 0 / 0 |
| gf256 | 38 | 38 | 0 | 38 / 0 / 0 |
| zeroize | 9 | 9 | 0 | 8 / 1 / 0 |
| x25519 | 8 | 8 | 0 | 6 / 2 / 0 |
| lzw | 21 | 21 | 0 | 4 / 17 / 0 |
| wasm-leb128 | 44 | 44 | 0 | 18 / 24 / 0 (42 claims; the two encoder bounds were split, VER04r D28) |
| logic-gates | 88 | 85 | 3 | 28 / 57 / 3 |
| hash-functions | 56 | 35 | 19 (+ 2 `err-value`) | 4 / 31 / 21 |
| arithmetic | 63 | 8 | 55 | 7 / 11 / 45 |

(Before S10t-1: wasm-leb128 10 / 8 / 24 tuple, logic-gates 28 / 55 / 2 tuple / 3, arithmetic 5 / 9 / 4 tuple / 45.
S11a: **every suite of logic-gates and arithmetic now lowers whole**; arithmetic's 55 is the 45 that used to be refused at
lowering plus the 10 that counted a record type in their `requires`, and hash-functions' 21 became 19 that lower (the claims that
`match` on a bare tuple) and 2 refused as `err-value`.)

### `ir.data`: lowering, typing, `requires` and cost, with evaluation still refusing (slice S11a)

Records, variants (with and without a payload), `Option<T>` as a real type, the four tags of VER04r Appendix A.2 and
`match` on **any** scrutinee are lowered by `lower.py`, typed and costed again by `ir_eval.Program(analyze_data=True)` (what
`dump-lowering` uses) and counted in `requires`:

| Source | IR | Cost |
|---|---|---|
| `Some(e)` | `["some", e]`, typed exactly `Option<type(e)>`, which flows to a wider `Option` by its payload (R39 (a), [D60], R51) | `1 +` the operand |
| `None` | `["none", "T"]`, `T` the canonical type name the context fixes; no context is a `type` error (R39 (b)) | 1 |
| `Name(e1, .., en)`, a bare payload-free `Name` | `["ctor", "Name", [e1, .., en]]` | `1 +` the operands |
| `r.name` | `["field", r, i]` (the index, never the name); `e.i` on a non-tuple and `e.name` on a non-record are `type` errors (R39 (c)) | `1 +` the operand |
| `match s with ..` | `["match", s, [[pat, arm], ..]]` with the patterns `some none ctor bool` beside the six of A.1 | `1 +` the scrutinee `+` the worst arm |

* **`requires`** (R9 as extended): by tag, by pattern and by **type**. A `match` is `ir.tuple` when its scrutinee is a `Result`
  and `ir.data` for any other type, and `ir.data` as well when a data pattern appears (so `Ok(true) -> ..` needs both). The type rule adds
  `ir.data` for a record, a variant or an `Option` anywhere in a type, **except the intermediate use** of R29: a producer node
  (`try checked_add checked_sub checked_mul div rem div_trunc rem_trunc at`) directly the first operand of `unwrap_or`
  adds none, so `unwrap_or(checked_add(a, b), 0u8)` still requires `ir.fixed` only and **no `requires` of the seven committed corpora
  moved** (their `verify-v2` lines pass unchanged).
* **Patterns** are checked before any arm body, outside in, with the structure (unknown constructor, a type that does not fit, a wrong field
  count) before the duplicate-bind and shadowing rules (R35 (3)); then the usefulness check (`patterns.py`, now over `Bool`, `Option`,
  `Result`, variants, records and tuples), unreachable arms before fall-through. A name that is a declared constructor is a constructor
  pattern, never a binder.
* **Refusal order** (R35): the declaration limits of section 2.2 first (`limit`: 64 constructors, 8 fields per constructor, 32 per
  record, 64 error cases), then a reserved level or feature before a type error of the same unit, then units in declaration order.
* **Evaluation still refuses** (R37): a unit that lowered and needs `ir.data` carries the *evaluation* refusal `level ir.data`
  (`LoweredModel.lowered` tells the two refusals apart; `emit-v2` reports the suite `not-live`), until slice S11c-1.
* **The committed dumps** (`tests/fixtures/data-forms-v1/`, generated by `make_data_forms.py`, a new directory on purpose):
  `data_forms_v1.oracle-lowering.json` (86 models, 94 suites, 11 examples, the corpus `types` rows) and
  `data_forms_v1.oracle-refusals.json` (109 documents: every reserved level, the corpus limits, the typing rules and corpus errors of
  Appendix A.2, and the refusal order). **They make Appendix A.2 stop being provisional**; slice S11b must reproduce them byte for byte.
  The one row this slice moved in an existing dump is `tuple-forms-v1`'s `a-projection-of-a-non-tuple` (now a `type` error).

### `ir.tuple`: tuples and `match` on a `Result` (slice S10t-1)

* **Forms** (Appendix A.1, R2): the type `(T1,..,Tn)` (2 to 8 components), the literal
  `["tuple", [e1, .., en]]`, `["project", e, i]` (`t.i`, a chain `t.0.1`, or `f(x).1`), values as
  Python tuples and JSON arrays, stream tag `64` with a `u32` count, TFV `L<n> v1 .. vn`, the
  descriptor `tuple` (Cartesian product, last component fastest) and component-wise `==`/`!=`.
* **`match` on a `Result`** with the patterns `ok err err_any tuple bind wild`
  (`["match", s, [[pat, arm], ..]]`) in claims, models, op `admit`s and examples. It is `ir.tuple`
  exactly when the scrutinee is a `Result` (D35); any other scrutinee is `ir.data`. Cost is
  `1 +` the scrutinee `+` the maximum over the arms; exactly one arm is evaluated. A literal pattern
  (`Ok(true)`) is not one of the six and is `ir.data`; `Err(e)` is `err_any`, and `e` may not be used
  (an error value has no IR form).
* **Corpus errors** (code `corpus`, checked before any arm body is lowered, `patterns.py`): a match
  that can fall through, an arm no value can reach (the usual usefulness check), the same `bind`
  twice in one pattern, and a `bind` that shadows a binder, a parameter, `result` or an outer `bind`.
* **Lowering-time `W0701`** (CV2-F1): a subject call under an impure guard (VER04r 4.4) and a claim that
  calls no op (4.3) are refused when the claim is lowered, as the generator refuses them (recorded in
  `LoweredSuite.refusal`, raised by `derive`, `emit-v2` and `dump-lowering`). A non-omitted example with a
  `Bytes` over 4,096 bytes is a `limit` error (VER04r 2.3).
* **A `match` arm is a guard** for call counting (VER04r 4.4): a subject call in an arm needs a pure
  scrutinee, and then only the arm the scrutinee selects counts; the wrap trace, the D19 admit check
  and the v0 rule-3 walk follow the same arm (`irtree.children` and `ir_eval.walk_scoped`).
* **Emission**: `derive`, `emit-v2` and `verify-v2` evaluate `ir.tuple` suites and examples
  (`levels_implemented` lists `ir.tuple`; since S10c-1 also `ir.fixed`).
* **`requires`** (R9) was already type-directed since S10a; S10t-1 pins every place a tuple type
  can hide (binder, op or model parameter or result, an example value, a pattern binder) and a
  fault row's `requires` now also counts the types of its op.
* **Faults** (`faults.py`): `ok-as-err`, `drop-consumed`, `swap-components`, the tuple-payload
  `missing-error` (R3) and the `Ok` forms of `flip-result-bit` and `drop-last-result-byte` are
  instantiated as data. **Their evaluation is gated** exactly like `ir.fixed`'s: since the atomic flip (S10c, 0.5.0)
  `IMPLEMENTED` is `("ir.core", "ir.fixed", "ir.tuple")`, so their controls carry witness chunks; under the
  closed gate `derive(..., implemented=("ir.core",))` (tests and the committed dumps) a control that needs
  `ir.tuple` has `{"claim": C, "deferred": "ir.tuple"}` witnesses.
* **`off-by-one-consumed`** (S10o-1, VER04r 8.2 and 16.10 [D36]): the second component `n` of a tuple result,
  or of the tuple inside an `Ok`, becomes `n + 1` when `n` is even and `n - 1` when odd (a `Nat` component:
  `if unwrap_or(rem(C, 2), 0) == 0 ...`; a fixed-width one: `bit_xor` with `1`). `requires` is
  `["ir.fixed", "ir.tuple"]`; it refuses what `drop-consumed` refuses. Section 8.2 fixes its JSON, and the
  committed dump in `tests/fixtures/off-by-one-consumed-v1/` (a directory of its own, generated by
  `make_off_by_one_consumed.py`, which `BUILD` runs with `--check`) must be reproduced byte for byte by the
  generator's slice S10o-2. The registry now holds fourteen names.
* **`saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error`** (S11f-1, VER04r 8.2 and
  16.14): the scalar faults of the Rust harness (`V1_FAULTS`). The saturating pair is kind `replace` over the
  first two parameters (one fixed-width type `T`; result `T` or `Result<T,E>`, wrapped in `["ok", V]`
  unconditionally, because the harness never calls the export); `shift-mask` is kind `pre` on the last parameter
  (`rem`, or `rem_trunc` when it is signed, by the width of the FIRST parameter's type). All three are
  `["ir.fixed"]` and live. `missing-error` now also takes a scalar payload (`Bool`, `Nat`, a fixed-width type:
  the first parameter of exactly that type, else the zero) and a `Bytes` payload with no `Bytes` parameter
  (the zero literal); the record and `Option` payloads need `ir.data` and stay refused. `Nat` parameters, mixed
  widths and other result types have no template (the code `fault`). The committed dump is in
  `tests/fixtures/scalar-faults-v1/` (`make_scalar_faults.py`, which `BUILD` runs with `--check`); the
  generator's slice S11f-2 must reproduce it byte for byte. The registry now holds seventeen names.
* **The committed dumps** (`tests/fixtures/tuple-forms-v1/`, generated by `make_tuple_forms.py`,
  which `BUILD` runs with `--check`): the document, the lowering dump, the evaluation dump
  (`parity_dumps.evaluation_view`), the fault dump (`parity_dumps.faults_view`, **normative for the
  composite fault JSON**, R11) and every corpus-error document with its refusal. Slice S10t-2 (the
  generator) must reproduce them byte for byte. They carry no generator name or version.

### `ir.fixed` lowering, `requires` and the lowering dump (slice S10a)

* **Lowering and typing** (`lower.fixed_call`, `ir_eval.Program._fixed_type`): one IR node per
  built-in, same spelling, positional operands, a static type argument as the canonical type
  name in position 1 (`["widen", "U32", e]`, `["max_value", "I64"]`). Two independent typers
  (the source typer and the IR typer) implement Appendix A.1; `div`/`rem` on two `Nat`
  operands are `Option<Int>` (R7). `Option<T>` is an IR-only intermediate whose only consumer is
  `unwrap_or` as its first operand, directly (D23, R8); any other use, and an `Option` in a
  signature or binder, is the level `ir.data`.
* **`requires`** (`levels.ir_levels`, R9): the sorted union of the levels of every tag of a
  suite's, op's `admit`'s expressions and of the call closure of every `model_call`; a tuple
  *type* adds `ir.tuple` and a list, record or variant type adds `ir.data`; `ir.core` is always
  included. `derive` writes it into suite and op rows. Every committed fixture verifies unchanged;
  a tuple-binder document gains `ir.tuple` in `requires`/`levels`, and an `Option` binder is
  refused at `ir.data`.
* **Cost**: every new tag is `1 +` its operands (a static type argument is not an operand).
* **The dump** (`dump-lowering`, `lowering_dump`): the canonical JSON of the lowered IR, the
  `requires` and the cost of a document. `tests/fixtures/fixed-forms-v1/` holds the synthetic
  `ir.fixed` document (every one of the 35 built-ins at every width the spec allows; generated
  by `make_fixed_forms.py`) and its committed dump, which slice S10b must reproduce byte for
  byte. `BUILD` runs both `--check`s.

### `ir.fixed` evaluation, the domains of D21/D25/D26 and the `fixed-eval-v1` dumps (slice S10c-1)

* **The evaluator** (`ir_eval._fixed_fn`): all 35 built-ins of Appendix A.1 on Python integers with
  explicit masking. A fixed-width value is its mathematical integer; bit operations work on the
  `w`-bit pattern and re-read the result in the type; a shift count is an exact `Nat` compared with `w`
  **before** any shift (`shl`/`shr` give 0, `sar` gives `-1` or 0, `bit` gives false), `rotl`/`rotr` reduce
  `n mod w` first; `div`/`rem` on `Int`/`Nat` are floor division and the divisor-signed remainder,
  `div_trunc`/`rem_trunc` truncate and are `None` for `b == 0` and `MIN / -1`; `from_be`/`from_le` of a
  runtime width other than the target's is an `ir` error, never a truncation; `unwrap_or` evaluates both
  operands. **`wraps` counts only `add sub mul neg`** [D31]. Cost is as S10a defined it (`1 +` the
  operands). `Program(analyze=...)` is kept and changes nothing. `tests/test_fixed_eval.py` holds every
  built-in at every width against an independent bit-list reference.
* **Domains** (`domains.py`): the signed `fx_edge` is `wrap_S(E_U(w)) | {-255..-1}`, de-duplicated and
  ascending (I16 527, I32 559, I64 623: the generator's set) [D21]; `fx_edge` at 16 bits exists by
  budget demotion only and an 8-bit one is refused [D25]; the new leaf `{"d":"nat_shift","size":73}`
  (`0..=64` then `65 127 128 129 255 256 65535 65536`) is chosen for a `Nat` binder that **is** the count
  operand of `shl shr sar rotl rotr bit` in the claim body, and for nothing else (R10: not
  `shl(x, f(n))`, not a `model_call` argument, not a count only in an `admit`) [D26]. `descriptor_size` is
  the consumer-side check of VER04r 5.1 (sizes recomputed, depth, the legality of every leaf).
* **Emission**: `derive`/`emit-v2`/`verify-v2` accept `ir.fixed` documents. The synthetic documents
  and `lzw.warrant` evaluate (`tests/test_fixed_lzw.py`: with `omit_example` lines for the six bomb
  examples, four `value-over-limit` and two `not-op-eq-pure`, and `max_ir_cost 16384`; a full derive
  took about 21 seconds).
* **The fault gate is open since the flip (S10c, D29, 0.5.0)**: `faults.IMPLEMENTED` is
  `("ir.core", "ir.fixed", "ir.tuple")`; the `ir.fixed` templates (`xor-lsb`, `ignore-high-bit`,
  `flip-result-bit`, `drop-last-result-byte`, `truncate-last`) are searched, so ct-compare's
  `eq_u64/ignore-high-bit` is live with its witness at chunk 0 (`a = 0`, `b = 2**63`, ordinal 311).
  `verify-v2` therefore refuses a corpus that still says `deferred` for `ir.fixed` or `ir.tuple`
  (`tests/fixtures/ct-compare-v2-preflip/` is the negative test). The closed gate stays reachable through
  `derive(..., implemented=("ir.core",))`; the committed dumps pin their gate explicitly.
* **The committed dumps** (`tests/fixtures/fixed-eval-v1/`, generated by `make_fixed_eval.py`, run by
  `BUILD` with `--check`; slice S10c-2 must reproduce them byte for byte; none carries a generator
  name or version): the document (every built-in at every width, as tuples of up to 8 built-ins per
  suite, the shift-count cases, the faults), its profile (`budget 2^20`, so the 16-bit binders are
  demoted), the lowering dump, the evaluation dump (`parity_dumps.evaluation_view`), the fault dump (the
  controls under the closed gate **and** the open gate `ir.core + ir.fixed + ir.tuple`), the value table
  (`eval_dumps.values_view`, about 9,800 rows `shl U8 255 7 = 128`) and the domain table
  (`eval_dumps.domain_view`: the full `fx_edge` and `nat_shift` value lists and the descriptors a consumer
  must reject). Where VER04r 8.2 leaves a form loose, the committed dump is normative (R11).

## Usage

```bash
# The ct-compare gate: all three checks against the committed v1 corpus.
python -m coding_adventures_warrant_oracle verify-v1 \
    --spec code/specs/ct-compare.warrant \
    --corpus code/specs/fixtures/ct-compare-warrant-v1/corpus.json \
    --emit-v2 scratch/ct-compare-v2.json

# The oracle's own v2 enumeration digests for any document (slim or full depth),
# ready to diff against S2/S3 generator output.
python -m coding_adventures_warrant_oracle emit-v2 \
    --spec code/specs/gf256.warrant --profile corpus.profile \
    --out scratch/gf256-v2.json --depth slim

# Exit 3 means some suites were not-live or refused; --allow-partial accepts that.
# Parse and lower a document; print what level each suite needs.
python -m coding_adventures_warrant_oracle check --spec code/specs/lzw.warrant

# The canonical lowering dump of a document (S10a): write it, or check a committed one.
python -m coding_adventures_warrant_oracle dump-lowering \
    --spec tests/fixtures/fixed-forms-v1/fixed_forms_v1.warrant --check \
    tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json
```

Exit status: `0` the gate passed, `1` a mismatch, `2` an input was unreadable or
refused (size cap, symbolic link, malformed document, an output path that is an
input or lies in `code/specs`), `3` (`emit-v2`) some suites were `not-live` or
`refused`. Failure messages name a
suite and a check, never input bytes.

### The ct-compare gate

`verify-v1` reproduces, from `ct-compare.warrant` alone, everything the generator
committed in `ct-compare-warrant-v1/corpus.json`:

1. **Lowering**: operations, types, models, examples and every suite's
   `admit`/`lhs`/`rhs`/binders equal the corpus IR as canonical JSON.
2. **Enumeration and evaluation**: `cases`, `discarded`, `expected_calls`,
   `inputs_sha256` and `expected_sha256` of all nine suites (572,109 cases, about
   1.3 s), computed through the *chunked* enumeration path, so it also proves that
   chunking reproduces the v1 order (VER04r section 1.2).
3. **Faults and controls**: the fault registry and the eight control rows.

## Findings recorded while building it

These are facts the S3/S8 owners need; none is hidden by a flag.

* **`MAX_IR_COST = 2,000` cannot hold gf256 as first written (VER04r 16.1 amended).**
  Charging both branches of an `if` (the VER04i 6.2 reading) `xor8` costs 294,900.
  The amended rule is the **worst path** (one branch of an `if`): `gmul` costs
  2,011, `ginv` 26,204, the largest gf256 expression 26,210, and four of its 38
  suites exceed the default 2,000, so `emit-v2` records them `refused: limit`.
  A corpus raises the bound only with a `max_ir_cost N` line in `corpus.profile`
  (`--profile`; 2,000 to 65,536); there is no command-line or environment
  override. Each suite row carries `ir_cost` (admit + body + expected) and the
  total work (cases and IR work) is metered against `MAX_TOTAL_WORK_*` and
  `MAX_IR_WORK_*`.
* **gf256 never wraps** (`wraps == 0` on every suite it enumerates), so the VER04r
  3.3 guard for a v0 document passes rule 2.
* **Composite demotion differs from VER04j.** VER04j 14.1 item 20 demotes both
  components of `(U16, U16)` at `2^26` because the Rust generator stores a
  composite domain as one vector under a 64 MiB byte budget. VER04r enumerates
  chunk-wise and never materialises a domain, so this oracle demotes by product
  size only. If the generator keeps the VER04j rule the descriptor comparison will
  say so.
* **Reserved words.** VER04 section 2 reserves `op`, but `arithmetic.warrant` has a
  parameter named `op`; the reader treats declaration keywords as contextual.
* **Assumptions where the spec is silent** (each is a place the first v2 corpus may
  disagree, which is exactly what the gate is for): a bare decimal literal is typed
  `Nat` when its context is `Nat` and `Int` otherwise; several refinements fold
  left into binary `and`; an unsigned `neg` is allowed at the IR level and refused
  by the source checker; `wraps` counts subject-independent nodes the oracle
  evaluated (`admit`, `expected`, call-count guards); the two slim digests are
  `slim_inputs_sha256` and `slim_expected_sha256`.

## `verify-v2` (slice S4 part 2)

```bash
python -m coding_adventures_warrant_oracle verify-v2 code/specs/fixtures/ct-compare-warrant-v2 \
    --warrant code/specs/ct-compare.warrant \
    --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile \
    [--bindings code/packages/rust/ct-compare/evidence.bindings] [--repo-root .] \
    [--expect-generator warrant-corpus-gen@0.6.0]
```

`spec` in the corpus must equal the repo-relative path of `--warrant` (root: nearest ancestor with a
`code/` directory, or `--repo-root`). The spec's effective size cap is the lexer's 1 MiB.

The oracle **derives** the corpus (`corpus.json`, `chunks/*.txt`, `cases.tfv`) from the
`.warrant` source and `corpus.profile` with its own reader, lowering, evaluator,
enumeration, stream writer, fault registry and witness search, then compares the result
with the directory. It reads no generator code and no generator output beyond the
directory it is asked to check, so a bug the generator wrote into a corpus is a mismatch
here, not a shared assumption. The comparison groups what it finds by the three checks of
VER04r section 10.3:

| Check | Group | What is re-derived and compared |
|---|---|---|
| 1. Lowering | `lowering` | `types ops models model_order examples`, each suite's `id kind claim ops requires admit body oracle result_type`, binder types, and `cases.tfv` |
| 2. Enumeration and evaluation | `enumeration` | `budget chunk_size max_ir_cost`, each binder's resolved descriptor, `cases discarded expected_calls wraps ir_cost product chunks stream domain_label demoted`, every chunk line (`first_admitted`, `calls`, both chunk digests), `chunks_sha256`, the suite digests, the v1 whole-suite digests, `slim_chunks` and both slim digests |
| 3. Faults and controls | `faults`, `controls` | every fault row (incl. `rewrites` and the opaque `ir.fixed` expressions), every control row, `required_claims`, `blind_claims`, `slim_witness`, `accepted_blind_faults` (the control ids of the profile's `accept_blind` lines whose fault is all-blind, sorted; `[]` for ct-compare) |

Also: `header` (identity and digest binding: `profile module document spec spec_sha256
profile_sha256 generator levels assumes`), `files` (exactly the derived chunk files, regular
files only, size caps, no symbolic links), `bytes` (the canonical writer: key order,
indent, LF, final LF) and `drift` (`corpus.profile` against the bindings' `faults=`).
Exit status `0` pass, `1` mismatch, `2` an input was unreadable or the oracle refuses the
document (a level above `ir.core`, a resource bound, a generation error).

Bytes are compared exactly where VER04r fixes them (chunk files, `cases.tfv`, the canonical
form of `corpus.json`) and structurally, type-strict and closed, where the `corpus.json`
key set is provisional (section 1.4). The `generator` key (name and version) cannot be
derived and is checked for shape only, unless `--expect-generator name@version` is given
(owner decision [D13], VER04r 16.8): any other recorded pair is a `header` difference at
`generator`. The ct-compare BUILD passes it, so a generator version bump fails until the fixture
is regenerated and the pin reviewed with it.

A fault whose level the **fault-evaluation gate** (`faults.IMPLEMENTED`, `ir.core` only until the flip)
lacks (`ir.fixed` or `ir.tuple`, e.g. ct-compare's `ignore-high-bit`) is
**instantiated and compared as opaque data, never evaluated**; its candidate claims are
`deferred` (VER04r 10.4). The witness search is the one of section 5.6: the lowest admitted
**product ordinal** at which the faulted expected value differs, as a chunk; a claim is
*witnessed*, *blind* or *unproven*, and a fault every candidate is blind to fails generation.

Gaps closed by part 2 (all were listed in VER04r 16.3 to 16.5 as "S4 part 2"): `corpus.profile`
strictness (repeated `fault`/`slim_chunk` lines, an empty token, `budget 0`, `slim_stride 0`
are errors); `concat` is evaluated on the pure trace (a result past `MAX_IR_VALUE` in a
subject argument fails generation); v0 rules 2 and 3 of section 3.3 (also through a
`model_call` whose callee's call closure wraps); the IR depth is 64 levels, not 65; the
`slim_stride` clamp per suite; witness chunks and `slim_chunk` lines in the slim set.

`accept_blind <op> <fault>` ([D15], VER04r 16.8) is a profile line: a fault every candidate claim is
blind to passes when named (its control row has no `required_claims`; its control id goes in
`accepted_blind_faults`) and fails generation otherwise; naming a fault a claim can see, one with
no `fault` line, or repeating the line is a profile error (`corpus`).

`omit_example <id>` ([D22], [D34], VER04r 8.4, S10-1) removes one example from `examples` when the
tool can decide that it is inexpressible. The line carries only the id; the **reason is computed**:
`not-op-eq-pure` (the shape is not `op(args) == pure`; tested first) or `value-over-limit` (the
normative order of 8.4: arguments left to right then the pure side; a non-size error refuses; a
size error, `errors.SizeError`, qualifies only if it is the first error; on a completed evaluation
the D18/D19 checks run and refuse first; only then does an argument or pure value over 4,096 bytes
qualify). An unknown id, a repeated line, a malformed line and a line for an emittable example (a stale
profile) are errors (`corpus`); a D18/D19 failure or any other error still refuses generation
(`generation` or the error's own code). `corpus.json` gains the **conditional** key
`omitted_examples` (`[{"id", "reason"}]`, sorted by id) only when an example is omitted, so no
committed fixture moves a byte; `verify-v2` recomputes it, rejects `[]`, unsorted or duplicate rows,
extra row keys and unknown reasons, and ties it to `profile_sha256` through the derived comparison. An
omitted example is neither evaluated for `cases.tfv`, encoded nor admit-checked. `verify-v2` prints the
omitted count after the suite count when it is non-zero (this oracle has no `not_live` report; the lane
reports of S9 show the count beside it). `tests/fixtures/omit-example-v2` is the agreement fixture the
generator's parity PR is compared with.

Tests use `tests/fixtures/synth-v2` and `tests/fixtures/accept-blind-v2`, small synthetic
documents and the corpora the Rust generator produced from them (the first's README says how; the
second is regenerated byte for byte by the generator's tests, so the two implementations agree on
`accept_blind`). The committed ct-compare fixture, `code/specs/fixtures/ct-compare-warrant-v2/`
(CV2-S3d), is verified by a test and by this package's `BUILD` with `--profile
<fixture>/corpus.profile` (an edit to the hand-written profile fails through `profile_sha256`, [D14]).
The second adopter's fixture, `code/specs/fixtures/x25519-warrant-v2/`, is verified by a second line of the
same shape (with `--bindings code/packages/rust/x25519/evidence.bindings`).
The third adopter's fixture, `code/specs/fixtures/lzw-warrant-v2/` (21 suites, 17,563,819 cases, six
`omit_example` lines, `max_ir_cost 16384`), is verified by a third line of the same shape (with `--bindings
code/packages/rust/lzw/evidence.bindings`); its full derive takes about 20 s, so no sharding is needed
(VER04r 16.10 [D33]).
The fourth adopter's fixture, `code/specs/fixtures/wasm-leb128-warrant-v2/` (44 suites, 46,566,930 cases, 740
chunks, 20 faults including the composite ones and `off-by-one-consumed`), is verified by a fourth line of the
same shape (with `--bindings code/packages/rust/wasm-leb128/evidence.bindings`). Its full derive takes **4 min
53 s** on one core (15 times lzw's), by far the most expensive line of this package; it is one line, not
sharded (VER04r 16.11 [D38]), and sharding by chunk stays the documented remedy if it ever stops fitting.

## Two refusals (S3e, owner decisions D18 and D19)

The same two rules as the generator, derived independently: an example whose evaluation wraps is
`W0701` in a v0 document (`f(255u8 + 2u8) == 1u8`) and accepted in v1; and a claim or example that
calls an op outside that op's `admit` is `W0701` naming the claim, the op and the first
admitted-case ordinal (`SuitePlan.check_admits`, `ir_eval.compile_admit_check`), with a
subject-dependent argument the op's `admit` reads refused statically (VER04r 4.5 and 16.9).
`tests/test_admit_calls.py` holds the same documents as the generator's `tests/admit_calls.rs`.

## The v2 conformance kit (S5a, S5b)

VER04r section 11 adds six categories to the shared fixture family `warrant-corpus-conformance-v1`:
`v2-value`, `v2-arith`, `v2-ir`, `v2-domain`, `v2-chunk` and `v2-fault` (a seventh, `v2-binding`, is S5c's).
The oracle **derives every expectation**: `src/coding_adventures_warrant_oracle/conformance.py` is the
derivation module, and the hand-written case tables, the two small kit documents the chunk cases come from
and `make_conformance.py` live in `tests/fixtures/conformance-v2/` (its README says how to add a case).

| Category | The module derives it from |
|---|---|
| `v2-value` | the strict JSON reader of VER04i 9.1, the section 2.3 decoder (`decode_int_text` and the typed forms), the section 2.4 stream encoder |
| `v2-arith` | one node of the typed IR evaluator over two `lit` operands |
| `v2-ir` | the IR evaluator with each `subject_op` rewritten to a pure stand-in model for the value, and the pure call trace of section 4.4 on the unrewritten tree for `calls` |
| `v2-domain` | a descriptor-JSON reader over `descriptor_size` and the leaf constructors, then `size()` and `nth()` |
| `v2-chunk` | the chunk-file check of sections 5.5 and 14, then `SuitePlan.run_chunk` (one chunk, by unranking) |
| `v2-fault` | the existing fault templates and the three wrappers of section 8.1 |

A table row states a case's inputs and its **intent**; the oracle derives the outcome independently, and a row
whose intent disagrees fails the build naming the case id. The oracle's own codes are mapped to the kit's closed
vocabulary (section 16.13 R25) in `classify_ir_refusal`; each kit case triggers exactly one error.

```bash
python3 tests/fixtures/conformance-v2/make_conformance.py --write --fixture DIR   # the six v2-*.json files
python3 tests/fixtures/conformance-v2/make_conformance.py --check --fixture DIR   # rebuild and compare bytes
```

`--fixture` defaults to the family directory `code/specs/fixtures/warrant-corpus-conformance-v1`; `--check` also
requires an `index.json` that is present to hold one row per derived file and skips that check when there is none.
Exit codes: 0 identical, 1 the bytes differ, 2 a table and its derivation disagree or any other error. The script
never writes `index.json` (the family's v1 script does).

S5b commits the six `v2-*.json` files (734 cases) and their rows of `index.json`. **Regeneration is two steps**:
`make_conformance.py --write` (the six files), then `python3 code/scripts/warrant_corpus_conformance.py --write`
(`index.json`: the four v1 rows, then one row per `v2-*.json` on disk in the fixed order). `BUILD` runs
`make_conformance.py --check --fixture ../../../specs/fixtures/warrant-corpus-conformance-v1` (a second derivation of
the `2**24` suite, about a minute, that the pytest session fixture does not share), the v1 checker
`code/scripts/warrant_corpus_conformance.py` (which indexes and structurally checks the `v2-*` files without importing
the oracle or a lane) and its tests, so either tool alone catches a stale index. An edit to the fixture directory,
the checker or its test selects this package (build-tool `isWarrantOracleInput`).

`tests/test_conformance_mutants.py` is the mutant test of [D42]: sixteen named wrong evaluators (no wrap,
saturation, a signed compare on the bit pattern, eager `and`, first-binder-fastest enumeration, a missing chunk
index in the stream prefix, sequential `pre` rewrites ...), each of which must change the derived expectation of at
least one case of each category it is named for. `tests/test_conformance_cross.py` re-derives slices of the kit by
different algorithms (Python integers, `struct.pack`, a naive `itertools.product` enumerator, the cost model written
out) and runs the v1 checker's strict reader and canonical writer over the v2 files.

## Not yet done

* Evaluating `ir.data` (S11c-1; lowering, typing, `requires` and cost are done, S11a) and `inout`/`ensures`
  monitoring (S12): a document that needs more than `ir.fixed` and `ir.tuple` is refused at evaluation, naming
  the level, and the `ir.list`, `ir.seq`, `err-value` and `fixed-cast` constructs are refused at lowering. The
  fault-evaluation gate for `ir.fixed` and `ir.tuple` is turned on at the atomic flip (S10c).
* The oracle's `domain_label`/`depth` rows are emitted but not yet compared with a
  lane manifest.
* The shape of `assumes` (VER04r 16.4 item 4); `verify-v2` expects `[]`.

## Security

The oracle parses a `.warrant` file from a pull request, so it treats it as
hostile: size cap (1 MiB), token cap (100,000), identifier (128 bytes) and integer
(4,096 digits) caps, nesting cap 64 (types, expressions, patterns, IR), no `eval`,
no `exec`, no regular expression on attacker text, no subprocess, no network, and
file access limited to the two paths on the command line (regular files only,
symbolic links refused). `MAX_IR_VALUE` is checked before any allocation, so a
`concat` chain cannot exhaust memory.

## Tests

```bash
uv venv --quiet --clear && uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
```

Parser goldens over every adopter, evaluator vectors for the section 3.4 wrapping
cases, enumeration-order tests against a naive reference enumerator, chunk
independence and boundary tests, resource-bound and hostile-input tests, and
seeded-wrong corpora that the gate must reject. Coverage is held at 95 percent.
