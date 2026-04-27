# HF06 — BLAKE2b

## Overview

BLAKE2b (RFC 7693) is a cryptographic hash function designed by
Jean-Philippe Aumasson, Samuel Neves, Zooko Wilcox-O'Hearn, and
Christian Winnerlein in 2012. It is:

- **Faster than MD5, SHA-1, SHA-2, and SHA-3 in software** on 64-bit
  platforms — often 2–3× faster than SHA-256.
- **As secure as SHA-3** against known attacks (collision and preimage
  resistance at 2^(n/2) and 2^n respectively for an n-bit digest).
- **Simpler than SHA-3** — its compression function is a tweaked BLAKE
  round, itself based on the ChaCha stream cipher's quarter-round.
- **Variable output length** — anywhere from 1 to 64 bytes without
  truncation or re-hashing.
- **Natively keyed** — produces a MAC in a single pass without the
  HMAC construction. A keyed BLAKE2b is faster and just as secure as
  HMAC-SHA-512.
- **Parameterized** — salt, personalization string, and tree-hashing
  parameters are folded directly into the initial state.

BLAKE2b is the 64-bit-word variant (producing up to 64-byte digests);
BLAKE2s is the 32-bit variant (up to 32 bytes). This spec covers
BLAKE2b only. Sequential mode is specified; tree-mode parameters are
reserved but not exercised here.

### Why it matters for this repo

- **Argon2 (KD03)** uses BLAKE2b-long as its internal compression
  primitive and initial hash step (H0). We cannot ship Argon2 without
  BLAKE2b.
- **libsodium, Noise Protocol, IPFS, WireGuard, and Zcash** all use
  BLAKE2b. Having our own implementation gives us a reference for
  reading those ecosystems.
- **Educational**: BLAKE2b's G function is a clean showcase of the
  ARX (Add-Rotate-XOR) design paradigm that also underlies ChaCha20,
  Salsa20, and Skein.

## Algorithm

### Parameters

| Parameter | Description | Range |
|-----------|-------------|-------|
| `nn` | Digest length in bytes | 1..64 |
| `kk` | Key length in bytes (0 for unkeyed) | 0..64 |
| `salt` | Optional salt | exactly 16 bytes or empty |
| `personal` | Optional personalization | exactly 16 bytes or empty |

### Constants

**Block size:** 128 bytes (16 × 64-bit words).
**State:** 8 × 64-bit words `h[0..7]`.
**Rounds:** 12.

**Initial hash values (IV)** — identical to SHA-512 IVs, derived from
the fractional parts of the square roots of the first 8 primes:

```
IV[0] = 0x6a09e667f3bcc908
IV[1] = 0xbb67ae8584caa73b
IV[2] = 0x3c6ef372fe94f82b
IV[3] = 0xa54ff53a5f1d36f1
IV[4] = 0x510e527fade682d1
IV[5] = 0x9b05688c2b3e6c1f
IV[6] = 0x1f83d9abfb41bd6b
IV[7] = 0x5be0cd19137e2179
```

**Message schedule (SIGMA)** — 10 permutations of (0..15), indexed by
round number modulo 10. Round `i` uses `SIGMA[i % 10]`.

```
SIGMA[ 0] =  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
SIGMA[ 1] = 14 10  4  8  9 15 13  6  1 12  0  2 11  7  5  3
SIGMA[ 2] = 11  8 12  0  5  2 15 13 10 14  3  6  7  1  9  4
SIGMA[ 3] =  7  9  3  1 13 12 11 14  2  6  5 10  4  0 15  8
SIGMA[ 4] =  9  0  5  7  2  4 10 15 14  1 11 12  6  8  3 13
SIGMA[ 5] =  2 12  6 10  0 11  8  3  4 13  7  5 15 14  1  9
SIGMA[ 6] = 12  5  1 15 14 13  4 10  0  7  6  3  9  2  8 11
SIGMA[ 7] = 13 11  7 14 12  1  3  9  5  0 15  4  8  6  2 10
SIGMA[ 8] =  6 15 14  9 11  3  0  8 12  2 13  7  1  4 10  5
SIGMA[ 9] = 10  2  8  4  7  6  1  5 15 11  9 14  3 12 13  0
```

Rounds 10 and 11 reuse SIGMA[0] and SIGMA[1].

### The G function (quarter-round)

Given a 16-word working vector `v[0..15]` and two message words `x`
and `y`, the mixing function G operates on four indices `a`, `b`, `c`,
`d`:

```
G(v, a, b, c, d, x, y):
    v[a] = (v[a] + v[b] + x) mod 2^64
    v[d] = rotr64(v[d] XOR v[a], 32)
    v[c] = (v[c] + v[d])     mod 2^64
    v[b] = rotr64(v[b] XOR v[c], 24)
    v[a] = (v[a] + v[b] + y) mod 2^64
    v[d] = rotr64(v[d] XOR v[a], 16)
    v[c] = (v[c] + v[d])     mod 2^64
    v[b] = rotr64(v[b] XOR v[c], 63)
```

Rotation constants are `(R1, R2, R3, R4) = (32, 24, 16, 63)`.

**Truth table of ARX steps** (why this works):

| Step | Op     | Provides |
|------|--------|----------|
| `+`  | Add    | Non-linearity in GF(2) (carry propagation) |
| `X`  | XOR    | Linearity in GF(2), diffusion of added bits |
| `R`  | Rotate | Bit-position diffusion across word boundaries |

ARX ciphers lean on the fact that `+` and `XOR` alone each have weak
cryptographic properties, but their composition — interleaved with
rotations — defeats linear and differential attacks cheaply.

### The compression function F

Given state `h`, message block `m` (16 × 64-bit words, little-endian),
byte counter `t` (128-bit, stored as two 64-bit words `t0`, `t1`), and
final-block flag `f` (true for the last block, false otherwise):

```
F(h, m, t, f):
    # Initialize working vector
    v[0..7]   = h[0..7]
    v[8..15]  = IV[0..7]
    v[12]    ^= t0          # low counter
    v[13]    ^= t1          # high counter
    if f: v[14] ^= 0xFFFFFFFFFFFFFFFF

    # 12 rounds
    for i in 0..11:
        s = SIGMA[i mod 10]
        G(v, 0, 4,  8, 12, m[s[ 0]], m[s[ 1]])   # column 0
        G(v, 1, 5,  9, 13, m[s[ 2]], m[s[ 3]])   # column 1
        G(v, 2, 6, 10, 14, m[s[ 4]], m[s[ 5]])   # column 2
        G(v, 3, 7, 11, 15, m[s[ 6]], m[s[ 7]])   # column 3
        G(v, 0, 5, 10, 15, m[s[ 8]], m[s[ 9]])   # diagonal 0
        G(v, 1, 6, 11, 12, m[s[10]], m[s[11]])   # diagonal 1
        G(v, 2, 7,  8, 13, m[s[12]], m[s[13]])   # diagonal 2
        G(v, 3, 4,  9, 14, m[s[14]], m[s[15]])   # diagonal 3

    # Finalize: XOR both halves of v into h
    for j in 0..7:
        h[j] ^= v[j] XOR v[j + 8]
```

The column-then-diagonal pattern is identical to ChaCha20's double
round; each round touches every word twice.

### Parameter block and initialization

The parameter block is 64 bytes (8 × 64-bit words). For sequential,
unkeyed hashing with no salt/personalization:

```
P[0] = 0x0000000001010000 XOR nn   # fanout=1, depth=1, node=0, kk=0, nn
P[1..7] = 0                         # leaf_length, node_offset, node_depth,
                                    # inner_length, reserved, salt, personal
```

With a key of length `kk`:

```
P[0] = 0x0000000001010000 XOR (kk << 8) XOR nn
```

With salt (16 bytes) and personalization (16 bytes):

```
P[4..5] = salt as two 64-bit LE words
P[6..7] = personal as two 64-bit LE words
```

Initial state:

```
h[i] = IV[i] XOR P[i]   for i in 0..7
```

### Padding and processing

1. **Keyed mode (kk > 0):** prepend the key, zero-padded to a full
   128-byte block, to the message. This first block is processed
   through F like any other, but counts toward the byte total.
2. **Process all but the last block** with `f = false` and
   `t = bytes_so_far` (counter updated *before* F is called, because
   it represents the number of bytes fed in *up to and including* the
   current block).
3. **Last block:** zero-pad to 128 bytes, set `f = true`, set
   `t = total_bytes_input` (which excludes zero padding but includes
   the prepended key block if keyed).
4. **Output:** the first `nn` bytes of `h[0..7]` serialized
   little-endian. For `nn < 64`, truncate; there is no separate
   "BLAKE2b-256" construction — it is BLAKE2b with `nn = 32`.

### Edge case: empty message, unkeyed

Process a single zero-padded 128-byte block with `t = 0`, `f = true`.
This is *not* the same as "no blocks" — the finalization must run.

### Edge case: empty message, keyed

The key block itself is the only block. Process it with `t = 128`
(one full block of bytes was fed) and `f = true`.

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `blake2b` | `(data: bytes, digest_size: int = 64, key: bytes = b"", salt: bytes = b"", personal: bytes = b"") -> bytes` | One-shot hash. |
| `blake2b_hex` | same args | Returns lowercase hex string of the digest. |
| `Blake2bHasher` | class/struct | Streaming hasher. |
| `.update(data)` | | Feed bytes; returns self for chaining. |
| `.digest()` | | Finalize and return `digest_size` bytes. Multiple calls return the same value. |
| `.hex_digest()` | | Lowercase hex. |
| `.copy()` | | Deep copy the hasher state. |

**Validation rules:**

- `digest_size` in `[1, 64]` — else raise / return error.
- `len(key)` in `[0, 64]` — else raise / return error.
- `len(salt)` in `{0, 16}` — else raise / return error.
- `len(personal)` in `{0, 16}` — else raise / return error.

## Test Vectors (RFC 7693 Appendix A and official BLAKE2 test suite)

```
# Empty message, unkeyed, 64-byte output:
blake2b("") =
    786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419
    d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce

# "abc", unkeyed, 64-byte output:
blake2b("abc") =
    ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1
    7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923

# Keyed example, 64-byte output:
# key  = 0x01 0x02 ... 0x40 (64 bytes, values 1..64)
# data = 0x00 0x01 ... 0xff (256 bytes, values 0..255)
blake2b(data, key) =
    402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927
    ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3

# The quick brown fox (default 64-byte output):
blake2b("The quick brown fox jumps over the lazy dog") =
    a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673
    f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918

# Short digest:
blake2b_hex("", digest_size=32) =
    0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8

# Check: digest_size scales output, not state — the full 64-byte
# internal state is computed then truncated.
```

## Streaming Behavior

Streaming must produce the same digest as one-shot for any chunk
boundary:

```
h = Blake2bHasher()
h.update(b"hel"); h.update(b"lo")
h.digest() == blake2b(b"hello")
```

Implementations hold a 128-byte internal buffer. When the buffer fills
mid-update, the filled block is compressed and the remainder becomes
the new buffer contents. The last partial buffer is only compressed
at `digest()` time (with the final flag set).

**Key invariant:** the final block must always go through F with
`f = true`. If the message is exactly a multiple of 128 bytes, we do
*not* add a zero block — we just flag the last real block final at
`digest()` time. This is different from Merkle-Damgård-style
paddings (SHA-2, MD5) and is often a source of off-by-one bugs.

## Security Properties

| Attack | Cost (n-bit output) |
|--------|---------------------|
| Collision | 2^(n/2) |
| Preimage | 2^n |
| Second preimage | 2^n |

For the full 512-bit output: 2^256 collision, 2^512 preimage. The
best known cryptanalysis covers 7.5 of 12 rounds (Guo et al., 2014);
full BLAKE2b has no known attack better than generic.

**Keyed BLAKE2b vs HMAC-SHA-512:**

- Keyed BLAKE2b runs the key as a single prepended block — one extra
  compression call total.
- HMAC-SHA-512 runs *two* SHA-512 invocations (inner and outer), each
  with its own padded key block — four extra compression calls.
- Both offer the same security level. BLAKE2b is simpler to analyze
  because the key is not XOR-folded with `ipad`/`opad`.

## Language-Specific Notes

- **Lua/Perl:** Require 64-bit integer arithmetic with well-defined
  wraparound on addition. Lua 5.3+ integers are signed 64-bit;
  emulate `u64` addition by masking with `0xFFFFFFFFFFFFFFFF`. Perl
  on 64-bit platforms works with `use integer;` plus masking, but
  be careful on 32-bit builds — prefer `Math::Int64` or BigInt
  fallbacks.
- **TypeScript:** Use `BigInt` throughout. Performance is acceptable
  for educational workloads; a pure-`number` two-word emulation is
  roughly 3× faster but tripled in code size. This repo uses `BigInt`.
- **Go/Rust/Swift:** Native `uint64` / `u64` / `UInt64` with wrapping
  arithmetic (`&+` in Swift, `wrapping_add` in Rust, plain `+` in Go).
- **Elixir:** Use `Bitwise` and `rem/2` with `0xFFFFFFFFFFFFFFFF` mask
  for wraparound; or binary pattern matching on `<<_::unsigned-64>>`.
- **Haskell:** `Data.Word.Word64` plus `Data.Bits`.

## Dependencies

None. BLAKE2b is self-contained — its G function, IVs, and SIGMA
permutations are the only primitives.

## Package Matrix

Same 10 languages as ChaCha20-Poly1305 (SE03): Python, TypeScript,
Go, Rust, Ruby, Elixir, Lua, Perl, Swift, Haskell. Package directory
name: `blake2b/` under each language tree.

## Non-Goals

- **Tree hashing:** RFC 7693 describes tree-mode parameters; this spec
  initializes `fanout=1`, `depth=1` (sequential mode) and does not
  implement the tree hash variants.
- **BLAKE2s, BLAKE2bp, BLAKE2sp:** separate variants; not covered here.
- **BLAKE3:** a different algorithm despite the name lineage; not
  covered here.
- **XOF mode:** BLAKE2Xb extends output beyond 64 bytes; not covered
  here. If Argon2 needs more than 64 output bytes (it does, for
  `T' > 64` in Argon2 initial fill), the Argon2 package implements
  the BLAKE2b-long construction (recursive 32-byte overlap) on top of
  this package.
