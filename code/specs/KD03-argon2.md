# KD03 — Argon2 (d / i / id)

## Overview

Argon2 (RFC 9106) is the memory-hard password hashing function that
won the 2015 Password Hashing Competition.  It was designed by Alex
Biryukov, Daniel Dinu, and Dmitry Khovratovich at the University of
Luxembourg.  Argon2 supersedes bcrypt, scrypt, and PBKDF2 as the
recommended password hashing primitive (OWASP, NIST SP 800-63B, IETF
CFRG).

Argon2 is a tunable key-derivation and password-hashing function whose
cost is dominated by memory, not CPU.  Its internal primitive is the
BLAKE2b round function (HF06) applied to 128-byte rows and 128-byte
columns of a 1024-byte block; its outer hashing is BLAKE2b itself.

### Three variants

| Variant   | `y` | Addressing     | Side-channel | GPU/ASIC | Recommended use |
|-----------|-----|----------------|--------------|----------|-----------------|
| Argon2d   | 0   | Data-dependent | Vulnerable   | Strong   | Cryptocurrency, server-to-server (no secret-dependent access) |
| Argon2i   | 1   | Data-independent | Resistant  | Weaker   | Side-channel-sensitive contexts (shared hardware, VM neighbours) |
| Argon2id  | 2   | Argon2i for pass 0, slices 0–1; Argon2d elsewhere | Near-resistant | Strong | **Default / password storage** |

The three share the same algorithm, memory layout, and compression
function.  They differ only in how reference-block indices (J1, J2)
are derived during the fill step.  One implementation body can cover
all three by dispatching on `y` at the addressing step.

### Why memory hardness

A modern GPU has ~10,000 cores but ~1 GB of shared memory bandwidth.
A hash function whose critical path touches gigabytes of unique
addresses per evaluation forces an attacker to allocate gigabytes per
concurrent guess.  Argon2 improves on scrypt by:

- **Tunable parallelism (p)** — multiple independent lanes usable per
  password evaluation.
- **Tunable passes (t)** — iterate the entire memory t times; each
  pass overwrites the previous contents (pass 0) or XORs onto them
  (pass ≥ 1), increasing tradeoff-attack cost.
- **Argon2d addressing** — block indices depend on block contents,
  which an attacker cannot prefetch.
- **Large-block BLAKE2b permutation** — each G call mixes 1024 bytes
  in 16 BLAKE2b rounds, giving strong diffusion per memory touch.

### Relationship to HF06

Argon2 uses BLAKE2b from HF06 as an external primitive for:

1. **H0 (initial hash)** — a single BLAKE2b-512 call that absorbs all
   parameters and inputs.
2. **H' (variable-length hash)** — a BLAKE2b-based XOF used both to
   produce the first two blocks of each lane and to derive the final
   tag.

It also reuses the BLAKE2b round function (the `G`/permutation-P
described in HF06 §G) as the inner primitive of its own compression
function, but operating on 16-word rows of a 1024-byte block instead
of the 128-byte block of BLAKE2b proper.  Implementations should
depend on the `blake2b` package for the full-hash calls and inline a
128-byte-row variant of the BLAKE2b round for the compression.

## Algorithm

### Parameters

| Parameter | Symbol | Description | Range |
|-----------|--------|-------------|-------|
| Password  | P | Secret input | 0..2^32−1 bytes |
| Salt      | S | Random salt | 8..2^32−1 bytes (16 recommended) |
| Secret key | K | Optional MAC key | 0..2^32−1 bytes |
| Associated data | X | Optional bound context | 0..2^32−1 bytes |
| Parallelism | p | Lane count | 1..2^24−1 |
| Tag length | T | Output bytes | 4..2^32−1 |
| Memory | m | KiB of working memory | 8·p..2^32−1 |
| Iterations | t | Passes over memory | 1..2^32−1 |
| Version | v | Protocol version | 0x13 (19) |
| Type | y | 0=Argon2d, 1=Argon2i, 2=Argon2id |  |

**Validation rules** (enforced before any BLAKE2b call):

- `T ≥ 4` and `T ≤ 2^32 − 1`.
- `p ≥ 1` and `p ≤ 2^24 − 1`.
- `m ≥ 8·p`.  Implementations MUST compute `m' = 4·p·⌊m / (4·p)⌋`
  (round down to a multiple of `4·p`) before use.  All lane/segment
  arithmetic uses `m'`.
- `t ≥ 1`.
- `v = 0x13`.  Version `0x10` is obsolete and out of scope.
- `y ∈ {0, 1, 2}`.
- `len(S) ≥ 8`.

### Constants

- **Block size:** 1024 bytes (128 × 64-bit words, little-endian).
- **Sync points per pass:** 4 (one "slice" each).  Within a slice,
  lanes can execute in parallel; between slices, lanes synchronise.
- **Lane length:** `q = m' / p` blocks.
- **Segment length:** `SL = q / 4` blocks.
- **Version byte:** 0x13 (encoded as `LE32(0x13)` inside H0).

### Step 1 — H0 (64 bytes)

```
H0 = BLAKE2b-512(
         LE32(p)  || LE32(T)  || LE32(m)  || LE32(t) ||
         LE32(v)  || LE32(y)  ||
         LE32(|P|) || P ||
         LE32(|S|) || S ||
         LE32(|K|) || K ||
         LE32(|X|) || X
     )
```

All length prefixes are **unsigned** little-endian 32-bit integers.
If `K` or `X` is empty, emit `LE32(0)` followed by zero bytes (i.e.
just the 4-byte length).  The full-hash BLAKE2b call produces 64
bytes with no key, salt, or personalisation.

### Step 2 — H' (variable-length hash)

Argon2 needs BLAKE2b-style output both shorter and longer than 64
bytes.  `H'(T, X)` returns `T` bytes:

```
H'(T, X):
    if T ≤ 64:
        return BLAKE2b-T(LE32(T) || X)          # digest_size=T
    else:
        r = ⌈T / 32⌉ − 2
        V_1 = BLAKE2b-512(LE32(T) || X)          # 64 bytes
        for i in 2..r:
            V_i = BLAKE2b-512(V_{i-1})           # 64 bytes
        # Final partial: digest_size = T − 32·r (between 1 and 64).
        V_{r+1} = BLAKE2b-(T − 32r)(V_r)
        # Concatenate first 32 bytes of V_1..V_r, then all of V_{r+1}.
        return V_1[0..32] || V_2[0..32] || ... || V_r[0..32] || V_{r+1}
```

**Length check.**  The RFC defines `r = ⌈T / 32⌉ − 2` so that
exactly `r` 32-byte prefixes plus one final `(T − 32r)`-byte block
sum to `T` bytes (`32·r + (T − 32·r) = T`).  When `T = 64`, the `T ≤
64` branch applies and `H'` is just one BLAKE2b-512 call with no
32-byte overlap.

### Step 3 — First two blocks of each lane

For each lane `i ∈ [0, p)`:

```
B[i][0] = H'(1024, H0 || LE32(0) || LE32(i))
B[i][1] = H'(1024, H0 || LE32(1) || LE32(i))
```

Each block is 1024 bytes = 128 × `u64` words in little-endian.

### Step 4 — Fill the rest of memory

Process the matrix in **sync-point order**: for each pass `r`, for
each slice `s ∈ {0,1,2,3}`, for each lane `l ∈ [0, p)` (lanes may run
concurrently within a slice), for each column offset `c` inside the
segment:

```
z = s · SL + c
if r == 0 and z < 2: continue        # slots already filled in Step 3
prev = B[l][z − 1]                    # wraps within the lane: B[l][q − 1] if z == 0
(J1, J2) = addressing(r, l, s, c)     # see Step 5
l'  = J2 mod p                        # reference lane
    if r == 0 and s == 0: l' = l      # pass 0 slice 0 is lane-local
z'  = index_alpha(J1, reference_area) # see Step 6
ref = B[l'][z']
if r == 0:
    B[l][z] = G(prev, ref)
else:
    B[l][z] = G(prev, ref)  XOR  B[l][z]          # XOR-in-place (v1.3 only)
```

**Pass-0-slice-0 exception.**  During the very first slice of the
very first pass, the reference block MUST lie in the same lane.  All
other passes/slices allow cross-lane references.

**Previous-block wrap.**  When `z == 0` (start of a lane, possible
only for `r ≥ 1`), `prev = B[l][q − 1]` (the final block of the
previous pass in the same lane).

### Step 5 — Addressing

Let `previous` be the 1024-byte block feeding the current column
(i.e. `B[l][z − 1]`, wrapped as in Step 4).  Argon2 produces two
32-bit pseudo-random integers `J1` and `J2` per slot.

**Argon2d (y = 0).**

```
J1 = LE32( previous[0..4]   )   # low 32 bits of word 0
J2 = LE32( previous[4..8]   )   # high 32 bits of word 0
```

**Argon2i (y = 1).**  Address blocks are generated by applying `G`
twice, in counter mode, to a "seed block" whose first seven words are:

```
seed = LE64(r) || LE64(l) || LE64(s) || LE64(m') || LE64(t) || LE64(y) || LE64(counter) || 0...0
# (total: 128 × u64, only the first seven words are non-zero; `counter` starts at 1
#  and is incremented once per address-block generation within the current segment.)
address_block = G( ZeroBlock, G( ZeroBlock, seed ) )
```

The segment is walked in chunks of 128 slots: the first address block
is generated at counter = 1 and supplies `(J1, J2)` pairs for slots
0..127 of the segment; the next address block (counter = 2) supplies
128..255; and so on.  Each 1024-byte address block is split into 128
`u64` words; word `k` provides `J1 = low32(word_k)` and
`J2 = high32(word_k)` for the k-th slot consuming this block.

**Argon2id (y = 2).**  Use Argon2i addressing if `r == 0 AND
s ∈ {0, 1}`; otherwise use Argon2d addressing.  The mode is decided
per slot, not per segment, but because (r, s) are fixed inside a
segment the decision is really per-segment.

### Step 6 — index_alpha

Given `J1 ∈ [0, 2^32)`, map it to a reference-block index inside a
**reference set** that depends on (r, l, l', s, z):

```
# Reference-set size W (number of candidate blocks for this slot):
if r == 0:
    if s == 0:
        W = z − 1                        # current segment so far (same lane only)
    else:
        if l' == l:
            W = s · SL + z − 1           # finished slices plus current slice prefix
        else:
            W = s · SL − (z == 0 ? 1 : 0) # other lane, finished slices only;
                                         #   subtract 1 if we are at a slice boundary
else:
    if l' == l:
        W = q − SL + z − 1               # last three slices (rolling window) + current prefix
    else:
        W = q − SL − (z == 0 ? 1 : 0)

# Relative position (biased toward recent blocks):
x   = (J1 * J1) >> 32                    # mul, keep high 32 bits; all in u64
y   = (W * x)   >> 32
rel = W − 1 − y

# Absolute column inside lane l':
if r == 0:
    start = 0                            # window starts at column 0 of lane l'
else:
    start = (s + 1) · SL mod q           # window starts one slice ahead
z' = (start + rel) mod q
```

The `J1²` multiplication "biases toward the end of the window"
(recent blocks are more likely to be referenced), which gives the
fill step strong mixing without creating an exploitable distribution.

**Boundary rule.**  Argon2 forbids referring to the **immediately
preceding** block within the current lane (that block is already
`prev`).  The subtraction `(z == 0 ? 1 : 0)` in the other-lane W
handles the specific corner at a slice boundary where `prev` would
otherwise fall outside the reference set.

### Step 7 — Compression function G

Given two 1024-byte blocks `X` and `Y`, the compression function
produces a 1024-byte block.  Argon2 views each 1024-byte block as an
**8 × 8 matrix of 128-bit registers** — equivalently, a flat array
of 128 × `u64` words where row `r`, register-column `c` occupies the
word pair `(r·16 + 2c, r·16 + 2c + 1)`.  Both a "row" (8 registers)
and a "column" (8 registers) therefore carry exactly 16 × `u64`
words, which is what the permutation `P` consumes.

```
G(X, Y):
    R = X XOR Y                                       # 1024 bytes
    Q = copy(R)                                        # Q will become the permuted matrix

    # Row pass — apply P to each of the 8 rows in place.
    for i in 0..7:
        Q[i*16 .. i*16+16] = P( Q[i*16 .. i*16+16] )

    # Column pass — apply P to each of the 8 columns in place.
    # Column c (0..7) gathers word pairs (2c, 2c+1) from every row.
    for c in 0..7:
        col = [
            Q[0*16 + 2c], Q[0*16 + 2c + 1],
            Q[1*16 + 2c], Q[1*16 + 2c + 1],
            Q[2*16 + 2c], Q[2*16 + 2c + 1],
            Q[3*16 + 2c], Q[3*16 + 2c + 1],
            Q[4*16 + 2c], Q[4*16 + 2c + 1],
            Q[5*16 + 2c], Q[5*16 + 2c + 1],
            Q[6*16 + 2c], Q[6*16 + 2c + 1],
            Q[7*16 + 2c], Q[7*16 + 2c + 1],
        ]
        col = P(col)
        # Scatter back into Q at the same positions.
        for r in 0..7:
            Q[r*16 + 2c]     = col[2r]
            Q[r*16 + 2c + 1] = col[2r + 1]

    return R XOR Q                                     # feed-forward
```

So G does three things: XOR the operands, apply the BLAKE2b-style
permutation P to each 128-byte row, apply P to each 128-byte column
(gathered as word pairs), then XOR back the original XOR'd operand.

**Feed-forward.**  The final `R XOR Q` mirrors BLAKE2b's Davies-Meyer
construction — without it, G would be invertible given its output and
one operand, breaking the memory-hardness argument.

### Step 8 — Permutation P

P is the BLAKE2b round function applied to 16 × `u64` words, with the
message schedule set to the **identity** (no SIGMA — each round
consumes m[0..15] directly from the input).  Only one round is
applied per call to P (not 12 as in BLAKE2b proper).

```
P(v[0..15]):
    # Column step (4 parallel G_B calls):
    G_B(v, 0, 4,  8, 12)
    G_B(v, 1, 5,  9, 13)
    G_B(v, 2, 6, 10, 14)
    G_B(v, 3, 7, 11, 15)
    # Diagonal step (4 parallel G_B calls):
    G_B(v, 0, 5, 10, 15)
    G_B(v, 1, 6, 11, 12)
    G_B(v, 2, 7,  8, 13)
    G_B(v, 3, 4,  9, 14)
```

`G_B` is the BLAKE2b "big G" with the multiplications folded in:

```
G_B(v, a, b, c, d):
    v[a] = v[a] + v[b] + 2 * trunc32(v[a]) * trunc32(v[b])
    v[d] = rotr64(v[d] XOR v[a], 32)
    v[c] = v[c] + v[d] + 2 * trunc32(v[c]) * trunc32(v[d])
    v[b] = rotr64(v[b] XOR v[c], 24)
    v[a] = v[a] + v[b] + 2 * trunc32(v[a]) * trunc32(v[b])
    v[d] = rotr64(v[d] XOR v[a], 16)
    v[c] = v[c] + v[d] + 2 * trunc32(v[c]) * trunc32(v[d])
    v[b] = rotr64(v[b] XOR v[c], 63)
```

`trunc32(x)` = `x AND 0xFFFFFFFF` (low 32 bits, then zero-extended to
`u64`).  The `2 * trunc32(a) * trunc32(b)` term is the Argon2-specific
addition to BLAKE2b's round (it replaces the XOR-of-message-words),
and it provides the non-linear mixing that makes G invertible only
with knowledge of both operands.

All arithmetic is `u64` with wraparound.  The multiplication
`trunc32(a) * trunc32(b)` fits in `u64` without overflow because both
factors are ≤ 2^32 − 1.

### Step 9 — Finalize

After t passes complete:

```
B_final = B[0][q − 1] XOR B[1][q − 1] XOR ... XOR B[p − 1][q − 1]
Tag     = H'(T, serialize_le64(B_final))    # T bytes
```

`serialize_le64` writes each of the 128 `u64` words in little-endian,
producing the 1024 bytes fed to `H'`.

## Interface contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `argon2d`  | `(password, salt, t, m, p, tag_length, *, key=b"", associated_data=b"", version=0x13) -> bytes` | Data-dependent variant. |
| `argon2i`  | same | Data-independent variant. |
| `argon2id` | same | Hybrid (recommended). |
| `argon2d_hex` / `argon2i_hex` / `argon2id_hex` | same | Lowercase hex tag. |

All three functions share the same parameter validation.  Invalid
parameters raise / return an error before any allocation.

**Streaming:** Argon2 is **not** a streaming hash — the password and
salt are one-shot inputs to the initial H0 call.  There is no
`Hasher` struct.

**Recommended parameters** (OWASP 2024):

- **Interactive login:** `t=2, m=19456, p=1` (19 MiB).
- **Sensitive storage:** `t=2, m=65536, p=1` (64 MiB).
- **Maximum-security offline:** `t=3, m=1048576, p=4` (1 GiB).

Servers MUST pick parameters based on their own hardware and target
verification latency; the numbers above are floors, not recommendations
that fit every deployment.

## Test vectors (RFC 9106 §5)

All three variants use:

```
P  = 0x01 × 32
S  = 0x02 × 16
K  = 0x03 × 8           (secret key)
X  = 0x04 × 12          (associated data)
m  = 32, t = 3, p = 4, T = 32, v = 0x13
```

Expected tags:

```
Argon2d  = 512b391b6f1162975083e271b3d8011b312d18100fbb0871e2f91ed18f0d23ce
Argon2i  = c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8
Argon2id = 0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659
```

Implementations MUST additionally verify the three **empty-K,
empty-X** variants (password and salt as above, but no key / no AD)
against values cross-computed from the PHC reference implementation
at <https://github.com/P-H-C/phc-winner-argon2>.  These supplementary
vectors live alongside the language test suites and are mirrored
across all ports.

### Parameter-edge vectors

To catch boundary bugs, every port MUST also verify:

- `p = 1, m = 8, t = 1` — smallest legal memory.
- `p = 4, m = 32, t = 1` — multi-lane with `SL = 2` (tight segments).
- `T = 4`, `T = 64`, `T = 128` — short, at-the-fold, and long-output
  exercising both branches of `H'`.
- `len(K) = 0`, `len(X) = 0` — zero-length optional fields.
- `len(S) = 8` — minimum salt.

## Edge cases

- **`T = 64`.**  `H'` takes the `T ≤ 64` branch; no 32-byte overlap.
- **`T` such that `T − 32·(⌈T / 32⌉ − 2) = 64`.**  The final `V_{r+1}`
  reduces to a full BLAKE2b-512.  Do not special-case; the general
  formula handles it.
- **`p = 1`.**  There is a single lane, and all cross-lane references
  in Step 4 become same-lane references.
- **`m` not a multiple of `4·p`.**  Round down before allocating.
  Memory allocation MUST use `m'`, not the user's `m`.
- **Pass 1 / slice 0 / lane-local reference.**  `W = z − 1`; `z − 1`
  is the very-previous block and MUST NOT be in the reference set.
  The `index_alpha` formula handles this implicitly because the
  window excludes the block at position `W` itself; however,
  implementations often off-by-one here.  Explicitly verify against
  the PHC reference on `m = 32, p = 4, t = 1`.

## Security properties

| Attack                                       | Argon2d | Argon2i | Argon2id |
|----------------------------------------------|---------|---------|----------|
| Time-memory tradeoff (Alwen–Blocki)          | Strong  | Weak at low t | Strong for t ≥ 2 |
| Side-channel on memory access                | Broken  | Resistant | Near-resistant (first half of pass 0 is TRNG-like) |
| GPU/ASIC speedup                             | ≈ 2×    | ≈ 4×      | ≈ 2.5× |

**Parameter choice drives security, not the variant.**  Argon2id
with t=1, m=8 is a worse hash than PBKDF2.  A secure deployment
picks m ≥ 2^16 KiB and t ≥ 2.

## Language-specific notes

- **TypeScript** — `BigInt` for all `u64` arithmetic.  The
  `trunc32(a)*trunc32(b)` term requires care: JavaScript `Number`
  loses precision above 2^53, so even the "low 32" product must run
  in `BigInt`.  Cast early and often.
- **Lua** — Lua 5.3+ signed integers wrap on overflow; `trunc32` is
  `x & 0xFFFFFFFF`.  The multiplication fits in `i64` without saturation
  because both operands are ≤ 2^32 − 1.
- **Perl** — `use integer` for signed-64-bit wrap.  The low-32
  multiplication still fits; `unpack("Q<16", $block)` is the
  idiomatic 16-word LE parse.
- **Go / Rust / Swift / Haskell** — native `uint64` / `u64` /
  `UInt64` / `Word64`; use wrapping addition (`&+` in Swift,
  `wrapping_add` in Rust) everywhere.
- **Ruby** — Integer is arbitrary-precision; mask every intermediate
  with `& 0xFFFFFFFFFFFFFFFF` to match `u64` semantics.
- **Elixir** — `Bitwise` plus masking.  Beware: `Integer` is also
  arbitrary-precision; perform `band` with the 64-bit mask at every
  ARX step.

Memory allocation: all ports MUST allocate `m' · 1024` bytes once
(or a lane-by-lane equivalent) and reuse the buffer across passes.
Repeated per-pass allocation is both slow and masks bugs where the
XOR-in-place of pass ≥ 1 interacts with stale data.

## Dependencies

- **HF06 (BLAKE2b)** — required for H0 and H'.  Implementations
  depend on the `blake2b` package in their language; they MUST NOT
  re-implement BLAKE2b inline.
- The BLAKE2b round function is reused for permutation P inside the
  compression function `G`, but with an Argon2-specific twist: the
  integer multiplication term.  This means P cannot be imported from
  the `blake2b` package verbatim — every port inlines the modified
  round.

## Package matrix

Three variant packages per language, 10 languages, = **30 packages**:

```
packages/{python,typescript,go,rust,ruby,elixir,lua,perl,swift,haskell}/
    argon2d/
    argon2i/
    argon2id/
```

Each package exposes a single function pair (`argon2*` and
`argon2*_hex`) plus parameter validation.  No streaming `Hasher`.

Shared KAT table (JSON) lives at `code/specs/kat/argon2.json` and is
mirrored across every port's test suite.  Every language MUST
verify the same byte-precise vectors — this is what keeps all 30
packages in lockstep.

## Non-goals

- **Argon2 v1.0** (version byte 0x10) — superseded; no useful
  deployment still uses it.  Only v1.3 (0x13) is implemented.
- **PHC string format** (`$argon2id$v=19$m=19456,t=2,p=1$...$...`) —
  this is a serialization concern; the raw-tag API comes first.  A
  separate `phc-format` package can layer on top later.
- **Custom BLAKE2b variants** — Argon2's bespoke round (with the
  multiplication term) stays inside the Argon2 packages; HF06's
  BLAKE2b packages remain pure RFC 7693.
- **Parallel execution within a lane** — the RFC permits inter-lane
  parallelism (across slices) but requires intra-lane serialisation.
  The reference ports are single-threaded; parallelism is a
  future optimisation.
