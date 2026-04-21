# coding_adventures_argon2d

A pure-Rust, from-scratch implementation of **Argon2d** (RFC 9106) —
data-dependent memory-hard password hashing. `#![forbid(unsafe_code)]`.

## What is Argon2d?

Argon2d uses **data-dependent** addressing throughout every segment: the
reference block index for each new block is derived from the first 64
bits of the previously computed block. This maximises GPU/ASIC
resistance at the cost of leaking a noisy channel through memory-access
timing. Use Argon2d only in contexts where side-channel attacks are
*not* in the threat model — e.g. proof-of-work. For password hashing,
prefer [`argon2id`](../argon2id/).

See the spec at [code/specs/KD03-argon2.md](../../../specs/KD03-argon2.md).

## Usage

```rust
use coding_adventures_argon2d::{argon2d, argon2d_hex, Options};

let tag = argon2d(b"password", b"somesalt", 3, 64, 1, 32, &Options::default()).unwrap();
let hex = argon2d_hex(b"password", b"somesalt", 3, 64, 1, 32, &Options::default()).unwrap();
```

### Keyed / authenticated data

```rust
let tag = argon2d(
    password, salt, 3, 64, 1, 32,
    &Options { key: Some(secret), associated_data: Some(b"challenge-id"), version: None },
).unwrap();
```

## API

| Function | Returns |
| -- | -- |
| `argon2d(password, salt, t, m, p, T, &opts)` | `Result<Vec<u8>, Argon2Error>` |
| `argon2d_hex(password, salt, t, m, p, T, &opts)` | `Result<String, Argon2Error>` |

Parameters follow RFC 9106 §3.1.

## Where this fits in the stack

- **Dependencies:** [`coding_adventures_blake2b`](../blake2b/) (H0 and H' extender).

## Security notes

- **Trust boundary on `memory_cost` and `tag_length`.** RFC 9106 permits
  both up to `2^32 - 1`, which translates to multi-TiB allocations. Clamp
  caller-controlled values at the application layer.
- **Verify in constant time.** Use `subtle::ConstantTimeEq` rather than
  `==` when comparing a stored tag against a freshly computed one.

## Running the tests

```bash
cargo test -p coding_adventures_argon2d -- --nocapture
```

Tests include the canonical RFC 9106 §5.1 gold-standard vector, plus
15 unit tests covering validation, determinism, binding to key/AD,
tag-length variants, and multi-lane / multi-pass parameters.

## Part of [coding-adventures](https://github.com/adhithyan15/coding-adventures)

One of 30 Argon2 packages across 10 languages × 3 variants (d/i/id).
