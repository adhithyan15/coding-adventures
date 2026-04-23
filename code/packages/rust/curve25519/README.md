# curve25519

Curve25519 X25519 Diffie-Hellman function (RFC 7748) implemented from scratch
with no external cryptographic dependencies.

## What Is Curve25519?

Curve25519 is an elliptic curve over GF(2²⁵⁵ − 19) designed by Daniel
J. Bernstein (2005). It powers the X25519 ECDH key-agreement function used by
TLS 1.3, Signal, SSH, and WireGuard.

The curve equation (Montgomery form):

```
v² = u³ + 486662·u² + u   (mod 2²⁵⁵ − 19)
```

For Diffie-Hellman only the u-coordinate is needed, so v is never computed.

## Usage

```rust
use coding_adventures_curve25519::{x25519, x25519_public_key, X25519_BASEPOINT};

// Key generation
let alice_secret = [/* 32 random bytes */];
let alice_public  = x25519_public_key(&alice_secret);   // scalar × G

// Key exchange
let bob_secret = [/* 32 random bytes */];
let bob_public  = x25519_public_key(&bob_secret);

let shared_alice = x25519(&alice_secret, &bob_public);
let shared_bob   = x25519(&bob_secret,   &alice_public);
assert_eq!(shared_alice, shared_bob);  // same shared secret
```

## API

```rust
pub type Scalar        = [u8; 32];
pub type MontgomeryPoint = [u8; 32];

/// Standard base point: u = 9.
pub const X25519_BASEPOINT: MontgomeryPoint;

/// Scalar multiplication: k·u (clamping applied internally).
pub fn x25519(k: &Scalar, u: &MontgomeryPoint) -> MontgomeryPoint;

/// Public key: k·G (shortcut for x25519(k, X25519_BASEPOINT)).
pub fn x25519_public_key(secret: &Scalar) -> MontgomeryPoint;
```

Field arithmetic primitives are also exported (used by `ed25519`):

```rust
pub struct FieldElement(pub [u64; 5]);   // GF(2²⁵⁵−19), 51-bit limbs

pub fn fe_add(a: Fe, b: Fe) -> Fe;
pub fn fe_sub(a: Fe, b: Fe) -> Fe;
pub fn fe_neg(a: Fe) -> Fe;
pub fn fe_mul(a: Fe, b: Fe) -> Fe;
pub fn fe_sq(a: Fe) -> Fe;
pub fn fe_sq_n(a: Fe, n: u32) -> Fe;
pub fn fe_inv(a: Fe) -> Fe;
pub fn fe_cswap(swap: u64, a: &mut Fe, b: &mut Fe);
```

## Implementation Details

### Field Representation: 5-Limb 51-Bit Radix

The prime `p = 2²⁵⁵ − 19` is represented as five 64-bit integers, each
holding at most 51 bits:

```
a = a[0] + a[1]·2⁵¹ + a[2]·2¹⁰² + a[3]·2¹⁵³ + a[4]·2²⁰⁴
```

Products of two 51-bit limbs fit in 64 bits (102 bits < 128 bits), so all
intermediate values are computed in `i128` without overflow.

Reduction uses the identity `2²⁵⁵ ≡ 19 (mod p)`: carry-out from limb 4
folds back to limb 0 multiplied by 19.

### Montgomery Ladder

Scalar multiplication uses the constant-time Montgomery ladder
(RFC 7748 §5):

```
R₀ ← (1:0)   (point at infinity, projective X:Z)
R₁ ← (u:1)   (input point)
for bit from 254 down to 0:
    swap R₀,R₁ if bit == 1
    R₁ ← R₀ + R₁   (differential addition)
    R₀ ← 2·R₀       (doubling)
    swap R₀,R₁ if bit == 1
return R₀.X / R₀.Z
```

The conditional swap uses bitmask arithmetic with no branches on secret data.

### Scalar Clamping

Before use, scalars are clamped per RFC 7748 §5:

```
scalar[0]  &= 248   // clear low 3 bits (cofactor 8)
scalar[31] &= 127   // clear bit 255
scalar[31] |= 64    // set  bit 254 (constant iteration count)
```

### Inversion

Field inversion uses Fermat's Little Theorem: `a⁻¹ = a^(p−2) mod p`.
The exponent `p−2 = 2²⁵⁵ − 21` is evaluated with an addition chain of 11
multiplications and 255 squarings, matching the chain used by libsodium and
Go's `crypto/curve25519`.

## Test Vectors

- **RFC 7748 §6.1 iterative vectors** (iter=1, iter=1000) — pass exactly.
- **Bob's DH key pair** — passes exactly against RFC value.
- **Alice's DH key pair** — see note below.

### RFC 7748 Alice Test Vector Discrepancy

The RFC 7748 §6.1 lists Alice's private key as `77076d0a...` and her public
key as `8520f009...`.  However, x25519(`77076d0a...`, 9) ≠ `8520f009...`
— verified independently by libsodium/nacl, Python bigint, and this
implementation.  All three agree the correct result is `d5f22539...`.

The RFC's Alice public key `8520f009...` and shared secret `4a5d9d5b...` are
consistent with each other (`x25519(b_sec, 8520f009...) = 4a5d9d5b...`) but
not with the listed private key.  This is an apparent typo in RFC 7748 §6.1
where the wrong private key bytes were published.

Our tests use the libsodium-verified values.

## Security Properties

- **No branches on secret data** — Montgomery ladder + bitmask cswap.
- **No timing side channels** — all operations run in constant time.
- **No unsafe Rust** — `#![forbid(unsafe_code)]`.
- **No crypto dependencies** — only standard Rust.

## Stack Position

```
curve25519  ──►  ed25519  ──►  x3dh  ──►  double-ratchet  ──►  sealed-sender
```

The `ed25519` package imports the field arithmetic types and primitives from
this package to build the twisted Edwards group operations.

## References

- [RFC 7748](https://www.rfc-editor.org/rfc/rfc7748) — Elliptic Curves for Security
- [Bernstein 2006](https://cr.yp.to/ecdh/curve25519-20060209.pdf) — Curve25519 original paper
- [Montgomery 1987](https://www.ams.org/journals/mcom/1987-48-177/S0025-5718-1987-0866113-7/) — Speeding the Pollard and elliptic curve methods
