# ct-compare-warrant-v2

The committed `warrant-corpus-v2` corpus of `code/specs/ct-compare.warrant` (VER04r, slice
CV2-S3d). It is the v2 counterpart of `../ct-compare-warrant-v1/` and replaces it when v1 is
deleted (S13). Until then both exist; no lane reads this directory yet (S6 and S7 move the
Python and TypeScript runners to it).

| File | Written by | What it is |
|---|---|---|
| `corpus.json` | `warrant-corpus-gen` | the canonical corpus: types, ops, models, nine suites, examples, eight faults and controls, digests |
| `chunks/<claim>.txt` | `warrant-corpus-gen` | one file per suite: the chunk lines (`chunks_sha256` in `corpus.json` is the SHA-256 of the file) |
| `cases.tfv` | `warrant-corpus-gen` | the document's `example` rows as a TFV file |
| `corpus.profile` | **a person** | the knobs and the fault list (VER04r 8.4); the only file here that is hand-written |
| `.gitattributes` | a person | `* -text`: git must not touch a byte, because every file is hashed or compared byte for byte |

Nine suites, 572,109 cases, one slim set each (all suites have at most 16 chunks, so every chunk
is in the slim set). `eq_u64/ignore-high-bit` is emitted as data with `requires: ["ir.fixed"]`
and a `deferred` witness; three controls list a *blind* claim (VER04r 5.6, 16.6); `accepted_blind_faults` is `[]`.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand.

```bash
cargo run -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/specs/ct-compare.warrant \
  --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile \
  --bindings code/packages/rust/ct-compare/evidence.bindings \
  --out code/specs/fixtures/ct-compare-warrant-v2
```

`--warrant` must be the repo-relative path (it is recorded as `spec`). `--out` is this directory;
the generator refuses to write into a `chunks/` that holds a file it does not own, and never
deletes one.

## What checks it

Two independent implementations, each run by its package's `BUILD` and `BUILD_windows`:

* **`rust/warrant-corpus-gen`** runs the same command with `--check`: it regenerates the corpus
  and compares `corpus.json`, `cases.tfv`, every chunk file and the exact set of files in
  `chunks/` byte for byte, naming file, line and column of the first difference.
* **`python/warrant-oracle`** runs `verify-v2 code/specs/fixtures/ct-compare-warrant-v2 ...`,
  which re-derives every value of `corpus.json` from the `.warrant` file and the profile with its
  own lowering, evaluator, enumerator and fault search, and compares them (VER04r 10.3). It
  shares no code with the generator, so an agreement is evidence about the document and not
  about one program.

A change to `code/specs/ct-compare.warrant`, to anything under this directory or to
`code/packages/rust/ct-compare/evidence.bindings` selects both packages in the build tool
(`code/programs/go/build-tool/repository_inputs.go`, the rows named `rust/warrant-corpus-gen` and
`python/warrant-oracle`).

## Owner decisions of 2026-11-17 recorded here (VER04r 16.8)

* **[D12] Claims are named by suite id** (`warrant/ct_compare/exhaustive/<claim>`) in
  `required_claims`, `blind_claims` and every `slim_witness` entry.
* **[D13] The generator is pinned.** `corpus.json` records `generator`
  (`{"name": "warrant-corpus-gen", "version": "0.4.0"}`). The generator's `--check` compares those
  bytes, so **a version bump of the generator fails `--check` until this directory is regenerated
  and the diff reviewed**. The oracle's `BUILD` also passes `--expect-generator
  warrant-corpus-gen@0.4.0` to `verify-v2`, so the pin is checked twice and a regenerated
  fixture that is committed without updating the pin fails the oracle's build.
* **[D14] Edits to `corpus.profile` are caught.** `profile_sha256` in `corpus.json` is the SHA-256
  of this directory's `corpus.profile`; both `BUILD` lines pass `--profile
  <this directory>/corpus.profile`, so an edit to the profile that is not followed by a
  regeneration fails both builds. (`--check` itself does not compare the authored files.)
* **[D15] `accept_blind <op> <fault>`** is the profile line that lets a corpus accept an
  all-blind fault. ct-compare has none (its three blind pairs each keep a witnessed claim), so
  `accepted_blind_faults` is `[]` here; the oracle's
  `tests/fixtures/accept-blind-v2/` is a small synthetic corpus that exercises the line in both
  implementations.

## Hashes

`corpus.json` SHA-256 `4a61bfb6561b902a7f91db67638bb4cf449635c321794ff7d5d6179d14654c45`,
`profile_sha256` `848056c69a04716c92117d75e7eb034ecf47710787ec81afd425336302338cee`. They are
informational: the two checks above, not this table, are the gate. If they ever disagree with the
files, regenerate and fix this table in the same commit.
