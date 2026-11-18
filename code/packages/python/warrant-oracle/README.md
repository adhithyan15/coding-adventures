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

## What is here (slice S4, parts 1 and 2, and S10a)

| Part | Module | Notes |
|---|---|---|
| reader | `lexer`, `parser`, `ast` | layout tokens, v0 + VER04j v1 + VER04k WX-G syntax, fail-closed, admission limits |
| checker + lowering | `lower`, `levels` | typing, contextual literals, **unoptimised** syntax-directed IR, the 35 `ir.fixed` built-ins, `requires` inference, named level errors |
| evaluator | `ir_eval` | `ir.core`, the single wrapping rule (VER04 section 3), `MAX_IR_VALUE`, cost bound, call counting |
| domains | `domains` | every descriptor of VER04r 5.1, `size`/`nth`, budget demotion |
| streams | `stream` | `warrant-stream-v1/v2` encoders, chunk and suite framing, FNV-1a slim selection |
| suites | `suites` | product enumeration, admission arithmetic, chunking, per-chunk digests |
| gate | `corpus_v1`, `emit`, `cli`, `lowering_dump` | the ct-compare v1 gate, v2 digest emitter, level census, lowering dump |
| v2 gate (part 2) | `corpus_v2`, `faults`, `tfv`, `verify_v2` | derives a whole v2 fixture from the `.warrant` and `corpus.profile`; `verify-v2` compares it with the directory |

Every adopter parses: ct-compare, gf256, wasm-leb128, logic-gates, lzw,
arithmetic, x25519 (and the WX-G `inout`/`ensures` syntax of zeroize).

### Feature levels

The evaluator implements **`ir.core`** only (VER04r section 4.1). Since slice S10a the
lowerer also **lowers, types and costs the 35 built-ins of `ir.fixed`** (Appendix A.1; see
below), so a model, op `admit`, claim or example that uses them has its IR and its computed
`requires`, and is then recorded `not-live` at the level `ir.fixed`: *evaluation still
refuses*. A construct the lowerer cannot lower, `ir.tuple` (tuple tags, `match` on a
`Result`), `ir.data` (records, variants, `match` on anything else, `Option` anywhere but
the intermediate use of `ir.fixed`) or the reserved `ir.seq`, is recorded with that level
*named*; it is never skipped silently. A unit is named by the first construct beyond what the
lowerer can lower, and `ir.fixed` only when nothing else is. Census at this commit:

| Adopter | suites | `ir.core` | `ir.fixed` | `ir.tuple` | `ir.data` |
|---|---:|---:|---:|---:|---:|
| ct-compare | 9 | 9 | 0 | 0 | 0 |
| gf256 | 38 | 38 | 0 | 0 | 0 |
| x25519 | 8 | 6 | 2 | 0 | 0 |
| lzw | 21 | 4 | 17 | 0 | 0 |
| wasm-leb128 | 42 | 10 | 8 | 24 | 0 |
| logic-gates | 88 | 28 | 55 | 2 | 3 |
| arithmetic | 63 | 5 | 9 | 4 | 45 |

### `ir.fixed` lowering, `requires` and the lowering dump (slice S10a)

* **Lowering and typing** (`lower.fixed_call`, `ir_eval.Program(analyze=True)`): one IR node per
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
    [--expect-generator warrant-corpus-gen@0.5.0]
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

A fault whose level this oracle lacks (`ir.fixed`, e.g. ct-compare's `ignore-high-bit`) is
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

Tests use `tests/fixtures/synth-v2` and `tests/fixtures/accept-blind-v2`, small synthetic
documents and the corpora the Rust generator produced from them (the first's README says how; the
second is regenerated byte for byte by the generator's tests, so the two implementations agree on
`accept_blind`). The committed ct-compare fixture, `code/specs/fixtures/ct-compare-warrant-v2/`
(CV2-S3d), is verified by a test and by this package's `BUILD` with `--profile
<fixture>/corpus.profile` (an edit to the hand-written profile fails through `profile_sha256`, [D14]).

## Two refusals (S3e, owner decisions D18 and D19)

The same two rules as the generator, derived independently: an example whose evaluation wraps is
`W0701` in a v0 document (`f(255u8 + 2u8) == 1u8`) and accepted in v1; and a claim or example that
calls an op outside that op's `admit` is `W0701` naming the claim, the op and the first
admitted-case ordinal (`SuitePlan.check_admits`, `ir_eval.compile_admit_check`), with a
subject-dependent argument the op's `admit` reads refused statically (VER04r 4.5 and 16.9).
`tests/test_admit_calls.py` holds the same documents as the generator's `tests/admit_calls.rs`.

## Not yet done

* Evaluating `ir.fixed` (slice S10c-1), `ir.tuple` (S10t), `ir.data` (S11) and `inout`/`ensures`
  monitoring (S12): a document above `ir.core` is refused whole, naming the level.
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
