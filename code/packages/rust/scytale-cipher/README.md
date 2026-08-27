# Scytale Cipher (Rust)

Ancient Spartan transposition cipher implementation in Rust.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force` returns a checked result and rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

The native test suite includes a generated, dependency-free consumer for all 18 normative Scytale cases in `classical-ciphers-v1`; `generate_scytale_fixture_consumers.py --check` prevents fixture or language-roster drift.

## Usage

```rust
use scytale_cipher::{encrypt, decrypt, brute_force};

let ct = encrypt("HELLO WORLD", 3).unwrap();
assert_eq!(ct, "HLWLEOODL R ");

let pt = decrypt(&ct, 3).unwrap();
assert_eq!(pt, "HELLO WORLD");

let results = brute_force(&ct).unwrap();
// results contains {key: 2, text: "..."}, {key: 3, text: "HELLO WORLD"}, ...
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
