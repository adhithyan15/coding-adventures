# X25519 — Elliptic Curve Diffie-Hellman (RFC 7748)

A zero-dependency Rust implementation of the X25519 key agreement protocol. All field arithmetic over GF(2^255 - 19) uses 51-bit limbs in `[u64; 5]` with `u128` intermediates for multiplication.

## Usage

```rust
use coding_adventures_x25519::{x25519, x25519_base, generate_keypair};

// Generate keypairs
let alice_private = [/* 32 random bytes */];
let alice_public = generate_keypair(&alice_private).unwrap();

let bob_private = [/* 32 random bytes */];
let bob_public = generate_keypair(&bob_private).unwrap();

// Derive shared secret
let shared_ab = x25519(&alice_private, &bob_public).unwrap();
let shared_ba = x25519(&bob_private, &alice_public).unwrap();
assert_eq!(shared_ab, shared_ba);
```

## Implementation Details

- **Radix-2^51 limbs**: Five `u64` limbs carry 51 bits each, totaling 255 bits
- **u128 intermediates**: Multiplication uses `u128` to avoid overflow
- **Constant-time cswap**: Bitwise masking for branch-free conditional swap
- **Fermat inversion**: `a^(p-2) mod p` via optimized addition chain (254 squarings + 11 multiplications)

## Testing

```bash
cargo test -v
```

All RFC 7748 test vectors pass, including the iterated 1000-round test.
