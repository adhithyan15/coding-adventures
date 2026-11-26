# warrant-corpus-gen

The **Warrant corpus v2 emitter** ([VER04r](../../../specs/VER04r-warrant-corpus-v2.md),
slice S3). It reads a `.warrant` specification and will write the language-neutral
`warrant-corpus-v2` fixture that eighteen language lanes run against their own libraries.

> **Status: slice S3c of four.** The generator is complete for `ir.core` documents:
> everything of S3a and S3b (canonical JSON, profile reader, lowering, cost, the evaluator,
> enumeration, the 18 ct-compare v1 digests, the census) plus, in S3c, the **fault IR** and
> the **witness search**, **control rows** with the three claim states, **slim witness
> chunks**, the emission of `corpus.json`, `chunks/*.txt` and `cases.tfv`, a byte-for-byte
> `--check`, and the `warrant-corpus-gen` command line. `generate` returns a whole corpus
> or an error; it never emits a part of one. The committed ct-compare fixture directory is
> slice S3d (with or after the oracle's `verify-v2`, VER04r 16.3 [D10]); nothing under
> `code/specs/fixtures/` is written by this crate's tests.
>
> **S10b** (`ir.fixed`): the 35 built-ins of VER04r Appendix A.1 are lowered, typed, given
> `requires` and costed, and reproduce the oracle's lowering dump byte for byte (see
> "`ir.fixed` lowering parity").
>
> **S10t-2** (`ir.tuple`): tuples, `project` and `match` on a `Result` are lowered, typed, costed,
> evaluated and emitted, and the generator reproduces the oracle's four `tuple-forms-v1` dumps byte
> for byte (see "`ir.tuple` (S10t-2)").
>
> **S11b** (`ir.data`, generator half of the lowering): records, variants and `Option` are real types,
> the four tags `some none ctor field` and the four data patterns `some none ctor bool` are lowered, typed
> and costed, `match` works on any scrutinee, `requires` follow the tag, the pattern and the type
> (R9, R29), and the generator reproduces the oracle's `data-forms-v1` lowering dump **byte for byte,
> including the conditional `types` object**, and refuses its 121 documents with the generator code of
> each row (see "`ir.data` (S11b)").
>
> **S11c-2** (`ir.data`, generator half of the evaluation): the four tags and the data patterns are
> **evaluated**, the stream writes `40`/`41` for an `Option`, `64` for a record and `65` for a variant, TFV
> writes `L0`/`L1 x`/`L2 S<ctor> L<k> ..` (and refuses a 65th container, R99), the `variant`, `option` and `tuple`
> (record) descriptors are described with their demotion paths [D54], the corpus carries `types` rows, the two data
> faults `field-zero` and the record, `Option` and variant payloads of `missing-error` are instantiated, **every fault
> row is typed** (R67, R94, R96 to R98), the generator's `ir.data` gate is open (`SHIPPED_GATE` holds `ir.data`) and
> `generate` accepts a document that needs it. It reproduces all six dumps of the oracle's `data-eval-v1` **byte for
> byte** (see "`ir.data` evaluation (S11c-2)").
>
> **S10c-2** (`ir.fixed`, generator half): the 35 built-ins are **evaluated**, the domains of D21,
> D25 and D26 are described (`nat_shift`, the signed and the 16-bit `fx_edge`), the `ir.fixed` fault
> templates are evaluable, and `generate` emits `ir.fixed` documents. The generator reproduces the
> five dumps of the oracle's `fixed-eval-v1` (lowering, evaluation, faults, values, domains) byte for
> byte (see "`ir.fixed` evaluation (S10c-2)"). The **fault gate was opened by the flip (S10c, 0.6.0)**:
> `SHIPPED_GATE` was `ir.core`, `ir.fixed` and `ir.tuple` (and holds `ir.data` since S11c-2), so a control that needs `ir.fixed` or `ir.tuple` has a
> witness chunk, and the ct-compare `eq_u64/ignore-high-bit` control is live.

## How it fits

```text
  code/specs/<pkg>.warrant
         │  parse + check            coding_adventures_warrant_grammar  (S2 public API)
         ▼
  WarrantFile ── lower ──▶ ir.core IR ── cost ──▶ ir_cost, wraps rules       S3a
         │                     │
         │                     ├─ evaluate, enumerate, 18 digests, census ──── S3b
         │                     ├─ faults, witnesses, controls, emit, --check ── S3c (here)
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
| `profile` | the strict reader of `corpus.profile` (VER04r 8.4): LF only, only an empty line is skipped, canonical decimals, `max_ir_cost` the only way to raise the cost bound, `accept_blind <op> <fault>` and `omit_example <id>` lines |
| `types` | the small type language of `ir.core` (`Bool Int Nat U8..I64 Bytes Bytes<N> Result<T,E>`) and, since S10b, the IR-only `Option<T>` of the `ir.fixed` built-ins |
| `patterns` | S10t-2 and S11b: the ten `match` patterns (six of A.1, four data patterns of A.2), the usefulness check over `Result`, `Option`, `Bool`, tuples, records and variants, the corpus errors (code `ir`) |
| `ir` | the 26 `ir.core` tags of VER04r Appendix A and their JSON form; S10b adds `Ir::Fixed`, the 35 `ir.fixed` built-ins of A.1 (evaluated since S10c-2, on the width `Ir::Fixed::at` records), and `ir::levels`, the tag part of `requires` (R9) |
| `lower` | syntax-directed, unoptimised lowering with a small annotating typer; the level refusals; `census` |
| `corpus_check` | CV2-F17: the document check that `Int` is named in no record or variant field, op signature or binder (VER04r 2.1, `W0701`); `generate` runs it after lowering and before costing, as the oracle's `derive` does; the lowering still accepts `Int` |
| `analysis` | the pure-guard rule (4.4) and the v0 static arithmetic guard (3.3 rule 3) |
| `arith` | the single wrapping rule (3.2): `wraps`, the 2^127 window, `MAX_IR_VALUE` |
| `cost` | worst-path `ir_cost`, `max_ir_cost`, total IR work (4.2 rule 6) |
| `view` | the "parity view" of a lowered document, compared with the oracle (`oracle_view` for `ir.core`, `oracle_view_v1` for the S10 dump with `requires`) |
| `value`, `eval` | S3b: the `ir.core` evaluator (own implementation over this crate's IR): exact `Nat`/`Int` in the 2^127 window, wrapping fixed-width arithmetic counted as `wraps`, short-circuit `and`/`or`/`implies`/`if`, `MAX_IR_VALUE` checked before allocation, and `trace`, the pure-trace walk behind `expected_calls` and `wraps` |
| `domains` | S3b: descriptor JSON, labels and sizes over the S2 `resolve_binders`/`Domain` API (S10c-2: `nat_shift`, `fx_edge`, the `demoted` paths; refuses only a list of byte strings); the consumer-side `descriptor_size`; value and encoding caches |
| `stream` | S3b: the 2.4 value encoders (a `Nat`, unsigned or signed value outside its type is `W0701`, `a value is outside the range of <type> (VER04r 2.3)`, the oracle's sentence since CV2-F17), the context prefix, the v1/v2 header, `warrant-chunks-v1` framing, FNV-1a-64, the slim base set |
| `suite` | S3b: one suite: product enumeration (`unrank_product`, last binder fastest), admission evaluated once per prefix tuple, chunks, counts, suite and v1 whole-suite digests, slim set |
| `enumerate` | S3b: a whole document at `full` depth, the total-work bounds, the v0 divergence guard rule 2, and the enumeration parity view |
| `faults` | S3c: the fault IR (`pre`/`post`/`replace`, `Expr::Core` or `Expr::Data` for a level not implemented), the registry of the portable names instantiated by signature, the stand-in rule and the witness search (`ClaimState`: witnessed, blind, unproven, deferred) |
| `tfv` | S3c: the `examples` rows of `corpus.json` and `cases.tfv` (TFV v1), evaluated by this crate's own evaluator |
| `emit` | S3c: `generate`: profile, faults, enumeration with a per-suite witness search, slim sets with witness chunks, control rows, `corpus.json`, chunk files |
| `fsio` | S3c: the only file-system writer: path guard, atomic write (temp + rename, `corpus.json` last), symlink refusal, `check_corpus` with a located diff |
| `cli` | S3c: `run(args, out, err)` behind `src/bin/warrant-corpus-gen.rs` |
| `census` | S3b: per claim, why the generator can or cannot emit it, and which VER04r 16.2 items its binders touch |

## Usage

### The command line

```bash
cargo run -q -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
    --warrant code/specs/ct-compare.warrant \
    --profile path/to/corpus.profile \
    --bindings code/packages/rust/ct-compare/evidence.bindings \
    --out path/to/ct-compare-warrant-v2            # add --check to compare instead of write
```

Run it from the repository root: `--warrant` must be a repo-relative path (no `..`, no `\`, no
leading `/`) because it is recorded verbatim as `spec` in `corpus.json`. `--bindings` is optional:
when given, its `faults=` lists must equal the profile's `fault` lines per operation. Exit codes, the independent oracle's convention (1 = the two sides differ, 2 = refused or error):

| exit | meaning |
| --- | --- |
| `0` | success: written, or `--check` found the directory byte-identical |
| `1` | drift: `--check` ran to the end and the committed bytes differ from the regenerated ones (the only exit 1) |
| `2` | refused or error: any `GenError` that refuses generation (a level or feature beyond `ir.core`, a profile or checker error, a rule or limit, bindings whose `faults=` lists differ from the profile's), an unreadable input, a refused path, an I/O error writing the output, or a usage error |

The status is chosen by where the error arose, not by the `GenError` code alone: the `drift` code is
also raised by the bindings cross-check, which is a refusal (exit 2). A file missing from `--out` is
one of the listed drift findings (exit 1). The binary needs `std::process::exit` to say 2 (an `Err`
from `main` always exits 1), declared as `proc:signal` in `required_capabilities.json`.
It is a `[[bin]]`, not an example, so the
dev-only `test-hooks` feature (seeded-wrong evaluators) is never linked into it.

**Write mode** validates every path first, refuses a symbolic link and a `chunks/` that holds a
file the corpus does not own (it never deletes anything), writes each file to a temporary name and
renames it, and renames `corpus.json` last. **`--check`** regenerates in memory and compares
`corpus.json`, `cases.tfv`, every chunk file and the exact file set of `chunks/` byte for byte,
reporting each difference as `file: differs at line L, column C (byte B)`; it writes nothing.

**The committed ct-compare fixture.** `code/specs/fixtures/ct-compare-warrant-v2/` is this
command's output for ct-compare (its README has the exact command), and this package's `BUILD`
runs it with `--check`. `corpus.json` records `generator` `{"name", "version"}`, so bumping this
crate's version fails that line until the fixture is regenerated and the diff reviewed (VER04r
16.8 [D13]). `--check` does not compare the authored files of the directory, but
`profile_sha256` in `corpus.json` is the digest of the `--profile` file, so the build passes the
fixture's own `corpus.profile` and an edit to it fails the line ([D14]).

**The committed x25519 fixture.** `code/specs/fixtures/x25519-warrant-v2/` (the second adopter, CV2
S10) is checked the same way by a second `--check` line in `BUILD` and `BUILD_windows`; its README
has the command and what it holds (eight suites, five faults).

**The committed lzw fixture.** `code/specs/fixtures/lzw-warrant-v2/` (the third adopter, CV2 S10) is
checked by a third `--check` line in `BUILD` and `BUILD_windows` (the line runs with `--release`: about 25 s, against about 3 min in a debug build). It is
the first fixture whose profile carries `omit_example` lines (the six `bomb_*` examples, two
`not-op-eq-pure` and four `value-over-limit`) and a raised `max_ir_cost` (16384); its README has the
command and what it holds (21 suites, 17,563,819 cases, five faults).

**The committed wasm-leb128 fixture.** `code/specs/fixtures/wasm-leb128-warrant-v2/` (the fourth adopter, CV2
S10) is checked by a fourth `--check --release` line in `BUILD` and `BUILD_windows` (2 min 11 s measured; about
20 min in a debug build, so the line is release, VER04r D38; one line, not sharded). It is the first fixture
with tuple values, `match` on a `Result` and the composite faults (`drop-consumed`, `ok-as-err`,
`off-by-one-consumed`, the tuple-payload `missing-error`): 44 suites, 46,566,930 cases, 740 chunks, 20 faults, no
profile knob beyond the fault lines. Its spec was changed first (VER04r D28: the two `W0701` claims split, 42 to
44 claims) and `tests/census.rs` pins the result (364 claims, 242 emittable since VOS-039c added seven `vos-sched-any` claims); its README has the command and
what it holds.

### As a library

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

`census::census_document` reports, per claim, the first thing that blocks emission (a level, a
VER04r generation rule, a cost bound, an over-budget product) and, independently, which of the 16.2
descriptor items its binders touch that are still undecided (since S10c-2 only items 1/2, a list of
byte strings). Over the **eleven adopters that bind `gen_evidence`** (profile `max_ir_cost 65536`, so
no adopter is blocked by a cost it would simply declare), as of S11b:

| adopter | claims | lower (`ok`) | first blockers |
|---|---:|---:|---|
| ct-compare | 9 | **9** | none |
| gf256 | 38 | **38** | none (declares `max_ir_cost 32768`) |
| x25519 | 8 | **8** | none |
| logic-gates | 88 | **88** | none (S11b: `Option` is a real type; the document still has examples that need an `omit_example` line) |
| hash-functions | 56 | 48 | `err-value` 2, `W0701` 6 (was `ir.data` 21: 19 lower, 2 use a bound error value, [D55]) |
| lzw | 21 | **21** | none |
| arithmetic | 63 | **63** | none (S11b: records, the payload-free variant and a `match` on it lower; was `ir.data` 55) |
| wasm-leb128 | 44 | **44** | none (was 42 claims, 40 emittable, `W0701` 2, before the VER04r D28 split) |
| vos-sched-any | 15 | 5 | `W0701` 10 (what `ir.data` hid: a claim that calls no op, an impure guard) |
| vos-sched-fp-bitmap | 13 | 11 | `W0701` 2 |
| zeroize | 9 | 0 | `inout` 9 |
| **total** | **364** | **335** | `W0701` 18, `inout` 9, `err-value` 2 |

The table is pinned per adopter in `tests/census.rs` (the generator's `PINS` and the oracle's `ORACLE_PINS`).
**Since S11b the census counts what lowers**: `ok` is no longer "`generate` can emit it" (S11c-2 evaluates `ir.data`;
arithmetic still does not generate whole, for its nine-binder claim, VER04r 5.3).
History of the totals: before S10b `ir.data` 154,
`ir.fixed` 101, `inout` 9, `W0701` 2; S10b `ir.data` 79, `ir.fixed` 99, `ir.tuple` 71, `W0701` 8 (89
emittable); S10t-2 (`ir.tuple` lowers) `ir.data` 100, `ir.fixed` 130, `W0701` 8, `inout` 9, `16.2` 3 (105
emittable); **S10c-2 evaluates `ir.fixed` and describes the signed and 16-bit `fx_edge` and `nat_shift`,
so no claim is blocked on `ir.fixed` or `16.2` any more** (238 emittable); **the wasm-leb128 adopter
splits its two `W0701` claims (VER04r D28: 42 to 44 claims)**, so 357 claims, 242 emittable, `W0701` 6; **S11b
lowers `ir.data`** (the name is never a lowering refusal again, R28): 364 claims, 335 lower.

The generator's census now **equals the oracle's lowering census exactly** (oracle numbers from running its
lowering: `ok`, `W0701` for a lowering refusal, the level or feature name for a refused suite) for **ten of the
eleven adopters**, arithmetic and logic-gates included (VER04r 16.14 R60, [D53]: CV2-F10 and R13 (d)'s arithmetic
difference are closed). The one difference left is a generator-side refusal of a feature, never a level
(`tests/census.rs`): **zeroize** (`inout` parameters and `ensures`: the oracle lowers all 9, the generator refuses
9 until a later slice). The oracle's *evaluation* census still counts a unit that lowered and needs `ir.data` under
`ir.data` (arithmetic 8 `ok` and 55, logic-gates 85 and 3) until S11c-1 evaluates it. The `W0701` claims of
hash-functions (6), vos-sched-any (10) and vos-sched-fp-bitmap (2) have a subject call under an impure guard or call no
operation (VER04r 4.3, 4.4), refused at lowering in both tools.

Only a **list of byte strings** (16.2 items 1 and 2) is still refused by the domain stage; no adopter
binder is one, and no adopter binder was a `nat_count` (item 3). Of the decided items, item 4 (the
signed `fx_edge`, `I64`) occurs in 8 wasm-leb128 claims (7 before the D28 split) and item 5 (the 16-bit `fx_edge`) in 2 lzw
claims, both now emittable. Run the report with
`cargo test -p coding_adventures_warrant_corpus_gen --test census -- --nocapture`.

## Fail closed

A construct no level of this tool implements refuses the **whole document**, naming the level or feature:

```text
  level [ir.list] at `m`: a list literal needs the reserved ir.list level
```

`ir.list` (a list type, a list literal, `slice to_list to_bytes replicate`, R27, R28), `ir.seq` (`fold map all any
count range`), the feature names `fixed-cast` (`fixed<N>(b)`) and `err-value` (an arm that uses the error value an
`Err(e)` pattern bound, [D55]), and the features `inout`, `ensures`, `vectors`, `assume` rows. Nothing is skipped and
nothing is written. `ir.fixed`, `ir.tuple` and `ir.data` are **lowered and evaluated** (`ir.data` since S11c-2, which
removed the guard S11b kept: R37).
`census` reports, per operation, model, claim and example, what would block it, without failing the document.

## `ir.data` (S11b)

Records, variants (with and without payload) and `Option<T>` are real types; the four tags `["some", e]`,
`["none", "T"]` (typed: the payload type the position expects), `["ctor", "Name", [e..]]` and `["field", e, i]`, and
`match` on **any** scrutinee with the patterns `["some", p]`, `["none"]`, `["ctor", "Name", [p..]]` and
`["bool", v]` (VER04r Appendix A.2, [D46] to [D48], R27 to R39) are lowered, typed and costed (`some`, `ctor` and `field`
`1 +` their operands, `none` 1, `match` `1 +` the scrutinee `+` the worst arm). Written from the spec and compared byte for
byte with the oracle's committed dumps:

- **`requires`** by tag, by pattern and by type (R9, R29): an `Option`, a record or a variant anywhere in the type of a
  binder, a parameter, a result or a sub-expression adds `ir.data`, except the static type of a producer node that is
  directly the first operand of `unwrap_or` (the *intermediate use*, so no committed `requires` moves); a `match` is
  `ir.tuple` on a `Result` and `ir.data` otherwise, and `ir.data` as well for a data pattern at any depth.
- **Typing** (R39, R51, R52, [D60]): `Some(e)` is `Option<type(e)>`; an `Option` flows and unifies by its payload,
  nothing lifts through a tuple, a record or a variant; **the width is part of the type**, so `Ok(concat(a, b"0102"))`
  is a `Result<Bytes<3>,E>` (the S10t over-refusal R13 (c), turned round); a `None` is typed by the **expected type of
  its position and by nothing else**.
- **Refusal order** (R35, R53, R54): the declaration limits first (`limit`: 64 constructors, 8 fields, 64 error cases); a
  reserved level or feature (`ir.list`, `ir.seq`, `fixed-cast`, `err-value`) beats a typing error of the same unit (the
  checker's error is not the answer: `lower` reads the unit's source for the first reserved construct, as the oracle's
  `_scan` does); an error never hides a refusal an earlier-declared unit holds; a document with refusals and no other
  error is named by the refusal of the unit declared on the earliest line.
- **Corpus errors** (code `ir`) over every scrutinee: fall-through, an unreachable arm (found first), the same bind twice,
  a bind that shadows a binder, a parameter, `result` or an outer bind, and a bind named like a model, an operation, an
  error type or a reserved built-in (the oracle's sentence for it since S11b).
- **Projection errors** (`e.i` on a non-tuple, `e.name` on a non-record) are typing errors, no longer a level.

`tests/data_parity.rs` lowers `data_forms_v1.warrant` and demands `oracle_view_v1` (with its conditional `types` object:
records and variants only, keys sorted as canonical JSON sorts them) equal the committed
`data_forms_v1.oracle-lowering.json`, and refuses each of the 121 documents of `data_forms_v1.oracle-refusals.json`
with the generator code of its row: `level` 39 (22 `ir.list`, 10 `ir.seq`, 4 `fixed-cast`, 3 `err-value`), `ir` 28 with the
oracle's sentence, `limit` 4, and 50 typing errors that are never a level and never a corpus error (the generator's
checker or typer, R13 (a)). `tests/data_lowering.rs` states each rule once, small.

The two off-by-one and scalar fault refusal dumps no longer have a lowering carve-out: an `Option` or a variant signature
lowers, so the five rows that the lowering used to stop (`saturate-add/an-option-result`, `saturate-sub/an-option-result`,
`missing-error/an-option-payload`, `missing-error/a-variant-payload`, `missing-error/an-option-result`) and the
`off-by-one-consumed` `an-option-result` reach `instantiate` and carry the oracle's reason text with the code `fault`.

## `ir.tuple` (S10t-2)

The tuple type `(T1,..,Tn)` (2 to 8), the literal, `project` (`t.i`, `t.0.1`, `f(x).1`) and `match` on
a `Result` with the patterns `ok err err_any tuple bind wild` (VER04r A.1, D20, D35): lowering,
typing, cost (`1 +` the scrutinee `+` the maximum over the arms), evaluation (exactly one arm), the
`tuple` descriptor, stream tag `64`, TFV `L<n>` and JSON arrays. A `match` arm is a guard for call
counting, wraps, D19 and v0 rule 3; a subject call in an arm under a subject scrutinee is `W0701`.
`Err(e)` is `err_any` and an arm that uses `e` is the feature `err-value` ([D55]); a `match` on a non-`Result` is `ir.data`
(lowered since S11b).

The corpus errors (a `match` that falls through, an unreachable arm, a duplicate or shadowing bind)
are refused with the generator code `ir` (unreachable first, R12). The S2 checker raises the same
rules (`W0301`, `W0203`); `lower` maps exactly those kinds to `ir`. `concat`, bytes literals and `if`
over sized operands flow to `Bytes<N>` everywhere (A.1).

Faults: `ok-as-err`, `drop-consumed`, `swap-components`, `off-by-one-consumed` [D36] (`requires` `ir.fixed` and `ir.tuple`), `saturate-add`, `saturate-sub` and `shift-mask` [D51] (`ir.fixed`, S11f-2), `missing-error` for every scalar, `Bytes` or tuple payload [D50], and the bare
and `Ok` forms of `flip-result-bit` / `drop-last-result-byte`, in the exact 8.2 JSON; a fault's
`requires` is the sorted union of its template's levels and its op's types (R9). Fault evaluation is
gated by `SHIPPED_GATE`, open to `ir.core`, `ir.fixed` and `ir.tuple` since the flip (S10c, 0.6.0); under a
narrower gate passed to `emit::generate_gated` (the closed gate is `&[Level::Core]`) the controls of a tuple
fault are `deferred`, naming the first unimplemented level (R12).
`ir.fixed` suites still refuse.

`tests/tuple_parity.rs` reproduces `python/warrant-oracle/tests/fixtures/tuple-forms-v1/` byte for
byte: the lowering, evaluation, fault (shipped and `ir.tuple`-evaluated gates) and refusals dumps,
with negative controls. `tests/tuple_forms.rs` pins the rules one at a time.

## `ir.fixed` lowering parity (S10b)

Since slice S10b the lowerer reads the 35 built-ins of VER04r Appendix A.1 (`bit_and` ... `unwrap_or`):
one `Ir::Fixed` node per call, operand types inferred as for `add`, a static type argument written as
the canonical type name in position 1, `Option<T>` a real type (S11b, [D48]; until then confined to the first operand of
`unwrap_or`), `div`/`rem` on two `Nat` operands typed `Option<Int>` (R7), and the
static width of a `from_be`/`from_le` operand part of its type (a bytes literal, a
`concat` of sized operands, an `if` of equal widths: the checker's rule, R51). Every lowered unit records its
`requires` (R9: the sorted union of its tags' levels and its callees' own `requires`, `ir.core` always
present) and each new tag costs `1 +` its operands. A **v0** document refuses the built-in names (they
are version scoped).

`tests/fixed_parity.rs` lowers the oracle's `fixed_forms_v1.warrant` (all 35 built-ins at every width)
and demands the text of `oracle_view_v1` equal the oracle's committed `fixed_forms_v1.oracle-lowering.json`
(schema `warrant-lowering-parity-v1`) **byte for byte**. Both files are read from
`python/warrant-oracle/tests/fixtures/fixed-forms-v1/`, not copied, so an edit to either reruns it (the
build tool's `rust/warrant-corpus-gen` row); the oracle's own BUILD checks the dump against its
`dump-lowering` subcommand. `tests/fixed_lowering.rs` pins the refusals, `requires`, cost and naming on
small documents, including a deep-nesting regression for the width rule (the oracle's first version
re-lowered the operand at every level and was exponential).

## `ir.fixed` evaluation (S10c-2)

Since S10c-2 the evaluator runs the 35 built-ins and `generate` emits `ir.fixed` documents
(`lower_document_with_fixed` is gone: `lower_document` is the one path). The bit work is
`warrant-grammar`'s `fixed` module (which agrees with the oracle's independent evaluator on every
operand of the table below); `eval::fixed_builtin` adds the dispatch and the conventions:

```text
  width         the static type argument (widen wrap try max_value min_value from_be from_le) or the
                first operand's type, which the lowering records in `Ir::Fixed::at` (not in the JSON)
  Option        try checked_* div rem div_trunc rem_trunc at -> `Value::Opt`; `unwrap_or` is its one
                consumer and evaluates both operands (no short circuit)
  shift counts  an exact Nat, compared with the width before any host shift (shl shr -> 0, sar -> -1 or
                0, bit -> false); rotl/rotr reduce n mod w first
  wraps         only add sub mul neg count a wrap [D31]; wrap shl bit_not ... never do
  from_be/le    a runtime width other than the target's is a corpus error (code `ir`), never a truncation
```

**Domains** (VER04r 5.1): `nat_shift` `{"d":"nat_shift","size":73}` [D26] for a `Nat` binder that is
directly the count of `shl shr sar rotl rotr bit` in the claim body (a parenthesised count selects, an
expression of the binder, a `model_call` argument and an `admit` do not: R10; the S2 resolver's
`count_binders` now looks through parentheses); `fx_edge` for the signed 32- and 64-bit leaves [D21]
(`I32` 559, `I64` 623) and for a 16-bit binder the budget demotes [D25] (`U16` 281, `I16` 527, recorded
in `demoted` as `0`, `1`, `0.1`, `0.ok` ...). The S10b refusal of a signed `fx_edge` ("16.2 item 4") is
gone; only a list of byte strings is still refused. `domains::descriptor_size` is the consumer-side
check of VER04r 5.1 (checked sizes, 16 levels, every leaf's legality, the oracle's messages).

**Faults**: the `ir.fixed` templates (`xor-lsb`, `ignore-high-bit`, `flip-result-bit`, the `Bytes` and
`Ok` forms) are ordinary IR now, so the shipped gate (`generate`, open to `ir.fixed` and `ir.tuple` since the
flip) searches them; `generate_gated(.., &[Level::Core])` is the closed gate, under which every fault that
needs `ir.fixed` is `deferred`.

`tests/fixed_eval_parity.rs` reproduces the five dumps of `python/warrant-oracle/tests/fixtures/fixed-eval-v1/`
**byte for byte**: lowering, evaluation (112 suites, every chunk line and digest, `cases.tfv`), faults
(the shipped and the open gate), values (about 9,800 evaluations: every built-in at every width on edge
values and the count rows) and domains (the `fx_edge` and `nat_shift` value lists, and the consumer's
verdicts), with negative controls. The files are read from the oracle's directory, so an edit there
reruns it (the build tool's `rust/warrant-corpus-gen` row). The test never enumerates an adopter
(wasm-leb128 alone is 46 million cases).

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

## Faults, controls and the witness search (VER04r 5.6, 8)

A fault is data. `pre` faults carry a `rewrites` list (all read the original parameters, applied
simultaneously), `post` and `replace` faults one `expr` over the parameters and `result`. For each
control the generator takes the *candidate claims* (claims that call the op) and classifies each:

```text
  stand-in?  reference suite whose subject side is exactly op(binder, ..., binder)
  no                          -> unproven   {"claim", "unproven": true}      stays in required_claims
  yes, level not implemented  -> deferred   {"claim", "deferred": "ir.fixed"} stays in required_claims
  yes: scan every admitted case, lowest one where faulted != expected
         found               -> witnessed  {"claim", "chunk": c}   c joins that suite's slim set
         none                -> blind                              goes to blind_claims
```

Generation fails (`fault`) only if every candidate of a fault is blind, or no claim calls its op.
A profile line `accept_blind <op> <fault>` ([D15], VER04r 16.8) excuses the first: the control row is
emitted with an empty `required_claims` and its control id (`warrant/<module>/negctl/exhaustive/<op>/<fault>`)
is listed, sorted, in `accepted_blind_faults`. The line must repeat a `fault` line, may not repeat, and
naming a fault a claim *can* see is a `profile` error (a stale acceptance excuses nothing); it never
excuses an op no claim calls. ct-compare has none; `python/warrant-oracle/tests/fixtures/accept-blind-v2/`
is the small synthetic corpus both implementations agree on.

### `omit_example <id>` ([D22], [D34], VER04r 8.4)

A profile line `omit_example warrant/<module>/example/<name>` removes one example from `examples`
when the generator can *decide* that it is inexpressible; the profile carries only the id and the
tool computes the reason, written to the conditional `omitted_examples` key
(`[{"id", "reason"}]`, sorted by id, **omitted** when there is none, so no existing fixture moves a byte):

```text
  shape is not  op(args) == pure                      -> not-op-eq-pure    (tested first)
  evaluate: arguments, then the pure side             first error a size error (a Bytes literal or
                                                      concat past MAX_IR_VALUE)  -> value-over-limit
                                                      any other error             -> refuses
  then the D19 check, then the D18 check              a failure                   -> refuses
  completed, and an argument or the pure value is
  over 4,096 bytes (also under Ok)                    -> value-over-limit
  anything else                                       -> profile error: a stale line
```

Without a line, since CV2-F1, an example whose argument or pure value holds a `Bytes` over 4,096 bytes
(itself, under `Ok` or in a tuple) is refused at emission (code `limit`), after the D19 and D18 checks.
A size error while evaluating a callee's `admit` refuses even with a line (it is not `value-over-limit`).

An omitted example is **still lowered** (a level refusal refuses the document), but it is not
evaluated for `cases.tfv`, not encoded and not admit-checked beyond the decision; its shape
refusal (`W0701`) and the guard refusals are deferred to this decision. Without a line the same
example is refused as before. Every example is evaluated in this order (arguments, pure side,
D19, D18), and a `GenError` carries `size` for exactly the two `Bytes` limits (the 2^127 integer
window is not a size error). The oracle's `tests/fixtures/omit-example-v2/` is reproduced byte for
byte by `tests/omit_example.rs`.

`ignore-high-bit` (`ir.fixed`, VER04r 8.2 [D9]) is emitted with `requires: ["ir.fixed"]` and, under
the shipped gate (open since the flip, S10c), a chunk witness: ct-compare's `eq_u64` control is live (chunk 0,
`a = 0`, `b = 2^63`). Under the closed gate (`&[Level::Core]`, used by `tests/fixed_eval_parity.rs` for its
shipped view) the witness is `deferred`.

ct-compare's eight controls come out as the spec predicted: `eq_fixed/ignore-last-byte` has
`blind_claims` `eq_fixed_agrees_w0`, `select_bytes/missing-error` has `select_picks`, and
`select_bytes/swap-choice` has `select_rejects_mismatch`; the other five are witnessed at chunk 0
of every candidate except the deferred one. The tests check this against a brute-force search
written separately in `tests/emission.rs`.

## Capabilities

`required_capabilities.json` declares the crate's file access (targets are the caller-named input files and output directory; the guards are in `fsio.rs`: a name whitelist, a symbolic link present at `chunks/` or a target refused, an O_EXCL temporary, regular-file-only capped reads; concurrent modification of the output directory is unsupported): **read** the specification, the
profile, the bindings and (for `--check`) the output directory; **list** `chunks/` (to refuse and
to report files the corpus does not own); **create** and **write** only below the output
directory (temporary file then rename, within `fsio.rs`), **delete** only that temporary file
after a failed write; **env read** of the arguments and **stdout write** in the CLI. The library's
generator (`generate`) does no I/O; everything is in `fsio.rs`, `cli.rs` and the bin.

## Two refusals (S3e, owner decisions D18 and D19)

- **v0 rule 2 on examples.** In a v0 document an `example` whose evaluation wraps
  (`f(255u8 + 2u8) == 1u8`) is `W0701`, like a claim that wraps; in v1 it is accepted.
- **A call outside the op's `admit`.** A claim whose admitted cases call an op outside that op's
  `admit` (its binder refinement is weaker than the op's) fails with `W0701` naming the claim, the
  op and the first admitted-case ordinal; an example outside it is refused too, and a call whose op
  `admit` reads a parameter whose argument depends on a subject result is refused statically. Nothing is
  discarded silently, and no emitted byte of an accepted document changes. See VER04r 4.5 and 16.9.

## Known gaps (deferred to S10 and later)

- The S3a review gaps L5 (a claim with no subject call is now `W0701`), L6 (`classify` keeps
  the first typing pass) and L8 (the profile reader refuses repeated `fault`/`slim_chunk` lines,
  an empty token, `budget 0` and `slim_stride 0`) are closed, and so is the whitespace-only line
  the reader used to skip (S3d).
- The evaluator is iterative; per-node `has_subject`/`may_fail_or_wrap` are recomputed per case
  on traced suites (a cache is a possible later optimisation, not needed for the bounds).
- `has_subject`/`may_fail_or_wrap` are still recomputed per case on traced suites (review LOW 4,
  not needed for the bounds).
- The oracle admits IR nesting of 65 (off by one against the limit of 64); a note for S4 part 2.
- A v1 `U8`/`U64` binder resolves to `Fx(U8)`/`Fx(U64)` in the S2 API; this crate names the
  descriptors `u8_all`/`u64_edge` as the oracle does, whatever the document version (VER04r 5.1
  is silent on which name a narrow `fx_full` takes).

## Tests

`cargo test -p coding_adventures_warrant_corpus_gen` runs the unit tests (every module), the
doctest and the integration tests:

| file | what it pins |
|---|---|
| `enumeration` | the 18 digests and counts, the hand-written reproduction, the grammar-`Evaluator` differential, chunk structure and independence, **seeded-wrong negative controls** (a wrong evaluator op, a reordered enumeration, an off-by-one discard, a wrong wrap: each changes a digest), hand-computed `wraps` and call counts, slim stride, every refusal |
| `enumeration_parity` | enumeration, `wraps`, descriptors, digests and every chunk line equal the oracle's, on ct-compare and three synthetic documents (including `enum_probe`) |
| `census` | the census over the eleven adopters, the oracle's census beside it, and the settled and open 16.2 items on synthetic documents |
| `fixed_parity`, `fixed_lowering` | S10b: the lowering dump of `fixed-forms-v1`, the refusals, `requires` and cost of the built-ins |
| `fixed_eval_parity` | S10c-2: the five dumps of `fixed-eval-v1` (lowering, evaluation, faults under both gates, values, domains) byte for byte, the shift-count rule, the demotions, negative controls |
| `tuple_parity`, `tuple_forms` | S10t-2: the four dumps of `tuple-forms-v1`, the tuple forms one rule at a time; S11b: the width in the type (R51) |
| `data_eval_parity` | S11c-2: the six dumps of `data-eval-v1` byte for byte (evaluation, both fault gates, values with the consumer's reading of 23 refused JSON forms, domains with the demotion paths, refusals with the `typing` rows), negative controls |
| `data_parity`, `data_lowering` | S11b: the lowering dump of `data-forms-v1` byte for byte (with its conditional `types` object), its 121 refused documents with the generator code of each row, `generate` fail-closed, negative controls; each `ir.data` rule once, small |
| `limits` | the resolver, planner and total-work refusals; the evaluator at the real bounds (64 model frames) on a default test thread; pure subject-argument failures refused at generation |
| `oracle_parity`, `lowering`, `fail_closed` | S3a: lowering parity, ct-compare against the v1 corpus, one test per refused construct, `generate` refusals |
| `emission` | S3c: the eight control rows, the 18 digests re-derived from the emitted `corpus.json`, every digest re-derived from the chunk files, `cases.tfv` equal to v1, chunk files equal to the oracle's, the key set, `corpus.json` pinned by digest, brute-force witness check, seeded-wrong controls |
| `faults` | S3c: the registry, refusals, `bindings` cross-check, simultaneous rewrites, witness chunk outside the stride, unproven claims |
| `check_cli` | S3c: write/`--check`, a flipped byte, a missing chunk file, an extra file, a reordered key, a CRLF, a stale profile, the path guard, symlinks, the binary's exit codes |

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
