# typescript/warrant-corpus

The **TypeScript lane runner** for the language-neutral Warrant evidence corpus
(`warrant-corpus-v1`, [VER04i](../../../specs/VER04i-polyglot-warrant-evidence.md)).
It is the second non-Rust lane, written to match the
[Python runner](../../python/warrant-corpus) check for check.

A package's `.warrant` file is compiled, once, by the Rust compiler
(`warrant-grammar`) into a corpus directory under
`code/specs/fixtures/<module>-warrant-v1/`. This package runs that corpus against
a **TypeScript** library and writes the lane's `warrant-lane-evidence-v1`
manifest. It has no runtime dependency (Node's built-in `crypto` does the
hashing) and depends on no library it tests.

## Where it fits

```text
code/specs/<pkg>.warrant ──(Rust: parse, check, compile)──► corpus.json + cases.tfv
                                                              │
      typescript/<pkg>/evidence.bindings ──►  warrant-corpus (this package)
      (op -> export, error class -> Err)          │  enumerate domains in TypeScript
                                                  │  interpret the model IR (no eval)
                                                  │  hash inputs + expected, compare with the corpus
                                                  │  call the REAL export, compare
                                                  │  break one op at a time (negative controls)
                                                  ▼
                            typescript/<pkg>/evidence/test-manifest.json
```

The corpus does not ship its ~570,000 exhaustive inputs. It ships a descriptor
(`bytes_d1`, `u64_edge`, ...) plus two SHA-256 digests per suite, computed by
Rust over the inputs and over the *model's* answers. This runner regenerates
both streams in TypeScript; a skipped case, a reordered domain or a different
model evaluation changes a digest and fails the suite (`stream-mismatch`).

## Running it

The runner is TypeScript executed directly by Node's type stripping, so the code
that runs is the code that is digested (`runner.src_sha256`). A package keeps a
thin entry that fixes its paths (for `ct-compare`: `tests/warrant-evidence.ts`):

```sh
node --experimental-strip-types tests/warrant-evidence.ts run --out evidence/test-manifest.json   # regenerate
node --experimental-strip-types tests/warrant-evidence.ts check evidence/test-manifest.json       # CI drift check
node --experimental-strip-types tests/warrant-evidence.ts sabotage                                # prove it can fail
```

Or directly:

```sh
node --experimental-strip-types src/bin.ts run \
    --repo-root <repo> --corpus <corpus dir> \
    --binding <pkg>/evidence.bindings --package-dir <pkg> --out <manifest>
```

| Command | Meaning |
|---|---|
| `run [--out F]` | examples, exhaustive suites, negative controls; writes the manifest and `domain-labels.json` only on a clean full pass |
| `check F` | the same run; exit 1 if the regenerated manifest or labels differ from the committed files (ignoring `toolchain.version`) |
| `sabotage` | for every control, a child process with `WARRANT_EVIDENCE_FAULT=<fault>:<op>` must print `not ok` and exit 1 |
| `list-faults` | `<fault>:<op>` for each control |
| `source-digest D` | `warrant-src-v1` digest of a source tree |
| `validate-manifest F [--registry R] [--require-active]` | structural validation against the corpus (it accepts a manifest from any lane) |

Exit status: `0` all passed, `1` a verdict failed, `2` the run was refused (bad
corpus, bad binding, bad usage, or a bug in the runner). Output is TAP-style:
`ok <id>` and `not ok <id> <failure-id>`, with failures named by suite and case
*index* only (never the input bytes).

Environment: `WARRANT_EVIDENCE_FAULT=<fault>[:<op>]`, `WARRANT_EVIDENCE_FILTER=<substring>`,
`WARRANT_EVIDENCE_OUT=<path>`. A filter without a real fault is refused; a faulted
or filtered run never writes a manifest.

## The binding table

`evidence.bindings` is data, one directive per line (VER04i section 5.2):

```text
lane typescript
package typescript/ct-compare
module ct_compare
import src/index.ts
source src
seed 0x5741_5252_414e_5421
values Bool=boolean Bytes=Uint8Array Nat=bigint U64=bigint
op eq export=ctEq
op eq_fixed export=ctEqFixed width=implicit
op select_bytes export=ctSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:RangeError
op eq_u64 export=ctEqU64
```

At start-up the runner checks the table is *total* (every corpus op has a row;
every declared error variant has a mapping and vice versa), that every export
exists, and that the imported module's real path lives under `source`, so the
manifest's `source_sha256` describes the code that actually ran. `import` is a
path, never a bare package name, so a copy under `node_modules` cannot stand in.

What differs from Python, and why:

- **`U64` and `Nat` are `bigint`.** A `number` is a double: `2^64-1` and
  `2^64-2` would round to the same value. The binding refuses `U64=number`.
- **Exact result types.** A `U64` result that is the `number` `1`, a `Bytes`
  result that is a Node `Buffer` or any subclass, a detached array and a
  promise all fail with `type-mismatch`.
- **Errors map by exact prototype.** A thrown value is the declared `Err` only if
  its prototype *is* `RangeError.prototype` (a closed list of built-in classes;
  `Error` itself is refused). A subclass, a thrown string and `null` are
  `unexpected-error`.
- **Bytes are mutable.** Every `Bytes` argument is a fresh copy, so a library that
  writes to its input cannot corrupt the enumerated domain, and a `fresh` result
  may not share an `ArrayBuffer` with an argument (rule 4).
- **JSON is read by the runner's own parser**, because `JSON.parse` keeps the last
  of two equal keys and rounds integers above 2^53 silently. Corpus text must be
  well-formed Unicode: invalid UTF-8 and unpaired surrogate escapes are refused
  (the Python lane still accepts a lone-surrogate escape).

## The model IR runs without `eval`

Claim bodies are tiny expression trees. They are compiled once, before any case
runs, into closures over an array of variable slots. Nothing turns corpus text
into code, so a hostile corpus can only select among the node kinds
`src/ir.ts` implements; an unknown node is a refusal, never a skipped case.

## The ten faults

`invert`, `ignore-last-byte`, `ignore-high-bit`, `swap-choice`, `missing-error`,
`xor-lsb`, `off-by-one`, `return-first-arg`, `truncate-last`, `drop-to-none`.
Each wraps the *callable the runner holds* (never the library) and perturbs
exactly one op. The corpus lists which `(op, fault)` controls it requires, and
which claims must catch each; a control passes only if a required claim fails
*and* the call reached the real export.

## Honest labels (LP-012)

The corpus calls every enumerated suite `exhaustive`, but exhaustive over
`Bytes[347]` means 347 chosen byte strings. `run --out` therefore also writes
`domain-labels.json` beside the manifest, labelling each suite from its domain
descriptors: `full` (the descriptor enumerates the whole type), `bounded-set` (a
fixed strict subset) or `sample` (reserved). Use its `cases_by_label`, not a
summed "exhaustive" total: suite-level totals conflate the kinds until backlog
**LP-012** lands.

## Shared conformance fixture (WD-100)

`tests/conformance-fixture.test.ts` runs every case of
`code/specs/fixtures/warrant-corpus-conformance-v1/` (VER04i 9.1) through this
lane's own reader, renderer, tagged-value decoder, containment helper (driven
with `path.win32` and `path.posix`) and label digest: 397 cases. The fixture's
own checker is `python3 code/scripts/warrant_corpus_conformance.py`.

Two behaviours JavaScript gets wrong by default, both pinned by cases: object
keys must be sorted by **code point** (a plain `.sort()` uses UTF-16 order, and
integer-like keys such as `"10"` are enumerated numerically no matter how they
were inserted), and DEL (U+007F) is written `\u007f`. `renderJson` therefore
serialises by hand instead of calling `JSON.stringify`.

## The v2 runner library at `ir.core` (`src/v2/`, VER04r, slice S7a)

Corpus **v2** ([VER04r](../../../specs/VER04r-warrant-corpus-v2.md)) widens the
corpus: typed values (signed integers, `Bytes<N>`, `Result`), one arithmetic rule
that wraps, an IR with feature *levels*, resolved domain descriptors, chunked
suites with per-chunk digests, and faults that are data. `src/v2/` is this lane's
**library** for it at the first level, `ir.core`: the part of a v2 runner that
has no binding, no session and no manifest in it. It is the TypeScript
counterpart of the Python lane's S6a, and S7b (corpus session, binding, manifest)
and S7c (`typescript/ct-compare` moves to v2) will build on it.

> **On independence.** The point of a second lane is a *second, independent
> reading of one specification*, so that it can disagree with the oracle where
> the oracle is wrong. This lane was written from VER04r (with its errata 16.12
> to 16.20), VER04i, VER04j, the committed conformance kit and the committed v2
> corpora, and from this package's own v1 code. It was written without reading
> the oracle, the generator or another lane's v2 code. The repository **cannot
> prove** that: what it can show is that the code agrees with every case the kit
> pins and that the falsification tests below make a wrong rule fail.

```text
levels -> types -> values -> domains -> ir -> chunks -> faults -> corpus -> kit
```

| module | what it holds |
|---|---|
| `levels.ts` | the closed tag set of every level (`ir.core` 26 tags, `ir.fixed` 35, `ir.tuple`, `ir.data`, the reserved `ir.list` and `ir.seq`), so that a tag of a level this lane lacks is `level-missing` and a tag of no level is `unknown-tag` |
| `types.ts` | the section 2.1 type parser (depth 16, `Bytes<N>` up to 4096, named static widths), the `types` table, and R51's two relations: `flows` ("fits where wanted") and `unify` ("the common type") |
| `values.ts` | values decoded **by declared type** (decimal text, lowercase hex, `Result`), the canonical stream (`10`/`11` integers, `20` bytes, `50`/`51` results), TFV values, and a buffered stream sink that allocates nothing per case |
| `domains.ts` | `size()` and `nth()` of every `ir.core` descriptor, recomputing each declared `size` |
| `ir.ts` | scan, reach, typer, worst-path cost, closure compiler and the pure call trace |
| `chunks.ts` | suite rows (closed key sets, bounds), the chunk file grammar and its cross-checks, and enumeration by unranking |
| `faults.ts` | the three wrappers `pre`, `post`, `replace` |
| `corpus.ts` | a committed v2 corpus read **per unit**: suites and controls classified live or not-live, paths checked component by component, `checkWork` before any suite runs |
| `kit.ts` | `runKit` (and `runCase`, one case): the six `v2-*` conformance categories, run by `requires` |

### Arithmetic and the two kinds of overflow

Every `+ - *` and unary `-` on a fixed-width type **wraps** modulo 2^w (two's
complement when signed); `Nat` and `Int` are exact, every arithmetic node on them
has type `Int`, and a result outside `-(2^127-1) ..= 2^127-1` is `ir-overflow`.
Narrow types (up to 32 bits) are held in a `number` and 64-bit types and `Nat` in a
`bigint`, chosen by the *static* type, so the evaluator is exact and a `U64` never
touches a double.

An overflow is a **corpus** error when the generator would have evaluated the node
and a **case** failure otherwise (R71). The compiler threads one boolean through
exactly the places R71 names: a node with a `subject_op` beneath it, the body of
a model entered through a `model_call` with a subject argument, and a branch,
right operand or arm whose guard contains a `subject_op`.

### Levels are decided by what an expression reaches [D63, D64, D65]

This lane implements `ir.core` and nothing else. It still runs the `ir.core`
suites of a corpus that also holds `ir.fixed` and `ir.tuple` ones (lzw, x25519,
wasm-leb128), in three tiers:

1. a model or op whose own tags and types are inside the lane's levels is compiled
   and validated **whether or not anything reaches it**;
2. one that needs a missing level is **not compiled**, but the checks that read
   tag names alone still apply (`unknown-tag`, `subject-forbidden`,
   `model-order`, the expression depth and the model-chain depth of R77). A
   *suite* gets the same treatment: `loadSuite` scans every expression it holds
   (`admit`, `body`, `oracle.expected`, `oracle.subject`) by tag name **before** it
   records `level-missing`, so an unknown tag cannot hide in a not-live suite;
3. only `level-missing` is scoped by reach.

A `list` descriptor, a `List<T>` type and the `list` tag are **names of the
reserved level `ir.list`** and are refused as such ([D63]; never "unknown"), and
a missing level is named in the order `ir.list`, `ir.seq`, `ir.data`, `ir.tuple`,
`ir.fixed`. A model or an op that needs a missing level gets **no row** anywhere
([D65]): `loadV2Corpus` reports suites and controls only, and `notLive` counts
both.

### Running the kit

```ts
import { runKit } from "@coding-adventures/warrant-corpus/src/v2/kit.ts";
const report = runKit("code/specs/fixtures/warrant-corpus-conformance-v1");
// report.categories[i] = { category, indexed, run, skipped, passed, failed: [{ id, note }], unrunnable }
```

A case runs if and only if its `requires` is a subset of the lane's levels; per
category, run plus skipped equals the index row's count; a category the runner
does not know (`v2-binding`, until S7b) is digest-checked against the index and
skipped whole, and its cases that this lane *could* have run are counted in
`unrunnable` so a manifest writer can refuse to write. A `v2-chunk` case marked
`also_sequential` is also walked from chunk 0 in sequence, and chunk `c` alone
must equal that walk. The whole kit (782 cases at S5e) passes in under half a
second, plus about five seconds for the three sequential walks of the 2^24-case
suite (25 million cases, no allocation per case).

### Falsification comes first [D42]

`tests/v2/mutants.test.ts` copies `src/` to a scratch directory, applies **one
textual edit** that breaks one rule, loads the mutated library, runs the kit
through it and demands failures by case id in the categories the mutant is named
for. There are 28 mutants across values, arithmetic, IR, domains, chunks and
faults: no wrap, saturation, a signed order on the bit pattern, an unchecked `Nat`
product, an `and`/`if` that evaluates both sides, a `take` that does not clamp,
an overflow class that ignores the subject, static widths not substituted,
first-binder-fastest enumeration, a chunk index missing from the stream prefix,
`first_admitted` counted on product ordinals, `pre` rewrites applied
sequentially, and more. Editing text instead of threading switches through the
code keeps the library free of test hooks, and an edit that no longer matches the
source exactly once fails the test instead of passing vacuously.

### The committed corpora

`tests/v2/corpus.test.ts` loads the four committed v2 corpora at `ir.core`:

| corpus | live suites | not live | faults compiled | not live |
|---|---|---|---|---|
| `ct-compare-warrant-v2` | 9 | 0 | 7 | 1 (`eq_u64/ignore-high-bit`, `ir.fixed`) |
| `x25519-warrant-v2` | 6 | 2 | 2 | 3 |
| `lzw-warrant-v2` | 4 | 17 | 2 | 3 |
| `wasm-leb128-warrant-v2` | 4 | 40 | 2 | 18 |

Every chunk of every live suite reproduces its `cases`, `calls` and digests, and
the v1 whole-suite digests over the unchunked stream where a suite has them.

### What is not here

The corpus **session**, the binding (`evidence.bindings` v2 directives), the
manifest writer and `check`, `conformance_sha256`/`conformance_run`, the
`ensures`/`inout` machinery and the aliasing fixtures are S7b; moving
`typescript/ct-compare` to a v2 corpus is S7c. `ir.fixed`, `ir.tuple`, `ir.data`
and the reserved levels are not implemented, and TFV is only *decoded* (the
`cases.tfv` examples), never written.

### Readings this lane had to choose (all conservative: refuse)

- A `match` is named `ir.tuple` when the lane lacks it (R76 needs the scrutinee's
  type to say `ir.data`); either way it is `level-missing`.
- The names `fold map all any range count` of the reserved `ir.seq` are
  `level-missing` naming `ir.seq`, not `unknown-tag`, since VER04r reserves them.
- A bare `err("CASE")` whose case more than one error type declares carries the
  candidate list and unifies with any of them, instead of being refused.
- A suite whose row says `requires` inside the lane's levels but that reaches a
  missing level is a corpus error (R69 e); the opposite is `not-live`.
- A chunk file whose line is consistent with the suite row but not with the
  enumeration is **not** a `chunk-file` refusal: section 11 `v2-chunk` (b) and (c)
  have the lane recompute `cases`, `calls` and the digests from its own
  enumeration and report those, so the mismatch shows as values. (`runSuite`,
  which compares the enumeration with the file, does refuse it.) The check that
  the file's `calls` sum to `expected_calls` stays: section 5.4 defines it.
- `v2-chunk` never answers `level-missing` (it is outside the closed set): a suite
  whose `requires` is inside the lane's levels but whose reach is not is `corpus`
  in the kit as in a corpus (R69 e).
- `expect.type` is the **static** type and keeps byte widths: `Bytes<2>` for a
  `var` declared so, a `concat` of two literals, an `if` of equal widths, a
  `subject_op` at a width, and `Bytes<len>` for a bare literal (R51, R73). R51's
  "written nowhere" is about the corpus IR, which states no result width.
- A kit row with an undetermined result type (`Result<?,?>`), a stand-in that is
  missing, not live or of another signature than its op (R100), or the wrong
  number of `args` is a **harness error**, not an outcome (R41, R98 b).
- Where VER04r is silent this lane refuses: a duplicated parameter or binder name,
  an op whose `requires` is not a list of defined levels, a control whose
  `required_claims` is not a list of suite ids, a `reference` suite whose `body` is
  not `subject == expected` in one order or the other, an `oracle.subject` that
  does not fit `result_type`, and a suite that admits no case.
- `levels` must be the union of the `requires` of the suites, ops, faults and
  controls; a present `omitted_examples` is a non-empty, sorted list of
  `{id, reason}` rows with a reason of the closed set.
- A path is opened only after every component below the fixture directory has been
  looked at with `lstat`: a symbolic link is refused, not followed, and so is a
  missing file (a clean `corpus` or `chunk-file` error, never a raw `ENOENT`). The
  directory the caller passes is the caller's and may itself be a link.
- Names that `Object.prototype` also has (`constructor`, `toString`, `__proto__`)
  are ordinary type, error-type and descriptor names: every table keyed by a name
  read from input is a `Map`.
- Domain values are shared by the cases of a suite. That is safe here because no
  node mutates an operand; a lane that runs a subject which may (S7b) must copy.

## Bounded work

`checkWork` (exported, and called by `runSuite` before anything runs) meters a run
in the two units of section 14: product ordinals and IR work. A `slim` run counts
each suite's `slim_chunks` once and again for every suite a live control re-runs
(its `required_claims`), against 32,000,000 cases and 64,000,000,000 node
evaluations; a `full` run counts every product ordinal. Every slim chunk is
re-validated as an integer below `m`, so a bad entry can never turn a total into
`NaN` and slip under a limit. A suite's `slim_chunks` is itself validated when the
row is read (at most 32 entries, the first and last chunk, ascending, integers).

Before any case runs, a suite's domain product must equal its declared
`cases + discarded`, a suite must admit at least one case, and the product stays
under `maxSuiteProduct` (1,000,000). Total work (suites plus control re-runs) is
capped by `maxTotalWork` (8,000,000), model IR has a static cost cap
(`MAX_IR_COST`, 2,000) and a depth cap, and arithmetic operands are range-checked
before the operation. A corpus that breaks any of these is refused (exit 2).

## Honest limits

A passing run proves the enumerated domain only (for `ct-compare`:
`Bytes[347] x Bytes[347]`, `U64[377] x U64[377]`, ...), never "all inputs". The
Warrant model, the corpus compiler and the binding file are trusted. Nothing here
proves constant-time behaviour of JavaScript code, which a JIT is free to
reshape (VER04i section 11, item 5). The runner is not yet run under the VER08b
sandbox (backlog CAP-RT-1xx).

## Development

```sh
npm ci
npm run lint            # tsc --noEmit (strict, erasable syntax only)
npx vitest run --coverage
```

The suite includes one run of the whole real v1 corpus (572,109 cases) and, for
v2, the 782-case conformance kit with its sequential walks, the four committed v2
corpora and the 28 mutants; everything else uses a small slice so the plumbing
tests stay fast. Test fixtures are copied as bytes, never read and rewritten as
text. Coverage is at least 95 percent for the library (`src/v2` is above 98
percent of statements).
