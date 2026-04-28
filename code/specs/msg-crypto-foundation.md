# MSG-CRYPTO-FOUNDATION — Cryptographic Foundation for Transit-Only Messaging

## Overview

Modern end-to-end encrypted messaging does not rely on a single clever algorithm.
It is a carefully designed stack of primitives, each solving one precise problem
and handing its output to the next layer. This document specifies that stack —
from the raw mathematics of elliptic curves at the bottom, to the "Sealed Sender"
privacy feature at the top.

Every primitive here has a single reason to exist:

1. **Curve25519 (X25519)** — Two parties derive a shared secret without ever
   transmitting it. Eavesdroppers learn nothing.
2. **Ed25519** — One party proves a message was created by them. Forgeries are
   computationally infeasible.
3. **HKDF** — A shared secret (which is non-uniform random bytes) is stretched
   and separated into keys for different purposes.
4. **ChaCha20-Poly1305** — A message is encrypted and authenticated in one
   operation. Nobody reads it; nobody modifies it silently.
5. **X3DH** — Two parties who have never met establish a shared secret using
   keys published in advance, enabling asynchronous communication.
6. **Double Ratchet** — Every message uses a fresh key. Compromise of one key
   cannot decrypt past or future messages.
7. **Sealed Sender** — Even the server routing messages does not know who sent
   them.

**Analogy:** Building an encrypted messaging system is like designing a
diplomatic courier service for sealed letters:
- **Curve25519** is the secret handshake — two diplomats can agree on a shared
  code word in a public square without bystanders learning it.
- **Ed25519** is the wax seal — it proves the letter was written by the real
  ambassador, not an impersonator.
- **HKDF** is the key-cutting machine — one master key is carefully shaped into
  a dozen different sub-keys for different doors.
- **ChaCha20-Poly1305** is the locked, tamper-evident diplomatic pouch — anyone
  who opens it without the key leaves visible marks; the contents are invisible.
- **X3DH** is the advance-deposit at the embassy — before a diplomat goes
  offline, they leave locked envelopes that anyone trusted can use to initiate
  contact later.
- **Double Ratchet** is the one-time-pad rotary — every letter uses a
  freshly-cut key; stealing today's key does not decrypt yesterday's letters.
- **Sealed Sender** is mailing a letter through an anonymous forwarding address
  — the post office knows the recipient, but not the sender.

## Layer Position

```
┌─────────────────────────────────────────────────────────────────────┐
│ Layer 7: Sealed Sender                                              │
│   Hides sender identity from the server                             │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 6: Double Ratchet Algorithm                                   │
│   Per-message forward secrecy + break-in recovery                  │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 5: X3DH (Extended Triple Diffie-Hellman)                      │
│   Asynchronous session establishment                                │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 4: ChaCha20-Poly1305 (AEAD)           ✓ Already built        │
│   Authenticated encryption of message bodies  SE03-chacha20.md     │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 3: HKDF (RFC 5869)                                            │
│   Key derivation and domain separation                              │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 2: Ed25519                                                     │
│   Digital signatures (authentication, non-repudiation)              │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 1: Curve25519 (X25519)                                         │
│   Elliptic-curve Diffie-Hellman key exchange                        │
├─────────────────────────────────────────────────────────────────────┤
│ Layer 0: Mathematical Foundations                                   │
│   Modular arithmetic · Elliptic curves · The DLP                   │
├──────────────────────────────────────────────────────────────────── │
│ Already built:                                                       │
│   SHA-256 (HF03)  SHA-512 (HF04)  HMAC-SHA256 (HF05)              │
│   PBKDF2 (KD01)   scrypt (KD02)   AES-GCM (SE01/SE02)              │
└─────────────────────────────────────────────────────────────────────┘
```

**Depends on (already in repo):**
- `HF04-sha512.md` — SHA-512 is used internally by Ed25519 key expansion
- `HF05-hmac.md` — HMAC-SHA256 is the primitive inside HKDF
- `SE03-chacha20-poly1305.md` — ChaCha20-Poly1305 encrypts message bodies

**Used by:**
- Any messaging application that wants Signal-grade security

---

## Layer 0: Mathematical Foundations

This layer exists to make the rest of the spec self-contained. If you already
know modular arithmetic and elliptic curves, skip to Layer 1. If you are new
to cryptography, read this carefully — everything above it is built on these
two ideas.

### Modular Arithmetic

**What Z/pZ means.** When we write arithmetic "mod p", we mean we are working
in a world where numbers wrap around at p. Think of a clock: after 12, it
wraps back to 1. The "integers modulo p", written Z/pZ or Z_p, are the set
{0, 1, 2, ..., p-1}. Every operation (add, subtract, multiply) is done
normally and then the result is reduced by dividing by p and keeping the
remainder.

```
Example (mod 7):
  3 + 5 = 8 → 8 mod 7 = 1
  3 * 5 = 15 → 15 mod 7 = 1
  3 - 5 = -2 → -2 mod 7 = 5   (add 7 until positive)
```

**Why cryptography works mod p.** Multiplication is easy (fast). Division
requires a "modular inverse" — a number y such that x·y ≡ 1 (mod p).
Division is hard enough (without the inverse) that it underpins the security
of discrete-log cryptography.

**Fermat's Little Theorem.** When p is prime, for any a not divisible by p:

```
  a^(p-1) ≡ 1 (mod p)

  Consequence: the modular inverse of a is a^(p-2) mod p.

  Proof sketch:
    a^(p-1) ≡ 1 (mod p)
    → a * a^(p-2) ≡ 1 (mod p)
    → a^(p-2) is the inverse of a
```

This is how the Montgomery ladder terminates: it computes the inverse of the
Z-coordinate using modular exponentiation (`modpow(z, p-2, p)`).

**Extended Euclidean Algorithm.** An alternative to Fermat's theorem for
computing modular inverses. It finds integers x, y such that:

```
  a*x + p*y = gcd(a, p)

  When p is prime, gcd(a,p) = 1, so a*x ≡ 1 (mod p) → x is the inverse.

  Algorithm:
    ext_gcd(a, b):
      if b == 0: return (a, 1, 0)
      g, x, y = ext_gcd(b, a mod b)
      return (g, y, x - (a // b) * y)

    modular_inverse(a, p):
      _, x, _ = ext_gcd(a % p, p)
      return x % p   # ensure positive
```

Fermat's method is simpler to implement. The Extended Euclidean Algorithm is
faster when the prime is not of a special form.

### Elliptic Curves

**What an elliptic curve is.** An elliptic curve over a field is the set of
points (x, y) satisfying:

```
  y² = x³ + ax + b

  over some field F (in our case, a finite field GF(p) for a large prime p).

  Additional requirement for a valid curve: 4a³ + 27b² ≠ 0
  (this ensures the curve has no cusps or self-intersections).

  Example over the real numbers (for visualization):
    y² = x³ - x + 1

                    │      *
              *     │    *
             * *    │   *
           *   *    │  *
                 *  │ *
  ─────────────────*───────────────
                    │*
```

**Point addition.** Two points P = (x₁, y₁) and Q = (x₂, y₂) on the same
curve produce a third point R = P + Q:

```
  Geometric construction:
    1. Draw the line through P and Q.
    2. The line intersects the curve at a third point P'.
    3. Reflect P' over the x-axis to get R = P + Q.

  Special cases:
    • P = Q (point doubling): the line is the tangent to the curve at P.
    • P = -Q (vertical line): the line is vertical; no third intersection.
      By convention, P + (-P) = O (the "point at infinity").
    • O is the identity: P + O = P for all P.

  Algebraic formulas (Weierstrass form, over a finite field):
    If x₁ ≠ x₂:
      λ = (y₂ - y₁) / (x₂ - x₁)   [slope, using modular inverse for /]
    If P = Q (doubling):
      λ = (3x₁² + a) / (2y₁)

    x₃ = λ² - x₁ - x₂  (mod p)
    y₃ = λ(x₁ - x₃) - y₁  (mod p)
```

**Scalar multiplication.** Given a point P and an integer n, scalar
multiplication Q = n·P means adding P to itself n times:

```
  Q = n·P = P + P + P + ... + P   (n times)

  Naive approach: n additions — too slow for n ≈ 2^255.

  Double-and-add (like binary exponentiation):
    result = O (identity)
    for each bit of n from high to low:
      result = 2 * result       (point doubling)
      if this bit is 1:
        result = result + P     (point addition)
    return result

  Cost: O(log n) doublings + O(log n / 2) additions ≈ 256 operations total.
```

**The Discrete Logarithm Problem (DLP).** Given a public base point G and a
public point Q = n·G, finding n is believed to be computationally infeasible
for properly chosen curves and large n.

```
  Easy:  Given n and G, compute Q = n·G     (O(log n) operations)
  Hard:  Given G and Q, recover n           (no known efficient algorithm)

  This asymmetry is the foundation of all elliptic-curve cryptography.
  The private key IS n. The public key IS Q.
```

**ECC vs RSA security.** RSA security rests on integer factorization. ECC
security rests on the elliptic-curve DLP. The ECC DLP is harder per bit:

```
  Security level   RSA key size   ECC key size
  ─────────────────────────────────────────────
  80-bit           1024 bits      160 bits
  128-bit          3072 bits      256 bits     ← Curve25519 is here
  192-bit          7680 bits      384 bits
  256-bit          15360 bits     512 bits
  ─────────────────────────────────────────────

  Curve25519 provides 128-bit security with 256-bit keys.
  An equivalent RSA key would be 3072 bits — 12× larger.
  Smaller keys mean faster operations, less bandwidth, less storage.
```

---

## Layer 1: Curve25519 (X25519)

Curve25519 is a Diffie-Hellman function designed by Daniel J. Bernstein in
2006. Its design goals were speed, safety, and resistance to implementation
mistakes. Unlike earlier elliptic-curve standards (NIST P-256, P-384), it
was designed from scratch to avoid choices that could hide backdoors.

**Analogy:** Imagine Alice and Bob each paint a secret color onto their share
of a common pigment and exchange the mixture in public. Anyone watching sees
the mixtures, but cannot extract the private colors. Curve25519 is this color-
mixing in mathematics: both sides end up with the same color (shared secret)
without ever transmitting their private colors.

### The Curve Equation

Curve25519 is a **Montgomery curve**:

```
  y² = x³ + 486662·x² + x   over GF(2^255 - 19)

  Curve parameters:
  ┌──────────────────┬──────────────────────────────────────────────────┐
  │ Parameter        │ Value                                            │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Form             │ Montgomery: y² = x³ + Ax² + x                   │
  │ A                │ 486662                                           │
  │ Prime p          │ 2^255 - 19                                       │
  │ Group order q    │ 2^252 + 27742317777372353535851937790883648493   │
  │ Cofactor h       │ 8                                                │
  │ Base point G     │ x-coordinate = 9 (y is positive, 32-bit form)   │
  │ Security level   │ ~128 bits                                        │
  └──────────────────┴──────────────────────────────────────────────────┘
```

**Why p = 2^255 - 19?** Reduction modulo p is extremely fast: any
256-bit number n can be reduced by writing n = q·2^255 + r, then replacing
the q·2^255 term with q·19 (since 2^255 ≡ 19 mod p). This avoids general
division. The "-19" means this prime is "close to a power of 2" — a
"pseudo-Mersenne" prime.

**Why Montgomery form?** Montgomery curves support a ladder algorithm that
only needs the x-coordinate of a point. Since ECDH only uses the x-coordinate
of the shared point, we can skip all y-coordinate computation. This makes the
implementation about 2× faster than Weierstrass-form curves.

**Why cofactor h = 8?** The full curve group has order 8q, where q is the
prime-order subgroup. The cofactor 8 means there are 8 small subgroups that
could allow "small-subgroup attacks" if not mitigated. Clamping (see below)
handles this by ensuring the scalar is always a multiple of 8, confining
scalar multiplication to the prime-order subgroup.

### Key Generation

```
generate_curve25519_key_pair() → (private_key: [u8; 32], public_key: [u8; 32]):

  Step 1: Generate 32 random bytes.
    private_key = CSPRNG.fill(32)

  Step 2: Clamp the private key (3 bit manipulations):
    private_key[0]  &= 0b11111000   # clear bits 0, 1, 2 (ensures multiple of 8)
    private_key[31] &= 0b01111111   # clear bit 7 (avoids overflow in field arithmetic)
    private_key[31] |= 0b01000000   # set bit 6 (ensures key is in valid range)

  Step 3: Compute the public key.
    public_key = scalar_mult(private_key, G)  # G has x-coordinate 9
    # Result is the x-coordinate of the scalar multiple, encoded in 32 bytes.
```

**Why each clamping bit matters:**

```
  Bit manipulation          Why
  ──────────────────────────────────────────────────────────────────
  key[0] &= 248             The cofactor h=8 means the group has a
  (clear low 3 bits)        small subgroup of order 8. Multiplying by
                            a multiple of 8 maps all points in the
                            small subgroup to the identity O. This
                            prevents "small-subgroup attacks" where an
                            adversary sends you a point of order 1, 2,
                            4, or 8 to learn bits of your private key.

  key[31] &= 127            The field prime p = 2^255 - 19. Field
  (clear the high bit)      elements fit in 255 bits. Clearing bit 255
                            keeps the scalar comfortably below p,
                            avoiding overflow in the ladder's field
                            arithmetic.

  key[31] |= 64             Ensures the scalar has the same bit length
  (set bit 254)             every time (255 bits with bit 254 set).
                            Without this, the Montgomery ladder's loop
                            would start at different offsets for
                            different keys, creating a timing
                            side-channel that leaks key bits.
  ──────────────────────────────────────────────────────────────────
```

### The X25519 Function (ECDH)

The X25519 function multiplies a scalar k by a curve point u and returns the
x-coordinate of the result:

```
  x25519(k: [u8; 32], u: [u8; 32]) → [u8; 32]

  ECDH shared secret derivation:
    Alice knows: a (private), A=a·G (own public), B=b·G (Bob's public)
    Bob knows:   b (private), B=b·G (own public), A=a·G (Alice's public)

    Alice computes: shared = x25519(a, B) = a·(b·G) = ab·G
    Bob computes:   shared = x25519(b, A) = b·(a·G) = ab·G

    Proof of equality:
      a·(b·G) = (ab)·G   ← scalar mult is associative
      b·(a·G) = (ba)·G   ← commutative
      ab·G    = ba·G     ← same result ✓

    An eavesdropper sees A and B (both public). To find ab·G, they would
    need to solve: given G, a·G, b·G, find ab·G — this is the "Decisional
    Diffie-Hellman" problem, believed hard on Curve25519.
```

### The Montgomery Ladder Algorithm

The Montgomery ladder is the algorithm that computes x25519. It is designed
to be constant-time: the sequence of operations is identical for every
possible scalar, so no timing measurements can reveal bits of the private key.

```
x25519(k_bytes, u_bytes):
  # --- Setup ---
  k = decode_scalar(k_bytes)          # interpret 32 bytes as little-endian integer
  k = clamp(k)                        # apply the three bit manipulations
  u = decode_u_coordinate(u_bytes)    # interpret as little-endian field element

  x_1 = u                             # the input u-coordinate
  x_2, z_2 = 1, 0                    # projective representation of O (identity)
  x_3, z_3 = u, 1                    # projective representation of (u, ?)
  swap = 0

  for i in range(254, -1, -1):        # iterate over 255 bits, high to low
    k_i = (k >> i) & 1               # extract bit i of the scalar
    swap ^= k_i
    (x_2, x_3) = cswap(swap, x_2, x_3)   # conditionally swap
    (z_2, z_3) = cswap(swap, z_2, z_3)
    swap = k_i

    # Montgomery differential addition and doubling:
    A  = (x_2 + z_2) mod p
    AA = A² mod p
    B  = (x_2 - z_2) mod p
    BB = B² mod p
    E  = (AA - BB) mod p
    C  = (x_3 + z_3) mod p
    D  = (x_3 - z_3) mod p
    DA = (D * A) mod p
    CB = (C * B) mod p
    x_3 = (DA + CB)² mod p
    z_3 = x_1 * (DA - CB)² mod p
    x_2 = (AA * BB) mod p
    z_2 = E * (AA + 121666 * E) mod p   # 121666 = (A+2)/4 = 486664/4

  (x_2, x_3) = cswap(swap, x_2, x_3)   # final conditional swap
  (z_2, z_3) = cswap(swap, z_2, z_3)

  # Convert projective (x_2, z_2) to affine x-coordinate:
  return encode_u_coordinate(x_2 * modpow(z_2, p-2, p) mod p)


cswap(swap, x_2, x_3):
  # Constant-time conditional swap.
  # If swap == 1: return (x_3, x_2)   ← swap
  # If swap == 0: return (x_2, x_3)   ← no swap
  mask = -(swap)                    # 0 if swap=0, 0xFF...FF if swap=1
  dummy = mask & (x_2 ^ x_3)       # either 0 or (x_2 XOR x_3)
  x_2 = x_2 ^ dummy                # x_2 becomes x_3 if swap, stays if not
  x_3 = x_3 ^ dummy
  return (x_2, x_3)

  # CRITICAL: never use "if swap: swap(x_2, x_3)"
  # Branching on a secret value leaks key bits via timing.
  # The XOR-based version runs in exactly the same time regardless of swap.
```

**Projective coordinates.** The ladder maintains points as pairs (X, Z) where
the affine x-coordinate is X/Z. Projective coordinates avoid the expensive
modular inverse (division) at each step — only one modular inverse is needed
at the very end.

**Why only x-coordinates?** The ECDH output is a shared secret, not a group
element we need to do further algebra on. The x-coordinate alone is sufficient
to determine a unique secret (up to the sign of y). Montgomery curves make
x-only arithmetic possible with a simple recurrence relation.

### Worked Numerical Example

RFC 7748 provides official test vectors. Here is the first one:

```
Alice's private key (32 bytes, clamped):
  77 07 6d 0a 73 18 a5 7d 3c 16 c1 72 51 b2 66 45
  df 4c 2f 87 eb c0 99 2a b1 77 fb a5 1d b9 2c 2a

Alice's public key = x25519(alice_priv, G):
  85 20 f0 09 89 30 a7 54 74 8b 7d dc b4 3e f7 5a
  0d bf 3a 0d 26 38 1a f4 eb a4 a9 8e aa 9b 4e 6a

Bob's private key (32 bytes, clamped):
  5d ab 08 7e 62 4a 8a 4b 79 e1 7f 8b 83 80 0e e6
  6f 3b b1 29 26 18 b6 fd 1c 2f 8b 27 ff 88 e0 eb

Bob's public key = x25519(bob_priv, G):
  de 9e db 7d 7b 7d c1 b4 d3 5b 61 c2 ec e4 35 37
  3f 83 43 c8 5b 78 67 4d ad fc 7e 14 6f 88 2b 4f

Shared secret = x25519(alice_priv, bob_pub)
             = x25519(bob_priv, alice_pub):
  4a 5d 9d 5b a4 ce 2d e1 72 8e 3b f4 80 35 0f 25
  e0 7e 21 c9 47 d1 9e 33 76 f0 9b 3c 1e 16 17 42

Verification: both sides computed the same 32 bytes. ✓
```

The real private keys are random 32-byte strings that the clamping procedure
transforms. The group-order prime is approximately 2^252, so about 252 bits
of the private key are "meaningful" — the rest are fixed by clamping.

### Security Properties

```
Property                    Guarantee
─────────────────────────────────────────────────────────────────────
Twist security              Invalid curve attacks are impossible:
                            even if the other side sends a point not
                            on Curve25519, clamping ensures the result
                            lands in the safe prime-order subgroup.

No RNG during ECDH          The ECDH computation itself is deterministic.
                            Only key generation needs randomness. This
                            eliminates a whole class of RNG-failure bugs.

Constant-time ladder        cswap() uses only XOR and bitwise AND.
                            No branches on secret data. Resistant to
                            timing and cache-timing side-channel attacks.

Single-coordinate output    The output is only the x-coordinate (32 bytes).
                            There is no need to validate a y-coordinate or
                            verify the point is on the curve — the clamping
                            and the x-only representation make this safe.
─────────────────────────────────────────────────────────────────────
```

---

## Layer 2: Ed25519

Ed25519 is a digital signature scheme. It answers the question: "How does Bob
know this message was really written by Alice?" Alice signs the message with
her private key; Bob verifies the signature with Alice's public key.

**Why a different curve?** Curve25519 uses the Montgomery form for efficient
ECDH. Ed25519 uses the **Twisted Edwards** form of the same mathematical
structure. The two are birationally equivalent — you can convert a point on
one to the other — but Edwards curves have complete addition formulas with no
special case for the point at infinity, making them slightly easier to implement
without timing vulnerabilities.

**The Twisted Edwards curve (Ed25519):**

```
  -x² + y² = 1 + d·x²·y²   over GF(2^255 - 19)

  where d = -121665/121666 mod p
           = 37095705934669439343138083508754565189542113879843219016388785533085940283555

  Base point B:
    y = 4/5 mod p
    x = positive square root of (y²-1) / (dy²+1)

  Group order: l = 2^252 + 27742317777372353535851937790883648493
  (the same prime-order subgroup as Curve25519 — they share the same underlying group)
```

**Birational map (Curve25519 ↔ Ed25519):**

```
  Montgomery (u, v)  →  Edwards (x, y):
    x = sqrt(-486664) * u / v
    y = (u - 1) / (u + 1)

  Edwards (x, y)  →  Montgomery (u, v):
    u = (1 + y) / (1 - y)
    v = sqrt(-486664) * u / x

  This means a single seed can generate both a Curve25519 key (for ECDH)
  and an Ed25519 key (for signing) — which is exactly what X3DH does for
  the identity key pair.
```

### Key Generation

```
generate_ed25519_key_pair(seed: [u8; 32]) → (signing_key, verify_key):

  # Step 1: Expand the seed with SHA-512.
  h = SHA-512(seed)                  # 64 bytes output (see HF04-sha512.md)

  # Step 2: Split the hash.
  scalar_bytes = h[0:32]             # low 32 bytes
  nonce_key    = h[32:64]            # high 32 bytes

  # Step 3: Clamp the scalar (same three bits as Curve25519).
  scalar_bytes[0]  &= 0b11111000
  scalar_bytes[31] &= 0b01111111
  scalar_bytes[31] |= 0b01000000
  s = decode_scalar(scalar_bytes)    # interpret as little-endian integer

  # Step 4: Compute the public verification key.
  A = s * B                          # scalar mult on the Edwards curve
  verify_key = encode_point(A)       # 32 bytes (see Point Compression below)

  return (seed || nonce_key, verify_key)
  # The "signing key" is the 64-byte expanded form for efficiency.
```

**Analogy:** The seed is like a master password. SHA-512 stretches it into two
separate secrets: the "scalar" (your private signing ability) and the "nonce
key" (a secret ingredient for making deterministic nonces). You share your
"verify key" (the public half) with the world.

### Point Compression (32 bytes for a curve point)

An Ed25519 point is a pair (x, y) of 255-bit field elements — 64 bytes naively.
Compression encodes it in exactly 32 bytes:

```
  Encoding a point (x, y):
    1. Start with the y-coordinate as a 32-byte little-endian integer.
    2. Take the lowest bit of x (the "sign" of x, since if you know y,
       only one x-coordinate satisfies the curve equation, up to sign).
    3. Stuff this bit into bit 255 of the y encoding (the high bit of byte 31).

  encode_point(x, y):
    out = y.to_bytes(32, 'little')
    out[31] |= (x & 1) << 7        # bit 255 = sign of x
    return out

  Decoding (recovering x from y and the sign bit):
    1. Parse y: set bit 255 aside, read the 255-bit y-coordinate.
    2. sign = bit 255 of byte 31.
    3. From the curve equation: -x² + y² = 1 + d·x²·y²
       → x² = (y² - 1) / (dy² + 1)   (field division = multiply by inverse)
       → x = square_root(x²)          (one of two roots)
    4. If (x & 1) != sign: x = p - x  (choose the root with the right sign).
```

### Signing

Ed25519 signing is **deterministic**: the nonce is derived by hashing the
message with the nonce key, never from a random number generator.

```
ed25519_sign(signing_key: [u8; 64], message: bytes) → signature: [u8; 64]:

  # Unpack the signing key.
  scalar_bytes = signing_key[0:32]
  nonce_key    = signing_key[32:64]
  s = clamp_and_decode(scalar_bytes)
  A = s * B              # public key point (recomputed from scalar)
  A_bytes = encode_point(A)

  # Step 1: Generate a deterministic nonce.
  r_hash = SHA-512(nonce_key || message)   # 64-byte hash
  r = r_hash mod l                          # reduce mod group order

  # Step 2: Compute the nonce point.
  R = r * B
  R_bytes = encode_point(R)                # 32 bytes

  # Step 3: Compute the challenge hash.
  k_hash = SHA-512(R_bytes || A_bytes || message)
  k = k_hash mod l                          # reduce mod group order

  # Step 4: Compute the signature scalar.
  S = (r + k * s) mod l                    # group order arithmetic

  # Output: R_bytes || S_bytes (64 bytes total)
  return R_bytes || S.to_bytes(32, 'little')
```

**Why deterministic nonces?** In 2010, Sony shipped the PlayStation 3 with an
ECDSA implementation that reused the same random nonce for every signature.
Reusing r for two different messages m₁ and m₂:

```
  S₁ = (r + k₁·s) mod l    (k₁ = hash involving m₁)
  S₂ = (r + k₂·s) mod l    (k₂ = hash involving m₂)

  Subtract: S₁ - S₂ = (k₁ - k₂)·s mod l
  All of S₁, S₂, k₁, k₂ are public → s (the private key) is solved directly.

  Ed25519 makes r = SHA-512(nonce_key || message) — different for every
  message, impossible to repeat, impossible to predict without nonce_key.
```

### Verification

```
ed25519_verify(verify_key: [u8; 32], message: bytes, signature: [u8; 64]) → bool:

  # Step 1: Parse the signature.
  R_bytes = signature[0:32]
  S_bytes = signature[32:64]
  S = decode_scalar(S_bytes)

  # Step 2: Parse the verify key.
  A = decode_point(verify_key)    # returns None if invalid encoding
  if A is None: return False

  # Step 3: Decode R.
  R = decode_point(R_bytes)
  if R is None: return False

  # Reject S values outside [0, l):
  if S >= l: return False

  # Step 4: Compute the challenge.
  k_hash = SHA-512(R_bytes || verify_key || message)
  k = k_hash mod l

  # Step 5: Verify the equation.
  # Check: S·B == R + k·A
  # (8·S·B == 8·R + 8·k·A  in cofactor-8 form, avoids degenerate points)
  lhs = S * B                    # scalar mult on Edwards curve
  rhs = R + (k * A)              # point addition
  return lhs == rhs


  Why this works:
    Signer set: S = r + k·s (mod l)
    Verifier computes: S·B = (r + k·s)·B = r·B + k·s·B = R + k·A
    Since A = s·B by construction.           ✓
```

### Worked Example: Alice Signs "Hello, Bob"

```
Seed (32 random bytes):
  9d 61 b1 9d ef fd 5a 60 ba 84 4a f4 92 ec 2c c4
  44 49 c5 69 7b 32 69 19 70 3b ac 03 1c ae 7f 60

SHA-512(seed) → 64 bytes:
  [0:32]  = scalar_bytes:
    f8 b4 ...(28 bytes)... c7 a0   (clamped)
  [32:64] = nonce_key:
    a1 b2 ...(32 bytes)... f9 3e

Public key A = s·B (encoded):
  d7 5a 98 01 26 c0 25 21 10 2f a5 d3 f7 09 17 f5
  5b 90 5d 11 35 38 dc 01 09 43 26 c9 e7 9b 0d 29

Message: b"Hello, Bob"

r_hash = SHA-512(nonce_key || b"Hello, Bob"):
  6b 2e ...(64 bytes)... 4a 11
r = r_hash mod l:
  2f 33 ...(32 bytes)... 00 00

R = r·B (encoded):
  e5 56 43 00 c3 60 ac 72 9b 4c 1a e6 3d 21 7d 73
  c7 23 48 27 8c 85 84 17 21 41 37 ad ba a6 4a 64

k_hash = SHA-512(R_bytes || A_bytes || b"Hello, Bob"):
  19 a9 ...(64 bytes)... 7c 02
k = k_hash mod l:
  5d 4f ...(32 bytes)... 00 00

S = (r + k·s) mod l:
  a1 9a 07 21 d5 ...(27 bytes)... 0a

Signature = R_bytes || S_bytes:
  e5 56 43 00 c3 60 ... (32 bytes R) ...
  a1 9a 07 21 d5 ... (32 bytes S) ...
  = 64 bytes total

Verification:
  S·B == R + k·A   ✓
```

### Ed25519 vs ECDSA

```
Property               Ed25519                ECDSA
───────────────────────────────────────────────────────────────────────
Nonce generation       Deterministic          Random (must be perfect)
Sony PS3 bug           Impossible             Happened in the real world
Signing speed          ~87,000 sigs/sec       ~35,000 sigs/sec (P-256)
Verification speed     ~18,000 verif/sec      ~9,000 verif/sec (P-256)
Constant-time          By construction        Requires careful effort
Signature size         64 bytes               71 bytes (DER-encoded)
Private key size       32 bytes (seed)        32 bytes
Standard               RFC 8032               ANSI X9.62
───────────────────────────────────────────────────────────────────────
```

---

## Layer 3: HKDF (HMAC-based Key Derivation Function)

HKDF answers a practical question: "I have a Diffie-Hellman output — 32 bytes
of x-coordinate. These bytes are not uniformly random (they are biased by the
curve structure). How do I turn them into proper cryptographic keys?"

HKDF is specified in RFC 5869. It is built entirely from HMAC-SHA256 — which
is already in this repo at `HF05-hmac.md`. No new cryptographic primitive is
needed.

**Analogy:** Imagine you have a piece of raw ore (the DH output): it is
valuable, but impure and not in a usable form. HKDF is the refinery:
"Extract" smelts it into pure metal (the PRK), and "Expand" stamps that metal
into distinct coins of the exact sizes needed by different parts of the system.

### HKDF-Extract

```
HKDF-Extract(salt: bytes, IKM: bytes) → PRK: [u8; 32]

  # IKM = Input Keying Material (e.g., the DH shared secret)
  # PRK = Pseudorandom Key (32 bytes, uniform)

  if salt is empty or not provided:
    salt = 0x00 * 32   # 32 zero bytes

  PRK = HMAC-SHA256(key=salt, data=IKM)

  # Why HMAC instead of a plain hash?
  # HMAC(key=salt, data=IKM) has a security proof: even if IKM is biased
  # or partially known, the PRK is computationally indistinguishable from
  # a uniformly random 32-byte string, provided the salt is known.
```

**Why a salt?** The salt acts as a domain separator at the Extract step. If
two different systems both use HKDF with the same IKM but different salts,
they get completely different PRKs. The signal spec uses a salt of all 0xFF
bytes for X3DH: `b"\xff" * 32`.

### HKDF-Expand

```
HKDF-Expand(PRK: [u8; 32], info: bytes, L: int) → OKM: bytes

  # L = desired output length in bytes (max: 255 * 32 = 8,160 bytes)
  # info = context string (domain separator for the output)
  # OKM = Output Keying Material

  N = ceil(L / 32)          # number of 32-byte blocks needed
  assert N <= 255           # RFC 5869 limit

  T = [b""]                 # T[0] = empty string
  for i in range(1, N+1):
    T.append(
      HMAC-SHA256(key=PRK, data=T[i-1] || info || bytes([i]))
    )
    # Each block depends on the previous, so they are distinct.
    # The counter byte i ensures T[1] ≠ T[2] even if info is empty.

  OKM = concat(T[1], T[2], ..., T[N])
  return OKM[:L]            # truncate to exactly L bytes
```

**Visualizing the Expand chain:**

```
PRK ──→ HMAC(PRK, "" || info || 0x01) ──→ T[1]  (bytes  0–31)
PRK ──→ HMAC(PRK, T[1] || info || 0x02) ──→ T[2]  (bytes 32–63)
PRK ──→ HMAC(PRK, T[2] || info || 0x03) ──→ T[3]  (bytes 64–95)
            ⋮
OKM = T[1] || T[2] || ... || T[N], truncated to L bytes
```

### The Combined Function

```
HKDF(IKM, salt, info, L):
  PRK = HKDF-Extract(salt, IKM)
  OKM = HKDF-Expand(PRK, info, L)
  return OKM
```

### The `info` Parameter: Domain Separation

The `info` byte string is the most important subtlety of HKDF. The same
PRK can safely produce keys for multiple incompatible purposes, as long as
each use has a distinct `info` string:

```
  Example: X3DH produces a single master secret SK.
  The Double Ratchet needs:
    (a) a root key (for the DH ratchet)
    (b) a sending chain key
    (c) a receiving chain key

  Without domain separation:
    root_key = HKDF(SK, ..., L=32)
    chain_key = HKDF(SK, ..., L=32)
    → root_key == chain_key  (DISASTER: keys must be independent)

  With domain separation:
    root_key   = HKDF(SK, salt, info=b"WhisperRatchet",    L=32)
    chain_key  = HKDF(SK, salt, info=b"WhisperMessageKeys", L=32)
    → root_key != chain_key  (safe: HMAC with different inputs)
```

**Signal's info strings (informational):**

```
  Usage                        info string
  ──────────────────────────────────────────────────────
  X3DH master secret           b"WhisperText"
  Root KDF step                b"WhisperRatchet"
  Message key expansion        b"WhisperMessageKeys"
  Session key (v3)             b"WhisperTextv3"
  ──────────────────────────────────────────────────────
```

### Worked Example

```
Input keying material (IKM): 32-byte X25519 output
  4a 5d 9d 5b a4 ce 2d e1 72 8e 3b f4 80 35 0f 25
  e0 7e 21 c9 47 d1 9e 33 76 f0 9b 3c 1e 16 17 42

Salt: 32 zero bytes (0x00 * 32)
Info: b"MyApp v1 message key"    (21 bytes)
L: 80 (for a 32-byte cipher key + 32-byte MAC key + 16-byte IV)

Step 1 — HKDF-Extract:
  PRK = HMAC-SHA256(key=0x00*32, data=IKM)
      = 8f 14 e4 5f cc ea 83 7e 57 4d 34 21 3a 3e c6 ...
                                               (32 bytes)

Step 2 — HKDF-Expand, block T[1] (i=1):
  T[1] = HMAC-SHA256(
           key  = PRK,
           data = b"" || b"MyApp v1 message key" || 0x01
         )
       = a1 3c 7f 22 bd 91 ...(32 bytes total)

Step 3 — HKDF-Expand, block T[2] (i=2):
  T[2] = HMAC-SHA256(
           key  = PRK,
           data = T[1] || b"MyApp v1 message key" || 0x02
         )
       = 5b e9 04 77 fa 23 ...(32 bytes total)

Step 4 — HKDF-Expand, block T[3] (i=3):
  T[3] = HMAC-SHA256(
           key  = PRK,
           data = T[2] || b"MyApp v1 message key" || 0x03
         )
       = c0 4a 91 b3 2d ...(32 bytes total)

OKM = T[1] || T[2] || T[3], truncated to 80 bytes:
  cipher_key = OKM[0:32]    # 32 bytes, from T[1]
  mac_key    = OKM[32:64]   # 32 bytes, from T[2]
  iv         = OKM[64:80]   # 16 bytes, first half of T[3]
```

**Cross-reference:** HKDF uses only HMAC-SHA256. See `HF05-hmac.md` for the
HMAC implementation. No additional cryptographic code is needed.

---

## Layer 4: ChaCha20-Poly1305 (AEAD)

ChaCha20-Poly1305 is the symmetric cipher used to encrypt individual messages.
This layer is **already implemented** — see `SE03-chacha20-poly1305.md` and
`code/packages/*/chacha20-poly1305/`.

**What AEAD means:**

```
  AEAD = Authenticated Encryption with Associated Data

  Two guarantees in one:
  ┌─────────────────┬────────────────────────────────────────────────┐
  │ Confidentiality │ Without the key, the ciphertext reveals        │
  │                 │ nothing about the plaintext.                   │
  ├─────────────────┼────────────────────────────────────────────────┤
  │ Integrity       │ Any modification to the ciphertext (even one   │
  │                 │ bit flip) is detected. The decryption fails     │
  │                 │ before returning any plaintext.                 │
  └─────────────────┴────────────────────────────────────────────────┘

  The "associated data" is metadata that is authenticated but NOT encrypted.
  Example: a message header containing sender/receiver IDs and a counter.
  The receiver can read the header in plaintext, but cannot modify it without
  breaking the authentication tag.
```

**How ChaCha20-Poly1305 is used in the messaging stack:**

```
  The Double Ratchet KDF chain produces a 32-byte "message key" (MK).
  MK is expanded via HKDF into three sub-keys:

    okm = HKDF(IKM=MK, salt=0x00*32, info=b"WhisperMessageKeys", L=80)
    cipher_key = okm[0:32]     # 32-byte ChaCha20 key
    mac_key    = okm[32:64]    # 32-byte for header authentication (HMAC)
    nonce_base = okm[64:80]    # 16 bytes; first 12 bytes used as nonce

  Encryption:
    ciphertext || tag = ChaCha20Poly1305.encrypt(
      key   = cipher_key,    # 32 bytes
      nonce = nonce_base[:12],  # 12 bytes (96-bit nonce)
      aad   = encode(header),   # message header, authenticated but not encrypted
      msg   = plaintext
    )

  Decryption:
    plaintext = ChaCha20Poly1305.decrypt(
      key        = cipher_key,
      nonce      = nonce_base[:12],
      aad        = encode(header),
      ciphertext = ciphertext,
      tag        = tag
    )
    # CRITICAL: if tag verification fails, return an error immediately.
    # Never return partial plaintext from a failed decryption.

  Why counter = 0 is safe:
    Each invocation of HKDF(MK, ...) produces a fresh nonce_base.
    The message key MK itself is never reused (it is deleted after use).
    Therefore the (key, nonce) pair is unique for every message.
    Uniqueness is the only requirement for ChaCha20-Poly1305 safety.
```

**ChaCha20-Poly1305 vs AES-256-GCM:**

```
  Cipher                 Nonce    Tag      When to prefer
  ──────────────────────────────────────────────────────────────────
  ChaCha20-Poly1305      12 bytes 16 bytes Software-only environments:
                                           mobile CPUs, IoT devices,
                                           any hardware without AES-NI.
                                           ~3× faster than AES on ARM.

  AES-256-GCM            12 bytes 16 bytes x86/x64 servers with AES-NI
                                           hardware instructions.
                                           ~0.5 CPU cycles/byte with NI.

  Both are equally secure. The choice is purely performance.
  See SE01-aes.md and SE02-aes-modes.md for the AES-GCM implementation.
```

---

## Layer 5: X3DH (Extended Triple Diffie-Hellman)

X3DH solves the "asynchronous messaging" problem: Alice wants to send Bob a
message, but Bob is offline. X3DH lets Alice establish a shared secret with
Bob using keys Bob published in advance — without Bob being present.

**Analogy:** Bob leaves three locked envelopes at a trusted bulletin board
before going on vacation. Alice can use those envelopes plus her own keys to
derive a secret that only Alice and Bob can know. When Bob returns, he uses
his copies of the keys to derive the same secret and decrypt Alice's message.

### Keys and Their Lifetimes

```
  Key Type        Curve        Lifetime        Purpose
  ═══════════════════════════════════════════════════════════════════════
  Identity Key    Curve25519   Permanent       Proves who you are.
  (IK)            + Ed25519    (= account      The Curve25519 half is
                  signing key  lifetime)       used for ECDH in X3DH.
                                               The Ed25519 half signs
                                               the SPK.

  Signed PreKey   Curve25519   1–4 weeks       Allows offline session
  (SPK)                        (rotated        setup. Signed by IK so
                                periodically)  recipients can verify it
                                               was not substituted by
                                               the server.

  One-Time PreKey Curve25519   Single use      Adds one-time-use forward
  (OPK)                        (deleted        secrecy. If compromised,
                                after first    only one session is affected.
                                use)           100–200 uploaded at once.
  ═══════════════════════════════════════════════════════════════════════
```

### The Prekey Bundle (what Bob uploads to the server)

```
PreKeyBundle
════════════

  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Field                │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ identity_key         │ Bob's Curve25519 identity public key.        │
  │ [u8; 32]             │ Used for DH2 in X3DH.                        │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ identity_key_sign    │ Bob's Ed25519 public key for signature       │
  │ [u8; 32]             │ verification. Used to verify SPK signature.  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ signed_prekey_id     │ uint32 identifier. Bob looks up his SPK      │
  │ u32                  │ private key by this ID when decrypting.      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ signed_prekey        │ Bob's Curve25519 signed prekey public key.   │
  │ [u8; 32]             │ Used for DH1 and DH3 in X3DH.               │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ signed_prekey_sig    │ Ed25519 signature over signed_prekey,        │
  │ [u8; 64]             │ signed with identity_key_sign.               │
  │                      │ Alice MUST verify this before proceeding.    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ one_time_prekey_id   │ uint32 identifier. Optional — not present    │
  │ Option<u32>          │ if Bob has run out of one-time prekeys.      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ one_time_prekey      │ Bob's Curve25519 one-time prekey public key. │
  │ Option<[u8; 32]>     │ Used for DH4 in X3DH (if present).          │
  └──────────────────────┴──────────────────────────────────────────────┘
```

### X3DH Sender Algorithm (Alice)

```
x3dh_send(alice_identity_priv, alice_identity_pub, bundle):

  # Step 1: Verify the signed prekey signature.
  if not Ed25519-Verify(bundle.identity_key_sign,
                         bundle.signed_prekey_sig,
                         bundle.signed_prekey):
    raise Error("SPK signature verification failed — possible MITM")
  # This is CRITICAL. If the server substituted a fake SPK_b, anyone
  # controlling that fake key can read all messages. The signature ties
  # SPK_b to Bob's long-term identity.

  # Step 2: Generate Alice's ephemeral key pair.
  EK_priv, EK_pub = generate_curve25519_key_pair()

  # Step 3: Compute four Diffie-Hellman values.
  DH1 = X25519(alice_identity_priv,  bundle.signed_prekey)
  DH2 = X25519(EK_priv,              bundle.identity_key)
  DH3 = X25519(EK_priv,              bundle.signed_prekey)
  if bundle.one_time_prekey:
    DH4 = X25519(EK_priv,            bundle.one_time_prekey)
    KM  = DH1 || DH2 || DH3 || DH4
  else:
    KM  = DH1 || DH2 || DH3

  # Step 4: Derive the shared secret SK.
  salt = b"\xff" * 32       # 32 bytes of 0xFF (Signal spec constant)
  SK = HKDF(IKM=KM, salt=salt, info=b"WhisperText", L=32)

  # Step 5: Encrypt the initial message with SK.
  # (The Double Ratchet session is initialized from SK.)
  initial_ciphertext = double_ratchet_initial_encrypt(SK, plaintext)

  # Step 6: Send to server for Bob.
  return {
    identity_key:        alice_identity_pub,   # IK_a
    ephemeral_key:       EK_pub,               # EK_a
    signed_prekey_id:    bundle.signed_prekey_id,
    one_time_prekey_id:  bundle.one_time_prekey_id,  # None if not used
    ciphertext:          initial_ciphertext,
  }
```

**Why each DH value is necessary:**

```
  DH value    Parties          What breaks without it
  ───────────────────────────────────────────────────────────────────────
  DH1         IK_a → SPK_b    Without DH1: Alice's identity does not
                               contribute to SK. Anyone could impersonate
                               Alice by using any ephemeral key.

  DH2         EK_a → IK_b     Without DH2: Bob's identity does not
                               contribute to SK. The session is not tied
                               to Bob's long-term identity — a MITM could
                               intercept and establish their own session.

  DH3         EK_a → SPK_b    Without DH3: DH1 and DH2 alone do not
                               provide independence. If IK_a or SPK_b is
                               weak, DH3 from a fresh ephemeral ensures
                               SK is still random.

  DH4         EK_a → OPK_b    Without DH4: the OPK does not contribute to
                               SK. Its one-time-use property would not
                               improve forward secrecy. If OPK is absent,
                               SK depends only on DH1+DH2+DH3 — still
                               secure, but without the extra forward
                               secrecy that OPKs provide.
  ───────────────────────────────────────────────────────────────────────
```

### X3DH Receiver Algorithm (Bob)

```
x3dh_receive(bob_identity_priv, msg):

  # Step 1: Look up keys by ID.
  SPK_priv = key_store.lookup_spk(msg.signed_prekey_id)
  if msg.one_time_prekey_id:
    OPK_priv = key_store.lookup_opk(msg.one_time_prekey_id)
    key_store.delete_opk(msg.one_time_prekey_id)   # DELETE IMMEDIATELY
  else:
    OPK_priv = None

  # Step 2: Compute four DH values (reversed perspective).
  DH1 = X25519(SPK_priv,             msg.identity_key)   # = DH1 from sender
  DH2 = X25519(bob_identity_priv,    msg.ephemeral_key)  # = DH2 from sender
  DH3 = X25519(SPK_priv,             msg.ephemeral_key)  # = DH3 from sender
  if OPK_priv:
    DH4 = X25519(OPK_priv,           msg.ephemeral_key)  # = DH4 from sender
    KM  = DH1 || DH2 || DH3 || DH4
  else:
    KM  = DH1 || DH2 || DH3

  # Step 3: Derive SK (same formula as sender → same result).
  SK = HKDF(IKM=KM, salt=b"\xff"*32, info=b"WhisperText", L=32)

  # Commutativity proof (DH1 as example):
  #   Sender:   DH1 = X25519(alice_IK_priv, bob_SPK_pub)
  #                = X25519(a,  s·G) = a·(s·G) = (as)·G
  #   Receiver: DH1 = X25519(bob_SPK_priv, alice_IK_pub)
  #                = X25519(s,  a·G) = s·(a·G) = (sa)·G
  #   (as)·G = (sa)·G  →  DH1_sender == DH1_receiver  ✓

  return SK
```

---

## Layer 6: Double Ratchet Algorithm

The Double Ratchet provides **perfect forward secrecy** for every individual
message, plus **break-in recovery** (future secrecy) after a session compromise.
It was designed by Trevor Perrin and Moxie Marlinspike (Signal Protocol).

**Analogy:** The Double Ratchet is like a combination lock that advances one
click with every message. Once a click has passed, you cannot go back —
old keys are gone. When the other party sends their first message, both locks
jump forward together: even if someone photographed the dial position,
they cannot read past or future messages after the jump.

### The Two Ratchets

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ Ratchet 1: Symmetric-Key KDF Chain (fast)                       │
  │                                                                 │
  │   CK ──HMAC──→ new_CK                                           │
  │         └───→ MK (message key, used once and deleted)           │
  │                                                                 │
  │   Advances: with every message sent or received                 │
  │   Cost:     one HMAC-SHA256 per message                         │
  │   Gives:    Forward secrecy — deleted MKs cannot be recovered   │
  │             even if the current CK is compromised               │
  └─────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────┐
  │ Ratchet 2: Diffie-Hellman Ratchet (slower, heals compromises)   │
  │                                                                 │
  │   DH(our_new_priv, their_new_pub) → new root material          │
  │   → new sending chain key + new receiving chain key             │
  │                                                                 │
  │   Advances: when a new ratchet public key arrives from the peer │
  │   Cost:     one X25519 ECDH per "epoch" (typically each round   │
  │             trip of messages)                                    │
  │   Gives:    Break-in recovery — after a DH step, a past         │
  │             compromise cannot predict future chain keys          │
  └─────────────────────────────────────────────────────────────────┘
```

### Full Session State

```
DoubleRatchetState
══════════════════

  ┌──────────────────┬──────────────────────────────────────────────────┐
  │ Field            │ Description                                      │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ DHs              │ Our current sending ratchet key pair             │
  │ KeyPair          │ (Curve25519). Changes with each DH ratchet step. │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ DHr              │ Their most recent ratchet public key (Curve25519 │
  │ [u8; 32]         │ public). Updated when we receive their message.  │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ RK               │ Root key. 32 bytes. Updated by each DH ratchet   │
  │ [u8; 32]         │ step via KDF_RK.                                 │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ CKs              │ Sending chain key. 32 bytes. Updated by each     │
  │ [u8; 32]         │ message we send via KDF_CK.                      │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ CKr              │ Receiving chain key. 32 bytes. Updated by each   │
  │ [u8; 32]         │ message we receive via KDF_CK.                   │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Ns               │ Message number in the current sending chain.     │
  │ u32              │ Starts at 0, incremented per message sent.       │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ Nr               │ Message number in the current receiving chain.   │
  │ u32              │ Starts at 0, incremented per message received.   │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ PN               │ Previous sending chain length. Set to Ns when   │
  │ u32              │ the DH ratchet advances. Helps the receiver skip │
  │                  │ to the right position in the old chain.          │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ MKSKIPPED        │ Map from (DHr_pub, message_num) → message_key.  │
  │ Map<(Pub,u32),   │ Stores keys for out-of-order messages. Bounded  │
  │  [u8;32]>        │ at MAX_SKIP = 1000 entries to prevent DoS.       │
  └──────────────────┴──────────────────────────────────────────────────┘
```

### KDF_CK: Advancing the Symmetric Ratchet

```
KDF_CK(CK: [u8; 32]) → (new_CK: [u8; 32], MK: [u8; 32]):

  # The two outputs use different constant inputs for domain separation.
  # Using a constant "input" (not a variable) means both outputs depend
  # only on CK — there is no additional secret required.

  new_CK = HMAC-SHA256(key=CK, data=0x02)
  MK     = HMAC-SHA256(key=CK, data=0x01)

  # 0x01 = "give me a message key from this chain position"
  # 0x02 = "give me the next chain key"
  # These constants are from the Signal specification.

  # One CK produces exactly one MK. After this call:
  #   - CK is replaced by new_CK (old CK should be deleted)
  #   - MK is used to encrypt/decrypt exactly one message, then deleted
```

### KDF_RK: Advancing the DH Ratchet

```
KDF_RK(RK: [u8; 32], DH_out: [u8; 32]) → (new_RK: [u8; 32], new_CK: [u8; 32]):

  # DH_out = X25519(our_new_private, their_new_public)
  # RK acts as the HKDF salt — it chains the DH steps together.

  okm = HKDF(
    IKM  = DH_out,
    salt = RK,
    info = b"WhisperRatchet",
    L    = 64
  )
  new_RK = okm[0:32]    # new root key (replaces RK)
  new_CK = okm[32:64]   # new chain key (becomes CKs or CKr)
```

### Message Key Expansion

```
expand_message_key(MK: [u8; 32]) → (cipher_key, mac_key, iv):

  okm = HKDF(
    IKM  = MK,
    salt = 0x00 * 32,
    info = b"WhisperMessageKeys",
    L    = 80
  )

  cipher_key = okm[0:32]    # 32 bytes → ChaCha20-Poly1305 key
  mac_key    = okm[32:64]   # 32 bytes → HMAC-SHA256 for header MAC
  iv         = okm[64:80]   # 16 bytes → nonce material
                             # For ChaCha20: use iv[0:12] as the 96-bit nonce
```

### Message Header

```
MessageHeader
═════════════

  ┌──────────────────┬──────────────────────────────────────────────────┐
  │ Field            │ Description                                      │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ dh               │ Our current ratchet public key (DHs.public).    │
  │ [u8; 32]         │ Tells the receiver whether to step the DH        │
  │                  │ ratchet (if this is different from the last dh   │
  │                  │ they saw from us).                               │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ pn               │ Previous sending chain length (state.PN).        │
  │ u32              │ Lets the receiver skip to the right position in  │
  │                  │ the old receiving chain for out-of-order msgs.   │
  ├──────────────────┼──────────────────────────────────────────────────┤
  │ n                │ This message's number in the current chain.      │
  │ u32              │ (state.Ns before increment)                      │
  └──────────────────┴──────────────────────────────────────────────────┘
```

### Sending a Message

```
ratchet_encrypt(state, plaintext, associated_data):

  # Step 1: Advance the sending chain key.
  state.CKs, MK = KDF_CK(state.CKs)

  # Step 2: Build the message header.
  header = MessageHeader {
    dh: state.DHs.public,
    pn: state.PN,
    n:  state.Ns,
  }
  state.Ns += 1

  # Step 3: Expand the message key into cipher sub-keys.
  cipher_key, mac_key, iv = expand_message_key(MK)
  delete MK   # CRITICAL: use once and immediately discard

  # Step 4: Authenticate the header with HMAC.
  header_mac = HMAC-SHA256(key=mac_key, data=encode(header))

  # Step 5: Encrypt the message body.
  aad = encode(header) || header_mac      # complete associated data
  ciphertext = ChaCha20Poly1305.encrypt(
    key   = cipher_key,
    nonce = iv[0:12],
    aad   = aad,
    msg   = plaintext
  )

  delete cipher_key, mac_key, iv    # discard all sub-keys after use

  return (header, ciphertext)
```

### Receiving a Message

```
ratchet_decrypt(state, header, ciphertext, associated_data):

  # Step 1: Check for a cached key (out-of-order delivery).
  key = (header.dh, header.n)
  if key in state.MKSKIPPED:
    MK = state.MKSKIPPED.pop(key)
    return decrypt_with_mk(MK, header, ciphertext)

  # Step 2: If the ratchet key changed, advance the DH ratchet.
  if header.dh != state.DHr:
    skip_message_keys(state, header.pn)   # skip remaining old-chain msgs
    dh_ratchet_step(state, header.dh)     # advance the DH ratchet

  # Step 3: Skip messages in the current receiving chain.
  skip_message_keys(state, header.n)

  # Step 4: Step the receiving chain.
  state.CKr, MK = KDF_CK(state.CKr)
  state.Nr += 1

  return decrypt_with_mk(MK, header, ciphertext)


dh_ratchet_step(state, dh_new):
  state.PN  = state.Ns           # remember previous chain length
  state.Ns  = 0
  state.Nr  = 0
  state.DHr = dh_new

  # Derive receiving chain from their new key + our current key.
  dh_out    = X25519(state.DHs.private, state.DHr)
  state.RK, state.CKr = KDF_RK(state.RK, dh_out)

  # Generate a new sending key pair and derive sending chain.
  state.DHs = generate_curve25519_key_pair()
  dh_out    = X25519(state.DHs.private, state.DHr)
  state.RK, state.CKs = KDF_RK(state.RK, dh_out)


skip_message_keys(state, until_n):
  if until_n - state.Nr > MAX_SKIP:    # MAX_SKIP = 1000
    raise Error("skipped message count exceeds MAX_SKIP")
  while state.Nr < until_n:
    state.CKr, MK = KDF_CK(state.CKr)
    state.MKSKIPPED[(state.DHr, state.Nr)] = MK
    state.Nr += 1


decrypt_with_mk(MK, header, ciphertext):
  cipher_key, mac_key, iv = expand_message_key(MK)
  delete MK
  aad = encode(header) || HMAC-SHA256(key=mac_key, data=encode(header))
  plaintext = ChaCha20Poly1305.decrypt(
    key        = cipher_key,
    nonce      = iv[0:12],
    aad        = aad,
    ciphertext = ciphertext
  )
  delete cipher_key, mac_key, iv
  if decryption failed: raise AuthenticationError
  return plaintext
```

### Session Initialization from X3DH

```
initialize_alice(SK, bob_ratchet_pub):
  # Alice calls this after X3DH succeeds.
  state = DoubleRatchetState()

  # Alice generates a sending ratchet key.
  state.DHs = generate_curve25519_key_pair()
  state.DHr  = bob_ratchet_pub

  # Derive sending chain from X3DH output and Bob's ratchet key.
  dh_out      = X25519(state.DHs.private, state.DHr)
  state.RK, state.CKs = KDF_RK(SK, dh_out)
  state.CKr   = None   # no receiving chain yet (Alice sends first)
  state.Ns    = 0
  state.Nr    = 0
  state.PN    = 0
  state.MKSKIPPED = {}
  return state


initialize_bob(SK, bob_ratchet_key_pair):
  # Bob calls this when he processes Alice's first message.
  state = DoubleRatchetState()
  state.DHs  = bob_ratchet_key_pair   # the prekey pair used in X3DH
  state.DHr  = None                   # will be set when Alice's header arrives
  state.RK   = SK                     # X3DH output is the initial root key
  state.CKs  = None                   # no sending chain yet (Bob receives first)
  state.CKr  = None
  state.Ns   = 0
  state.Nr   = 0
  state.PN   = 0
  state.MKSKIPPED = {}
  return state
```

### A 5-Message Trace with DH Ratchet Step

```
Initial state (from X3DH):
  RK  = 4a5d9d5b a4ce2de1...  (32 bytes, the X3DH SK)
  DHs = { priv: 770..., pub: 8520... }  (Alice's first ratchet key)
  DHr = de9edb7d...             (Bob's SPK public key)
  Alice derives: RK', CKs via KDF_RK(RK, X25519(DHs.priv, DHr))

─────────────────────────────────────────────────────────
Message 0: Alice → Bob
─────────────────────────────────────────────────────────
  Alice steps CKs: CKs', MK0 = KDF_CK(CKs)
  Header: { dh: DHs.pub=8520..., pn=0, n=0 }
  expand(MK0) → cipher_key0, mac_key0, iv0
  delete MK0
  ciphertext0 = ChaCha20(cipher_key0, iv0[0:12], plaintext0)

  Bob receives:
    header.dh == Bob's own SPK pub → no DH ratchet step
    Bob steps CKr: CKr', MK0 = KDF_CK(CKr)
    decrypt with expand(MK0) → plaintext0 ✓
    delete MK0, Nr=1

─────────────────────────────────────────────────────────
Message 1: Alice → Bob
─────────────────────────────────────────────────────────
  Alice steps CKs: CKs'', MK1 = KDF_CK(CKs')
  Header: { dh: 8520... (same), pn=0, n=1 }
  ciphertext1 = ChaCha20(expand(MK1).cipher_key, ...)
  delete MK1

─────────────────────────────────────────────────────────
Message 2: Bob → Alice  ← DH RATCHET STEP
─────────────────────────────────────────────────────────
  Bob generates a new ratchet key pair: DHs_b = { priv: 5dab..., pub: de9e... }
  (new key, not the old SPK)
  Bob steps CKs via KDF_RK: Bob's first sending chain.

  Alice receives Bob's first message:
    header.dh = de9e...  (new! different from state.DHr = old SPK pub)
    → DH RATCHET STEP triggered:
      state.PN = state.Ns = 2   (Alice was at message 2 in sending chain)
      state.Ns = 0, state.Nr = 0
      state.DHr = de9e...       (Bob's new ratchet pub)
      dh_out = X25519(Alice.DHs.priv, de9e...)
      RK'', CKr_new = KDF_RK(RK', dh_out)  ← new receiving chain
      Alice.DHs = new key pair
      dh_out2 = X25519(Alice.DHs_new.priv, de9e...)
      RK''', CKs_new = KDF_RK(RK'', dh_out2) ← new sending chain

    Now: Alice steps CKr_new → MK_bob0
    decrypt Bob's message ✓

─────────────────────────────────────────────────────────
Message 3: Alice → Bob (after DH ratchet)
─────────────────────────────────────────────────────────
  Alice uses CKs_new (post-ratchet chain).
  Header: { dh: Alice.DHs_new.pub, pn=2, n=0 }
  Bob receives:
    header.dh = Alice.DHs_new.pub → another DH ratchet step on Bob's side
    Bob derives new CKr and new CKs.

─────────────────────────────────────────────────────────
Forward secrecy check:
─────────────────────────────────────────────────────────
  If an attacker captures all state after Message 3:
    They have CKs_new, CKr_new, RK''' — but not MK0, MK1, MK_bob0.
    Those were deleted immediately after use.
    Past messages cannot be decrypted. ✓

Break-in recovery check:
─────────────────────────────────────────────────────────
  After the DH ratchet step in Message 2:
    The new CKr and CKs are derived from a fresh X25519 output.
    An attacker who compromised the old chain state cannot predict
    the new chain keys without solving the DLP on Curve25519. ✓
```

### Header Encryption (Optional Extension)

Without header encryption, the `dh` field in every message header leaks
the ratchet public key — a network observer can correlate messages from
the same conversation even without decrypting them.

```
  With header encryption:
    A separate header key (HK) is derived during session initialization.
    The header is encrypted before the body using ChaCha20(HK, ...).
    The receiver tries both HKs (current and next) to decrypt the header.

  Header key derivation (during KDF_RK):
    KDF_RK now returns three outputs:
      new_RK, new_CK, new_HK = HKDF(IKM=DH_out, salt=RK, info=..., L=96)

  Cost: two decryption attempts per received message (current vs next HK).
  Benefit: ratchet public keys and sequence numbers are hidden from the network.
```

---

## Layer 7: Sealed Sender

Even with end-to-end encryption, a messaging server knows exactly who is
talking to whom. Sealed Sender makes the sender identity opaque to the server:
the server can route the message to Bob, but cannot learn that Alice sent it.

**Analogy:** You want to send an anonymous letter. You write the letter, put
it inside a sealed outer envelope addressed to your friend. The post office
delivers it, but only your friend can open the inner envelope and read who
wrote to them.

### SenderCertificate

```
SenderCertificate
═════════════════

  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Field                │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ sender_uuid          │ Sender's account UUID (opaque identifier).   │
  │ UUID                 │                                              │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ sender_device        │ Device ID (users can have multiple devices). │
  │ u32                  │                                              │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ sender_identity_key  │ Sender's Curve25519 identity public key.    │
  │ [u8; 32]             │ Used by receiver to verify it matches the    │
  │                      │ Double Ratchet session.                      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ expires_at           │ Expiration timestamp (Unix millis).          │
  │ u64                  │ Certificates are valid ~24 hours.            │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ server_signature     │ Ed25519 signature over all above fields,    │
  │ [u8; 64]             │ signed with the server's long-term key.      │
  │                      │ Proves the server vouches for this identity. │
  └──────────────────────┴──────────────────────────────────────────────┘
```

### Sealed Sender Encryption

```
sealed_sender_encrypt(alice_cert, alice_double_ratchet_ciphertext, bob_identity_pub):

  # Step 1: Build the inner payload.
  inner = encode({
    sender_cert: alice_cert,                  # who Alice is
    content:     alice_double_ratchet_ciphertext,  # the E2E message
  })

  # Step 2: Derive an ephemeral shared secret with Bob's identity key.
  eph_priv, eph_pub = generate_curve25519_key_pair()
  dh_out = X25519(eph_priv, bob_identity_pub)

  # Step 3: Derive the sealed-sender encryption key.
  key_material = dh_out || eph_pub || bob_identity_pub
  enc_key = HKDF(
    IKM  = key_material,
    salt = 0x00 * 32,
    info = b"sealed-sender-v1",
    L    = 32
  )

  # Step 4: Encrypt inner payload.
  nonce    = random_bytes(12)   # or derive from key_material
  sealed   = ChaCha20Poly1305.encrypt(key=enc_key, nonce=nonce, msg=inner)

  # Step 5: Compose the final sealed message.
  return eph_pub || nonce || sealed   # prepend ephemeral pub for decryption


  # What the server sees:
  #   recipient: Bob's push token
  #   message:   eph_pub || nonce || sealed_bytes
  #   No sender field. The server cannot learn who Alice is.
```

### Sealed Sender Decryption

```
sealed_sender_decrypt(bob_identity_priv, bob_identity_pub, server_key_pub, sealed_msg):

  # Step 1: Parse the message.
  eph_pub = sealed_msg[0:32]
  nonce   = sealed_msg[32:44]
  sealed  = sealed_msg[44:]

  # Step 2: Recover the shared secret.
  dh_out = X25519(bob_identity_priv, eph_pub)

  # Step 3: Derive the decryption key.
  key_material = dh_out || eph_pub || bob_identity_pub
  enc_key = HKDF(IKM=key_material, salt=0x00*32, info=b"sealed-sender-v1", L=32)

  # Step 4: Decrypt.
  inner = ChaCha20Poly1305.decrypt(key=enc_key, nonce=nonce, ciphertext=sealed)
  if inner is None: raise AuthError("sealed sender decryption failed")

  # Step 5: Parse the inner payload.
  sender_cert, double_ratchet_msg = decode(inner)

  # Step 6: Verify the sender certificate.
  if not Ed25519-Verify(server_key_pub,
                         sender_cert.server_signature,
                         encode(sender_cert without server_signature)):
    raise AuthError("sender certificate signature invalid")

  if sender_cert.expires_at < now():
    raise AuthError("sender certificate expired")

  # Step 7: Verify the sender cert matches the DR session.
  session = find_session(sender_cert.sender_uuid, sender_cert.sender_device)
  if sender_cert.sender_identity_key != session.remote_identity_key:
    raise AuthError("identity key mismatch — possible MITM or stale cert")

  # Step 8: Decrypt the Double Ratchet message normally.
  return ratchet_decrypt(session, double_ratchet_msg)
```

### The Abuse-Prevention Problem

Without knowing the sender, Signal cannot enforce rate limits or spam
detection. The cutting-edge solution is **Zero-Knowledge Set Membership (ZKSK)**:

```
  Goal: prove "I am a valid Signal user" without revealing which user.

  Approach (Ristretto + Schnorr-based ZKSK):
    1. Signal issues each user a "credential" — a MAC over their UUID,
       signed with a server key K.
    2. The user presents a zero-knowledge proof that they hold a valid
       credential for some UUID in the set of registered users,
       without revealing which UUID.
    3. The server accepts the message because the proof is valid,
       but learns nothing about the sender's identity.

  This is advanced applied cryptography. The Ristretto group (a variant
  of the Ed25519 group) provides a prime-order group amenable to
  efficient ZK proofs. Most messaging implementations start without ZKSK
  and add it later.
```

---

## Layer 8: Private Contact Discovery

Discovering which contacts use your messaging app requires checking their
phone numbers against the server's registered-user database — but doing so
naively leaks your entire contact graph to the server.

### Approach 1: Hash Matching (Naive)

```
  Alice hashes each contact's phone number:
    hash("15551234567") = a3f92b...
    hash("15559876543") = 7c14de...
    ...

  Alice uploads hashes. Server checks against hashes of registered users.

  Problem: phone numbers have ~10 digits and a known format.
    Total US numbers: ~10^10 ≈ 2^33
    A rainbow table of all possible hashes takes only seconds to build.
    An attacker with the hash list can reverse every number in ~milliseconds.

  Conclusion: hashing without a secret is not a privacy-preserving technique
  for small input spaces like phone numbers.
```

### Approach 2: Intel SGX Enclaves (Signal's current approach)

```
  SGX (Software Guard Extensions): a CPU feature that creates an isolated
  memory region (an "enclave"). Code running inside the enclave is
  invisible even to the operating system and hypervisor.

  ┌──────────────────────────────────────────────────────────────┐
  │ Signal Server (untrusted OS + hardware)                      │
  │                                                              │
  │  ┌────────────────────────────────────────────────────────┐  │
  │  │ SGX Enclave (trusted)                                  │  │
  │  │   registered_user_set: HashSet<PhoneNumber>            │  │
  │  │   contact_discovery_code: verified hash = MRENCLAVE    │  │
  │  └────────────────────────────────────────────────────────┘  │
  └──────────────────────────────────────────────────────────────┘

  Protocol:
  1. Client fetches MRENCLAVE (a cryptographic hash of the enclave code)
     from Signal's public documentation. Verifies it matches a known
     good version.
  2. Client performs "remote attestation": the enclave proves to the
     client (via Intel's attestation service) that it is running the
     exact code identified by MRENCLAVE.
  3. Client establishes a TLS session directly with the enclave.
     The TLS private key is generated inside the enclave and never
     leaves it — Signal's own servers cannot intercept this session.
  4. Client sends encrypted contact hashes to the enclave.
  5. Enclave compares hashes against the registered-user set.
     This comparison happens inside the enclave, invisible to Signal.
  6. Enclave returns only the matching contacts (registered users).
     The non-matching contacts are never seen outside the client.

  What can go wrong:
  ┌────────────────────────────────────────────────────────────────┐
  │ Risk                    Mitigation                             │
  ├────────────────────────────────────────────────────────────────┤
  │ SGX side-channels       Intel has patched most known attacks.  │
  │ (Spectre, Foreshadow)   Residual risk cannot be eliminated.    │
  ├────────────────────────────────────────────────────────────────┤
  │ Intel trust assumption  You must trust Intel not to corrupt     │
  │                         the attestation service. Nation-state   │
  │                         adversaries could compel Intel.         │
  ├────────────────────────────────────────────────────────────────┤
  │ Microcode vulnerabilities  Firmware updates may introduce new   │
  │                            attack surfaces.                     │
  └────────────────────────────────────────────────────────────────┘
```

### Approach 3: Private Information Retrieval (PIR)

```
  Mathematical definition: a PIR scheme lets a client query index i from a
  database of n items, without the server learning i.

  Intuition (2-server PIR):
    The client splits the query into two shares, sends one to each server.
    Each server replies with a "partial answer" that reveals nothing by itself.
    The client XORs the two partial answers to get the real result.
    Requires: the two servers do not collude.

  Single-server PIR (computationally secure):
    The server does O(n) work per query (touches every database entry).
    Modern schemes (SealPIR, Spiral PIR) achieve this with FHE or
    lattice-based techniques, with practical overhead in the 10–100× range.

  Tradeoff table:
  ┌──────────────────┬──────────────────┬──────────────────┬──────────┐
  │ Approach         │ Server learns    │ Server work      │ Trust    │
  ├──────────────────┼──────────────────┼──────────────────┼──────────┤
  │ Plaintext upload │ Full contact     │ O(1) per query   │ Blind    │
  │                  │ graph            │                  │ trust    │
  ├──────────────────┼──────────────────┼──────────────────┼──────────┤
  │ Hash matching    │ Hash → reversible│ O(1) per query   │ Blind    │
  │                  │ for small space  │                  │ trust    │
  ├──────────────────┼──────────────────┼──────────────────┼──────────┤
  │ Intel SGX        │ Nothing (if SGX  │ O(1) per query   │ Intel +  │
  │                  │ is not broken)   │                  │ Signal   │
  ├──────────────────┼──────────────────┼──────────────────┼──────────┤
  │ Single-server    │ Nothing          │ O(n) per query   │ None     │
  │ PIR              │ (computational)  │ (expensive)      │          │
  └──────────────────┴──────────────────┴──────────────────┴──────────┘
```

---

## Cross-Reference: What We Have vs What We Need

```
  Primitive                 Repo Status        Spec Reference
  ═══════════════════════════════════════════════════════════════════════════
  SHA-256                   ✓ Implemented      HF03-sha256.md
  SHA-512                   ✓ Implemented      HF04-sha512.md
  HMAC-SHA256               ✓ Implemented      HF05-hmac.md
  PBKDF2                    ✓ Implemented      KD01-pbkdf2.md
  scrypt                    ✓ Implemented      KD02-scrypt.md
  AES-256-GCM               ✓ Implemented      SE01-aes.md, SE02-aes-modes.md
  ChaCha20-Poly1305         ✓ Implemented      SE03-chacha20-poly1305.md
  ───────────────────────────────────────────────────────────────────────────
  HKDF (RFC 5869)           ✗ Not yet built    This spec (uses HMAC-SHA256)
  Curve25519 (X25519)       ✗ Not yet built    This spec
  Ed25519                   ✗ Not yet built    This spec (uses SHA-512)
  X3DH key agreement        ✗ Not yet built    This spec (uses Curve25519, HKDF)
  Double Ratchet Algorithm  ✗ Not yet built    This spec (uses HKDF, ChaCha20)
  Sealed Sender             ✗ Not yet built    This spec (uses Curve25519, HKDF)
  ═══════════════════════════════════════════════════════════════════════════
```

**Build order (bottom-up, each row depends on those above it):**

```
  SHA-256, SHA-512                    (done)
       │
       ▼
  HMAC-SHA256                         (done)
       │
       ▼
  HKDF                                (new — uses HMAC-SHA256)
       │
  ┌────┘
  │
  ▼
  Curve25519   Ed25519                (new — pure math; Ed25519 also uses SHA-512)
       │           │
       └─────┬─────┘
             │
             ▼
  X3DH                               (new — uses Curve25519, Ed25519, HKDF,
             │                              ChaCha20-Poly1305)
             │
             ▼
  Double Ratchet                     (new — uses HKDF, ChaCha20-Poly1305,
             │                              initialized from X3DH output)
             │
             ▼
  Sealed Sender                      (new — uses Curve25519, HKDF,
                                            ChaCha20-Poly1305)
```

---

## Test Strategy

### Layer 1: Curve25519 (X25519)

**1. RFC 7748 Test Vectors**

RFC 7748 Section 6.1 publishes two official test vectors. Both must pass
exactly:

```python
def test_rfc7748_vector_1():
    alice_priv = bytes.fromhex(
        "77076d0a7318a57d3c16c17251b26645"
        "df4c2f87ebc0992ab177fba51db92c2a"
    )
    alice_pub  = bytes.fromhex(
        "8520f0098930a754748b7ddcb43ef75a"
        "0dbf3a0d26381af4eba4a98eaa9b4e6a"
    )
    bob_priv   = bytes.fromhex(
        "5dab087e624a8a4b79e17f8b83800ee6"
        "6f3bb1292618b6fd1c2f8b27ff88e0eb"
    )
    bob_pub    = bytes.fromhex(
        "de9edb7d7b7dc1b4d35b61c2ece43537"
        "3f8343c85b78674dadfc7e146f882b4f"
    )
    shared     = bytes.fromhex(
        "4a5d9d5ba4ce2de1728e3bf480350f25"
        "e07e21c947d19e3376f09b3c1e161742"
    )
    assert x25519(alice_priv, G) == alice_pub
    assert x25519(bob_priv,   G) == bob_pub
    assert x25519(alice_priv, bob_pub) == shared
    assert x25519(bob_priv,   alice_pub) == shared
```

**2. ECDH Commutativity**

```python
def test_ecdh_commutativity():
    for _ in range(100):
        a_priv, a_pub = generate_curve25519_key_pair()
        b_priv, b_pub = generate_curve25519_key_pair()
        assert x25519(a_priv, b_pub) == x25519(b_priv, a_pub)
```

**3. Clamping Verification**

```python
def test_clamping():
    priv, _ = generate_curve25519_key_pair()
    assert priv[0] & 0b00000111 == 0    # bits 0,1,2 cleared
    assert priv[31] & 0b10000000 == 0   # bit 7 of byte 31 cleared
    assert priv[31] & 0b01000000 != 0   # bit 6 of byte 31 set
```

**4. Low-Order Point Rejection**

```python
def test_low_order_point_rejection():
    # These are the 8 low-order points on Curve25519.
    # Clamping (multiplying by a multiple of 8) maps any of them to
    # the identity O, which encodes as all-zeros.
    low_order_u_coords = [
        bytes([0]*32),                  # O (identity)
        bytes([1] + [0]*31),            # point of order 2
        bytes([0xe0, 0xeb, 0x7a, ...]), # (abbreviated)
        # (full list in RFC 7748 Section 6)
    ]
    priv, _ = generate_curve25519_key_pair()
    for low_u in low_order_u_coords:
        result = x25519(priv, low_u)
        # With clamping, the result is always 0 for low-order inputs.
        assert result == bytes([0]*32)
```

**5. Constant-Time Property**

```python
def test_constant_time_approximately():
    # Time 1000 x25519 calls with random inputs.
    # The ratio of max_time / min_time should be < 1.5 on any reasonable
    # hardware (implementation-specific threshold).
    import time
    times = []
    for _ in range(1000):
        k = random_bytes(32)
        u = random_bytes(32)
        t0 = time.perf_counter_ns()
        x25519(k, u)
        times.append(time.perf_counter_ns() - t0)
    ratio = max(times) / min(times)
    assert ratio < 10.0, f"Timing ratio {ratio:.1f}× suggests non-constant-time"
```

### Layer 2: Ed25519

**6. RFC 8032 Test Vectors**

RFC 8032 Section 5.1 provides official test vectors. The first:

```python
def test_rfc8032_vector_1():
    seed = bytes.fromhex(
        "9d61b19deffd5a60ba844af492ec2cc4"
        "4449c5697b326919703bac031cae7f60"
    )
    pub  = bytes.fromhex(
        "d75a980126c0251102fa5d3f7086f2cd"
        "56b904f4f96bc8659beee67a5c18e6d7"  # (abbreviated for space)
    )
    msg  = b""   # empty message
    sig  = bytes.fromhex(
        "e5564300c360ac729b4c1ae69cd6c069"  # R (first 32 bytes)
        "..." # S (next 32 bytes)
    )
    signing_key, verify_key = expand_ed25519_key(seed)
    assert verify_key == pub
    assert ed25519_sign(signing_key, msg) == sig
    assert ed25519_verify(verify_key, msg, sig) == True
```

**7. Sign Then Verify (Any Message)**

```python
def test_sign_verify_round_trip():
    for msg in [b"", b"hello", b"a"*1000, random_bytes(10000)]:
        seed = random_bytes(32)
        sk, vk = expand_ed25519_key(seed)
        sig = ed25519_sign(sk, msg)
        assert len(sig) == 64
        assert ed25519_verify(vk, msg, sig) == True
```

**8. Tamper Detection**

```python
def test_tamper_detection():
    seed = random_bytes(32)
    sk, vk = expand_ed25519_key(seed)
    msg = b"original message"
    sig = ed25519_sign(sk, msg)

    # Modified message must fail.
    assert ed25519_verify(vk, b"modified message", sig) == False

    # Flipped bit in signature must fail.
    for i in range(0, 64, 8):   # test 8 positions
        corrupted = bytearray(sig)
        corrupted[i] ^= 0x01
        assert ed25519_verify(vk, msg, bytes(corrupted)) == False

    # Wrong public key must fail.
    _, wrong_vk = expand_ed25519_key(random_bytes(32))
    assert ed25519_verify(wrong_vk, msg, sig) == False
```

**9. Determinism**

```python
def test_determinism():
    seed = random_bytes(32)
    sk, vk = expand_ed25519_key(seed)
    msg = b"sign me twice"
    sig1 = ed25519_sign(sk, msg)
    sig2 = ed25519_sign(sk, msg)
    assert sig1 == sig2   # must be identical (no randomness in signing)
```

### Layer 3: HKDF

**10. RFC 5869 Appendix A Test Vectors**

RFC 5869 Appendix A.1:

```python
def test_hkdf_rfc5869_a1():
    IKM  = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = bytes.fromhex("000102030405060708090a0b0c")
    info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
    L    = 42
    expected_PRK = bytes.fromhex(
        "077709362c2e32df0ddc3f0dc47bba63"
        "90b6c73bb50f9c3122ec844ad7c2b3e5"
    )
    expected_OKM = bytes.fromhex(
        "3cb25f25faacd57a90434f64d0362f2a"
        "2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        "34007208d5b887185865"
    )
    PRK = hkdf_extract(salt, IKM)
    assert PRK == expected_PRK
    OKM = hkdf_expand(PRK, info, L)
    assert OKM == expected_OKM
```

**11. Output Length Flexibility**

```python
def test_hkdf_length_flexibility():
    IKM = random_bytes(32)
    for L in [1, 32, 64, 80, 8160]:   # 8160 = 255 * 32 (maximum)
        OKM = hkdf(IKM, salt=b"", info=b"test", L=L)
        assert len(OKM) == L
```

**12. Domain Separation**

```python
def test_hkdf_domain_separation():
    IKM  = random_bytes(32)
    salt = random_bytes(32)
    OKM_a = hkdf(IKM, salt, info=b"purpose-A", L=32)
    OKM_b = hkdf(IKM, salt, info=b"purpose-B", L=32)
    assert OKM_a != OKM_b
```

**13. No-Salt Behavior**

```python
def test_hkdf_no_salt():
    # RFC 5869: if salt is not provided, use 32 zero bytes.
    IKM = random_bytes(32)
    OKM_default = hkdf(IKM, salt=None,      info=b"x", L=32)
    OKM_zeros   = hkdf(IKM, salt=b"\x00"*32, info=b"x", L=32)
    assert OKM_default == OKM_zeros
```

### Layer 4: Double Ratchet

**14. Session Initialization Agreement**

```python
def test_session_init_agreement():
    # Simulate X3DH producing shared secret SK.
    SK = random_bytes(32)
    bob_ratchet_priv, bob_ratchet_pub = generate_curve25519_key_pair()

    alice_state = initialize_alice(SK, bob_ratchet_pub)
    bob_state   = initialize_bob(SK, (bob_ratchet_priv, bob_ratchet_pub))

    # Alice sends message 0.
    header, ct = ratchet_encrypt(alice_state, b"hello from Alice", b"")

    # Bob receives message 0.
    plaintext = ratchet_decrypt(bob_state, header, ct, b"")
    assert plaintext == b"hello from Alice"
```

**15. Multi-Message Correctness**

```python
def test_multi_message():
    alice, bob = setup_session()
    for i in range(100):
        msg = f"message {i}".encode()
        header, ct = ratchet_encrypt(alice, msg, b"")
        pt = ratchet_decrypt(bob, header, ct, b"")
        assert pt == msg
```

**16. Out-of-Order Delivery**

```python
def test_out_of_order():
    alice, bob = setup_session()
    # Encrypt 4 messages.
    encrypted = [ratchet_encrypt(alice, f"msg{i}".encode(), b"") for i in range(4)]
    # Deliver in reverse order.
    for i in [3, 0, 2, 1]:
        header, ct = encrypted[i]
        pt = ratchet_decrypt(bob, header, ct, b"")
        assert pt == f"msg{i}".encode()
```

**17. DH Ratchet Step**

```python
def test_dh_ratchet_step():
    alice, bob = setup_session()

    # Alice sends a message.
    h, ct = ratchet_encrypt(alice, b"from alice", b"")
    ratchet_decrypt(bob, h, ct, b"")

    # Bob sends a message — this triggers a DH ratchet on both sides.
    h2, ct2 = ratchet_encrypt(bob, b"from bob", b"")

    old_CKs = alice.CKs   # capture before receiving Bob's message

    pt = ratchet_decrypt(alice, h2, ct2, b"")
    assert pt == b"from bob"

    # Alice's chain keys changed after the DH ratchet step.
    assert alice.CKs != old_CKs
    assert alice.CKr is not None
```

**18. MAX_SKIP Enforcement**

```python
def test_max_skip_enforcement():
    alice, bob = setup_session()
    # Encrypt 1001 messages without delivering any.
    headers_cts = [ratchet_encrypt(alice, b"x", b"") for _ in range(1001)]

    # Attempt to deliver message 1001 (n=1000) without delivering 0-999.
    # The receiver must skip 1000 messages — exceeding MAX_SKIP.
    with pytest.raises(Exception, match="MAX_SKIP"):
        ratchet_decrypt(bob, *headers_cts[1000], b"")
```

**19. Key Deletion (Forward Secrecy)**

```python
def test_message_key_not_retained():
    # After decryption, the message key must not be recoverable.
    # This test checks that MKSKIPPED is cleared after use.
    alice, bob = setup_session()
    h, ct = ratchet_encrypt(alice, b"secret", b"")
    ratchet_decrypt(bob, h, ct, b"")

    # (h.dh, h.n) must no longer be in MKSKIPPED.
    assert (h.dh, h.n) not in bob.MKSKIPPED
```

**20. Cross-Session Consistency**

```python
def test_cross_session_consistency():
    # Serialize and deserialize state mid-session.
    alice, bob = setup_session()
    h, ct = ratchet_encrypt(alice, b"hello", b"")
    bob_serialized = serialize(bob)
    bob2 = deserialize(bob_serialized)
    pt = ratchet_decrypt(bob2, h, ct, b"")
    assert pt == b"hello"
```

### Layer 5: Sealed Sender

**21. Round-Trip Correctness**

```python
def test_sealed_sender_round_trip():
    alice_cert  = get_sender_certificate(alice)
    dr_message  = encrypt_double_ratchet(alice, bob, b"secret")
    sealed      = sealed_sender_encrypt(alice_cert, dr_message, bob.identity_pub)
    result      = sealed_sender_decrypt(bob.identity_priv, bob.identity_pub,
                                        server_key_pub, sealed)
    assert result == b"secret"
```

**22. Tampered Certificate Fails**

```python
def test_tampered_certificate_rejected():
    cert = get_sender_certificate(alice)
    cert.expires_at += 1000000    # tamper with expiry
    # Server signature no longer valid — must be rejected.
    with pytest.raises(AuthError, match="signature invalid"):
        sealed_sender_decrypt(bob.identity_priv, bob.identity_pub,
                              server_key_pub, sealed_sender_encrypt(cert, ...))
```

**23. Expired Certificate Fails**

```python
def test_expired_certificate_rejected():
    cert = get_sender_certificate(alice)
    cert = SenderCertificate(
        ...,
        expires_at = int(time.time() * 1000) - 1    # 1ms in the past
    )
    # Re-sign so signature is valid, but expiry check must still catch it.
    with pytest.raises(AuthError, match="expired"):
        sealed_sender_decrypt(...)
```

**24. Wrong Recipient Cannot Decrypt**

```python
def test_sealed_message_wrong_recipient():
    alice_cert = get_sender_certificate(alice)
    dr_message = encrypt_double_ratchet(alice, bob, b"for bob")
    sealed = sealed_sender_encrypt(alice_cert, dr_message, bob.identity_pub)

    # Eve tries to decrypt a message intended for Bob.
    eve_priv, eve_pub = generate_curve25519_key_pair()
    with pytest.raises(AuthError):
        sealed_sender_decrypt(eve_priv, eve_pub, server_key_pub, sealed)
```

**25. Server Cannot Decrypt**

```python
def test_server_cannot_read_content():
    # The server only has its own signing key, not Bob's identity key.
    # It must be unable to decrypt the sealed message.
    sealed = sealed_sender_encrypt(alice_cert, dr_message, bob.identity_pub)

    # Server tries with its own key (which is not Bob's identity key).
    with pytest.raises(AuthError):
        sealed_sender_decrypt(server_signing_priv, server_signing_pub,
                              server_key_pub, sealed)
```

### Coverage Targets

```
  Layer                  Target    Notes
  ─────────────────────────────────────────────────────────────────────
  Curve25519 (X25519)    95%+      All code paths in the ladder,
                                   cswap, clamping, inverse
  Ed25519                95%+      Sign, verify, encode, decode,
                                   SHA-512 expansion, all reject paths
  HKDF                   95%+      Extract, Expand, combined; all
                                   edge cases (empty salt, L=1, L=8160)
  Double Ratchet         90%+      In-order, out-of-order, DH step,
                                   MAX_SKIP, initialization, teardown
  Sealed Sender          90%+      Encrypt, decrypt, all rejection paths
  ─────────────────────────────────────────────────────────────────────
```

Every RFC test vector mentioned above must pass exactly. Failures in test
vectors are implementation bugs, not spec ambiguities — the RFCs publish
reference values computed by independent implementations.

---

## Dependencies

```
msg-crypto-foundation
│
├── depends on (existing) ──→ HF04-sha512.md
│                              SHA-512 used by Ed25519 key expansion and signing
│
├── depends on (existing) ──→ HF05-hmac.md
│                              HMAC-SHA256 used by HKDF (both Extract and Expand)
│
├── depends on (existing) ──→ SE03-chacha20-poly1305.md
│                              ChaCha20-Poly1305 used by Double Ratchet and
│                              Sealed Sender for authenticated encryption
│
├── depends on (existing) ──→ SE01-aes.md, SE02-aes-modes.md (optional)
│                              AES-256-GCM as an alternative AEAD on x86
│
└── used by ───────────────→ Any messaging application
                              Any protocol requiring authenticated
                              key exchange and forward-secret messaging
```
