# warrant-corpus-gen

The **Warrant corpus v2 emitter** ([VER04r](../../../specs/VER04r-warrant-corpus-v2.md),
slice S3). It reads a `.warrant` specification and will write the language-neutral
`warrant-corpus-v2` fixture that eighteen language lanes run against their own libraries.

> **Status: slice S3a of four.** This crate currently contains the front half of the
> generator: the canonical JSON writer, the `corpus.profile` reader, lowering to the
> `ir.core` model IR with the level refusals, the single wrapping rule, and worst-path
> cost accounting. The evaluator, enumeration and digests (S3b), the faults, emission and
> `--check` (S3c) and the committed ct-compare fixture (S3d) are not here yet.
> [`generate`](src/lib.rs) is a stub that **fails closed**: it validates what S3a can
> validate and then refuses with `code == "unimplemented"`. It never emits a partial corpus.

## How it fits

```text
  code/specs/<pkg>.warrant
         │  parse + check            coding_adventures_warrant_grammar  (S2 public API)
         ▼
  WarrantFile ── lower ──▶ ir.core IR ── cost ──▶ ir_cost, wraps rules       S3a (here)
         │                     │
         │                     ├─ evaluate, enumerate, 18 digests, census ──── S3b
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

**Limits.** The committed JSON is a snapshot: the Rust test does not run Python, so if the
oracle's lowering changes, this test stays green until someone re-runs the script (it is
not in `BUILD`, because that would need a declared build edge from this Rust crate to the
Python package; S4 part 2 closes the gap by making the oracle verify the generator's
`corpus.json` directly). The comparison covers lowering and cost, not enumeration or
digests (S3b). The oracle and this crate agree on four documents, not on every possible
one; the synthetic documents were written to hit every rule of `lower.rs`.

## Tests

`cargo test -p coding_adventures_warrant_corpus_gen` runs the unit tests (every module),
the doctest, and four integration test files: `oracle_parity` (the comparison and its
negative controls), `lowering` (ct-compare against the committed v1 corpus, gf256 costs,
the 26-tag set, operand-order pins), `fail_closed` (one test per refused construct class
plus `census` and the `generate` stub). Coverage of `src/` is about 98% of lines; what is
left is a handful of defensive refusals the checker rules out first.

## Specification

[VER04r](../../../specs/VER04r-warrant-corpus-v2.md): sections 2 (types), 3 (arithmetic),
4 (the IR and its cost), 8.4 (`corpus.profile`), 13 ("Slice S3 in detail") and 16.3 (the
owner's decisions D8 to D11).
