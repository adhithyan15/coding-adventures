# DT17 Hash Functions v1 Fixture

This fixture binds the Dart DT17 profile to eight Rust public byte-hash
operations and records additional UTF-8 and caller-sample analysis cases.
`cases.json` is 7 MB because it carries one exact expected result for every
input in the bounded exhaustive domain; it does not replace per-case checks
with aggregate hashes.

## Exhaustive layout

`exhaustive.domain` defines every byte string of lengths 0, 1, and 2, in
ascending length and then lexicographic byte order. That is 65,793 inputs. The
`expected_hex_streams` object maps each operation to a concatenated string of
fixed-width lowercase hexadecimal results in that exact order: 8 hex digits
per U32 result and 16 per U64 result. Each consumer must assert the stream
length equals `count * width`, check lowercase hex, and consume all 65,793
entries. Important order boundaries are index 0 (empty), 1 (`00`), 256 (`ff`),
257 (`0000`), and 65,792 (`ffff`).

The eight streams cover FNV-1a 32/64, DJB2 64, polynomial rolling with both
default and default-valued parameter entry points, Murmur3-32 with seed zero
and fixed seed `0x9747b28c`, and SipHash-2-4 with key `00 01 … 0f`. The exact
fixed parameters are in `cases.json`.

## Other case groups

- `known_answers` holds literal known-answer vectors from DT17/Warrant prose.
  They are separate from generated-model values and the Rust integration test
  checks each through the crate's public API.
- `utf8_cases` covers the Rust UTF-8 convenience functions and the Dart string
  wrappers. Their expected values come from UTF-8 encoding plus the independent
  byte oracle. Dart-only invalid Unicode scalar cases are not Rust reference
  calls.
- `directed` covers Murmur tail/block lengths, SipHash tail and block
  boundaries, high bytes, wrap-heavy data, full-width polynomial parameters,
  maximum seed/key values, modulus one, and long multi-block input.
- `generated` and `mutations` use independent SplitMix64 streams with recorded
  seeds. They are deterministic generated cases and byte-flip pairs, not
  coverage-guided fuzzing.
- `analysis` has hand-checkable synthetic callbacks and explicit samples for
  real hash callbacks. It records exact flip/compared bit counts and exact
  bucket counts plus a rational chi-squared value. The Rust crate supplies the
  public hash outputs for those real callbacks; it does not expose the same
  deterministic sample API as the Dart profile.
- `dart_only_invalid` records invalid inputs that cannot be passed to the
  Rust API's typed `&[u8]`, `u32`, `u64`, or `[u8; 16]` parameters. The separate
  `rust_panics` case records that zero polynomial modulus panics in Rust; it
  does not claim Rust returns the Dart `ArgumentError`.

## Independent exact oracle

`oracle.py` implements each algorithm using Python arbitrary-precision
integers. FNV, DJB2, Murmur3, and SipHash reduce only at the specified word
widths; polynomial rolling computes the full multiply-add before modulo. The
Murmur3 and SipHash block and tail words use little-endian byte order. Known
answer values are literal inputs to the validator, not values copied from the
Rust implementation. The Rust public consumer independently checks them.

Regenerate the corpus and validate its canonical bytes and every expected
value with:

```powershell
py -3.13 generate.py
py -3.13 validate.py
```

The Dart VM-only integrity test compares the generated test snapshot to the
canonical `cases.json` bytes. The Dart VM and Node tests consume that snapshot
and execute every fixture case through public Dart APIs.

Warrant's generated Rust test evidence remains a separate suite: 277 TAP tests
passed on the baseline source, but the existing Warrant document covers only
the eight byte operations. Its step-bound proof is `not-live`, depth bounds
are `unchecked`, and SipHash constant-time proof is `not-live`; the fixture
does not claim proof or security properties.

## Execution and provenance boundary

| Evidence surface | Executed evidence | Scope |
| --- | --- | --- |
| Rust byte API | `hash_functions_v1_fixture` calls all eight Rust functions for all 65,793 inputs each, then checks 48 literal answers, 28 directed cases, 128 deterministic generated cases, 32 byte-flip pairs, four UTF-8 wrappers, and exact analysis answers for the public FNV callback. | Rust API behavior represented by its typed byte, seed, parameter, and key inputs. |
| Dart public API | The package test consumer uses the shared snapshot and calls Dart public methods on VM and Node. | Includes Dart-only invalid-value validation and profile analysis callbacks; these are not Rust API conformance claims. |
| Independent model | Python arbitrary-precision oracle validates every fixture value and canonical regeneration. | Cross-checks arithmetic and fixture integrity; it does not derive expected results from either production implementation. |
| Rust Warrant | Baseline `warrant_evidence` emitted 277 passing TAP rows. | Existing Warrant source covers the eight Rust byte operations; it does not cover Dart text wrappers or caller-sample analysis. Its proof rows remain `not-live`/`unchecked` as recorded above. |

The Rust consumer was run from an isolated 20-crate Cargo workspace because
the sparse implementation worktree does not contain the repository's full
1,530-member workspace. Package source files were copied byte-for-byte into
that scratch workspace; its only workspace-boundary adjustment was listing
the 20 recursively required Cargo roots. The test's fixture include path was
adjusted in the scratch copy to point to the identical canonical fixture.
