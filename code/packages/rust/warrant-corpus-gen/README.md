# warrant-corpus-gen

The **Warrant corpus v2 emitter** ([VER04r](../../../specs/VER04r-warrant-corpus-v2.md),
slice S3). It reads a `.warrant` specification and will write the language-neutral
`warrant-corpus-v2` fixture that eighteen language lanes run against their own libraries.

> **Status: slice S3b of four.** The generator now has its whole *computing* half: the
> canonical JSON writer, the `corpus.profile` reader, lowering to the `ir.core` model IR with
> the level refusals, the single wrapping rule, worst-path cost accounting (S3a), and (S3b)
> the `ir.core` **evaluator**, **enumeration** over the S2 domain API, the **stream encoders
> and digests** and the **adopter census**. ct-compare reproduces all eighteen v1 digests.
> The faults and controls, the emission of `corpus.json`, `chunks/*.txt` and `cases.tfv`, and
> `--check` (S3c) and the committed ct-compare fixture (S3d) are not here yet.
> [`generate`](src/lib.rs) is still a stub that **fails closed**: it validates what it can
> and then refuses with `code == "unimplemented"`. It never emits a partial corpus.

## How it fits

```text
  code/specs/<pkg>.warrant
         │  parse + check            coding_adventures_warrant_grammar  (S2 public API)
         ▼
  WarrantFile ── lower ──▶ ir.core IR ── cost ──▶ ir_cost, wraps rules       S3a
         │                     │
         │                     ├─ evaluate, enumerate, 18 digests, census ──── S3b (here)
         │                     ├─ faults, controls, emit, --check ──────────── S3c
         │                     └─ commit the ct-compare v2 fixture ─────────── S3d
         ▼
  corpus.json + chunks/*.txt + cases.tfv
         │
         ├──▶ python/warrant-oracle   an independent second reading of the .warrant file:
         │                            it lowers and enumerates on its own and COMPARES
         │                            with this crate's output, never trusting it   (S4)
         └──▶ lanes                   python, typescript, go, rust, ... each runs the
                                      corpus against its own implementation      (S6+)
```

This is a *leaf* crate: it depends on `warrant-grammar` and `sha256`, and nothing depends
on it, so changing it rebuilds no shared crate (VER04r section 13, "Rule").

## What is here

| module | what it is |
|---|---|
| `json` | the VER04i 9.1 canonical writer: sorted keys, two-space indent, LF, `\uXXXX` escapes. No `serde`: a canonical form must not inherit a library's defaults |
| `profile` | the strict reader of `corpus.profile` (VER04r 8.4): LF only, canonical decimals, `max_ir_cost` the only way to raise the cost bound |
| `types` | the small type language of `ir.core` (`Bool Int Nat U8..I64 Bytes Bytes<N> Result<T,E>`) |
| `ir` | the 26 `ir.core` tags of VER04r Appendix A and their JSON form |
| `lower` | syntax-directed, unoptimised lowering with a small annotating typer; the level refusals; `census` |
| `analysis` | the pure-guard rule (4.4) and the v0 static arithmetic guard (3.3 rule 3) |
| `arith` | the single wrapping rule (3.2): `wraps`, the 2^127 window, `MAX_IR_VALUE` |
| `cost` | worst-path `ir_cost`, `max_ir_cost`, total IR work (4.2 rule 6) |
| `view` | the "parity view" of a lowered document, compared with the oracle |
| `value`, `eval` | S3b: the `ir.core` evaluator (own implementation over this crate's IR): exact `Nat`/`Int` in the 2^127 window, wrapping fixed-width arithmetic counted as `wraps`, short-circuit `and`/`or`/`implies`/`if`, `MAX_IR_VALUE` checked before allocation, and `trace`, the pure-trace walk behind `expected_calls` and `wraps` |
| `domains` | S3b: descriptor JSON, labels and sizes over the S2 `resolve_binders`/`Domain` API; refuses the VER04r 16.2 descriptors; value and encoding caches |
| `stream` | S3b: the 2.4 value encoders, the context prefix, the v1/v2 header, `warrant-chunks-v1` framing, FNV-1a-64, the slim base set |
| `suite` | S3b: one suite: product enumeration (`unrank_product`, last binder fastest), admission evaluated once per prefix tuple, chunks, counts, suite and v1 whole-suite digests, slim set |
| `enumerate` | S3b: a whole document at `full` depth, the total-work bounds, the v0 divergence guard rule 2, and the enumeration parity view |
| `census` | S3b: per claim, why the generator can or cannot emit it, and which VER04r 16.2 items its binders touch |

## Usage

```rust
use coding_adventures_warrant_corpus_gen::{analyse, profile::parse_profile};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

let src = std::fs::read_to_string("code/specs/ct-compare.warrant").unwrap();
let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();

// gf256 would say `max_ir_cost 32768` here; ct-compare declares nothing.
let profile = parse_profile(b"").unwrap();
let analysis = analyse(&file, &profile).unwrap();
for (suite, cost) in analysis.lowered.suites.iter().zip(&analysis.suite_costs) {
    println!("{} ir_cost {}", suite.id, cost.total);
}
```

### Enumerating

```rust
use coding_adventures_warrant_corpus_gen::{enumerate_document, profile::Profile};
# use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
# use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
# use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
# let src = std::fs::read_to_string("code/specs/ct-compare.warrant").unwrap();
# let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).unwrap();
# let layout = capture_layout(&doc, LayoutLimits::default()).unwrap();
# let file = parse_document(&layout, &doc, ParserLimits::default()).unwrap();
let e = enumerate_document(&file, &Profile::default()).unwrap();
assert_eq!(e.cases(), 572_109);
let s = e.suite("select_picks").unwrap();
println!("{} chunks, v1 inputs {}", s.chunks_total, s.v1_inputs_sha256.as_deref().unwrap());
for line in s.chunk_lines() {
    println!("{line}"); // <c> <cases> <first_admitted> <calls> <inputs_hex> <expected_hex>
}
```

## Lowering, by example

```text
  forall a: Bytes, b: Bytes | len(a) == len(b), choice: Bool.
      select_bytes(a, b, choice) == Ok(if choice then a else b)
```

lowers to a `reference` suite:

```text
  admit    ["eq", ["len",["var","a"]], ["len",["var","b"]]]
  subject  ["subject_op","select_bytes",[["var","a"],["var","b"],["var","choice"]]]
  expected ["ok", ["if", ["var","choice"], ["var","a"], ["var","b"]]]
```

No folding, no inlining, no reassociation: one node per source construct, operands in source
order. That is what lets two independent implementations agree byte for byte (VER04r 10.3).

## The eighteen digests

VER04r 1.2 accepts the generator only if ct-compare reproduces **every one of the nine v1
`inputs_sha256` and nine `expected_sha256` values** of `ct-compare-warrant-v1/corpus.json`,
with the same `cases`, `discarded` and `expected_calls` (572,109 cases). How each is made:

```text
  inputs_sha256 / v1_inputs_sha256     SHA-256 of  prefix || every admitted case's binder values
  expected_sha256 / v1_expected_sha256 SHA-256 of  prefix || every admitted case's expected value
        prefix = "warrant-stream-v1\0" u32+suite id u32 #binders (u32+type)* u32+result type
        values = the 2.4 encoders (Bool 00/01, U64 10+8 LE, Bytes 20+u32+bytes, Ok 50, Err 51+name)
        expected = this crate's OWN evaluator run on the lowered `expected` IR of each case
```

The v1 whole-suite digest is computed in the same pass as the per-chunk digests (a second
hasher receives the same bytes), so a full run needs no second walk. The v2 suite digest is
the `warrant-chunks-v1` framing over the chunk digests. They are cross-checked by
**independent means** in `tests/enumeration.rs`:

1. pinned constants, confirmed equal to the bytes of the checked-in v1 corpus;
2. a **hand-written reproduction** in the test: its own descriptor lists written from
   VER04i 2.6, its own encoder, its own reading of the ct-compare models, enumerating with
   `warrant_grammar::domain::ProductIter` and nothing else of this crate;
3. the independent Python **oracle** (`tests/enumeration_parity.rs`): all nine suites, every
   chunk line, byte for byte;
4. the `warrant-grammar` `Evaluator` (the AST evaluator the v1 compiler used) as a differential
   on sampled cases. It wraps `U8` values but, handed a `Nat`, adds exactly (the WD-013 reading
   of the v0 harness); a test pins both behaviours.

The full ct-compare enumeration (572,109 cases) takes about 3 s in the debug profile.

## The evaluator, `wraps` and the pure trace

`expected_calls` and `wraps` are defined on the **pure trace** of a claim body (VER04r 4.4,
3.3 rule 1), never on what a subject returned:

```text
  if b != 0u8 then op(a + 1u8) == model(a, b) else true        binders a, b
        │
        ├─ the guard `b != 0u8` has no subject: EVALUATED (its wraps count), then ONE branch
        ├─ `op(..)` is one call; its pure argument `a + 1u8` is EVALUATED (wraps count)
        └─ a node with a subject beneath it is never evaluated, so never counts
```

`admit` counts once per **product ordinal**: it is evaluated once per prefix tuple of the
binders it reads (`select_picks` reads `a` and `b`, not `choice`), and that count is multiplied
by the number of ordinals the prefix stands for. There is no memoisation, so the count cannot
depend on cache state or on suite order; a test runs a claim alone and in its document and
compares. For a v0 document `wraps > 0` is `W0701` (rule 3.3.2); rule 3 is S3a's static check.

## Chunks and admission

A suite is cut into chunks of `chunk_size` *product ordinals* (65,536 by default). Any chunk
can be enumerated alone: `first_admitted` is O(1) from a bit table over the prefix tuples, and
the chunk walk starts at `unrank_product(c * chunk_size)` (the S2 API). A test enumerates every
chunk of four ct-compare suites alone, last chunk first, and compares it with the chunk made
after its predecessors. The slim base set follows the 5.6 formula (`k = ceil(m/14)`,
`o = FNV-1a-64(id) mod k`); ct-compare never exceeds four chunks, so the stride is exercised on
a synthetic document at `chunk_size 1024` (64 chunks per suite), where it is also compared with
the oracle. **Witness chunks and faults are S3c**: `SuiteResult::slim_set(witnesses)` is the
seam, taking them as a parameter.

## The census (VER04r 16.2)

`census::census_document` reports, per claim, the first thing that blocks emission at `ir.core`
(a level, a VER04r generation rule, a cost bound, an over-budget product) and, independently,
which of the five 16.2 descriptor items its binders touch. Over the **eleven adopters that bind
`gen_evidence`** (profile `max_ir_cost 65536`, so no adopter is blocked by a cost it would
simply declare), as of this slice:

| adopter | claims | emittable at `ir.core` | first blockers |
|---|---:|---:|---|
| ct-compare | 9 | **9** | none |
| gf256 | 38 | **38** | none (declares `max_ir_cost 32768`) |
| x25519 | 8 | 6 | `ir.fixed` 2 |
| logic-gates | 88 | 28 | `ir.fixed` 44, `ir.data` 16 |
| hash-functions | 56 | 4 | `ir.fixed` 35, `ir.data` 17 |
| lzw | 21 | 4 | `ir.fixed` 17 |
| arithmetic | 63 | 0 | `ir.data` 63 |
| wasm-leb128 | 42 | 0 | `ir.data` 37, `ir.fixed` 3, `W0701` 2 |
| vos-sched-any | 8 | 0 | `ir.data` 8 |
| vos-sched-fp-bitmap | 13 | 0 | `ir.data` 13 |
| zeroize | 9 | 0 | `inout` 9 |
| **total** | **355** | **89** | `ir.data` 154, `ir.fixed` 101, `inout` 9, `W0701` 2 |

Only **ct-compare and gf256 are emittable whole** (a document is emitted whole or not at all).
For S10's decisions on the five divergences of VER04r 16.2: **no adopter binder is a `nat_count`,
a list of byte strings (items 1 to 3 do not occur), item 4 (a signed `fx_edge`) occurs in 7
`wasm-leb128` claims (`I64`), and item 5 (a 16-bit `fx_edge`) in 2 `lzw` claims** (two
`U16`-sized binders demoted by the budget); every one of those claims is blocked by `ir.fixed`
or `ir.data` first, so S10 decides them without a regression. The two `W0701` are
`wasm-leb128` claims with a subject call under an impure guard (VER04r 4.4). Run the report with
`cargo test -p coding_adventures_warrant_corpus_gen --test census -- --nocapture`.

## Fail closed

A construct beyond `ir.core` refuses the **whole document**, naming the level:

```text
  level [ir.fixed] at `mix`: fixed-width built-ins need ir.fixed
```

`ir.fixed` (integer built-ins), `ir.data` (tuples, records, `match`, `Option`, lists),
`ir.seq` (reserved), and the features `inout`, `ensures`, `vectors`, `assume` rows. Nothing
is skipped and nothing is written. `census` reports, per operation, model, claim and
example, what would block it, without failing the document.

## The oracle parity test, and its limits

`python/warrant-oracle` is a second reader that shares no code with this crate. The
lowering is compared with it in `tests/oracle_parity.rs`:

1. `tests/fixtures/dump_oracle_lowering.py` runs the **oracle's own** `lower_document` and
   cost model on four documents (ct-compare, gf256, and two synthetic documents that
   write every `ir.core` construct) and prints, as canonical JSON, a *parity view*: every
   model, operation, claim and example as IR, plus worst-path costs.
2. The four `*.oracle-lowering.json` files next to it are that output, committed.
3. The Rust test renders the same view from its own lowering (`view::oracle_view`) and
   demands the two be **byte-identical**.
4. Negative controls take the correct Rust lowering, mis-lower one thing on purpose (`==`
   as `!=`, `a - b` as `b - a`, a dropped refinement, swapped subject arguments, a
   right-folded `and`, swapped reference sides, reversed model order) and require the
   parity to break.

Regenerate or re-check the fixtures with:

```bash
python3 code/packages/rust/warrant-corpus-gen/tests/fixtures/dump_oracle_lowering.py          # write
python3 code/packages/rust/warrant-corpus-gen/tests/fixtures/dump_oracle_lowering.py --check  # verify
```

**Limits.** What parity does and does not prove:

- The committed JSON is a snapshot: the Rust test does not run Python. The oracle package's
  `BUILD` therefore runs `dump_oracle_lowering.py --check`, so an oracle change re-verifies
  the snapshots; S4 part 2 closes the gap by making the oracle verify the generator's
  `corpus.json` directly.
- The lowering comparison covers lowering, op-row `admit` and costs. Enumeration and digests
  are compared separately (below); the `Ty` annotation on `Arith`/`Neg` nodes is exercised by
  the synthetic `wraps` counts, which the oracle reproduces.
- Parity catches transcription slips, not a **shared misreading** of VER04/VER04j: `lower.rs`
  mirrors the oracle's `lower.py`, so both can be wrong the same way. The grammar checker
  (`warrant-grammar`, run first on every document) is the third opinion.
- Four documents agree; that is not proof for every document.

### The enumeration parity test

`tests/fixtures/dump_oracle_enumeration.py` runs the oracle's `emit` at `full` depth and prints
per suite the counts, descriptors, labels, v1 and v2 digests, slim set and **every chunk line**;
`tests/enumeration_parity.rs` renders the same view from `enumerate_document` and demands byte
identity. The oracle's slim set is the base set only (witnesses are S3c / S4 part 2).

## What never reaches a corpus

Labels, `bound` rows, `assume ... none`, and `input` declarations including their
`rejects` conditions are not part of a v2 corpus (as in v1 and the oracle); they are read
and ignored, not refused.

## Known gaps (deferred to S3c, S4 part 2 and S10)

- The S3a review gaps L5 (a claim with no subject call is now `W0701`), L6 (`classify` keeps
  the first typing pass) and L8 (the profile reader refuses repeated `fault`/`slim_chunk` lines,
  an empty token, `budget 0` and `slim_stride 0`) are closed. The oracle's own reader still
  accepts them until S4 part 2.
- Faults, controls, witness chunks, blind claims and every file of a fixture are S3c.
- The oracle admits IR nesting of 65 (off by one against the limit of 64); a note for S4 part 2.
- A v1 `U8`/`U64` binder resolves to `Fx(U8)`/`Fx(U64)` in the S2 API; this crate names the
  descriptors `u8_all`/`u64_edge` as the oracle does, whatever the document version (VER04r 5.1
  is silent on which name a narrow `fx_full` takes).
- Examples are lowered (S3a) but not evaluated: `cases.tfv` is S3c.

## Tests

`cargo test -p coding_adventures_warrant_corpus_gen` runs the unit tests (every module), the
doctest and the integration tests:

| file | what it pins |
|---|---|
| `enumeration` | the 18 digests and counts, the hand-written reproduction, the grammar-`Evaluator` differential, chunk structure and independence, **seeded-wrong negative controls** (a wrong evaluator op, a reordered enumeration, an off-by-one discard, a wrong wrap: each changes a digest), hand-computed `wraps` and call counts, slim stride, every refusal |
| `enumeration_parity` | enumeration, `wraps`, descriptors, digests and every chunk line equal the oracle's, on ct-compare and two synthetic documents |
| `census` | the census over the eleven adopters and one synthetic hit per 16.2 item |
| `limits` | the resolver, planner and total-work refusals |
| `oracle_parity`, `lowering`, `fail_closed` | S3a: lowering parity, ct-compare against the v1 corpus, one test per refused construct, the `generate` stub |

The full ct-compare enumeration is checked several times per run but computed once
(`OnceLock`); the whole suite takes about 20 s in the debug profile. Line coverage of `src/`
is above 95% (measured with `-C instrument-coverage`); what is left are defensive refusals
that the checker and the planner rule out first, and S3a's `view` error paths.

Regenerate or re-check the enumeration snapshots with:

```bash
python3 code/packages/rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py          # write
python3 code/packages/rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py --check  # verify
```

The oracle package's `BUILD` runs both `--check` commands.

## Specification

[VER04r](../../../specs/VER04r-warrant-corpus-v2.md): sections 2 (types and streams), 3
(arithmetic), 4 (the IR and its cost), 5 (domains, chunks, slim), 8.4 (`corpus.profile`), 13
("Slice S3 in detail"), 16.2 (the five divergences) and 16.3 to 16.5 (decisions and findings).
