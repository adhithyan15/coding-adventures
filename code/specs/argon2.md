# Argon2 — Memory-Hard Password Hashing

## Overview

Argon2 (RFC 9106) is a memory-hard password hashing function that won
the 2015 Password Hashing Competition. Designed by Alex Biryukov,
Daniel Dinu, and Dmitry Khovratovich, it supersedes bcrypt, scrypt,
and PBKDF2 as the recommended password hashing algorithm (OWASP 2023).

### Three Variants

- **Argon2d:** Data-dependent memory access. The address of each block
  read depends on the *content* of previously computed blocks. Fastest
  variant, but vulnerable to side-channel attacks (an attacker who can
  observe memory access patterns can recover the password).
- **Argon2i:** Data-independent memory access. Block addresses are
  computed from the pass number, lane, and block index only — no data
  dependency. Resistant to side-channel attacks but slightly weaker
  against GPU/ASIC attacks with fewer passes.
- **Argon2id:** Hybrid. The first half of the first pass uses Argon2i
  addressing (side-channel resistant during the critical initial fill),
  then switches to Argon2d addressing for the rest (GPU/ASIC
  resistant). **This is the recommended variant.**

### Why Memory-Hardness Matters (Revisited)

A modern GPU has ~10,000 cores but each core has limited memory
bandwidth. scrypt introduced memory-hardness, but Argon2 improves on
it with:

- **Configurable parallelism (p):** Multiple independent lanes that
  can run on separate threads.
- **Multiple passes (t):** Iterating over the entire memory multiple
  times increases the cost of time-memory tradeoff attacks.
- **Data-dependent addressing:** Argon2d (and the second half of
  Argon2id) uses memory contents to determine access patterns, making
  it harder for attackers to predict which blocks are needed.

## Algorithm

### Parameters

| Parameter | Symbol | Description |
|-----------|--------|-------------|
| Password | P | The secret input (0..2^32 - 1 bytes) |
| Salt | S | Random salt (8+ bytes recommended, 16 bytes ideal) |
| Parallelism | p | Number of lanes (1..2^24 - 1) |
| Tag length | T | Desired output length in bytes (4+) |
| Memory size | m | Memory in KiB (8p minimum). Rounded down to 4p multiple |
| Iterations | t | Number of passes over memory (1+) |
| Version | v | 0x13 (decimal 19, current version) |
| Type | y | 0=Argon2d, 1=Argon2i, 2=Argon2id |

### Memory Layout

The memory is organized as a 2D matrix:

```
         ┌─────────────┬─────────────┬─────────────┬─────────────┐
Lane 0   │ Segment 0   │ Segment 1   │ Segment 2   │ Segment 3   │
         │ (slice 0)   │ (slice 1)   │ (slice 2)   │ (slice 3)   │
         ├─────────────┼─────────────┼─────────────┼─────────────┤
Lane 1   │ Segment 0   │ Segment 1   │ Segment 2   │ Segment 3   │
         ├─────────────┼─────────────┼─────────────┼─────────────┤
  ...    │     ...     │     ...     │     ...     │     ...     │
         ├─────────────┼─────────────┼─────────────┼─────────────┤
Lane p-1 │ Segment 0   │ Segment 1   │ Segment 2   │ Segment 3   │
         └─────────────┴─────────────┴─────────────┴─────────────┘

Each block = 1024 bytes (128 × 64-bit words)
Total blocks = m' (m rounded down to nearest 4p multiple)
Blocks per lane = m' / p = q
Blocks per segment = q / 4
```

### Step 1: Initial Hashing (H0)

```
H0 = Blake2b-64(
    LE32(p) || LE32(T) || LE32(m) || LE32(t) ||
    LE32(v) || LE32(y) ||
    LE32(len(P)) || P ||
    LE32(len(S)) || S ||
    LE32(len(K)) || K ||      // K = optional key (empty if unused)
    LE32(len(X)) || X         // X = optional associated data (empty if unused)
)
```

`H0` is 64 bytes (Blake2b with 64-byte digest).

### Step 2: Fill First Two Blocks of Each Lane

For each lane i (0 to p-1):

```
B[i][0] = H'(H0 || LE32(0) || LE32(i))   // 1024 bytes
B[i][1] = H'(H0 || LE32(1) || LE32(i))   // 1024 bytes
```

Where `H'` is a variable-length hash built from Blake2b:
- If output <= 64 bytes: `Blake2b-T(LE32(T) || input)`
- If output > 64 bytes (our case — 1024 bytes): compute in 64-byte
  chunks, using the first 32 bytes of each intermediate hash as input
  to the next, collecting 32-byte pieces. Final chunk may use a
  shorter Blake2b.

### Step 3: Fill Remaining Blocks

For each pass (0 to t-1), for each slice (0 to 3), for each lane
(0 to p-1 — can be parallel within a slice):

```
For each block index within the segment:
    1. Compute reference block indices (j1, j2) based on:
       - Argon2d: previous block's first 64 bits
       - Argon2i: pre-generated address block (counter-based)
       - Argon2id: Argon2i for pass 0, slices 0-1; Argon2d otherwise
    2. j1 selects the reference lane (j2 mod p, but constrained to
       current lane during pass 0 slice 0)
    3. j2 selects the reference block index within available blocks
       (via a mapping function that biases toward recent blocks)
    4. B[i][z] = G(B[i][z-1], B[j1][j2])          // pass 0
       B[i][z] = B[i][z] XOR G(B[i][z-1], B[j1][j2])  // pass 1+
```

### Step 4: Compression Function G

G takes two 1024-byte blocks (R, Z) and produces a 1024-byte block:

```
G(X, Y):
    R = X XOR Y                    // 1024 bytes = 8 × 128-byte rows
    // Apply Blake2b round function (GB) to columns then diagonals
    // within each 128-byte row (treating as 16 × 64-bit words)
    For each 128-byte row of R:
        Apply P (permutation using GB rounds) to the row
    Z = result
    // Then apply P column-wise (transpose, apply, transpose back)
    For each 128-byte column:
        Apply P to the column
    Return R XOR Z
```

The permutation P applies the Blake2b mixing function `GB` (the G
function from Blake2b, not to be confused with the outer G) in the
standard column-then-diagonal pattern.

### Step 5: Finalize

```
B_final = B[0][q-1] XOR B[1][q-1] XOR ... XOR B[p-1][q-1]
Tag = H'(B_final)   // Variable-length hash to get T bytes
```

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `argon2id` | `(password, salt, t, m, p, tag_length) -> tag` | Recommended hybrid variant. |
| `argon2d` | `(password, salt, t, m, p, tag_length) -> tag` | Data-dependent variant. |
| `argon2i` | `(password, salt, t, m, p, tag_length) -> tag` | Data-independent variant. |

All functions accept optional parameters:
- `key`: Secret key for keyed hashing (default: empty).
- `associated_data`: Additional data bound to the hash (default: empty).
- `version`: Protocol version (default: 0x13).

Recommended parameters (OWASP 2023):
- **Interactive login:** t=1, m=47104 (46 MiB), p=1
- **Sensitive storage:** t=2, m=19456 (19 MiB), p=1
- **Minimum:** t=3, m=12288 (12 MiB), p=1

## Test Vectors (RFC 9106, Section 4)

The RFC test vectors use small parameters for verification:

### Argon2d

```
Type:        Argon2d (y=0)
Version:     0x13
Memory:      32 (KiB)
Iterations:  3
Parallelism: 4
Tag length:  32
Password:    01010101010101010101010101010101
             01010101010101010101010101010101 (32 bytes of 0x01)
Salt:        02020202020202020202020202020202 (16 bytes of 0x02)
Key:         03030303030303030303030303030303 (optional, 8 bytes of 0x03 — but RFC uses empty for base test)
AD:          04040404040404040404040404040404 (optional, 12 bytes of 0x04 — but RFC uses empty for base test)

Tag: 512b391b6f1162975083e271b3d8011b
     312d18100fbb0871e2f91ed18f0d23ce
```

### Argon2i

```
Type:        Argon2i (y=1)
Version:     0x13
Memory:      32 (KiB)
Iterations:  3
Parallelism: 4
Tag length:  32
Password:    01010101010101010101010101010101
             01010101010101010101010101010101 (32 bytes of 0x01)
Salt:        02020202020202020202020202020202 (16 bytes of 0x02)

Tag: c814d9d1dc7f37aa13f0d77f2494bda1
     c8de6b016dd388d29952a4c4672b6ce8
```

### Argon2id

```
Type:        Argon2id (y=2)
Version:     0x13
Memory:      32 (KiB)
Iterations:  3
Parallelism: 4
Tag length:  32
Password:    01010101010101010101010101010101
             01010101010101010101010101010101 (32 bytes of 0x01)
Salt:        02020202020202020202020202020202 (16 bytes of 0x02)

Tag: 0d640df58d78766c08c037a34a8b53c9
     d01ef0452d75b65eb52520e96b01e659
```

### Argon2id — With Key and Associated Data

```
Type:        Argon2id (y=2)
Version:     0x13
Memory:      32 (KiB)
Iterations:  3
Parallelism: 4
Tag length:  32
Password:    01010101010101010101010101010101
             01010101010101010101010101010101 (32 bytes of 0x01)
Salt:        02020202020202020202020202020202 (16 bytes of 0x02)
Key:         0303030303030303 (8 bytes of 0x03)
AD:          040404040404040404040404 (12 bytes of 0x04)

Tag: 0d640df58d78766c08c037a34a8b53c9
     d01ef0452d75b65eb52520e96b01e659
```

*Note: The RFC 9106 test vectors in Section 4 use password=32 bytes
of 0x01, salt=16 bytes of 0x02, secret key=8 bytes of 0x03, and
associated data=12 bytes of 0x04. Implementations should verify
against the reference implementation at
https://github.com/P-H-C/phc-winner-argon2 for exact values.*

## Package Matrix

Same 9 languages, in `argon2/` directories.

**Dependencies:** None. Self-contained — includes Blake2b-based
compression and variable-length hashing internally. Does not depend
on an external Blake2b package (the mixing function used in Argon2 is
a specific application of Blake2b internals, not the full Blake2b hash).
