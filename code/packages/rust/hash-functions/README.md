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

let key = [0u8; 16];

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
