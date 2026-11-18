# lzw-warrant-v2

The committed `warrant-corpus-v2` corpus of `code/specs/lzw.warrant` (VER04r, CV2 S10 "lzw
adopter"), the third adopter after `../ct-compare-warrant-v2/` and `../x25519-warrant-v2/`, and the
first whose profile carries `omit_example` lines and a raised `max_ir_cost`. No lane reads this
directory yet; the two checks below are its only consumers.

| File | Written by | What it is |
|---|---|---|
| `corpus.json` | `warrant-corpus-gen` | the canonical corpus: types, ops, models, 21 suites, 21 examples, five faults and controls, `omitted_examples`, digests |
| `chunks/<claim>.txt` | `warrant-corpus-gen` | one file per suite: the chunk lines (`chunks_sha256` in `corpus.json` is the SHA-256 of the file) |
| `cases.tfv` | `warrant-corpus-gen` | the document's 21 emitted `example` rows as a TFV file (the six omitted bomb examples are not in it) |
| `corpus.profile` | **a person** | `max_ir_cost`, the fault list and the `omit_example` lines (VER04r 8.4); the only file here that is hand-written |
| `.gitattributes` | a person | `* -text`: git must not touch a byte, because every file is hashed or compared byte for byte |

21 suites, 17,563,819 cases, 281 chunks (`chunk_size` 65,536, default knobs `budget` 2^24 and
`slim_stride auto`). 20 suites have one or two chunks, so for them the slim set is every chunk. The
exception is `decompress_three_bytes_rejected`, the exhaustive enumeration of every 3-byte input
(16,777,216 cases, a `U16` times a `U8`): 256 chunks, of which the slim set holds 15 (chunk 0 and then
every 19th, ending with chunk 255). So **slim is not full here**: 40 chunks in all across the 21 slim
sets against 281 for the full corpus. `decompress_missing_clear_rejected` discards 313 candidate
inputs (the guard of the claim); every other suite discards none.

## Controls

Five faults, taken from `faults=` of `code/packages/rust/lzw/evidence.bindings` (the generator and the
oracle both check that the profile equals that list):

| op | fault | `requires` |
|---|---|---|
| `compress` | `drop-last-result-byte` | `ir.core` |
| `compress` | `flip-result-bit` | `ir.fixed` |
| `decompress` | `drop-last-result-byte` (inside the `Ok`) | `ir.core`, `ir.tuple` |
| `decompress` | `flip-result-bit` (inside the `Ok`) | `ir.fixed`, `ir.tuple` |
| `decompress` | `missing-error` | `ir.core` |

The corpus `levels` are `ir.core`, `ir.fixed`, `ir.tuple`. All five controls are **live** in the shipped
gate (none is `deferred`), **no claim is *proven* blind** (`blind_claims` is `[]` in each) and **no
control has a witnessed claim**: every candidate claim is **unproven** (VER04r 5.6), because a stand-in
exists only for a `reference` claim whose subject side is a bare call on plain binders (VER04r 16.6
item 5). In lzw, one claim is a predicate (`compress_is_at_least_seven_bytes`), five nest the subject call
inside another (the round trips), and every other one (15) passes a computed argument (e.g. `be_bytes(x)`,
`concat(..)`) or a literal (`b""`) rather than the bare binders. So `slim_witness`
is `{"claim": ..., "unproven": true}` for each of the 8 required claims of the two `compress` controls
and each of the 18 of the three `decompress` ones, and the lane's control fails closed until a runner
shows each fault is caught. `accepted_blind_faults` is `[]`, so the profile has no `accept_blind` or
`slim_chunk` line.

## Omitted examples and `max_ir_cost`

The six `bomb_*` examples of `lzw.warrant` (decompression bombs, SEC-002, CWE-409) cannot be emitted as
`examples` rows, so the profile has an `omit_example` line for each (VER04r 8.4, D22, D34). The reason is
computed by the tool, never written in the profile, and `corpus.json` records it under the conditional key
`omitted_examples`, sorted by id:

| example (`warrant/lzw/example/...`) | reason | why |
|---|---|---|
| `bomb_1024_units_declaring_1024_bytes_delivers_them` | `value-over-limit` | the argument `bomb1024(1024u32)` is a 285,696-byte stream, over the 4,096-byte limit |
| `bomb_1024_units_declaring_exactly_the_cap_delivers_the_cap` | `not-op-eq-pure` | the body is `match decompress(..) with Ok(r) -> .. \| Err(e) -> ..`, not an equality |
| `bomb_1024_units_declaring_one_byte_over_the_cap_is_rejected` | `value-over-limit` | the argument is a 285,696-byte stream |
| `bomb_1024_units_declaring_one_gibibyte_is_rejected` | `value-over-limit` | the argument is a 285,696-byte stream |
| `bomb_1024_units_is_rejected` | `value-over-limit` | the argument is a 285,696-byte stream |
| `bomb_512_units_is_under_the_cap_and_decodes` | `not-op-eq-pure` | the body is a `match` on the `Result`, not an equality |

The two `match` examples are still lowered (which needs `ir.tuple`); the four equality-shaped ones need
`be_bytes` evaluated (`ir.fixed`) to size their arguments. No other example is omitted: the 21 others are
emitted and appear in `cases.tfv`. A line for an emittable example would be an error.

`max_ir_cost 16384`: 15,105 is the total `ir_cost` (admit, body and expected) of the suite
`round_trip_of_long_runs` (the next largest suite costs 237). The `max_ir_cost` knob bounds each expression,
and the default 8192 is refused at that suite; 16,384 is the value the spec prescribes (the smallest power of
two above 8,192 that admits it). The profile line is the only way to raise it (VER04r 4.2 rule 6, ceiling
65,536) and the value is recorded in `corpus.json`.

## Regenerate

From the repository root. Never edit `corpus.json`, `cases.tfv` or `chunks/` by hand. About 25 s in
release mode (`--release`; about 3 min in a debug build).

```bash
cargo run --release --manifest-path code/packages/rust/Cargo.toml -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/specs/lzw.warrant \
  --profile code/specs/fixtures/lzw-warrant-v2/corpus.profile \
  --bindings code/packages/rust/lzw/evidence.bindings \
  --out code/specs/fixtures/lzw-warrant-v2
```

`--warrant` must be the repo-relative path (it is recorded as `spec`). `--out` is this directory;
the generator refuses to write into a `chunks/` that holds a file it does not own, and never
deletes one.

## What checks it

Two independent implementations, each run by its package's `BUILD` and `BUILD_windows`:

* **`rust/warrant-corpus-gen`** runs the same command with `--check`: it regenerates the corpus
  and compares `corpus.json`, `cases.tfv`, every chunk file and the exact set of files in
  `chunks/` byte for byte, naming file, line and column of the first difference.
* **`python/warrant-oracle`** runs `verify-v2 code/specs/fixtures/lzw-warrant-v2 ...
  --expect-generator warrant-corpus-gen@0.6.0`, which re-derives every value of `corpus.json` from
  the `.warrant` file and the profile with its own lowering, evaluator, enumerator and fault
  search, and compares them (VER04r 10.3), including the six `omitted_examples` rows. It shares no
  code with the generator. Its full derive of all 17,563,819 cases takes about 20 s on one core, so
  one line suffices and no sharding by chunk is needed (VER04r 16.10 D33).

A change to `code/specs/lzw.warrant`, to anything under this directory or to
`code/packages/rust/lzw/evidence.bindings` selects both packages in the build tool: the rows named
`rust/warrant-corpus-gen` and `python/warrant-oracle` of
`code/programs/go/build-tool/repository_inputs.go`, driven by its `warrantAdopters` table (VER04r
16.10 [D32]).

## Bumping the generator version

Exactly the checklist of `../ct-compare-warrant-v2/README.md` ("Bumping the generator version" and
"Changing the shipped gate"), with this directory as one more fixture to regenerate and the
`--expect-generator` pin of the lzw line in the oracle's `BUILD` and `BUILD_windows` as one more pin
to move. The `corpus.json` hash below changes with `generator.version`.

## Hashes

`corpus.json` SHA-256 `8e29136ca73e6887e574153bce7c8e271d6b63ed09c2658b5d9db9fd63962948`,
`cases.tfv` SHA-256 `7afcd94debd97c56728aaaf0414525ff753d45bc435982abe23dda2c074c1e0d`,
`profile_sha256` `fb99a35bab8c1dc016a7d34352f3b23a892851c49ceea2f061483081563fde39`. They are
informational: the two checks above, not this table, are the gate. If they ever disagree with the
files, regenerate and fix this table in the same commit.
