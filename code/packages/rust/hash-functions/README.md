# coding_adventures_hash_functions

DT17 hash functions implemented from scratch in Rust.

This crate covers the full DT17 surface:

- FNV-1a 32-bit
- FNV-1a 64-bit
- DJB2
- Polynomial rolling hash
- MurmurHash3 32-bit
- SipHash-2-4
- `avalanche_score`
- `distribution_test`

## Usage

```rust
use coding_adventures_hash_functions::{
    avalanche_score, djb2, distribution_test, fnv1a_32, fnv1a_64, hash_str_fnv1a_32,
    hash_str_siphash, murmur3_32, polynomial_rolling, siphash_2_4, Fnv1a32, HashFunction,
    SipHash24,
};

// The SipHash-2-4 reference key 00 01 .. 0f, so the first published test
// vector (empty message) applies.
let key: [u8; 16] = core::array::from_fn(|i| i as u8);

assert_eq!(fnv1a_32(b"abc"), 0x1a47_e90b);
assert_eq!(fnv1a_64(b"abc"), 0xaf63_dc4c_8601_ec8c);
assert_eq!(djb2(b"abc"), 193_485_963);
assert_eq!(polynomial_rolling(b"abc"), 96_354);
assert_eq!(murmur3_32(b"abc"), 0xB3DD_93FA);
assert_eq!(siphash_2_4(b"", &key), 0x726f_db47_dd0e_0e31);

assert_eq!(hash_str_fnv1a_32("hello"), fnv1a_32(b"hello"));
assert_eq!(hash_str_siphash("hello", &key), siphash_2_4(b"hello", &key));

let hasher = SipHash24::new(key);
assert_eq!(hasher.output_bits(), 64);

let chi2 = distribution_test(
    fnv1a_32,
    vec![&b"alpha"[..], &b"beta"[..], &b"gamma"[..]],
    8,
);
assert!(chi2 >= 0.0);

let score = avalanche_score(fnv1a_32, 32, 8);
assert!(score >= 0.0 && score <= 1.0);
```

## Building and Testing

```bash
cargo test -p coding_adventures_hash_functions -- --nocapture
```

## Warrant status

[`code/specs/hash-functions.warrant`](../../../specs/hash-functions.warrant) covers
the eight byte-string hashes (`fnv1a_32`, `fnv1a_64`, `djb2`,
`polynomial_rolling`, `polynomial_rolling_with_params`, `murmur3_32`,
`murmur3_32_with_seed`, `siphash_2_4`): 67 known-answer examples plus bounded
exhaustive, property and fuzz suites, in `tests/generated/warrant_evidence.rs`
(generated, never edit it; `BUILD` fails if it drifts) and run against the real
exports. Eight sabotage lines in `BUILD` prove a deliberately broken result is
caught (`flip-result-bit` on each `u32`/`u64` result, `missing-error` on the
zero-modulus rejection). `tests/runtime_capability.rs` runs the covered hashes
inside a no-capability sandbox on Linux.

Not wired yet, and why: the capability analyzer and the `ca-assurance-report`
package report. `avalanche_score` draws OS randomness, which the pure-computation
manifest does not allow the analyzer to accept. Not covered, with reasons:
`hash_str_*` (text), `avalanche_score` and `distribution_test` (floats and a
generic closure), the `HashFunction` trait and its structs (stateful objects),
and the constants. See [DT17a](../../../specs/DT17a-hash-functions-warrant.md).

```bash
# Regenerate the harness after editing the .warrant document
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- hash-functions
```
