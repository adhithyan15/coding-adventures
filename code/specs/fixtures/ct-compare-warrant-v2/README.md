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
and, since the S10c flip (D29), a live witness (`{"claim": ".../eq_u64_agrees", "chunk": 0}`; before the flip
it was `{"deferred": "ir.fixed"}`); three controls list a *blind* claim (VER04r 5.6, 16.6); `accepted_blind_faults` is `[]`.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand.

```bash
cargo run --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
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

## Bumping the generator version

A new `warrant-corpus-gen` version changes `generator.version` in every corpus it writes, so
all of these change together (D13); the builds fail until each is done and the diffs reviewed:

1. `code/packages/rust/warrant-corpus-gen/Cargo.toml` and `code/packages/rust/Cargo.lock`.
2. Regenerate this directory (the command above) and review the diff: only `generator.version`
   should change unless the generator changed behaviour or the shipped gate changed (see "Changing
   the shipped gate" below).
3. Regenerate `code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v2/corpus/`
   (the generator's test `the_committed_accept_blind_fixture_is_what_the_generator_writes`
   compares its bytes).
4. `--expect-generator warrant-corpus-gen@<version>` in `code/packages/python/warrant-oracle/BUILD`
   and `BUILD_windows` (the lines for ct-compare **and** for `tests/fixtures/synth-v2`), and the
   version pinned in `tests/test_verify_v2.py`. Regenerate
   `code/packages/python/warrant-oracle/tests/fixtures/synth-v2/corpus/` too (command in its
   README): its `generator.version` is pinned by that BUILD line, so a stale one fails the build.
   Unlike the accept-blind fixture, the generator's own tests do not compare it.
5. Regenerate `code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/corpus/`
   (`python3 make_fixture.py` there, after moving its `GENERATOR` pin; the generator's test
   `the_committed_omit_example_fixture_is_what_the_generator_writes` compares its bytes, and the
   oracle's `BUILD` and `BUILD_windows` pin it with `--expect-generator`, the same lines as in step 4).
6. The `corpus.json` hash in the Hashes table below.

## Changing the shipped gate

The **fault-evaluation gate** (generator `SHIPPED_GATE`, oracle `faults.IMPLEMENTED`) decides which
control witnesses are searched and which are `deferred`. It is not a version pin, but it moves
bytes in this directory, so it changes in **both tools in one PR** (D29): the generator's `--check`
and the oracle's `verify-v2` both read `corpus.json`, and a tool that disagrees about a control
fails CI. Opening a level (the S10c flip, which opened `ir.fixed` and `ir.tuple` and bumped the
tools to 0.6.0 and 0.5.0) turns each `{"deferred": L}` witness of a fault needing `L` into a
`chunk` witness, here `eq_u64/ignore-high-bit`. Do the Bumping checklist above, then also:
regenerate the synth-v2 fixture (its `bump/xor-lsb` and `wide/xor-lsb` controls move) and review
every diff by hand, because synth-v2 has no generator `--check`; confirm the committed parity dumps
(`tuple-forms-v1`, `fixed-eval-v1`, `fixed-forms-v1`) did not move; and keep the closed-gate tests,
which pass the gate explicitly (`&[Level::Core]`, `implemented=("ir.core",)`) so the `deferred`
path stays covered. `python/warrant-oracle/tests/fixtures/ct-compare-v2-preflip/` holds this
corpus as the 0.5.0 generator wrote it, and a test asserts `verify-v2` refuses it.

## Owner decisions of 2026-11-17 recorded here (VER04r 16.8)

* **[D12] Claims are named by suite id** (`warrant/ct_compare/exhaustive/<claim>`) in
  `required_claims`, `blind_claims` and every `slim_witness` entry.
* **[D13] The generator is pinned.** `corpus.json` records `generator`
  (`{"name": "warrant-corpus-gen", "version": "0.6.0"}`). The generator's `--check` compares those
  bytes, so **a version bump of the generator fails `--check` until this directory is regenerated
  and the diff reviewed**. The oracle's `BUILD` also passes `--expect-generator
  warrant-corpus-gen@0.6.0` to `verify-v2`, so the pin is checked twice and a regenerated
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

`corpus.json` SHA-256 `9826f2ea9e0e37a5229d648b3ab56b2a7520b6491c4eee7fdc9475308114c716`,
`profile_sha256` `848056c69a04716c92117d75e7eb034ecf47710787ec81afd425336302338cee`. They are
informational: the two checks above, not this table, are the gate. If they ever disagree with the
files, regenerate and fix this table in the same commit.
