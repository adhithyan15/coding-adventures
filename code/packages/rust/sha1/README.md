# sha1 (Rust)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in Rust.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using any external crates, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package.

## Rust Highlights

Rust is ideal for cryptographic implementations:
- `u32::rotate_left(n)` compiles to a single `rol` instruction on x86.
- `wrapping_add` makes the intent of mod-2^32 arithmetic explicit.
- `[u8; 20]` is a fixed-size stack-allocated digest — no heap allocation needed.
- Pattern matching on `0..=19 | 20..=39 | 40..=59 | _` selects the round function clearly.

## Usage

```rust
use sha1::{sum1, hex_string, Digest};

// One-shot
let digest: [u8; 20] = sum1(b"abc");
let hex: String = hex_string(b"abc");  // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Streaming
let mut h = Digest::new();
h.update(b"ab");
h.update(b"c");
assert_eq!(h.sum1(), sum1(b"abc"));
println!("{}", h.hex_digest());

// Clone for prefix hashing
let h2 = h.clone_digest();
```

## FIPS 180-4 Test Vectors

```rust
assert_eq!(hex_string(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
assert_eq!(hex_string(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
```

## Development

```bash
cargo test -- --nocapture
```

Tests: 27 unit tests + 4 doc tests, all passing.
