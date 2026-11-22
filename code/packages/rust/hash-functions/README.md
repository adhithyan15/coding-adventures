# coding_adventures_hash_functions

DT17 hash functions implemented from scratch in Rust.

This crate covers the full DT17 surface:

- FNV-1a 32-bit
- FNV-1a 64-bit
- DJB2
- Polynomial rolling hash
- MurmurHash3 32-bit
- SipHash-2-4

The `avalanche_score` and `distribution_test` analysis helpers moved to
[`coding_adventures_hash_analysis`](../hash-analysis) in version 0.2.0: they draw OS
randomness, and this crate is pure computation.

## Usage

```rust
use coding_adventures_hash_functions::{
    djb2, fnv1a_32, fnv1a_64, hash_str_fnv1a_32,
    hash_str_siphash, murmur3_32, polynomial_rolling, siphash_2_4, Fnv1a32, HashFunction,
    SipHash24,
};

// The SipHash-2-4 reference key 00 01 .. 0f, so the first published test
// vector (empty message) applies.
let key: [u8; 16] = core::array::from_fn(|i| i as u8);

assert_eq!(fnv1a_32(b"abc"), 0x1a47_e90b);
assert_eq!(fnv1a_64(b"abc"), 0xe71f_a219_0541_574b);
assert_eq!(djb2(b"abc"), 193_485_963);
assert_eq!(polynomial_rolling(b"abc"), 96_354);
assert_eq!(murmur3_32(b"abc"), 0xB3DD_93FA);
assert_eq!(siphash_2_4(b"", &key), 0x726f_db47_dd0e_0e31);

assert_eq!(hash_str_fnv1a_32("hello"), fnv1a_32(b"hello"));
assert_eq!(hash_str_siphash("hello", &key), siphash_2_4(b"hello", &key));

let hasher = SipHash24::new(key);
assert_eq!(hasher.output_bits(), 64);
```

## Not cryptographic

FNV-1a, DJB2, the polynomial rolling hash and MurmurHash3 are non-cryptographic hashes
for hash tables and sketches: they are not collision resistant and must not be used for
integrity, signatures or passwords. SipHash-2-4 is a keyed pseudorandom function whose
secret key gives hash-flooding resistance for hash tables; it is not a MAC for
authentication and this crate makes no such claim.

## Building and Testing

```bash
cargo test -p coding_adventures_hash_functions -- --nocapture
```

`tests/hash_functions_v1_fixture.rs` also consumes the shared DT17 v1 corpus and
checks the eight public byte-hash operations, including parameterized
polynomial rolling with a nondefault base, plus the public UTF-8 helpers and
FNV callback analysis cases. This is fixture conformance coverage; it does not
extend the Warrant contract or change the crate's runtime API.

## Warrant status

[`code/specs/hash-functions.warrant`](../../../specs/hash-functions.warrant) covers
the eight byte-string hashes (`fnv1a_32`, `fnv1a_64`, `djb2`,
`polynomial_rolling`, `polynomial_rolling_with_params`, `murmur3_32`,
`murmur3_32_with_seed`, `siphash_2_4`): 73 known-answer examples plus bounded
exhaustive, property and fuzz suites, in `tests/generated/warrant_evidence.rs`
(generated, never edit it; `BUILD` fails if it drifts) and run against the real
exports. Nine sabotage lines in `BUILD` prove a deliberately broken result is
caught (`flip-result-bit` on each `u32`/`u64` result and on the hash inside
`polynomial_rolling_with_params`' `Ok`, `missing-error` on the zero-modulus
rejection). `BUILD` also fails if the committed `evidence/*.json` manifests differ
from the freshly written ones (`git diff --exit-code -- evidence/`; they are
deterministic; the report `--check` cross-checks them too). The usage
example above is run as a doctest (`cargo test --doc`). `tests/runtime_capability.rs` runs the covered hashes
inside a no-capability sandbox on Linux.

Every operation also declares `steps` and `depth` bound rows (declarations: the proof
manifest records the steps obligations `not-live`). The capability analyzer passes
against `"capabilities": []` and `effect_certificate.json` records it; with
`assurance-profile.json` (`non-cryptographic-hash`) the package report
`assurance_report.json` is generated from the manifests and checked in `BUILD`. It shows
CWE-330 excluded (no randomness in this crate), CWE-400 and CWE-208 as unapproved
exemptions (no live step-bound or constant-time proof; the SipHash key is `secret`),
verdict `report-only`.

Note on the zero modulus: the real `polynomial_rolling_with_params` panics on
`modulus == 0` (`assert!(modulus > 0)`). The Warrant row `ensures no_panic` on that
parameter holds only of the generated harness adapter, which catches any panic and
reports it as `MODULUS_ZERO`; it is not a statement about the crate.

Not covered, with reasons: `hash_str_*` (text), the `HashFunction` trait and its
structs (stateful objects), and the constants. The `avalanche_score` and
`distribution_test` helpers are no longer here: they moved to
[`coding_adventures_hash_analysis`](../hash-analysis), because `avalanche_score` draws
OS randomness. See [DT17a](../../../specs/DT17a-hash-functions-warrant.md).

```bash
# Regenerate the harness after editing the .warrant document
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- hash-functions
```
