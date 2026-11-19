# wasm-leb128-warrant-v2

The committed `warrant-corpus-v2` corpus of `code/specs/wasm-leb128.warrant` (VER04r, CV2 S10 "wasm-leb128
adopter"), the fourth adopter after `../ct-compare-warrant-v2/`, `../x25519-warrant-v2/` and
`../lzw-warrant-v2/`, and the first whose values are tuples (`Result<(U64,Nat),Leb128Error>`), whose
claims `match` on a `Result`, and whose controls include the composite faults. No lane reads this directory
yet; the two checks below are its only consumers.

| File | Written by | What it is |
|---|---|---|
| `corpus.json` | `warrant-corpus-gen` | the canonical corpus: types, ops, models, 44 suites, 39 examples, 20 faults and controls, digests |
| `chunks/<claim>.txt` | `warrant-corpus-gen` | one file per suite (44): the chunk lines (`chunks_sha256` in `corpus.json` is the SHA-256 of the file) |
| `cases.tfv` | `warrant-corpus-gen` | the document's 39 `example` rows as a TFV file (none is omitted) |
| `corpus.profile` | **a person** | the 20 `fault` lines (VER04r 8.4); the only file here that is hand-written. No `omit_example`, `max_ir_cost`, `accept_blind` or `slim_chunk` line is needed |
| `.gitattributes` | a person | `* -text`: git must not touch a byte, because every file is hashed or compared byte for byte |

44 suites (all `exhaustive`), 46,566,930 cases (product ordinals 46,763,382, of which 196,452 are discarded
by the claims' refinements), 740 chunks (`chunk_size` 65,536, default knobs `budget` 2^24 and `slim_stride
auto`), 48,115,758 expected subject calls. 28 suites have one chunk. The big ones: the two
`decode_*_three_bytes` claims (every 3-byte input, 2^24 cases = the `budget`, nothing demoted) have 256
chunks each, of which the slim set holds 15 and 16; the two `decode_*_bounded_two_bytes_every_width` 65
chunks (slim 14 and 15); the two `decode_*_bounded_result_fits_width` 23 (slim 12 and 13). So **slim is not
full here**: 137 chunks in all across the 44 slim sets against 740 for the full corpus.

`levels` are `ir.core`, `ir.fixed`, `ir.tuple`: 36 of the 44 suites require `ir.tuple` (every decoder
claim: a `Result<(U64,Nat),E>` value), the 8 others are encoder claims. The signed `fx_edge` domain
(VER04r D21, 623 values) is bound in **8** suites (7 before the D28 split below), the `u64_edge` domain
(377 values) in 14, `u8_all` in 10, the 16-bit `fx_full` (65,536) in 6, `bytes_d1` (347) in 16 and
`nat_0_64` (65) in 20 binders. Four claims `match` on a subject call's `Result`
(`decode_*_consumed_is_bounded`, `decode_*_bounded_result_fits_width`): none is `W0701`, because the subject
call is the scrutinee and no arm holds one (VER04r 16.10 R13(b)).

## The D28 split

`wasm-leb128.warrant` had two claims, `encode_unsigned_at_most_ten_bytes` and `encode_signed_at_most_ten_bytes`,
of the form `len(f(v)) >= 1 and len(f(v)) <= 10`: a subject call under an impure guard, `W0701` under VER04r
4.4, so neither tool could emit them. VER04r [D28] split each into two, made in the PR that added this
directory (42 to 44 claims, the Rust evidence of `code/packages/rust/wasm-leb128/` regenerated): a new
`encode_unsigned_at_least_one_byte` / `encode_signed_at_least_one_byte` (`len(f(v)) >= 1`) inserted
immediately before `encode_*_at_most_ten_bytes`, which keeps its name and now states `len(f(v)) <= 10`.

## Controls

Twenty faults, taken from `faults=` of `code/packages/rust/wasm-leb128/evidence.bindings` (the generator and
the oracle both check that the profile equals that list). All 20 controls are **live** in the shipped gate
(none is `deferred`), `target_kind` `exhaustive`:

| op | fault | form | `requires` | required claims | witnessed | unproven | proven blind |
|---|---|---|---|---:|---:|---:|---|
| `encode_unsigned` | `drop-last-result-byte` | post | `ir.core` | 8 | 1 (`encode_unsigned_matches_canonical_model`, chunk 0) | 7 | none |
| `encode_unsigned` | `flip-result-bit` | post | `ir.fixed` | 8 | 1 (same) | 7 | none |
| `encode_signed` | `drop-last-result-byte` | post | `ir.core` | 8 | 1 (`encode_signed_matches_canonical_model`, chunk 0) | 7 | none |
| `encode_signed` | `flip-result-bit` | post | `ir.fixed` | 8 | 1 (same) | 7 | none |
| `decode_unsigned`, `decode_signed` | `drop-consumed` | post | `ir.tuple` | 10 | 0 | 10 | `decode_*_offset_past_end_rejected` |
| `decode_unsigned`, `decode_signed` | `ok-as-err` | post | `ir.tuple` | 10 | 0 | 10 | `decode_*_offset_past_end_rejected` |
| `decode_unsigned`, `decode_signed` | `off-by-one-consumed` | post | `ir.fixed`, `ir.tuple` | 10 | 0 | 10 | `decode_*_offset_past_end_rejected` |
| `decode_unsigned`, `decode_signed` | `missing-error` | replace | `ir.tuple` | 11 | 1 (`decode_*_offset_past_end_rejected`, chunk 0) | 10 | none |
| `decode_unsigned_bounded`, `decode_signed_bounded` | `drop-consumed`, `ok-as-err`, `missing-error` | post, post, replace | `ir.tuple` | 8 | 0 | 8 | none |
| `decode_unsigned_bounded`, `decode_signed_bounded` | `off-by-one-consumed` | post | `ir.fixed`, `ir.tuple` | 8 | 0 | 8 | none |

(The row for the two plain decoders stands for four controls each, one per decoder; there are 4 + 8 + 8 = 20
controls.) A lane that declares only `ir.core` records 18 of the 20 as `not-live` (only the two encoder
`drop-last-result-byte` controls are `ir.core`).

* A witnessed claim is a stand-in (VER04r 16.6 item 5): only claims that are `op(b1..bk) == model/Err` on
  plain binders have one, which is why the encoders have one witness each (the canonical-model claim),
  `missing-error` is witnessed on the plain decoders (the claim expects `Err`, the fault returns `Ok`) and the
  bounded decoders, whose every claim passes `wrap<U32>(n)`, a literal or a computed argument, have none:
  all 8 of their required claims are *unproven* per control and the lane's control fails closed until a runner
  shows each fault is caught.
* `drop-consumed`, `ok-as-err` and `off-by-one-consumed` are **proven blind** on the plain decoders'
  `*_offset_past_end_rejected` claim (the stand-in expects an `Err`, which all three leave alone). Not a
  single control is blind on *all* of its claims, so `accepted_blind_faults` is `[]` and no `accept_blind` or
  `slim_chunk` line is needed.
* `off-by-one-consumed` (VER04r 16.11 [D36], 8.2): inside an `Ok`, the `Nat` consumed count `n` becomes
  `n + 1` for even `n` and `n - 1` for odd `n` (the Rust harness's rule); it needs `ir.fixed` (`rem`,
  `unwrap_or`) and `ir.tuple`, and is shaped exactly like `drop-consumed` for witnesses.
* `missing-error` (VER04r 16.11 [D37], R3): the tuple-payload form is `if is_err(result) then Ok((0, offset))
  else result`, which is **not value-identical** to the Rust harness (`Ok((0, 0))` unsigned, `Ok((i64::MIN,
  0))` signed). Both turn every `Err` into an `Ok` and leave every `Ok` alone, so the same claims catch both;
  the measured witness is `decode_*_offset_past_end_rejected`, chunk 0.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand. About 2 min 15 s in
release mode (`--release`; a debug build takes roughly 20 minutes, which is why the BUILD lines are release).

```bash
cargo run --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/specs/wasm-leb128.warrant \
  --profile code/specs/fixtures/wasm-leb128-warrant-v2/corpus.profile \
  --bindings code/packages/rust/wasm-leb128/evidence.bindings \
  --out code/specs/fixtures/wasm-leb128-warrant-v2
```

`--warrant` must be the repo-relative path (it is recorded as `spec`). `--out` is this directory;
the generator refuses to write into a `chunks/` that holds a file it does not own, and never
deletes one.

## What checks it

Two independent implementations, each run by its package's `BUILD` and `BUILD_windows`, **one line each, not
sharded** (VER04r 16.11 [D38]):

* **`rust/warrant-corpus-gen`** runs the same command with `--check --release`: it regenerates the corpus
  and compares `corpus.json`, `cases.tfv`, every chunk file and the exact set of files in
  `chunks/` byte for byte, naming file, line and column of the first difference. Measured on a 4-core VM:
  **2 min 11 s** (generating takes about the same).
* **`python/warrant-oracle`** runs `verify-v2 code/specs/fixtures/wasm-leb128-warrant-v2 ...
  --expect-generator warrant-corpus-gen@0.6.0`, which re-derives every value of `corpus.json` from
  the `.warrant` file and the profile with its own lowering, evaluator, enumerator and fault
  search, and compares them (VER04r 10.3). It shares no code with the generator. Its full derive of
  all 46,566,930 cases takes **4 min 53 s** on one core (CPython 3.13), 15 times lzw's 20 s: by far the most
  expensive line of the package (an estimate for a hosted runner is 8 to 12 minutes). `verify-v2` has no
  shard option today; sharding by chunk (VER04r 10.4, [D33]) is the documented remedy if the line ever stops
  fitting, and a miss is a blocker to report, never a reason to merge unverified.

A change to `code/specs/wasm-leb128.warrant`, to anything under this directory or to
`code/packages/rust/wasm-leb128/evidence.bindings` selects both packages in the build tool: the rows named
`rust/warrant-corpus-gen` and `python/warrant-oracle` of
`code/programs/go/build-tool/repository_inputs.go`, driven by its `warrantAdopters` table (VER04r
16.10 [D32]).

## Bumping the generator version

Exactly the checklist of `../ct-compare-warrant-v2/README.md` ("Bumping the generator version" and
"Changing the shipped gate"), with this directory as one more fixture to regenerate and the
`--expect-generator` pin of the wasm-leb128 line in the oracle's `BUILD` and `BUILD_windows` as one more pin
to move. The `corpus.json` hash below changes with `generator.version`.

## Hashes

`corpus.json` SHA-256 `14b905aca5685cc9b5d4c38e3140d2edecdcf60443b0995f2d41c4c0946025d0`,
`cases.tfv` SHA-256 `832ec1dfe846a910734f3c0c9a22ad83ca14a5aaeb1c3cef4b429200debdbac7`,
`profile_sha256` `dfab83620bc5f194e1238e3c73e3a7be813b8fb63d0e0b572efa6ca71c9f4e7f`,
`spec_sha256` `8cd7b64e104420d2aa7f66cca1b1cb638cf3c63d3112ec318f62c342beacc4e5`. They are
informational: the two checks above, not this table, are the gate. If they ever disagree with the
files, regenerate and fix this table in the same commit.
