# Scytale Cipher (Rust)

Ancient Spartan transposition cipher implementation in Rust.

## Usage

```rust
use scytale_cipher::{encrypt, decrypt, brute_force};

let ct = encrypt("HELLO WORLD", 3).unwrap();
assert_eq!(ct, "HLWLEOODL R ");

let pt = decrypt(&ct, 3).unwrap();
assert_eq!(pt, "HELLO WORLD");

let results = brute_force(&ct);
// results contains {key: 2, text: "..."}, {key: 3, text: "HELLO WORLD"}, ...
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
