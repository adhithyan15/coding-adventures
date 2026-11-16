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
[`coding_adventures_hash_analysis`](../hash-analysis) (0.2.0): they draw OS randomness,
and this crate is pure computation.

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

## Building and Testing

```bash
cargo test -p coding_adventures_hash_functions -- --nocapture
```

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
deterministic, and this stands in for the unwired report `--check`). The usage
example above is run as a doctest (`cargo test --doc`). `tests/runtime_capability.rs` runs the covered hashes
inside a no-capability sandbox on Linux.

Note on the zero modulus: the real `polynomial_rolling_with_params` panics on
`modulus == 0` (`assert!(modulus > 0)`). The Warrant row `ensures no_panic` on that
parameter holds only of the generated harness adapter, which catches any panic and
reports it as `MODULUS_ZERO`; it is not a statement about the crate.

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
