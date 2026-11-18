# x25519-warrant-v2

The committed `warrant-corpus-v2` corpus of `code/specs/x25519.warrant` (VER04r, CV2 S10 "x25519
adopter"), the second adopter after `../ct-compare-warrant-v2/` and the first whose faults need
`ir.fixed` and `ir.tuple` from the start (the S10c flip opened both). No lane reads this directory
yet; the two checks below are its only consumers.

| File | Written by | What it is |
|---|---|---|
| `corpus.json` | `warrant-corpus-gen` | the canonical corpus: types, ops, models, eight suites, examples, five faults and controls, digests |
| `chunks/<claim>.txt` | `warrant-corpus-gen` | one file per suite: the chunk lines (`chunks_sha256` in `corpus.json` is the SHA-256 of the file) |
| `cases.tfv` | `warrant-corpus-gen` | the document's nine `example` rows (the RFC 7748 vectors and the clamp and low-order examples) as a TFV file |
| `corpus.profile` | **a person** | the fault list (VER04r 8.4); the only file here that is hand-written |
| `.gitattributes` | a person | `* -text`: git must not touch a byte, because every file is hashed or compared byte for byte |

Eight suites, 76,130 cases, one chunk and one slim set each (every suite has a single chunk, so it
is its own slim set). Every operand is a `Bytes<32>` whose domain is a sample (100 values), except
`clamp_edges_match_the_definition` (`Bytes<1>` x `Bytes<1>`, all 256 x 256 values, `full`); the two
two-binder claims over `Bytes<32>` are 10,000 cases of the sample, and the RFC vectors in
`cases.tfv` are what pin real inputs. Default knobs (`budget`, `chunk_size`, `max_ir_cost 2,000`,
`slim_stride auto`), so the profile holds only fault lines.

## Controls

Five faults, taken from `faults=` of `code/packages/rust/x25519/evidence.bindings` (the generator
and the oracle both check that the profile equals that list):

| op | fault | `requires` |
|---|---|---|
| `clamp_scalar` | `drop-last-result-byte` | `ir.core` |
| `clamp_scalar` | `flip-result-bit` | `ir.fixed` |
| `x25519` | `flip-result-bit` (inside the `Ok`) | `ir.fixed`, `ir.tuple` |
| `x25519` | `missing-error` | `ir.core` |
| `x25519_base` | `flip-result-bit` (inside the `Ok`) | `ir.fixed`, `ir.tuple` |

All five controls are **live** in the shipped gate (none is `deferred`), and **no claim is *proven*
blind** (`blind_claims` is `[]` in each; `accepted_blind_faults` is `[]`, so the profile has no
`accept_blind`, `slim_chunk` or `omit_example` line). Every candidate claim is **unproven**
(VER04r 5.6): a stand-in exists only for a `reference` claim whose subject side is a bare call on
plain binders (VER04r 16.6 item 5), and every x25519 claim misses that. It passes a model call as
an argument (`clamp_edges_match_the_definition`, `clamp_leaves_the_middle_alone`), passes a literal
argument (`x25519_at_zero_is_low_order`), wraps the call in `len(..)` (`clamp_keeps_thirty_two_bytes`),
or is a predicate (`clamp_is_idempotent`, `x25519_clamps_its_scalar`, `x25519_at_nine_is_base`,
`x25519_base_is_x25519_at_nine`). So no witness chunk can be searched before a lane runs. The lane's
control therefore fails closed until a runner shows each fault is caught (`required_claims` lists
the claims, `slim_witness` is `{"claim": ..., "unproven": true}` per claim).

Some required claims are in fact blind for some faults but cannot be proven so by the stand-in rule:
`x25519/flip-result-bit` against `x25519_at_zero_is_low_order` (the result is `Err`, unchanged) and
against `x25519_clamps_its_scalar` (the fault cancels on both sides); `clamp_scalar/flip-result-bit`
against `clamp_keeps_thirty_two_bytes` (the length is unchanged) and against
`x25519_clamps_its_scalar` (bit 0 is clamped away). Each control still has at least one required
claim that catches it: `clamp_scalar/drop-last-result-byte` is caught by
`clamp_keeps_thirty_two_bytes` and `clamp_edges_match_the_definition`; `clamp_scalar/flip-result-bit`
by `clamp_edges_match_the_definition`; `x25519/flip-result-bit` by `x25519_at_nine_is_base`;
`x25519/missing-error` by `x25519_at_zero_is_low_order`; `x25519_base/flip-result-bit` by both base
claims. The empty-result forms of the two `Bytes` faults (`01` for `flip-result-bit`, `00` for
`drop-last-result-byte`) were confirmed against the Rust harness before this fixture was committed
(VER04r 8.2, R6): `fault_flip_result_bit` pushes `1` and `fault_drop_last_result_byte` pushes `0`
for an empty byte string (`code/packages/rust/x25519/tests/generated/warrant_evidence.rs`). A
`Bytes<32>` result is never empty, so no committed byte depends on them.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand.

```bash
cargo run --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/specs/x25519.warrant \
  --profile code/specs/fixtures/x25519-warrant-v2/corpus.profile \
  --bindings code/packages/rust/x25519/evidence.bindings \
  --out code/specs/fixtures/x25519-warrant-v2
```

`--warrant` must be the repo-relative path (it is recorded as `spec`). `--out` is this directory;
the generator refuses to write into a `chunks/` that holds a file it does not own, and never
deletes one.

## What checks it

Two independent implementations, each run by its package's `BUILD` and `BUILD_windows`:

* **`rust/warrant-corpus-gen`** runs the same command with `--check`: it regenerates the corpus
  and compares `corpus.json`, `cases.tfv`, every chunk file and the exact set of files in
  `chunks/` byte for byte, naming file, line and column of the first difference.
* **`python/warrant-oracle`** runs `verify-v2 code/specs/fixtures/x25519-warrant-v2 ...
  --expect-generator warrant-corpus-gen@0.6.0`, which re-derives every value of `corpus.json` from
  the `.warrant` file and the profile with its own lowering, evaluator, enumerator and fault
  search, and compares them (VER04r 10.3). It shares no code with the generator. (About one
  second; no sharding is needed, VER04r 16.10 D33.)

A change to `code/specs/x25519.warrant`, to anything under this directory or to
`code/packages/rust/x25519/evidence.bindings` selects both packages in the build tool: the rows
named `rust/warrant-corpus-gen` and `python/warrant-oracle` of
`code/programs/go/build-tool/repository_inputs.go`, driven by its `warrantAdopters` table
(VER04r 16.10 [D32]; one `{module, spec, fixture, bindings}` entry per adopter).
`code/packages/rust/x25519/`'s own `BUILD` does not mention this fixture, as ct-compare's does not
mention its own.

## Bumping the generator version

Exactly the checklist of `../ct-compare-warrant-v2/README.md` ("Bumping the generator version" and
"Changing the shipped gate"), with this directory as one more fixture to regenerate and the
`--expect-generator` pin of the x25519 line in the oracle's `BUILD` and `BUILD_windows` as one more
pin to move. The `corpus.json` hash below changes with `generator.version`.

## Hashes

`corpus.json` SHA-256 `d05e5e6e8ba55314a26d1b8d6a7bbf588e817628949d77a54777d23b1fb6a5d7`,
`cases.tfv` SHA-256 `ccf2fed7d2ede37ca77bd4f1effadfe342b4ef74f78d3d06c6e6e8f5b81ecb45`,
`profile_sha256` `d9309609db0d80a535258bdf4ff993721230f9c48d1835a207c16263b44c20e1`. They are
informational: the two checks above, not this table, are the gate. If they ever disagree with the
files, regenerate and fix this table in the same commit.
