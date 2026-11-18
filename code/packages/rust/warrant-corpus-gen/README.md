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
> "`ir.fixed` lowering parity"). They are **not evaluated or emitted**: `generate` still refuses an
> `ir.fixed` document.
>
> **S10t-2** (`ir.tuple`): tuples, `project` and `match` on a `Result` are lowered, typed, costed,
> evaluated and emitted, and the generator reproduces the oracle's four `tuple-forms-v1` dumps byte
> for byte (see "`ir.tuple` (S10t-2)"). `ir.fixed` documents still refuse.

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
| `patterns` | S10t-2: the six `match` patterns, the usefulness check, the corpus errors (code `ir`) |
| `ir` | the 26 `ir.core` tags of VER04r Appendix A and their JSON form; S10b adds `Ir::Fixed`, the 35 `ir.fixed` built-ins of A.1 (lowered and costed, never evaluated), and `ir::levels`, the tag part of `requires` (R9) |
| `lower` | syntax-directed, unoptimised lowering with a small annotating typer; the level refusals; `census` |
| `analysis` | the pure-guard rule (4.4) and the v0 static arithmetic guard (3.3 rule 3) |
| `arith` | the single wrapping rule (3.2): `wraps`, the 2^127 window, `MAX_IR_VALUE` |
| `cost` | worst-path `ir_cost`, `max_ir_cost`, total IR work (4.2 rule 6) |
| `view` | the "parity view" of a lowered document, compared with the oracle (`oracle_view` for `ir.core`, `oracle_view_v1` for the S10 dump with `requires`) |
| `value`, `eval` | S3b: the `ir.core` evaluator (own implementation over this crate's IR): exact `Nat`/`Int` in the 2^127 window, wrapping fixed-width arithmetic counted as `wraps`, short-circuit `and`/`or`/`implies`/`if`, `MAX_IR_VALUE` checked before allocation, and `trace`, the pure-trace walk behind `expected_calls` and `wraps` |
| `domains` | S3b: descriptor JSON, labels and sizes over the S2 `resolve_binders`/`Domain` API; refuses the VER04r 16.2 descriptors; value and encoding caches |
| `stream` | S3b: the 2.4 value encoders, the context prefix, the v1/v2 header, `warrant-chunks-v1` framing, FNV-1a-64, the slim base set |
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
| logic-gates | 88 | 28 | `ir.fixed` 57, `ir.data` 3 |
| hash-functions | 56 | 4 | `ir.fixed` 25, `ir.data` 21, `W0701` 6 |
| lzw | 21 | 4 | `ir.fixed` 17 |
| arithmetic | 63 | 3 | `ir.data` 55, `ir.fixed` 5 |
| wasm-leb128 | 42 | 13 | `ir.fixed` 24, `16.2` 3, `W0701` 2 |
| vos-sched-any | 8 | 0 | `ir.data` 8 |
| vos-sched-fp-bitmap | 13 | 0 | `ir.data` 13 |
| zeroize | 9 | 0 | `inout` 9 |
| **total** | **355** | **105** | `ir.fixed` 130, `ir.data` 100, `inout` 9, `W0701` 8, `16.2` 3 |

**Since S10b a claim is named by the first construct, in source order, that cannot be lowered,
and by `ir.fixed` only when nothing else blocks it** (the built-ins now lower, so a claim that
also uses a tuple or a constructor is named by that level, as the oracle's census names it). The
table above is pinned per adopter in `tests/census.rs`. Before S10b the totals were `ir.data` 154,
`ir.fixed` 101, `inout` 9 and `W0701` 2: tuples, projections and `match` on a `Result` moved from
`ir.data` to `ir.tuple` (VER04r 4.1, D20, D35), and six `hash-functions` claims
(`fnv1a_*_matches_both_models`, `djb2_*_matches_both_models`) that an `ir.fixed` built-in used to
hide are `W0701`, a subject call under an impure guard (VER04r 4.4), like the two `wasm-leb128`
claims of [D28].

**S10t-2 lowers `ir.tuple`, so no claim is blocked on it any more** (it was 71 claims): they are
emittable, `ir.fixed`, `ir.data`, or (3 `wasm-leb128` claims) a 16.2 item 4 descriptor refusal. The
generator's census now equals the oracle's exactly for ct-compare, gf256, logic-gates, lzw and
x25519; the two that differ do so for non-tuple causes (`tests/census.rs`): wasm-leb128 (oracle 18
`ir.core`: 13 here + 3 `16.2` + 2 `W0701` impure guard applied at lowering) and arithmetic (10 claims
over record/variant-typed ops, which the oracle lowers by counting `ir.data` in `requires` and the
generator refuses until S10d). hash-functions' 6 `W0701` and zeroize's `inout` are also
generator-only refusals.

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

`ir.fixed` (the integer built-ins), `ir.tuple` (tuple types and literals, projection, `match` on a
`Result`), `ir.data` (records, variants, `Option` and `List` types, lists, `match` on anything
else), `ir.seq` (reserved), and the features `inout`, `ensures`, `vectors`, `assume` rows. Nothing
is skipped and nothing is written. `census` reports, per operation, model, claim and
example, what would block it, without failing the document.

## `ir.tuple` (S10t-2)

The tuple type `(T1,..,Tn)` (2 to 8), the literal, `project` (`t.i`, `t.0.1`, `f(x).1`) and `match` on
a `Result` with the patterns `ok err err_any tuple bind wild` (VER04r A.1, D20, D35): lowering,
typing, cost (`1 +` the scrutinee `+` the maximum over the arms), evaluation (exactly one arm), the
`tuple` descriptor, stream tag `64`, TFV `L<n>` and JSON arrays. A `match` arm is a guard for call
counting, wraps, D19 and v0 rule 3; a subject call in an arm under a subject scrutinee is `W0701`.
`Err(e)` is `err_any` and an arm that uses `e` is `ir.data`; a `match` on a non-`Result` is `ir.data`.

The corpus errors (a `match` that falls through, an unreachable arm, a duplicate or shadowing bind)
are refused with the generator code `ir` (unreachable first, R12). The S2 checker raises the same
rules (`W0301`, `W0203`); `lower` maps exactly those kinds to `ir`. `concat`, bytes literals and `if`
over sized operands flow to `Bytes<N>` everywhere (A.1).

Faults: `ok-as-err`, `drop-consumed`, `swap-components`, the tuple-payload `missing-error`, and the bare
and `Ok` forms of `flip-result-bit` / `drop-last-result-byte`, in the exact 8.2 JSON; a fault's
`requires` is the sorted union of its template's levels and its op's types (R9). Fault evaluation is
gated to `ir.core` (`emit::generate_gated` is the library parameter; the flip, S10c, turns it on), so
the shipped controls of a tuple fault are `deferred`, naming the first unimplemented level (R12).
`ir.fixed` suites still refuse.

`tests/tuple_parity.rs` reproduces `python/warrant-oracle/tests/fixtures/tuple-forms-v1/` byte for
byte: the lowering, evaluation, fault (shipped and `ir.tuple`-evaluated gates) and refusals dumps,
with negative controls. `tests/tuple_forms.rs` pins the rules one at a time.

## `ir.fixed` lowering parity (S10b)

Since slice S10b the lowerer reads the 35 built-ins of VER04r Appendix A.1 (`bit_and` ... `unwrap_or`):
one `Ir::Fixed` node per call, operand types inferred as for `add`, a static type argument written as
the canonical type name in position 1, `Option<T>` confined to the first operand of `unwrap_or`
(anything else is `ir.data`, D23), `div`/`rem` on two `Nat` operands typed `Option<Int>` (R7), and the
static width of a `from_be`/`from_le` operand read from the one pass that lowers it (a bytes literal, a
`concat` of sized operands, an `if` of equal widths: the checker's rule). Every lowered unit records its
`requires` (R9: the sorted union of its tags' levels and its callees' own `requires`, `ir.core` always
present) and each new tag costs `1 +` its operands.

**Nothing is evaluated or emitted.** A unit whose `requires` holds `ir.fixed` is refused, naming the
level, by `lower_document`, the census and therefore `generate` (the binary exits 2, as for every
refusal, and writes nothing); `eval` refuses `Ir::Fixed` too. `lower_document_with_fixed` returns the
lowered IR for the dump only, and skips VER04r 4.4's impure-guard refusal, which the oracle applies when
it evaluates a suite and not when it lowers it.

`tests/fixed_parity.rs` lowers the oracle's `fixed_forms_v1.warrant` (all 35 built-ins at every width)
and demands the text of `oracle_view_v1` equal the oracle's committed `fixed_forms_v1.oracle-lowering.json`
(schema `warrant-lowering-parity-v1`) **byte for byte**. Both files are read from
`python/warrant-oracle/tests/fixtures/fixed-forms-v1/`, not copied, so an edit to either reruns it (the
build tool's `rust/warrant-corpus-gen` row); the oracle's own BUILD checks the dump against its
`dump-lowering` subcommand. `tests/fixed_lowering.rs` pins the refusals, `requires`, cost and naming on
small documents, including a deep-nesting regression for the width rule (the oracle's first version
re-lowered the operand at every level and was exponential).

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

An omitted example is **still lowered** (a level refusal refuses the document), but it is not
evaluated for `cases.tfv`, not encoded and not admit-checked beyond the decision; its shape
refusal (`W0701`) and the guard refusals are deferred to this decision. Without a line the same
example is refused as before. Every example is evaluated in this order (arguments, pure side,
D19, D18), and a `GenError` carries `size` for exactly the two `Bytes` limits (the 2^127 integer
window is not a size error). The oracle's `tests/fixtures/omit-example-v2/` is reproduced byte for
byte by `tests/omit_example.rs`.

`ignore-high-bit` (`ir.fixed`, VER04r 8.2 [D9]) is emitted as data with `requires: ["ir.fixed"]`
and `deferred` witnesses: ct-compare's `eq_u64` control is **not-live** until S10.

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
- `ir.fixed` data nodes (`bit_and`, `bit_xor`) have no Appendix A shape yet (16.6 OPEN 2).
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
| `census` | the census over the eleven adopters and one synthetic hit per 16.2 item |
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
