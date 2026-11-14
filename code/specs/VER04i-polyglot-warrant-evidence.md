# VER04i — Polyglot Warrant Evidence (VER-P031)

**Parent:** [VER00 §7–§9](VER00-verified-shipping-program.md), [VER01](VER01-assurance-report.md), [VER04 §5–§8](VER04-warrant-language.md), [VER04e](VER04e-warrant-codegen.md), [VER04f](VER04f-claims-to-properties.md), [VER04g](VER04g-typescript-warrant-compiler.md), [VER04h](VER04h-warrant-derived-evidence.md), [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [TF01a](TF01a-typescript-test-bootstrap.md), [VER26](VER26-ct-compare-pilot.md). Runtime capability conformance: VER08b (in flight on branch `worktree-agent-a9f1811c0377a9acf`; referenced by name, not yet on `main`).

**Status:** WD-003 is merged: it implements Rust corpus generation and the independent Python validator for the ct-compare Warrant v0 profile. WD-004a is merged as PR #19236: it delivers the strict lane-manifest parser, planned consumer registry, and corpus structural validator. WD-101 delivers the first non-Rust lane runner and manifest (Python, `ct-compare`: §5.1, §6.2, §15); the consumer row stays `planned` until WD-004b1 can load a lane artifact, so no report consumes it yet. WD-004b report integration, other lanes, native suites, samples, and proofs are not delivered.

**Implementation queue and receipts:** the current WD-003/WD-004/WX queue,
CI receipts, and subsequent discoveries are maintained in the
[portable Warrant backlog shard](VER-BACKLOG.d/portable-warrant.md). This
shard is authoritative for current implementation status; the design and
historical rollout snapshots below remain part of this specification.

## 0. Why this exists

VER04h made one lane honest. For `rust/ct-compare`, every number in the
assurance report is read from a manifest written by a run that executed the
Warrant spec against the real crate, and `--check` fails closed when the report
disagrees. That is a big result and a small footprint: the repository ships
the same leaf libraries (`ct-compare`, `gf256`, the `lz*` family, the hashes)
in about eighteen languages, and eleven of them already carry a `ct-compare`
port that has *nothing* to do with `ct-compare.warrant`. A port with a 20-line
hand-written test file is "release-ready" by habit, not by evidence.

The goal this spec serves:

> Every leaf library is exhaustively tested **from its Warrant spec**,
> validated in a sandbox, and release-ready in every language lane, with the
> assurance report for each lane computed from that lane's own manifests.

Languages in scope (the "lanes"): `rust python typescript go ruby c cpp java
kotlin csharp fsharp elixir lua perl haskell swift dart ocaml`.

The temptation is to translate VER04h eighteen times: emit a per-language
test generator, per-language model code, per-language exhaustive enumerators.
That is eighteen compilers to keep byte-identical with the Rust one, and the
Rust one is the only one that is tested against the language's real checker.
This spec instead keeps **one compiler** (the Rust `warrant-grammar`) and moves
only the *execution* into each lane, behind a small, data-driven runner.

The rule from VER04h still holds and is generalised, not weakened:

> **Evidence is never written by hand. Every count, seed, digest, verdict and
> label in a lane's assurance report is read from a manifest produced by that
> lane's run, and the checker rejects any report that disagrees with the
> manifests, with the corpus, or with the Warrant document.**

Nothing here proves more than VER04h does. A corpus run proves only the
enumerated and sampled behaviour (§11).

## 1. Three layers

```text
                        code/specs/<pkg>.warrant
                                  │  (Rust: parse ► check ► compile)
              ┌───────────────────┼─────────────────────────────┐
              ▼                   ▼                             ▼
    A. PORTABLE CORPUS     B. NATIVE SUITES              C. PROOF LAYER
    code/specs/fixtures/   generated per lane, run by    per-lane WIR front end
    <pkg>-warrant-v1/      that lane's own L2 engines    (not live outside Rust)
    corpus.json+cases.tfv  (pbt-v1 / fuzz-v1 corpora)
              │                   │                             │
   lane L0 runner (§6)    lane L2 engine (§7)           (§10, not-live)
   ► lane binding (§5)    ► same binding                      │
   ► real implementation  ► real implementation               │
              ▼                   ▼                             ▼
      evidence/<lane>/test-manifest.json (layer: portable | native)
                                  │
                      ca-assurance-report  generate / --check  (§9)
```

| Layer | Executes | Proves | Does **not** prove | Status |
|---|---|---|---|---|
| A. Portable | Examples, bounded-exhaustive domains (§3), a materialised relation-biased sample, and sabotage probes, all produced by the canonical Rust compiler, run by a small L0 runner written once per lane | The lane's *real export*, through its binding, agrees with the Warrant model on every case of the finite domain, and the runner fails when the export is wrong | Anything outside the domain; that the lane's own PBT/fuzz engines work; constant-time or step bounds | Live as lanes land (§13) |
| B. Native | Seeded property suites and fuzz targets, using the lane's L2 engines (VER05/VER06 family), generated per lane from the same Warrant spec | Randomised agreement beyond the exhaustive domain, with the biased generators of VER04h §4 and lane-native shrinking | Exhaustiveness; it is only as strong as the lane's engine | Per lane, after layer A, gated on `pbt-v1`/`fuzz-v1` corpora (§7) |
| C. Proof | Lowering the lane's source to WIR and discharging `labels`/`bound` obligations (VER04h §7) | Constant-time and step-bound obligations on the lowered lane source | Anything about code that does not lower (and nothing at all in a lane with no front end) | **Not live** except Rust. Every non-Rust proof label is `unchecked` (§10) |

Why portable first: layer A is the only layer whose expected answers are
produced *once, outside the lane under test*. A lane's own PBT engine, written
in that lane, being used to test that lane's library shares failure modes
(the same wrong idea of `u64`, the same truncating cast). The portable corpus
is the cross-lane check that cannot share them.

## 2. Corpus decision and format

### 2.1 Candidates and the measured problem

The Rust suites are large. From the checked-in
`rust/ct-compare/evidence/test-manifest.json`:

| Kind | Cases (ct-compare) |
|---|---:|
| `exhaustive` (7 claim suites) | **572,109** |
| `property` (4096 each) | 36,864 |
| `example` | 12 |

`exhaustive/eq_agrees` alone is 120,409 cases (347 x 347 byte strings). I
materialised exactly that suite as TFV records (`id op left right equal`, the
minimum a TFV v1 line needs) to measure rather than guess:

| Encoding of `eq_agrees` exhaustive | Size |
|---|---:|
| TFV v1 text, one record per case | 5,936,406 B (49 B/case) |
| same, gzip -9 | 606,383 B |
| output bitmap only (1 bit/case, inputs re-derived) | 15,051 B |
| domain descriptor + model IR + 2 digests (this spec) | about 1 KB |

Extrapolating at roughly 50 B/case, all of `ct-compare`'s exhaustive suites
are about **28 MB** of TFV (about 3 MB gzipped) *per library*. A repository
with 30–40 leaf libraries and git history (every regeneration is a new blob)
cannot carry that, and 18 lanes parsing 28 MB of text per run is also slow
for the interpreted lanes.

### 2.2 Decision: a sibling format, `warrant-corpus-v1`, that enumerates in-lane

Options:

1. **Extend TFV v1 with chunked/compressed suites.** Rejected: TFV's point
   (TF01 §2) is that any language parses it with no library in a few dozen
   lines. Adding compression breaks that (zlib is a dependency in the
   bootstrap layer), and chunking only moves the 28 MB around.
2. **Materialise everything, commit gzipped.** Rejected on repository weight
   and because the corpus would be opaque binary in review.
3. **Enumerate in-lane from a domain descriptor, with a cross-checked
   expected-value stream.** Chosen.

The corpus is therefore a *directory*, a sibling of the existing
`code/specs/fixtures/<name>-v1/` fixtures (TFV and `der-tlv-v1`-style
`cases.json`/`consumers.json` remain the pattern; this just adds a descriptor
file because exhaustive suites are generators, not lists):

```text
code/specs/fixtures/<module>-warrant-v1/
  corpus.json        authority: suites, domains, model IR, faults, digests
  cases.tfv          TFV v1: examples, materialised sample, sabotage probes
  consumers.json     registered lane consumers, with planned/active status
  schema.json  consumers.schema.json  README.md  CHANGELOG.md
```

`corpus.json` and `cases.tfv` are produced by the Rust compiler
(`warrant-grammar --example gen_corpus`) from the checked Warrant module and
are deterministic (sorted keys, no timestamps, LF). `module` is the Warrant
module name; for `ct-compare` the directory is
`ct-compare-warrant-v1/` and it complements, never replaces, the existing
hand-written `ct-compare-v1` CT01 corpus (which stays the independent oracle
for work-accounting fields; see §14 Q3).

### 2.3 Why this keeps the oracle outside the lane

Enumerating inputs in-lane is safe only if the lane cannot also *invent the
expected answer*. Two mechanisms close that, both data in `corpus.json`:

* **Model IR.** Each claim body is compiled, by the same generator that emits
  VER04h's Rust model code, into a tiny language-neutral expression tree
  (§2.5). The lane interprets it with a ~150-line evaluator that lives once
  per lane in `warrant-corpus` (§6).
* **Stream digests.** For every exhaustive suite the compiler *evaluates the
  model itself* over the whole enumeration and records two SHA-256 digests:
  `inputs_sha256` over the canonical encoding of every admitted input tuple in
  enumeration order, and `expected_sha256` over the canonical encoding of
  every oracle result in the same order (§2.6). The lane recomputes both
  while it runs and the manifest records them; `--check` demands equality
  with the corpus.

Consequences: a lane whose enumerator skips a case, enumerates in a different
order, or whose model evaluator disagrees with Rust's on a single case
produces a different digest and fails. The lane never ships 28 MB, and the
only trusted evaluator is the Rust one plus the independent Python oracle
(§2.8).

### 2.4 Case identity

Identity is the VER04h evidence id, which is itself derived from the spec:

| Corpus object | Id | Spec anchor |
|---|---|---|
| example case | `warrant/<module>/example/<example-name>` | `example <name>` |
| exhaustive suite | `warrant/<module>/exhaustive/<claim>` | `claim <claim>` |
| sampled case | `warrant/<module>/sample/<claim>#<k>` (`k` is 0-based) | `claim <claim>` |
| sabotage control | `warrant/<module>/negctl/<kind>/<op>/<fault>` | op + fault registry |

A single exhaustive *case* is addressed as `<suite id>@<i>` where `i` is the
0-based admitted-case ordinal after refinements, so a failure message is
`warrant/ct_compare/exhaustive/eq_agrees@7021` and can be reproduced in any
lane, or in Rust, by index. The ids used in lane manifests are *the same
strings* that Rust manifests use, which is what makes "lane X covers claim C"
a join over equal keys (§9) and not a naming convention.

### 2.5 `corpus.json` shape

```json
{
  "schema_version": 1,
  "profile": "warrant-corpus-v1",
  "module": "ct_compare",
  "spec": "code/specs/ct-compare.warrant",
  "spec_sha256": "<hex>",
  "generator": { "name": "warrant-grammar", "version": "<semver>" },
  "types": { "SelectError": ["CT_SELECT_LENGTH_MISMATCH"] },
  "ops": {
    "eq":          { "params": [["a","Bytes"],["b","Bytes"]], "ret": "Bool" },
    "eq_fixed":    { "width_params": ["N"], "params": [["a","Bytes<N>"],["b","Bytes<N>"]], "ret": "Bool" },
    "select_bytes":{ "params": [["a","Bytes"],["b","Bytes"],["choice","Bool"]],
                     "ret": "Result<Bytes,SelectError>", "fresh": true },
    "eq_u64":      { "params": [["a","U64"],["b","U64"]], "ret": "Bool" }
  },
  "models": { "same": { "params": ["a","b"], "body": ["eq", ["var","a"], ["var","b"]] } },
  "suites": [
    {
      "id": "warrant/ct_compare/exhaustive/eq_agrees",
      "kind": "exhaustive", "claim": "eq_agrees", "ops": ["eq"],
      "binders": [["a", {"domain":"bytes_d1"}], ["b", {"domain":"bytes_d1"}]],
      "admit": ["bool", true],
      "lhs": ["subject_op", "eq", [["var","a"],["var","b"]]],
      "rhs": ["model_call", "same", [["var","a"],["var","b"]]],
      "cases": 120409, "discarded": 0, "expected_calls": 120409,
      "domain_text": "Bytes[347] x Bytes[347]",
      "inputs_sha256": "<hex>", "expected_sha256": "<hex>"
    }
  ],
  "faults": [ ... ],
  "samples": { "per_claim": 256, "seed": "<u64>", "generator": "warrant-biased-v1" }
}
```

`expected_calls` is the exact number of subject-operation invocations in an
unfaulted complete run of the suite. WD-003's equality-only v0 profile permits
one subject call per admitted case, so `expected_calls == cases`; a later
profile that admits multiple subject calls must count them from the checked
subject expression and include the count in the corpus.

IR calls have separate subject and oracle forms. `subject_op(name, args)` is
the only node that dispatches through a lane's binding table to the real
export. It is forbidden inside every model body. `model_call(name, args)` is
the only node that invokes a named, pure model; model bodies can use only
`lit`, `var`, `model_call`, `ok`, `err`, `name`, `len`, pure comparisons and
arithmetic (`+ - *`, with width semantics in §2.7), `and or not implies`, and
`if`. Built-in pure operators such as `eq` and `lt` are IR tags, not operation
names and never resolve through the binding table. The `same` model above is
therefore `['eq', ['var','a'], ['var','b']]`; the subject side of a claim is
`['subject_op','eq', ...]`. This rejects the overloaded `['op', ...]` form.
The compiler fails unsupported operations or nodes with `W0701`; every
consumer rejects an unknown tag and never skips it.

WD-003's first profile accepts only equality claims with exactly one subject
expression and one pure reference expression (the sides may appear in either
order). It rejects claims with no pure reference side or with subject calls
on both sides; there is no implementation-independent expected value for
those forms in this profile. For each case, the consumer evaluates the subject
expression through the real export and the reference expression through pure
models. The corpus oracle evaluates and records only the pure reference
result. A wrong subject result cannot change the oracle value and must fail
the case comparison. The shared conformance fixture includes a deliberately
wrong subject result to prove this separation.

### 2.6 Domains, enumeration order and the canonical stream

Domains are exactly those of VER04h §3 (`D(Bool)`, `D(U8)`, `D(U64)` = 377
values, `D(Nat)` = 0..=64, `D(Bytes)` = 347 strings, `Bytes<N>`), referenced by
name and *defined once, here and in VER04h*, never re-described per lane:

| Descriptor | Order |
|---|---|
| `bool` | `false`, `true` |
| `u8_all` | `0..=255` ascending |
| `u64_edge` | `0..=255`; then `1<<i` for `i = 8..=63`; then `MAX`; then `MAX ^ (1<<i)` for `i = 0..=63` |
| `nat_0_64` | `0..=64` |
| `bytes_d1` | `[]`; each single byte `00..ff`; every length-2 then every length-3 string over `A = [00,01,80,ff]` in lexicographic order; for `L = 4..=8` the pair `00`x`L`, `a5`x`L` |
| `bytes_n(N)` | for `N <= 3`, the members of `bytes_d1` of length `N`; for `N >= 4`: uniform fills over `A` in order, then for `pos = 0..N`, for `s` in `[01,80,ff]`: `00…` with `pos` set to `s` |

The Cartesian product enumerates binders left to right, the **last binder
varying fastest** (row-major). Refinements filter in that order; a discarded
tuple is counted and receives no case id. A case index `@i` is the zero-based
ordinal among admitted cases after refinement.

The canonical stream encoding (`warrant-stream-v1`) uses two independent
SHA-256 streams. Both start with the same context prefix. After the header
`"warrant-stream-v1\0"`, encode the suite context as a
u32-LE byte length plus the UTF-8 suite id, a u32-LE binder count, then for
each binder a u32-LE byte length plus its canonical Warrant type name; finish
the context with the same length-prefixed canonical result type name. For each
admitted case, append its typed binder values in order to `inputs_sha256`, and
append only its typed pure-model result to `expected_sha256`. The fixed suite
context and binder/result types define case boundaries; stream position is
the admitted-case ordinal. There are no per-case delimiters or ordinal bytes.
Hash each stream separately:

| Value | Bytes |
|---|---|
| `Bool` | `00` or `01` |
| `U8`, `U64`, `Nat` | `10` then 8 bytes little-endian (unsigned) |
| `Bytes` | `20`, u32-LE length, the bytes |
| `List` | `30`, u32-LE count, elements |
| `Option` | `40` (none) or `41` + inner |
| `Ok(v)` / `Err(E.V)` | `50` + inner / `51`, u32-LE length, UTF-8 variant name |
| a tuple | concatenation of its elements |

The lane hashes each admitted case's input tuple and the *model's* result (not
the implementation's), in enumeration order. The suite descriptor, type
schema, and admitted-case ordinal define each stream's context and boundaries.
All lanes ship (or implement once in
`warrant-corpus`) a streaming SHA-256; none of them may depend on a third-party
digest library in the bootstrap layer, and every lane in §13 has SHA-256 in its
standard library or platform (Lua, Perl and OCaml included: Digest::SHA is
core Perl; Lua and OCaml's stdlib `Digest` is MD5, so those two lanes carry a
vetted 80-line SHA-256 in `warrant-corpus`, itself checked against NIST
vectors in the lane's own tests).

### 2.7 Value encoding for Warrant types (TFV mapping)

`cases.tfv` reuses TFV v1 unchanged (TF01 §2). Types map onto the value model
of `tfv.py`:

| Warrant type | TFV value | Notes |
|---|---|---|
| `Bool` | `T` / `F` | |
| `U8` | `I<n>`, `0..=255` | runner range-checks; out of range is a corpus error |
| `U64` | `I<decimal>`, `0..=2^64-1` | **no float or signed 64-bit parsing**: see width rules below |
| `Nat` | `I<n>`, `>= 0` | bounded by the spec's checked width |
| `Bytes`, `Bytes<N>` | `X<hex>` | `Bytes<N>`: length must be `N` |
| `List<T>` | `L` of elements | |
| `Option<T>` | `L` of length 0 (none) or 1 (some) | a bare `null` would conflate `Option<Option<T>>` |
| `Result<T,E>` | `M` with one entry: key `ok` -> value, or key `err` -> `S<VARIANT>` | the error *variant name*, e.g. `S CT_SELECT_LENGTH_MISMATCH` |
| expected outcome | the same encodings | an expected `Err` is data, not an exception |

A case record is `<id> R<n> Kop S<op> K<param> <v>... Kexpect <v>`; `width`
(`Kwidth I<n>`) for `eq_fixed<N>`-style ops, as in the existing ct-compare TFV.

**Width and overflow normalisation** is a lane concern with a corpus-side
guard. `U64` is the sharp edge:

| Lane family | Representation the runner must use |
|---|---|
| rust, c, cpp, go, csharp, fsharp, swift, java (`long`), kotlin (`Long`) | native 64-bit, parsed unsigned; Java/Kotlin compare/shift with the unsigned APIs |
| python, ruby, elixir, haskell (`Word64`) | arbitrary-precision ints, masked to 64 bits after every arithmetic node |
| typescript, dart (web-safe) | `BigInt`; `number` is forbidden for `U64` and `Nat` above 2^53 |
| lua | 64-bit integers; use `math.ult` and masks, never float division |
| perl | `use integer` is *not* used; use `Math::BigInt` or the 64-bit-int build check, whichever the binding file declares |
| ocaml | `Int64` with unsigned ops; plain `int` is 63-bit and is **forbidden** for `U64` |

The model IR's `+ - *` on `U64` wrap at 2^64 and on `Nat` are checked (overflow
of a spec-declared width is a corpus error at *generation time*, so a lane never
sees one). The corpus includes the 377-value `u64_edge` domain precisely so a
lane that silently holds `U64` in a 63-bit or double type fails an exhaustive
case, not a code review.

### 2.8 Regeneration and drift checks

**WD-003 first slice:** the Rust generator emits the `ct-compare` Warrant v0
examples, exhaustive suite descriptors and digests, plus generated control
rows. The independent Python validator uses only the standard library and
re-enumerates/re-evaluates that profile from the checked-in Warrant source. It
does not call Rust or consume Rust-generated expected values as its oracle.
This slice generates no materialised samples and executes no lane binding;
sample generation and Python L0 execution remain separate rollout work.
Unsupported `--shard` arguments, Warrant v1-only forms, and unknown IR nodes
are hard errors in the generator and validator.

| Check | Command (CI, in the corpus package BUILD) | Fails when |
|---|---|---|
| Regenerate | `cargo run -p coding_adventures_warrant_grammar --example gen_corpus -- --check` | the checked-in `corpus.json`/`cases.tfv` differ byte-for-byte from the compiler's output for the current spec (VER04h's `gen_evidence --check` pattern) |
| Independent oracle | `python3 code/scripts/warrant_corpus_oracle.py` | an independent stdlib-only Python re-enumeration of every domain and re-evaluation of the model IR gives a different `cases`, `inputs_sha256` or `expected_sha256`; it also re-derives every TFV record's `expect`. Same philosophy as `ct_compare_conformance.py` and `tfv.py`: the corpus is checked by code that shares nothing with the generator. |
| TFV pairing | `python3 code/scripts/tfv.py --check` | `cases.tfv` is not the compilation of its authoritative record list |
| Consumer registry | `python3 code/scripts/warrant_corpus_consumers.py --check` | `consumers.json` lists a lane whose binding/runner/evidence file is absent (the `der-tlv-v1/consumers.json` pattern, with a path-existence and BUILD-mentions-runner check) |

The corpus package BUILD declares the `.warrant` file and `warrant-grammar`
as dependencies, so a spec edit rebuilds the corpus and, through the
dependency graph, every lane that consumes it.

### 2.9 Later extension: sampled (property-shaped) cases are materialised, small, and labelled honestly

After the WD-003 first slice, Layer A may carry a *sample* per claim: `samples.per_claim = 256` cases drawn
with VER04h §4's relation-biased generator, using the per-claim seed
`SplitMix64(evidence_seed ^ fnv1a64("claim/" || module || "/" || claim))`,
written into `cases.tfv` with expected values. Measured at about 160 B/case for
0..64-byte strings, this is about 0.6 MB for `ct-compare`'s ~14 claims. Why
materialised, not re-generated: reproducing `SplitMix64` and the relation
selection in 18 languages is exactly the drift surface we are avoiding, and
the *large* seeded run belongs to layer B, where each lane uses its own engine.
The manifest records `equal_cases` (samples that took the equal branch) so the
bias is checkable (VER04h §4).

## 3. Exhaustive budget (inherited and tightened)

`EXHAUSTIVE_BUDGET = 400_000` per claim, checked arithmetic, `W0706`/`W0707`,
`DOMAIN_BYTES_BUDGET`: all as in VER04h §3. Two additions for lanes:

* **Runtime ceiling.** `corpus.json` records `cases` per suite; each lane's
  `consumers.json` entry has `max_wall_seconds` (default 120). A lane that
  cannot meet it for a given suite (slow interpreters on a 140,000-case
  suite) keeps the *full* enumeration. `warrant-corpus-v1` WD-003 is
  unsharded: `--shard` is rejected with a usage error. A later profile must
  define mergeable chunk commitments before any consumer accepts sharding;
  SHA-256 final digests cannot be merged to obtain the digest of a combined
  stream.
* **Descriptor sanity.** A descriptor whose product overflows, or that
  produces zero admitted cases, fails *generation* (so a refinement that
  excludes everything cannot pass vacuously, as in VER04h).

## 4. Sabotage in other languages

A lane runner that cannot fail proves nothing; neither can one whose fault was
applied somewhere the claim cannot see it. The gf256 lane taught the second
half the hard way: a fault applied to *every* operation can cancel inside a
relational claim. Consider the illustrative `gf256.warrant` claim

```text
claim mul_commutes:  forall a: U8, b: U8. mul(a, b) == mul(b, a)
claim div_inverts:   forall a: U8, b: U8 | b != 0. mul(div(a, b), b) == a
```

A fault "XOR the low bit of every op's result" turns `mul_commutes` into
`mul(a,b)^1 == mul(b,a)^1`, which is still true. The suite passes under the
fault and the sabotage check would wrongly conclude the suite is
*insensitive*, or worse, it would pass because some other claim happened to
fail and the control was credited to the wrong suite. Per-op scoping removes
the ambiguity.

### 4.1 Rule: faults are per (suite kind, operation, fault)

A negative control is a generated row for the triple *(target suite kind, op,
fault)*. For a claim that
mentions ops `o1..ok` there are `k` controls per applicable fault, each
perturbing **exactly one** op's binding. A control passes iff the suite, run
with that single-op fault, **fails with a registered failure id**. The
manifest entry records `caught_by` (claims that failed). A control that fails
to fail is a failed *control* and fails the lane (VER04h §6, unchanged); an
op that no claim can catch is reported in the manifest as `uncaught_faults`
and fails the report check unless the fault is listed in the spec's
`accepted_blind_faults` (empty for ct-compare and gf256).

### 4.2 The fault is a wrapper around the binding callable

The runner never edits, monkey-patches or recompiles the library under test.
It holds each op as a callable `call(args) -> Value` (§5), and a fault wraps
that callable:

| Fault (portable name) | Applies when | Behaviour |
|---|---|---|
| `invert` | op returns `Bool` | negate the result |
| `ignore-last-byte` | an op has `Bytes` params | zero the last byte of every `Bytes` input before the call (an early-exit scan) |
| `ignore-high-bit` | op has `U64` params | clear bit 63 of every `U64` input |
| `swap-choice` | op has a `Bool` param named `choice` | negate it |
| `missing-error` | op returns `Result` | map a declared `Err` to `Ok(first Bytes arg)` |
| `xor-lsb` | op returns `U8`/`U64`/`Nat` | `result ^ 1` |
| `off-by-one` | same | `(result + 1)` masked to the type's width |
| `return-first-arg` | op has two params of the return type | return the first argument |
| `truncate-last` | op returns `Bytes` | drop the final byte |
| `drop-to-none` | op returns `Option` | `None` |

`corpus.json.faults` is the transformer registry: each row declares a fault
and its signature-based applicability. The compiler separately derives a
`controls` array with one row per applicable `(target_kind, op, fault)`, a
stable id `warrant/<module>/negctl/<target_kind>/<op>/<fault>`, and the claim
ids that the control is required to falsify. Manifest `caught_by` must be
nonempty and a subset of those required claim ids; an unrelated failure cannot
credit a control. BUILD reruns exactly the listed target suites. A lane
implements the ten transformers once in
`warrant-corpus` (about 60 lines); the binding table is just a record/map of
callables in every lane, so wrapping is idiomatic everywhere:

| Lane | Binding callable shape |
|---|---|
| python, ruby, lua, perl, elixir | dict/hash/table/map of closures (`fn`/`lambda`/`sub`) |
| typescript, dart, swift | object/record of closures |
| go | `map[string]func(Args) (Value, error)` |
| java, kotlin, csharp, fsharp | `Function`/`Func` registry (an interface per op) |
| c | function-pointer table in a struct |
| cpp | `std::function` table |
| haskell, ocaml | record of functions |

### 4.3 BUILD-level sabotage (every lane)

Like VER04h §6, each lane BUILD runs the portable runner a second time with
`WARRANT_EVIDENCE_FAULT=<fault>[:<op>]` and requires `not ok` lines and a
non-zero exit:

```sh
# python lane BUILD (illustrative)
.venv/bin/python -m warrant_corpus run --corpus ../../../specs/fixtures/ct-compare-warrant-v1 \
    --binding warrant.binding.json --out evidence/test-manifest.json
for f in $(.venv/bin/python -m warrant_corpus list-faults --corpus … ); do
  WARRANT_EVIDENCE_FAULT=$f WARRANT_EVIDENCE_FILTER=/exhaustive/ \
    .venv/bin/python -m warrant_corpus run … | grep -q '^not ok '
done
```

The same refusal rules as VER04h: `WARRANT_EVIDENCE_FILTER` alone exits `2`;
`FAULT=none` with a filter is refused; a filter matching nothing exits `2`; a
faulted or filtered run **never writes a manifest** (a lane that writes one
fails its own check, because the manifest's `faulted` field must be `false`).

### 4.4 Controls against a runner that never calls the library

A runner that hashes the corpus and reports pass would satisfy §2.3 without
touching the implementation. Two guards:

* The wrapper counts invocations. `calls` is recorded per suite and must equal
  the number of `op` nodes evaluated, which the corpus also states
  (`expected_calls`). A runner that does not call the binding has `calls = 0`.
* Faults, which only fail if the call reaches the real export. A runner that
  fakes the call fails every negative control in §4.1.

## 5. Per-lane binding files

A binding is **data**, one per (lane, package), checked in next to the
package's `required_capabilities.json`: `warrant.binding.json`. It maps
Warrant ops to idiomatic exports and says how values and errors cross the
boundary. VER04h's Rust bindings (`binding.rs`) are the model; this file is the
portable form.

```json
{
  "schema_version": 1,
  "lane": "python",
  "package": "python/ct-compare",
  "module": "ct_compare",
  "entry": { "kind": "python-module", "path": "coding_adventures_ct_compare" },
  "values": { "Bytes": "bytes", "U64": "int", "Nat": "int", "Bool": "bool" },
  "ops": {
    "eq":           { "export": "ct_eq" },
    "eq_fixed":     { "export": "ct_eq_fixed", "width": "runtime" },
    "select_bytes": { "export": "ct_select_bytes",
                      "errors": [ { "raises": "ValueError", "variant": "CT_SELECT_LENGTH_MISMATCH" } ] },
    "eq_u64":       { "export": "ct_eq_u64" }
  }
}
```

Other lanes differ in the *error* column, which is where lanes diverge most:

| Lane | `select_bytes` length mismatch surfaces as | `errors` row |
|---|---|---|
| rust | `panic` caught with `catch_unwind` | `{"panics":"*","variant":…}` |
| python | `ValueError` | `{"raises":"ValueError",…}` |
| typescript, java, kotlin, csharp, fsharp, dart, swift | exception of a named class | `{"raises":"RangeError",…}` |
| go | returns `(out, error)`, sentinel `ErrLengthMismatch` | `{"returns_error":"ErrLengthMismatch",…}` |
| c | non-zero status code with an out-buffer | `{"status":-1,…}` |
| haskell, ocaml | `Left`/`Error` | `{"returns_error":"LengthMismatch",…}` |
| elixir | `{:error, :length_mismatch}` | `{"returns_error":"length_mismatch",…}` |
| ruby, lua, perl | raised error / `nil, err` / `die` | named in the row |

Rules, all enforced when the runner starts (any violation exits `2` before a
single case runs, and is a `BindingInvalid` check failure in §12):

1. **Total.** Every `ops` entry in `corpus.json` has a binding row; every
   declared error variant of the op's `Result` has a row; every binding row
   names an export that exists (resolved at start, not lazily).
2. **Error normalisation is closed.** An exception, status, panic or error
   value that matches a row becomes the corresponding `Err(variant)`. One that
   matches *no* row, from an op the spec says cannot fail (`ensures no_panic`),
   is verdict `fail` with failure id `unexpected-error`, never a mapped error
   and never swallowed. A call that *should* error and returns a value is
   `missing-error` (and is what the §4.2 fault of that name simulates).
3. **Width rules are stated, not assumed.** `values` names the lane type for
   each Warrant scalar and the runner refuses a binding whose `U64`/`Nat` type
   is one of the forbidden representations in §2.7. `width: "runtime"` means
   the library takes the width as a value (most lanes, which have no
   const-generics); `"static"` is for Rust/C++/templates and means the lane's
   generated dispatch table has one arm per width in `corpus.json.widths`
   (VER04h's `MAX_FIXED_WIDTH = 4096` still bounds it).
4. **Fresh outputs.** For `fresh` ops the runner mutates the returned buffer
   and re-reads the inputs (no alias) and mutates an input and re-reads the
   output (no sharing), per VER04e §4.3, in every lane. Languages with
   immutable byte values (Java `byte[]` is mutable; Elixir binaries are not)
   record `fresh: "immutable-by-construction"` and the runner skips the
   mutation, never the check's inclusion in the manifest.
5. **No library helpers in the binding.** A binding may translate values and
   errors. Any logic beyond that (a computed answer, a retry, a fallback) is a
   review reject; the binding file is JSON precisely so it cannot contain
   any.

The per-lane runner library `warrant-corpus` (one package per lane, written
once, checked by that lane's TF01 harness against the conformance cases in
`code/specs/fixtures/warrant-corpus-conformance-v1/`) provides: TFV parser,
descriptor enumerator, model IR evaluator, stream hashing, fault wrappers,
the binding loader, and the manifest writer. It has *no dependency on any leaf
library*. It is the only new per-lane code; the TFV parser and harness exist
already in TF01/TF01a for several lanes.

### 5.1 The Python binding file (`evidence.bindings`) -- WD-101 amendment

The merged WD-004a consumer registry already reserves the path
`<package_root>/evidence.bindings` for `python/ct-compare`, and the Rust lane
already uses a line-oriented `evidence.bindings` (VER04 §6). The registry is
the authority on paths, so the Python binding is **`evidence.bindings`, not
`warrant.binding.json`**; the JSON example in §5 stays the illustration of the
*content* every lane's binding carries. Like the JSON form it is data only: no
expression, no code, one directive per line, `#` comments, blank lines ignored.

```text
lane python
package python/ct-compare
module ct_compare
import coding_adventures_ct_compare
source src/coding_adventures_ct_compare
seed 0x5741_5252_414e_5421
values Bool=bool Bytes=bytes Nat=int U64=int
op eq export=ct_eq
op eq_fixed export=ct_eq_fixed width=implicit
op select_bytes export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError
op eq_u64 export=ct_eq_u64
```

| Directive | Meaning |
|---|---|
| `lane`, `package`, `module` | Lane id, `<lane>/<name>`, and the Warrant module; the runner refuses a binding whose module differs from the corpus's |
| `import` | Dotted module that holds every export. Resolved when the runner starts; every `export` must exist and be callable |
| `source` | Source root, relative to the package directory, whose `warrant-src-v1` digest is `source_sha256` (§8.1). The imported module's file **must** live under it (`source-mismatch` otherwise), so the digest describes the code that actually ran |
| `seed` | The `evidence_seed`: a `u64` (decimal or `0x` hex, `_` allowed) that orders suites for a run; case order stays canonical |
| `values` | Lane type of each Warrant scalar. Python accepts only `Bool=bool`, `Bytes=bytes`, `U64=int`, `Nat=int` (rule 3); anything else is `BindingInvalid` |
| `op` | `export=<name>` (required); `width=implicit` when the export infers a `Bytes<N>` width from its buffers (the runner still asserts the corpus width equals every buffer length and does not pass it); `width=runtime` when it takes the width as a trailing argument; `errors=<VARIANT>:<ExceptionName>[,...]` maps a raised built-in exception class to a declared `Err` variant |

Error normalisation is **closed** (rule 2): an exception whose type is *exactly* a mapped class
(`type(e) is cls`, so a subclass does not count) becomes `Err(variant)`; any other exception from any
op is failure id `unexpected-error`. Exception names resolve only among the interpreter's built-in
exception classes, never `BaseException`/`Exception` themselves, so a binding cannot swallow
arbitrary failures. The mapping is by class, not by message: a library that raised exactly the mapped
class for a *different* reason would still be accepted, which is a limit of the binding (§11 item 3).
Each port should therefore raise a dedicated exception type for each declared error (backlog note in
the portable Warrant queue).

Python results are checked with `type(x) is T`, not `==`: `True == 1` and a
`bytearray` compare equal to `bytes` in Python, so a library returning `1`
for `true` or a mutable buffer for `Bytes` fails with `type-mismatch`. This
also discharges rule 4 (fresh outputs) by construction: a result that must be
exactly `bytes` is immutable, so there is no buffer to alias or mutate.

### 5.2 The TypeScript binding file -- WD-102 amendment

The TypeScript lane uses the same line-file grammar as §5.1 (`evidence.bindings`,
data only, one directive per line) with these lane-specific rules:

```text
lane typescript
package typescript/ct-compare
module ct_compare
import src/index.ts
source src
seed 0x5741_5252_414e_5421
values Bool=boolean Bytes=Uint8Array U64=bigint Nat=bigint
op eq export=ctEq
op eq_fixed export=ctEqFixed width=implicit
op select_bytes export=ctSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:RangeError
op eq_u64 export=ctEqU64
```

| Directive | TypeScript rule |
|---|---|
| `import` | A package-relative POSIX path to the one module that holds every export (`src/index.ts`), loaded with a dynamic `import()` of its `file:` URL. It is a path, not a bare specifier, so no `node_modules` copy can stand in for the code under test. Its real path **must** lie under `source` (`source-mismatch` otherwise) |
| `source` | `src` (§8.1: `**/*.ts` except `**/*.test.ts`, `dist/`) |
| `values` | The only accepted representations are `Bool=boolean`, `Bytes=Uint8Array`, `U8=number`, `U64=bigint`, `Nat=bigint`. `U64` and `Nat` are **`bigint` always**, never `number` (a `number` cannot hold a `u64_edge` value, §2.7). Anything else is `BindingInvalid` |
| `errors=` | `<VARIANT>:<ClassName>` where the class is one of the built-in `RangeError`, `TypeError`, `SyntaxError`, `ReferenceError`, `EvalError`, `URIError`; `Error` itself is refused, as `Exception` is in §5.1. A thrown value maps only when its prototype **is** that class's prototype (`Object.getPrototypeOf(e) === RangeError.prototype`), so a subclass, a duck-typed object and a thrown non-`Error` (a string) are all `unexpected-error` |

Result typing is exact, as in §5.1, using the language's own distinctions rather
than `typeof` alone: `Bool` is `typeof === "boolean"`; `U64`/`Nat` are a
`bigint` in `0..2^64-1` (a `number` result is `type-mismatch`, even `1`);
`U8` is an integral `number` in `0..255`; `Bytes` is a `Uint8Array` whose
prototype is exactly `Uint8Array.prototype` (a Node `Buffer`, any other typed
array, an `ArrayBuffer`, a plain array and a subclass all fail), not detached.

Mutability is the difference from Python. A `Uint8Array` is mutable, so rule 4
(fresh outputs) is **not** discharged by construction here: the runner passes
every `Bytes` argument as its own fresh copy, so the library can never corrupt
the enumerated domain, and for a `fresh` op the result must not share an
`ArrayBuffer` with any argument it was given. A result that aliases an input is
`type-mismatch` (§6.2's closed failure-id set is unchanged).

The runner is TypeScript executed by Node's type stripping
(`node --experimental-strip-types`), with `.ts` import specifiers and no
syntax that needs a transform (`tsc --noEmit` with `erasableSyntaxOnly`
checks that), so the code that runs is the code that is digested and no build
output can drift from it. It has no runtime dependency and uses `node:crypto`
for SHA-256.

JSON is read by the runner's own strict parser, not `JSON.parse`, which keeps
the *last* of two equal keys and silently rounds a large integer. The parser
rejects duplicate keys, `NaN`, trailing data and nesting deeper than 256 levels, builds
objects with no prototype (a `__proto__` key is data), and refuses any number
that is not a safe integer. Every count and counter in the corpus is far below
2^53; a larger one is a corpus error, never a rounded value. `U64` values
cross the corpus as tagged decimal text and are parsed with `BigInt`.

The commands, environment variables, exit statuses, failure ids, work bounds
(§6.2) and manifest (§8) are identical to the Python lane's. The runner is the
package `typescript/warrant-corpus` (`@coding-adventures/warrant-corpus`, entry
`src/bin.ts`); the manifest's `toolchain` is `{"name": "node", "version": ...}`
and its `version` is, as for Python, diagnostic and ignored by `check`.

## 6. The L0 runner protocol (identical in every lane)

```text
warrant-corpus run
    --corpus <dir>  --binding <warrant.binding.json>
    [--out <manifest>] [--filter <sub>] [--shard i/n]
env: WARRANT_EVIDENCE_FAULT, WARRANT_EVIDENCE_FILTER, WARRANT_EVIDENCE_OUT
stdout (TAP-ish, as TF01 §6):  ok <id> | not ok <id> <failure-id>
exit: 0 all pass; 1 any failure; 2 usage/binding/corpus error
```

Suites run in shuffled order with a recorded seed (VER00 §5; the
`evidence_seed` for portable runs only controls suite order, because case
order is canonical). The runner:

1. loads and validates the corpus (`spec_sha256` of `corpus.json`'s own
   `spec` file must match the on-disk spec, else exit `2`; this is
   `StaleEvidence` caught at run time, not only at check time);
2. loads the binding (rules 1–3 of §5);
3. executes `cases.tfv` records, then each exhaustive suite by enumeration,
   hashing as it goes;
4. runs the negative controls (§4) *after* the clean pass, in-process by
   re-running the affected suites with the wrapped binding;
5. writes the manifest (§8) only when unfaulted, unfiltered, unsharded-or-merged.

### 6.1 Sabotage is not the only thing the runner must refuse

* An exhaustive suite whose computed `cases` differs from the corpus: fail
  (`count-mismatch`).
* `inputs_sha256`/`expected_sha256` differ: fail (`stream-mismatch`).
* `calls` differs from `expected_calls`: fail (`call-count-mismatch`).
* A case whose implementation result is not deep-equal to the model result:
  fail with the case index and *no input bytes printed* (VER00 §7.3: no
  reproducing material in public output; the index plus the corpus
  reproduces it).

### 6.2 Runner command line as delivered for Python -- WD-101 amendment

The Python runner is the package `warrant-corpus`
(`code/packages/python/warrant-corpus`, import `coding_adventures_warrant_corpus`,
run as `python -m coding_adventures_warrant_corpus`; the repository's Python
naming replaces the illustrative `warrant_corpus` of §4.3). Every command takes
`--repo-root`, `--corpus <dir>`, `--binding <file>` and `--package-dir <dir>`.
A package keeps a thin entry (`tests/warrant_evidence.py`, the registry's
`runner_entry`) that fixes those four and forwards the command.

| Command | Behaviour |
|---|---|
| `run [--out F]` | Full clean run: examples, exhaustive suites, then negative controls. Writes the manifest to `--out` (or `WARRANT_EVIDENCE_OUT`) only for an unfaulted, unfiltered, fully passing run |
| `check F` | Same run, but compares the regenerated manifest with the committed `F` and exits `1` on any difference. `toolchain.version` is excluded from the comparison (it is diagnostic, §8) so a different CPython patch release is not drift; everything else, including byte formatting, is |
| `sabotage` | For every corpus control, re-executes `run` in a child process with `WARRANT_EVIDENCE_FAULT=<fault>:<op>` and `WARRANT_EVIDENCE_FILTER=/exhaustive/`, and requires a `not ok ` line, exit code `1` and no manifest. This is §4.3's shell loop made portable, because the lane BUILD runs under both `sh` and `cmd` |
| `list-faults` | Prints `<fault>:<op>` for each corpus control |
| `source-digest --root D` | Prints the `warrant-src-v1` digest of a source tree (§8.1) |
| `validate-manifest M [--registry R] [--require-active]` | Structural validation of a lane manifest against the corpus with the closed `ct-compare-v0` rules of §8 (a Python port of WD-004a's checks); `--require-active` additionally demands an `active` registry row |

`WARRANT_EVIDENCE_FAULT` is `<fault>` (every applicable op) or `<fault>:<op>`.
`WARRANT_EVIDENCE_FILTER` is a substring of suite ids. As in §4.3: a filter
without a fault, `FAULT=none` with a filter, an unknown fault, and a filter
matching no suite are usage errors (exit `2`). A faulted or filtered run stops
each suite at its first failing case and never writes a manifest.

Failure ids are a closed set: `count-mismatch`, `stream-mismatch`,
`call-count-mismatch` (§6.1), `result-mismatch` (the export's value differs from
the model's), `type-mismatch` (right value, wrong Python type), `missing-error`
(the model expects `Err`, the export returned a value), `unexpected-error`
(an exception no binding row maps) and `uncaught-fault` (a negative control no
claim failed). A failing line is `not ok <suite id>@<case index> <failure id>`
followed by `not ok <suite id> <failure id>`; no input bytes are printed.

Bounded work (security review of WD-101): before any case runs, the product of a suite's binder
domains must equal its declared `cases + discarded`, a suite must admit at least one case, one suite's
product is capped (`MAX_SUITE_PRODUCT`, 1,000,000), and the total work (every suite once plus every
control's re-runs) is capped (`MAX_TOTAL_WORK`, 8,000,000); the real `ct-compare` corpus needs about
2.1 million. Model IR has a static cost bound per expression (`MAX_IR_COST`, 2,000 node evaluations
counting each model call at its body's full cost), so a chain of models that each call the next twice
is refused at compile time. Arithmetic operands are range-checked before the operation. A fixed-width
op must state its width in the IR, and any width a signature states (`Bytes<4>`) is asserted per call.

The runner treats the corpus as untrusted: size-capped, duplicate-key-rejecting
JSON; closed key sets; every IR node compiled before the first case runs (an
unknown tag is an error, never a skip); `spec_sha256` re-derived from the spec
bytes on disk; each control re-derived from the fault registry, the op
signatures and the suites that mention the op.

## 7. Layer B: native suites (per lane, gated)

Layer B is VER04h §4–§5 re-hosted: a *per-lane generator* in the Rust compiler
emits, from the same Warrant spec, a suite file in the lane's language that
runs under the lane's own L2 property engine and fuzz target format. It is
**not** a rollout blocker for layer A and is not required for a lane to be
`release-ready` at E3. It is required for E4:

| Prerequisite | Gate |
|---|---|
| lane has a property engine with seeded generators, shrinking and a recorded seed | a green `pbt-v1` corpus for the lane (VER05 family; TypeScript: VER05a) |
| lane has a coverage-guided or at least a deterministic structured fuzzer | a green `fuzz-v1` corpus for the lane (VER06 family; TypeScript: VER06b) |

Until both gates are green, the lane's report states `property: not-run` and
`fuzz: not-run`: **never** the layer-A sample presented as a property result.
The per-lane suite reuses §3's domain names, §4's relation-biased generators,
and the same suite ids, differing only in `kind: property|fuzz`; its manifest
entries carry `layer: "native"`, the lane's engine name and version, and the
seed, and are indistinguishable in schema from Rust's. The per-lane
generators are the ones VER04g (TypeScript) already sketches for TS; this spec
makes the Rust compiler the owner of all of them so one spec edit moves every
lane.

## 8. Per-lane evidence manifest

Same shape as `warrant-test-evidence-v1` (VER04h §8), as a new schema string so
a checker never mistakes a lane manifest for the Rust one, plus the fields a
polyglot check needs:

`code/packages/<lang>/<pkg>/evidence/test-manifest.json`,
`schema: "warrant-lane-evidence-v1"`:

```json
{
  "schema": "warrant-lane-evidence-v1",
  "lane": "python",
  "package": "python/ct-compare",
  "module": "ct_compare",
  "spec": "code/specs/ct-compare.warrant",
  "spec_sha256": "<hex>",
  "corpus": "code/specs/fixtures/ct-compare-warrant-v1",
  "corpus_sha256": "<hex of corpus.json bytes>",
  "cases_sha256": "<hex of cases.tfv bytes>",
  "runner": { "name": "warrant-corpus", "lane": "python", "src_sha256": "<hex>" },
  "binding_sha256": "<hex>",
  "source_sha256": "<hex, warrant-src-v1 over the lane source tree (§8.1)>",
  "toolchain": { "name": "python", "version": "3.12.4" },
  "evidence_seed": "<u64>",
  "faulted": false,
  "tests": [
    { "id": "warrant/ct_compare/example/eq_empty", "layer": "portable",
      "kind": "example", "op": "eq", "cases": 1, "verdict": "pass" },
    { "id": "warrant/ct_compare/exhaustive/eq_agrees", "layer": "portable",
      "kind": "exhaustive", "op": "eq", "claim": "eq_agrees",
      "cases": 120409, "discarded": 0, "calls": 120409,
      "inputs_sha256": "<hex>", "expected_sha256": "<hex>",
      "domain": "Bytes[347] x Bytes[347]", "verdict": "pass" },
    { "id": "warrant/ct_compare/negctl/exhaustive/eq/ignore-last-byte",
      "layer": "portable", "kind": "negctl", "fault": "ignore-last-byte",
      "op": "eq", "target": "exhaustive",
      "caught_by": ["warrant/ct_compare/exhaustive/eq_agrees"],
      "verdict": "pass" }
  ]
}
```

`kind` is the VER04h set. `layer` is `portable` or `native`. A `sample`
kind is added for the materialised sample (`cases`, `equal_cases`,
`seed`). The manifest is deterministic: sorted keys and entries, no timestamps
and no absolute paths. The lane `toolchain` is included for diagnosis and
is not part of equality across lanes.

For the WD-004a `ct-compare-v0` profile, the exact top-level manifest keys are
`schema`, `lane`, `package`, `module`, `spec`, `spec_sha256`, `corpus`,
`corpus_sha256`, `cases_sha256`, `runner`, `binding_sha256`, `source_sha256`,
`toolchain`, `evidence_seed`, `faulted`, and `tests`; the nested `runner`,
`toolchain`, and test-entry keys are closed as well. The only accepted test
kinds are `example`, `exhaustive`, and `negctl`, all with `layer: "portable"`;
native, sample, property, and fuzz entries are rejected. An example row has
exactly `id`, `layer`, `kind`, `op`, `cases`, and `verdict`. Every corpus
example appears once with its exact id and op and `cases: 1`. An exhaustive
row has exactly the keys shown in the first example row above; each exhaustive
suite appears once with its exact id, op, claim, domain, case/discard/call
counts, and input/reference digests. A negative-control row has exactly the
keys shown in the second example row above. Every generated control appears
once. Its manifest `target` value equals the corpus control's `target_kind`,
and `caught_by` contains one or more full claim IDs from that control's
`required_claims` list, with no duplicates. A short display name is not a
claim ID.

`consumers.json` is the per-corpus identity registry, not a list of supported
languages. Its exact top-level keys are `schema_version`, `profile`,
`corpus`, `consumer_count`, `active_consumer_count`, and `consumers`. A row's
exact keys are `lane`, `package`, `module`, `package_root`, `binding`,
`runner_entry`, `evidence_manifest`, `build_files`, and `status`.
`package` is `<lane>/<name>`; rows are unique and sorted by package. The
`warrant-lane-consumers-v1` profile records the lane, package root, binding
path, runner entry, evidence-manifest path, build files, and
`status: "planned"|"active"`. In this first profile, each `build_files` array
contains one or two unique paths. Planned entries reserve identity and paths
only; they cannot authorize a passing lane result. WD-004a registers
`python/ct-compare` as `planned`; no entry becomes active until its binding,
runner, evidence manifest, and build routing exist.

Registry path values are lexically normalized, slash-separated,
repository-relative paths with no empty, `.` or `..` component and no
backslash. The declared `package_root` must equal the path derived from
`package` (for example, `python/ct-compare` maps to
`code/packages/python/ct-compare`). Every file path must be beneath that exact
root; the evidence-manifest path is exactly
`<package_root>/evidence/test-manifest.json`. The registry's `consumer_count`
equals the row count, and `active_consumer_count` equals the number of rows
whose status is `active`. WD-004a checks these strings and joins only; it does
not open paths, verify symlinks or path existence, or attest a real active run.
Filesystem-safe opening and active-consumer readiness belong to the later
runner/report integration.

WD-004a closes the first parser profile to the exact keys shown in §8,
including the nested `runner`, `toolchain`, and test-kind objects.
`evidence_seed` is a canonical decimal string in `[0, 2^64-1]`; JSON numeric
values are not accepted for this field. Hash fields are lowercase 64-character
SHA-256 hex. Manifest, registry, corpus, spec, and TFV bytes are size-capped
before parse; the existing JSON parser rejects duplicate keys and enforces its
depth cap.

The WD-004a Rust API parses bounded manifest/registry bytes using the existing
duplicate-key-rejecting JSON parser, then validates typed values against the
registered consumer and the checked-in corpus profile. Structural checks join
exact IDs, counts, domains, digests, and controls. Synthetic active-consumer
test values exercise successful joins, but no checked-in manifest or report
claims a real Python run. This parser does not verify source/build/toolchain
freshness, execute a binding, produce an assurance report, or change the
Rust-only legacy evidence loader; those remain later WD-004b/WD-1xx work.

**Coverage labels sidecar (WD-101 review).** The manifest keeps the corpus's `kind: "exhaustive"`,
which cannot be split without breaking WD-004a's closed row schema. The honest word lives in
`evidence/domain-labels.json` (`schema: "warrant-lane-domain-labels-v1"`), written beside the manifest
by `run --out`, compared by `check` and `validate-manifest`, and bound to the manifest by a digest
that ignores `toolchain.version`. Each suite is labelled from its domain descriptors: `full` (every
binder enumerates its whole type: `bool`, `u8_all`, `Bytes<0>`, `Bytes<1>`), `bounded-set` (a fixed
strict subset: `u64_edge`, `bytes_d1`, `bytes_n(N>=2)`, `nat_0_64`) or `sample` (reserved for
unstructured draws; no portable descriptor yields one yet); a suite is as weak as its weakest binder.
`cases_by_label` is the headline, not the manifest's exhaustive total: suite-level "exhaustive" totals
conflate the kinds until backlog LP-012 lands. For `ct-compare`: 65,537 `full` cases and 506,572
`bounded-set` cases (2 and 7 suites).

Two field definitions the first lane fixed (WD-101): `runner.src_sha256` is
the same `warrant-src-v1` framing (§8.1) over the runner package's own
production source (`coding_adventures_warrant_corpus/**/*.py`), and
`evidence_seed` is the binding's `seed` (§5.1). `toolchain` is diagnostic and
`check` mode ignores `toolchain.version` (§6.2).

### 8.1 `source_sha256` and the per-language source tree

The digest is the same `warrant-src-v1` framing as Rust (VER04h §8):
length-prefixed, sorted by path bytes, paths relative to the *source root*,
symlink / non-UTF-8 / oversized / too-deep entries are errors. Only the
*definition of which files* is lane-specific. It must cover every file that can
change what the implementation does and nothing that changes only tests, so it
is a **production-source** subset of the existing
`build-tool-v1/language-source-input-registry.json` (which is a build-input
registry that includes tests and build scripts; reuse its component
exclusions and symlink rules, add a `production_roots` field per language):

| Lane | Source root | Included | Excluded |
|---|---|---|---|
| rust | `src/` | `**/*.rs` | (as VER04h) |
| python | `src/<pkg>/` | `**/*.py`, `**/*.pyi`, `py.typed` | `__pycache__` |
| typescript | `src/` | `**/*.ts` | `**/*.test.ts`, `dist/` |
| go | package dir | `*.go` | `*_test.go`, `vendor/` |
| ruby | `lib/` | `**/*.rb` | |
| c | `src/`, `include/` | `**/*.c`, `**/*.h` | `tests/`, `tools/` |
| cpp | `include/`, `src/` | `**/*.hpp`, `**/*.cpp`, `**/*.h` | `tests/`, `tools/` |
| java | `src/main/java/` | `**/*.java` | |
| kotlin | `src/main/kotlin/` | `**/*.kt` | |
| csharp | project dir | `*.cs` (not under `tests/`, `bin/`, `obj/`) | |
| fsharp | project dir | `*.fs` in `.fsproj` order | `tests/`, `bin/`, `obj/` |
| elixir | `lib/` | `**/*.ex` | |
| lua | `src/` | `**/*.lua` | |
| perl | `lib/` | `**/*.pm` | |
| haskell | `src/` | `**/*.hs` | |
| swift | `Sources/<Target>/` | `**/*.swift` | |
| dart | `lib/` | `**/*.dart` | |
| ocaml | `src/` | `**/*.ml`, `**/*.mli` | |

Native-extension packages (an `ext/` or `native/` tree: some Ruby, Python,
Elixir packages in this repo have one) add that tree to the lane's source
roots; a package with a native tree and no declared native root fails
`--check`. The framing is implemented once in `test-bootstrap`
(`manifest::frame_sources`) and exposed through the `ca-assurance-report` CLI
and the `warrant-corpus` runner for each lane to call (`source-digest
--lane python --root …`), so producers cannot drift from the checker.
Lanes never re-implement the framing.

## 9. Report generation and fail-closed checking per lane

`ca-assurance-report` is already generic in its evidence logic but not in its
shell. Today `checker.rs:76` rejects any `package` that does not start with
`rust/`, `lib.rs` is hard-wired to `rust/ct-compare`, and the proof/effect
inputs assume `Cargo`. Generalisation:

| Change | Description |
|---|---|
| Package id | `<lane>/<name>`; the lane must be in the lane registry (§13). The `rust/` prefix check becomes "known lane prefix, and equals the lane in every manifest" |
| Inputs | `--package-dir <dir>` instead of a baked-in path; spec path derived as `code/specs/<name>.warrant` as before |
| Per-lane report | `code/packages/<lang>/<pkg>/assurance_report.json`, one per (lane, package); generated by `ca-assurance-report generate --lane <lane> --package-dir …`, never hand-written |
| Manifest set | `test-manifest.json` is mandatory in every lane. `proof-manifest.json` and `bench-manifest.json` are **optional**; absent means every proof label is `unchecked` and timing is `not-measured` (§10, §11), not failure |
| Capability audit | the effect certificate / capability analyzer evidence is lane-specific (VER08 per language); absent for a lane means the capability rows are `unverified` and `--allow-unverified` is **not** permitted for that row in a lane that claims `release-ready` |

Checks added on top of VER04h §9 (items 1–16 apply to lane manifests unchanged
where they make sense; ids are never accepted from a different lane):

17. **Lane identity.** Report `package` lane == every manifest's `lane` == the
    binding's `lane`; a manifest from another lane is `LaneMismatch`.
18. **Corpus binding.** `corpus_sha256` / `cases_sha256` equal the digests of
    the files on disk, and `spec_sha256` equals both the spec on disk and the
    corpus's recorded `spec_sha256` (`StaleEvidence`).
19. **Stream agreement.** For every `exhaustive` entry, `cases`, `inputs_sha256`
    and `expected_sha256` equal the corpus suite's (`StreamMismatch`); `calls`
    equals the corpus `expected_calls` (`CallCountMismatch`).
20. **Coverage by join.** The set of exhaustive-suite and example IDs in the
    lane manifest equals the corresponding corpus ID sets (`UncoveredClaim`);
    extra ids (`UnknownEvidence`). For WD-004a's ct-compare-v0 profile, this
    means exactly 9 exhaustive suites and 12 examples, with exact op and
    expected case counts.
21. **Negative controls complete.** The control set in the manifest equals
    the corpus's generated `controls` rows, each appears exactly once and
    passes with a non-empty `caught_by` subset of that row's full required
    claim IDs. The manifest's `target` equals the row's `target_kind`. A
    registry fault with no catching claim fails generation unless the
    spec explicitly lists it in `accepted_blind_faults` (`UncaughtFault`).
22. **Faulted or partial manifests are rejected** (`faulted: true`, or a
    filter/shard record in the manifest). The WD-004a profile also rejects
    unrecognized fields and unsupported native/sample/property/fuzz kinds.
23. **Source freshness.** `source_sha256` equals the recomputed
    `warrant-src-v1` digest of the lane source tree (§8.1); editing a source
    file without rerunning makes `--check` fail, as `src/capabilities.rs` does
    for Rust.
24. **Labels are lane-honest.** In a lane whose proof layer is not live every
    `proof.label` is `unchecked`, property/fuzz are `not-run` until layer B's
    gates are green, and `--check` fails a report that states anything
    stronger (`LabelOverclaim`). Reports for lanes at layer A only render the
    exhaustive and example results, never a number copied from Rust.
25. **Cross-lane consistency (report-level, optional `--check-matrix`).** Over
    all lanes of a package, the claim-id set, the spec digest and the corpus
    digest are identical; lane results differ only in manifest fields. A lane
    that passes while another lane's real exhaustive run fails is *not*
    forbidden (a lane can have a real bug); a lane that passes with a
    different digest is.

Each check has a falsification test in `tests/test_evidence_lane.rs` (copy a
manifest, change one field, expect the named error), following VER04h's
practice.

## 10. Layer C: proofs per lane (not-live)

VER04h §7 proves `labels`/`bound` obligations by lowering the real Rust
source through `warrant-ir`. That lowering is the Rust subset front end of
VER25a/b. There is no equivalent front end for the other lanes.

| Lane | Proof layer status | Required for `release-ready`? |
|---|---|---|
| rust | live (VER04h §7) | yes for constant-time rows |
| any other | **not-live** | no; every proof label is `unchecked` |

A future per-lane front end (a subset grammar plus a lowering to WIR) must be
specified lane by lane, as VER25a/b is for Rust, and has its own negative
controls (`proofneg/…`); a front end becomes live only when its lowering is
differentially tested against that lane's execution on a corpus, as VER04h
§10 requires of the Rust inliner. Until then the report must not describe a
timing or constant-time claim for the lane as anything but `unchecked`
(§11). A Warrant spec's `labels … secret` rows are *carried* into the
report so the gap is visible: "constant-time: required by spec, unchecked in
lane python".

## 11. Honest limits

1. **A corpus run proves only enumerated and sampled behaviour.** The 347
   strings of `D(Bytes)` do not include a 1 MiB input. A passing exhaustive
   run says `exhaustive over Bytes[347] x Bytes[347], 120409 cases`, never
   "correct for all inputs". Every lane manifest and report states the domain
   text.
2. **The Warrant model is part of the trusted base.** A wrong `claim` is
   faithfully verified against; `W07xx` checks and the independent Python
   oracle only shrink this.
3. **The runner and binding are trusted at the boundary.** Sabotage (§4) shows
   the runner can fail and reaches the export; it does not show the binding is
   a *faithful* wrapper. Review of the (data-only) binding file and the
   size cap in §5.5 are the mitigation.
4. **Native proofs are not-live.** Constant-time, step and heap bounds are
   proved only for Rust, and even there VER04h §7.1 explains what a collapsed
   miter certifies.
5. **Timing is per lane and is never inherited from Rust.** `ct-compare`'s
   timing story is one Rust source lowered to WIR. A Python `bytes ==`, a JVM
   JIT, Go's `subtle.ConstantTimeCompare`, a .NET `CryptographicOperations`
   call, a C compiler's optimiser, and an interpreted Lua loop each have their
   own timing behaviour. A lane's report states constant-time for that lane
   only if lane-specific evidence exists (a lane proof layer, or measured
   equal-work with its own bench manifest). Otherwise the row reads
   "required by spec; not established in this lane", not "inherits rust".
6. **Different lanes can legitimately differ in what they export.** A lane
   with no `eq_fixed<N>` const-generic uses a runtime width check; the Warrant
   claim is satisfied through the binding, and the *type-level* guarantee
   (mismatched widths cannot compile) is a Rust property the corpus does not
   test elsewhere.
7. **Sandbox conformance is not a security proof** (§12).

## 12. Runtime capability conformance (VER08b hook)

VER08b specifies runtime capability conformance: executing a package's tests
under a sandbox derived from the package's `required_capabilities.json`, so a
package that declares `capabilities: []` (as every ct-compare port does) cannot
touch the filesystem, network, clock, process table or environment during its
tests. This spec adds the hook, deliberately thin because VER08b owns the
mechanism and its arming strategies:

* The corpus run (§6) is run **twice** per lane BUILD: once plain (to write the
  manifest) and once under the VER08b sandbox derived from the lane's
  `required_capabilities.json`, with the **same** corpus, binding and
  wrapper. The sandboxed run must produce an identical `inputs_sha256`,
  `expected_sha256`, `cases` and verdicts as the manifest; it never writes one.
* A sandbox violation (denied syscall, blocked path or host, forbidden
  nondeterminism source) is a runner failure with id
  `capability-violation`, reported as a manifest field `sandbox`:
  `{ "profile_sha256": "<hex>", "arming": "<strategy>", "verdict": "pass" }`.
  `--check` requires `sandbox.verdict == "pass"` and a `profile_sha256` that
  equals the digest of the profile recomputed from
  `required_capabilities.json`, for any lane that claims `release-ready`
  (`SandboxMissing`, `SandboxMismatch`).
* The runner itself needs to read the corpus and write `evidence/`; those
  two paths are the sandbox profile's declared exceptions for the runner, not
  the library, and are fixed (read: corpus dir, binding file; write: the
  manifest path, and only in the plain run).

Arming strategy summary (the mechanism for each lane is VER08b's to define;
this is the expectation each lane's rollout row must meet):

| Lane family | Expected arming strategy |
|---|---|
| rust, c, cpp, go | OS sandbox around the test process (seccomp/landlock profile or container); static link, no loader surprises |
| python, ruby, lua, perl, elixir | interpreter run under the OS sandbox; plus an in-language denylist of `open`/`socket`/`subprocess`-class entry points as defence in depth (not a substitute) |
| java, kotlin, csharp, fsharp | OS sandbox around the JVM/CLR; the in-language security managers are deprecated and are not relied upon |
| typescript | Node permission flags where available, else OS sandbox |
| haskell, ocaml, swift, dart | OS sandbox around the toolchain-produced binary / VM |

Because VER08b is not on `main`, a lane's `release-ready` status in §13 is
conditioned on it: before VER08b lands, `sandbox` is `not-armed` and the
report says so; no lane can be `release-ready` with `sandbox: not-armed`.

## 13. Rollout order and acceptance per lane

### 13.1 Acceptance (every lane, in order)

A lane is **E3-complete** for a package when *all* hold in CI, from a clean
checkout:

1. the lane's `warrant-corpus` runner builds, passes its own conformance
   fixture, and fails its own sabotage fixture;
2. the package has a valid `warrant.binding.json` (§5 rules 1–5 hold);
3. the corpus run passes every example and every exhaustive suite, with
   `cases`, `inputs_sha256` and `expected_sha256` equal to the corpus's;
4. **every** control in `corpus.json.faults` is caught (BUILD-level `grep -q
   '^not ok '` per fault, and the manifest's `negctl` entries all `pass`);
5. `evidence/test-manifest.json` was written by that real run (BUILD writes it;
   the checked-in file is regenerated and `git diff --exit-code` is part of
   the lane's drift step), with `faulted: false` and a `source_sha256` that
   `--check` recomputes;
6. `ca-assurance-report generate --lane <lane>` then `--check` passes in CI,
   with no `--allow-unverified` for evidence ids produced by the runner;
7. the lane's existing tests and coverage still pass (the corpus is added,
   not substituted; a lane's 20-line hand tests stay until the corpus
   makes them redundant, by a reviewed deletion).

A lane is **E4-complete** when, in addition, layer B (§7) runs under the gates
and its manifest entries are checked; **E5** (proof) is Rust only until a
front end exists (§10). **Release-ready** = E3 + sandbox-armed (§12) + the
VER00 publishing checklist (README, CHANGELOG, BUILD/BUILD_windows,
`required_capabilities.json`).

### 13.2 Order

Phase 1 uses lanes that have a toolchain in CI *and* a `ct-compare` port
today. WD-004a parses the new lane schema independently while the existing
Rust loader continues to consume `warrant-test-evidence-v1`. The Rust lane's
manifest migration and any Rust-only legacy acceptance window are deferred to
WD-004b, which must define the transition before report integration.

| Step | Lane | ct-compare port | Why this position | Per-lane runner notes | Gate beyond §13.1 |
|---|---|---|---|---|---|
| 0 | rust | yes (reference) | owns the compiler; emits `corpus.json`; legacy manifest remains on the Rust-only loader pending WD-004b | already has generated suites; add `gen_corpus` | rust runner consumes the same corpus (a *second* implementation of the check; proves the Rust compiler's generators agree with its corpus) |
| 1 | python | yes | stdlib only; arbitrary-precision ints; fastest to write and review; hosts `warrant_corpus_oracle.py`'s sibling | `hashlib`; closures | first lane: its runner conformance fixture is the template for every other lane |
| 2 | typescript | yes | TF01a bootstrap exists; BigInt for U64 | `node:crypto` | layer B first (VER05a/VER06b exist) |
| 3 | go | yes | stdlib `testing`, `crypto/sha256`, `math/bits`; fast enumeration | `(value, error)` normalisation | exhaustive runtime budget set from here |
| 4 | ruby | yes | no bundled-gem dependency in the runner | `Digest::SHA256` | |
| 5 | c | yes | exercises the status-code error model and the function-pointer fault table; native 64-bit | SHA-256 in `warrant-corpus` (vendored, vetted) | `tools/run.sh` BUILD style (`iso-harness`) |
| 6 | cpp | yes | template/`std::function` table | same C SHA-256 core | |
| 7 | java | yes | `MessageDigest`; unsigned-long discipline | gradle task, not a test-runner plugin | |
| 8 | kotlin | yes | shares the JVM runner design with java | | |
| 9 | csharp | yes | `SHA256`; `ulong` | dotnet test with the runner as a project | |
| 10 | fsharp | yes | shares csharp's runner library | | |
| 11 | elixir | yes | immutable binaries; `:crypto`; `{:ok,_}`/`{:error,_}` | `fresh` is `immutable-by-construction` | |
| 12 | lua | **needs port** | `ct-compare` port first (VER26 pattern) | custom SHA-256, `math.ult` | port + spec-conformance + corpus in one PR chain |
| 13 | perl | **needs port** | | `Digest::SHA`, big-int/64-bit check | |
| 14 | haskell | **needs port** | `Word64`, `Either` | cabal/stack toolchain in CI first | |
| 15 | swift | **needs port** | `UInt64`, `Result` | toolchain in CI must exist before the port | |
| 16 | dart | **needs port** | `BigInt` for web, native ints for VM | | |
| 17 | ocaml | **needs port** | `Int64` discipline mandatory | custom SHA-256 | |

A lane may not skip ahead: a lane's runner reuses the failure-mode lessons of
the previous lanes (steps 1 to 3 have to produce the runner conformance fixture
before step 4 starts). A port added for steps 12 to 17 is a *new package* and
follows the whole repository checklist (BUILD, README, CHANGELOG,
required_capabilities.json); its first PR is the port with its hand tests,
its second is the binding and corpus run.

### 13.3 After `ct-compare`: the other leaf libraries

`gf256` is the second package, chosen because its claims are relational
(§4's per-op scoping) and because it exists in 17 of the 18 rollout lanes;
the OCaml package port is still missing:
`mul_commutes`, `mul_assoc`, `add_self_is_zero`, `div_inverts`, and
`mul` agreeing with a `model` shift-and-reduce reference for all 65,536
`(U8, U8)` pairs. That is *under* the 400,000 budget and exhaustive over the
whole domain, so for gf256 the `exhaustive` label is genuinely `all inputs`,
which the report states as `exhaustive over U8 x U8 (complete)`. After it:
the `lz*` family (list/bytes round-trip claims, where `Bytes` domains give
bounded evidence only), then the hashes (known-answer `example`s plus
property claims; exhaustive domains are small, the sample and layer B carry
the weight). The rollout above repeats per package; a package is *spec-first*:
no lane work starts until the Warrant spec for the package is merged, and a
spec change after the corpus lands rebuilds every lane through the
dependency graph.

## 14. Open questions

1. **Lane runner packaging.** One `warrant-corpus` package per lane (this
   spec's assumption), or one runner per lane inside the existing
   test-bootstrap package (TF01a for TypeScript, `test-bootstrap` for Rust)?
   Separate packages keep leaf libraries free of test infrastructure
   dependencies; bootstrap packages already exist for some lanes.
2. **Model IR interpreter trust.** The in-lane model evaluator is checked
   against Rust per case through `expected_sha256`; do we additionally want it
   written *only once* (e.g. compiled to WASM and called from every lane)?
   Rejected for now: it adds a toolchain dependency to every lane's bootstrap.
3. **Relation to the CT01 `ct-compare-v1` corpus.** It also carries
   `byte_positions`/`byte_reads` work-accounting fields that the Warrant
   corpus does not. Keep both, with CT01 as the independent oracle for
   work accounting, or fold the work fields into `warrant-corpus-v1`?
4. **Sharding vs. ceiling.** Is a 120 s per-lane ceiling right for Lua/Perl on
   the 140,000-case suites, or should those lanes shard by default?
5. **Where the layer-B generators live.** In `warrant-grammar` (one compiler,
   this spec's assumption) or in each lane's own toolchain (VER04g's
   TypeScript-compiler approach)? The former keeps one source of truth; the
   latter keeps the lane runnable without Rust.
6. **`accepted_blind_faults`.** Should a spec ever be allowed to accept a
   blind fault, or should an uncatchable fault always force a new claim?
7. **Release-ready before VER08b.** Is E3 without an armed sandbox enough to
   publish a lane's package, or should publishing wait for VER08b?
8. **gf256 spec.** There is no `gf256.warrant` yet; the claims in §4 and §13.3
   are illustrative, and the spec has to be written (and its `exhaustive`
   statement reviewed) before the corpus work.

## 15. Divergences and follow-ups

* WD-004a defines a strict parser for `warrant-lane-evidence-v1` and a
  planned-only consumer registry. It does not migrate Rust's existing
  `warrant-test-evidence-v1` manifests or change the legacy loader. WD-004b
  must specify any Rust-only compatibility window before report integration;
  a legacy Rust manifest is never inferred as a non-Rust lane manifest.
* VER04h §6's fault names are a subset of §4.2's registry; VER04h is updated
  to say its table is the ct-compare slice of the portable registry.
* `ca-assurance-report`'s `rust/`-only `package` check
  (`checker.rs:76`) and the baked-in `rust/ct-compare` in `lib.rs` are
  replaced as in §9.
* The portable corpus relies on the independent Python oracle and on the model
  IR; adding a model form requires updating the compiler, the oracle, and
  every lane's evaluator, which is the cost of the in-lane design and is
  guarded by the shared conformance fixture
  `code/specs/fixtures/warrant-corpus-conformance-v1/`.
* **WD-101 (Python lane) amendments.** (1) The Python binding is the line
  file `evidence.bindings` named by the merged registry, not
  `warrant.binding.json` (§5.1). (2) New `width=implicit` binding value for
  exports that infer `Bytes<N>` from their buffers (§5.1). (3) Result typing is
  by exact Python type, closing the `True == 1` and `bytearray == bytes`
  conflations (§5.1). (4) The runner package is `warrant-corpus`, import
  `coding_adventures_warrant_corpus`; the CLI is §6.2, including the portable
  `sabotage` command and the `check` drift mode that ignores `toolchain.version`.
  (5) The closed failure-id set of §6.2 extends §6.1; mapped exceptions match by exact class; work
  caps and the IR cost bound are defined in §6.2; the coverage-label sidecar is defined in §8. (6) `runner.src_sha256` and
  `evidence_seed` are defined in §8. (7) Not done and still open: the
  `warrant-corpus-conformance-v1` fixture of §9/§13 does not exist yet, so the
  Python package's tests (falsification cases built from the real corpus) are the
  template other lanes should port; the consumer row stays `planned` because
  `ca-assurance-report`'s own tests pin the checked-in row to `planned`, and
  flipping it belongs with WD-004b1's loader; the VER08b sandbox run (§12) is
  not armed for Python (backlog CAP-RT-1xx).
* **WD-102 (TypeScript lane) amendments.** (1) The TypeScript binding is the same
  `evidence.bindings` line file, with the representations, the exact-prototype
  error mapping and the path-valued `import` of §5.2. (2) `U64` and `Nat` are
  `bigint`, `U8` is `number`; result checks are exact (`typeof`, exact
  `Uint8Array` prototype), and a `fresh` op's result may not alias an argument
  (§5.2), which is how rule 4 is discharged for a mutable byte type. (3) The
  corpus is read by a strict JSON parser that rejects duplicate keys and any
  non-safe-integer number. (4) The failure-id set, work bounds, CLI and
  manifest of §6.2 and §8 are unchanged. (5) Still open: the
  `warrant-corpus-conformance-v1` fixture; the registry row for
  `typescript/ct-compare` (the checked-in registry lists only
  `python/ct-compare`, and the TypeScript BUILD therefore validates the
  manifest without `--registry`); the VER08b sandbox run (CAP-RT-1xx,
  TypeScript/Node).
