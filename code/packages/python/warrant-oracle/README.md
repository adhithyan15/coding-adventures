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

## What is here (slice S4, part 1)

| Part | Module | Notes |
|---|---|---|
| reader | `lexer`, `parser`, `ast` | layout tokens, v0 + VER04j v1 + VER04k WX-G syntax, fail-closed, admission limits |
| checker + lowering | `lower` | typing, contextual literals, **unoptimised** syntax-directed IR, named level errors |
| evaluator | `ir_eval` | `ir.core`, the single wrapping rule (VER04 section 3), `MAX_IR_VALUE`, cost bound, call counting |
| domains | `domains` | every descriptor of VER04r 5.1, `size`/`nth`, budget demotion |
| streams | `stream` | `warrant-stream-v1/v2` encoders, chunk and suite framing, FNV-1a slim selection |
| suites | `suites` | product enumeration, admission arithmetic, chunking, per-chunk digests |
| gate | `corpus_v1`, `emit`, `cli` | the ct-compare gate, v2 digest emitter, level census |

Every adopter parses: ct-compare, gf256, wasm-leb128, logic-gates, lzw,
arithmetic, x25519 (and the WX-G `inout`/`ensures` syntax of zeroize).

### Feature levels

The evaluator implements **`ir.core`** only (VER04r section 4.1). A model or suite
that needs `ir.fixed` (shifts, masks, `wrap`, `be_bytes`, ...), `ir.data` (tuples,
records, `match`, `Option`) or the reserved `ir.seq` is recorded as `not-live`
with the level *named*; it is never skipped silently. Census at this commit:

| Adopter | suites | `ir.core` | `ir.fixed` | `ir.data` |
|---|---:|---:|---:|---:|
| ct-compare | 9 | 9 | 0 | 0 |
| gf256 | 38 | 38 | 0 | 0 |
| x25519 | 8 | 6 | 2 | 0 |
| lzw | 21 | 4 | 17 | 0 |
| wasm-leb128 | 42 | 10 | 21 | 11 |
| logic-gates | 88 | 28 | 48 | 12 |
| arithmetic | 63 | 5 | 45 | 13 |

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
  evaluated (`admit`, `expected`, call-count guards); `slim_sha256` is emitted as
  separate inputs and expected digests.

## Not yet done (part 2)

* `verify-v2`: reading a v2 `corpus.json` and `chunks/*.txt` (no generator output
  exists yet; `emit-v2` writes the oracle's side for the diff).
* Check 3 for v2: re-deriving fault IR, `fault_models`, controls and
  `required_claims` from `corpus.profile` (section 8).
* Slim **witness chunks** (section 5.6) and `corpus.profile` parsing.
* `ir.fixed`, `ir.data` (slices S10, S11) and `inout`/`ensures` monitoring (S12).
* The oracle's `domain_label`/`depth` rows are emitted but not yet compared with a
  manifest.

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
