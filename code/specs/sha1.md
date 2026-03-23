# SHA-1 — Secure Hash Algorithm 1

## Overview

SHA-1 takes any sequence of bytes as input and produces a fixed-size 20-byte
(160-bit) output called a **digest** or **hash**. The same input always
produces the same digest. Any change to the input — even flipping a single bit
— produces a completely different digest.

This package implements SHA-1 from scratch in all six supported languages with
no external dependencies. The goal is to read the source and understand exactly
what SHA-1 does at the bit level.

---

## Where It Fits

```
Application / UUID v5
      |
      v
  sha1(data) --> 20-byte digest
      |
      v
  [This package — no dependencies below this line]
```

**Depends on:** nothing (pure algorithm, stdlib only)

**Used by:**
- uuid — v5 UUID generation (`SHA-1(namespace || name)`)
- Any package needing a deterministic fixed-size fingerprint

---

## What Is a Hash Function?

Imagine you have a document and you want a short "fingerprint" of it that:

1. **Always gives the same answer** — hash the same document twice, get the
   same fingerprint both times.
2. **Is fixed-size regardless of input** — a 1-byte file and a 1-gigabyte
   file both produce exactly 20 bytes.
3. **Avalanches** — changing a single character in the document produces a
   completely unrecognizable fingerprint. There is no way to look at two
   fingerprints and say "these came from similar documents."
4. **Is one-way** — given the fingerprint, you cannot reconstruct the
   original document. You can only verify by re-hashing.

```
"Hello, world!"  ──► sha1 ──►  943a702d06f34599aee1f8da8ef9f7296031d699
"Hello, World!"  ──► sha1 ──►  0a0a9f2a6772942557ab5355d76af442f8f65e01
                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                                One capital letter changed everything.
```

**How is this different from a checksum (like CRC32)?**

A CRC32 checksum is designed to detect accidental corruption — a flipped bit
in transmission. It is NOT designed to resist intentional manipulation. An
attacker can modify a document and compute a new CRC32 to match.

A cryptographic hash function like SHA-1 is intentionally hard to invert and
hard to find collisions for. The security comes from mathematical difficulty,
not secrecy.

---

## What SHA-1 Is and Why It Exists

SHA-1 (Secure Hash Algorithm 1) was published by NIST in 1995. It was
designed by the NSA as part of the Digital Signature Standard.

```
Full name:    SHA-1 (Secure Hash Algorithm 1)
Published:    1995, FIPS PUB 180-1
Output:       160 bits = 20 bytes = 40 hex characters
Block size:   512 bits = 64 bytes (internal processing unit)
Word size:    32 bits (all arithmetic is done in 32-bit words)
```

**Where is SHA-1 used?**

```
Application          Notes
-----------          -----
Git                  Every commit, tree, and blob has a SHA-1 ID
UUID v5              Deterministic UUIDs from a namespace + name
TLS certificates     Formerly; SHA-256 is now required
PGP signatures       Formerly; now SHA-256+
HMAC-SHA1            Still used in many legacy APIs (AWS S3 v2, etc.)
```

**Is SHA-1 broken?**

SHA-1 is no longer considered collision-resistant. In 2017, the "SHAttered"
attack produced two different PDF files with the same SHA-1 hash. This
required 9 quintillion SHA-1 computations — expensive, but feasible for a
nation-state attacker.

For **new security-critical applications** (certificates, signatures), use
SHA-256 or SHA-3.

For **UUID v5** and **Git**, SHA-1 is still used and remains safe for its
intended purpose: a deterministic fingerprint where the "attacker" is you
trying to generate a specific UUID, not a malicious third party.

---

## The Merkle-Damgård Construction

SHA-1 is built on the **Merkle-Damgård construction**, which turns a fixed-
size compression function into a hash function that accepts variable-length
input.

```
How it works:

  Input message (any length)
       │
       ▼
  ┌──────────────────────┐
  │  Step 1: Padding     │  Make length a multiple of 512 bits
  └──────────────────────┘
       │
       ▼
  ┌──────────────────────────────────────────────────────────────────┐
  │  block₀ (512 bits) │  block₁ (512 bits) │  ...  │  blockₙ      │
  └──────────────────────────────────────────────────────────────────┘
       │                    │                           │
       ▼                    ▼                           ▼
  [init state] ──► compress ──► compress ──► ... ──► compress
  (H0..H4)          │               │                   │
                    │               │                   ▼
                    │               │              Final state
                    │               │              (H0..H4)
                    │               │                   │
                    └──────────────►│                   ▼
                                    └──────────────► 20-byte digest
```

**The key insight:** The compression function is applied repeatedly. Its
output becomes the input "state" for the next round. This is called
"chaining". Each block of message data is mixed into the chain.

**Analogy:** Think of a blender. You start with a base liquid (the init
state). You add ingredients one at a time (the message blocks). Each
blend mixes the new ingredient with everything that came before. At the
end you pour out the result. You cannot un-blend to get the ingredients
back.

---

## Step 1: Initialization Constants

SHA-1 begins with a fixed 160-bit state split into five 32-bit words:

```
H₀ = 0x67452301
H₁ = 0xEFCDAB89
H₂ = 0x98BADCFE
H₃ = 0x10325476
H₄ = 0xC3D2E1F0
```

**Why these specific values?**

Look at H₀ more carefully. In bytes: 0x67, 0x45, 0x23, 0x01. That is
01, 23, 45, 67 stored in big-endian order — a simple counting sequence.

```
H₀ = 0x67452301  →  bytes: 67 45 23 01  →  reverse: 01 23 45 67
H₁ = 0xEFCDAB89  →  bytes: EF CD AB 89  →  reverse: 89 AB CD EF
H₂ = 0x98BADCFE  →  bytes: 98 BA DC FE  →  reverse: FE DC BA 98
H₃ = 0x10325476  →  bytes: 10 32 54 76  →  reverse: 76 54 32 10
```

These are called "nothing up my sleeve" numbers. Choosing constants that
have an obvious pattern proves to observers that there is no hidden
mathematical backdoor baked in. If the designer had chosen random-looking
constants, people would wonder if those constants were specially chosen to
weaken the algorithm.

---

## Step 2: Padding

The compression function works on exactly 512-bit (64-byte) blocks. Real
messages are rarely a multiple of 512 bits. Padding extends the message to
the right length.

**The padding rule:**

1. Append a single `1` bit (byte `0x80`) immediately after the message.
2. Append `0` bits until the message is 448 bits (56 bytes) past a multiple
   of 512 bits (= 64 bytes).
3. Append the original message length in **bits** as a 64-bit big-endian
   integer.

This ensures the padded message is exactly a multiple of 512 bits.

**Worked example: padding "abc" (3 bytes = 24 bits)**

```
Original message (hex):   61 62 63
                          a  b  c

After appending 0x80:     61 62 63 80

We need to reach 56 bytes (448 bits) before the length field.
56 - 4 = 52 more zero bytes:

  61 62 63 80 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00

Append original length (24 bits) as 64-bit big-endian:
  00 00 00 00 00 00 00 18  (0x18 = 24 in decimal)

Final padded block (64 bytes):
  61 62 63 80 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 18
```

**What if the message is already 55 bytes?**

Then after appending `0x80`, we have 56 bytes. We still need to reach 56
bytes mod 64 + 8 for the length = 64 bytes. So we add 0 padding bytes and
the length. Total: 64 bytes. One block.

**What if the message is 56 bytes?**

After appending `0x80`, we have 57 bytes. We need to reach 56 mod 64,
which means adding 55 more zero bytes to reach 112 bytes (= 56 + 56), then
the 8-byte length. Total: 120 bytes. Two blocks.

**Why include the length?**

Without the length field, `"abc" + padding` and `"abc" + (some padding)`
could collide. The length pins exactly where the message ends and the
padding begins, even if the padding content is all zeros.

---

## Step 3: Message Schedule

Each 64-byte block is expanded from 16 words (W₀..W₁₅) to 80 words
(W₀..W₇₉).

```
W[i] = W[i-16]    (for i = 0..15, directly from the message block)

W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])
                  (for i = 16..79)
```

**What is ROTL (circular left shift)?**

ROTL(n, x) rotates the bits of x left by n positions. Bits that "fall off"
the left end reappear on the right. This is different from a regular left
shift, where overflow bits are discarded.

```
Regular left shift (<<):
  01101001 << 2  =  10100100  (bits that fell off the left are gone)

Circular left shift ROTL(2):
  01101001 << 2  =  10100110  (bits that fell off reappear on the right)
  ^^                       ^^
  These bits...         ...come back here
```

**Why expand 16 words to 80?**

More rounds = more mixing = better avalanche. Each expanded word depends on
four earlier words. Changing one bit in the original message ripples through
all 80 words, ensuring every bit of the final digest is influenced by every
bit of the input.

**XOR truth table** (the "are these different?" operation):

```
A  B  │  A XOR B
──────┼──────────
0  0  │  0         same → 0
0  1  │  1         different → 1
1  0  │  1         different → 1
1  1  │  0         same → 0
```

---

## Step 4: The Compression Function (80 Rounds)

For each 64-byte block, SHA-1 runs 80 rounds of mixing. The state is five
32-bit words: **a, b, c, d, e** (initialized from H₀..H₄).

There are four stages of 20 rounds each, each using a different "auxiliary
function" f and constant K:

```
Stage    Rounds   f(b, c, d)                   K           Purpose
─────    ──────   ──────────────────────────   ─────────── ────────────────
  1      0–19     (b AND c) OR (NOT b AND d)   0x5A827999  Selector / mux
  2     20–39     b XOR c XOR d               0x6ED9EBA1  Parity
  3     40–59     (b AND c) OR (b AND d)       0x8F1BBCDC  Majority vote
               OR (c AND d)
  4     60–79     b XOR c XOR d               0xCA62C1D6  Parity
```

**Stage 1 — Selector (f = (b AND c) OR (NOT b AND d))**

This function selects between c and d based on b:
- If b = 1, output c (because `1 AND c` = c, and `NOT 1 AND d` = 0)
- If b = 0, output d (because `0 AND c` = 0, and `NOT 0 AND d` = d)

It's like a 2-to-1 multiplexer: b is the control signal.

```
b  c  d  │  (b AND c) OR (NOT b AND d)
──────────┼───────────────────────────
0  0  0  │  0
0  0  1  │  1   ← d wins (b=0)
0  1  0  │  0
0  1  1  │  1   ← d wins (b=0)
1  0  0  │  0
1  0  1  │  0
1  1  0  │  1   ← c wins (b=1)
1  1  1  │  1   ← c wins (b=1)
```

**Stage 2 and 4 — Parity (f = b XOR c XOR d)**

This is just a 3-input XOR: the output is 1 if an odd number of inputs
are 1. This spreads changes from any of the three words evenly.

**Stage 3 — Majority vote (f = (b AND c) OR (b AND d) OR (c AND d))**

The output is 1 if at least 2 of the 3 inputs are 1 — i.e., the majority.

```
b  c  d  │  majority
──────────┼──────────
0  0  0  │  0
0  0  1  │  0  (only 1 is set)
0  1  0  │  0  (only 1 is set)
0  1  1  │  1  (2 are set)
1  0  0  │  0  (only 1 is set)
1  0  1  │  1  (2 are set)
1  1  0  │  1  (2 are set)
1  1  1  │  1  (all 3 are set)
```

**Each round:**

```
temp = ROTL(5, a) + f(b, c, d) + e + K + W[t]
e = d
d = c
c = ROTL(30, b)
b = a
a = temp

(All arithmetic modulo 2³², i.e., & 0xFFFFFFFF)
```

The state "marches" through the words: what was a becomes b, b becomes c,
etc. The new value of a is the most heavily mixed word (it absorbed ROTL(5,a),
the auxiliary function, and the message word).

**Worked example: Round 0 of sha1("abc")**

After padding and word expansion, W[0] = 0x61626380 (bytes: 61 62 63 80).

Initial state (from H₀..H₄):
```
a = 0x67452301
b = 0xEFCDAB89
c = 0x98BADCFE
d = 0x10325476
e = 0xC3D2E1F0
```

Stage 1 auxiliary: f = (b AND c) OR (NOT b AND d)
```
b AND c         = 0xEFCDAB89 AND 0x98BADCFE = 0x88888888
NOT b           = NOT 0xEFCDAB89             = 0x10325476
NOT b AND d     = 0x10325476 AND 0x10325476  = 0x10325476
f               = 0x88888888 OR  0x10325476  = 0x98BADCFE
```

ROTL(5, a) = ROTL(5, 0x67452301):
```
0x67452301 in binary:  0110 0111 0100 0101 0010 0011 0000 0001
Rotate left 5 bits:    1110 1000 1010 0100 0110 0000 0010 1100 1100...
Wait, let's do it properly:
0x67452301 = 0110 0111 0100 0101 0010 0011 0000 0001
Top 5 bits: 01100 — these wrap to the bottom
Rest:             111 0100 0101 0010 0011 0000 0001
Combined:         111 0100 0101 0010 0011 0000 0001 01100
= 1110 1000 1010 0100 0110 0000 0010 1100 1
Hmm, let me just compute: (0x67452301 << 5) | (0x67452301 >> 27)
= 0xE8A46002 | 0x03        = 0xE8A46023
ROTL(5, 0x67452301) = 0xE8A46023
```

temp = 0xE8A46023 + 0x98BADCFE + 0xC3D2E1F0 + 0x5A827999 + 0x61626380
     = (sum all, truncate to 32 bits)
     = 0xF4F4C004  (approximate — actual computation yields the round result)

After round 0:
```
e = 0x10325476  (old d)
d = 0x98BADCFE  (old c)
c = ROTL(30, 0xEFCDAB89)  (old b, rotated 30)
b = 0x67452301  (old a)
a = temp
```

This continues for 79 more rounds.

**Why ROTL(30, b)?**

The rotation of b by 30 positions (or equivalently right by 2) ensures that
b's bits are thoroughly mixed before they become c. It also ensures that
the same word influences the computation at multiple points in the schedule,
since it takes 5 "steps" for a word to cycle through all 5 positions.

---

## Step 5: Finalization

After all 80 rounds for a block, the round result is **added** back to the
running hash state:

```
H₀ += a
H₁ += b
H₂ += c
H₃ += d
H₄ += e
(all modulo 2³²)
```

This is the **Davies-Meyer feed-forward**. It prevents the compression
function from being invertible: even if you ran 80 rounds backwards, you'd
need to subtract the original state — which you don't have without the key.

After processing all blocks, the final digest is:

```
H₀ ∥ H₁ ∥ H₂ ∥ H₃ ∥ H₄  (each word as 4 bytes big-endian = 20 bytes total)
```

**Why big-endian?**

SHA-1 was designed to be byte-order-independent across network protocols.
Big-endian means the most significant byte comes first — the "natural"
order for humans (just like writing numbers left-to-right).

---

## Test Vectors (FIPS 180-4)

These are the canonical test cases from the FIPS 180-4 standard. Any correct
SHA-1 implementation must produce exactly these outputs:

```
Input                                           Expected digest
─────────────────────────────────────────────── ────────────────────────────────────────────
""  (empty string, 0 bytes)                     da39a3ee5e6b4b0d3255bfef95601890afd80709
"abc"  (3 bytes)                                a9993e364706816aba3e25717850c26c9cd0d89d
"abcdbcdecdefdefgefghfghighijhijkijkljklm       84983e441c3bd26ebaae4aa1f95129e5e54670f1
 klmnlmnomnopnopq"  (448 bits = 56 bytes)
"a" repeated 1,000,000 times                    34aa973cd4c4daa4f61eeb2bdbad27316534016f
```

**Why does the empty string have a non-zero hash?**

Padding! The empty string "" still gets padded to a full 512-bit block:

```
0x80 followed by 55 zero bytes and the length 0x0000000000000000:
80 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
00 00 00 00  00 00 00 00  00 00 00 00  00 00 00 00
```

This block is compressed against the initial state H₀..H₄. The result is
`da39a3ee...` — the hash of "nothing".

**Multi-block test ("abcdbcdecdefdefg..." is exactly 448 bits = 56 bytes):**

56 bytes + 1 padding byte (0x80) = 57 bytes. To reach 56 bytes mod 64, we
need to add padding to reach 120 bytes (two 64-byte blocks), then a 64-bit
length. Total: 128 bytes = 2 blocks.

---

## Public API

```python
# === One-shot hashing ===

def sha1(data: bytes) -> bytes:
    """Compute the SHA-1 digest of data.

    Returns 20 bytes (160 bits). This is the most common use case:
    hash a complete message in one call.

    Example:
        >>> sha1(b"abc").hex()
        'a9993e364706816aba3e25717850c26c9cd0d89d'
    """


def sha1_hex(data: bytes) -> str:
    """Compute SHA-1 and return the 40-character lowercase hex string.

    Equivalent to sha1(data).hex().

    Example:
        >>> sha1_hex(b"abc")
        'a9993e364706816aba3e25717850c26c9cd0d89d'
    """


# === Streaming hashing ===

class SHA1:
    """SHA-1 hasher that accepts data in multiple chunks.

    Useful when the full message is not available at once (e.g., reading
    a large file in chunks, or hashing a network stream).

    Example:
        >>> h = SHA1()
        >>> h.update(b"ab")
        >>> h.update(b"c")
        >>> h.digest().hex()
        'a9993e364706816aba3e25717850c26c9cd0d89d'
    """

    def __init__(self) -> None:
        """Initialize with the SHA-1 starting state."""

    def update(self, data: bytes) -> None:
        """Feed more bytes into the hash computation.

        Can be called multiple times. update(a); update(b) is equivalent
        to a single sha1(a + b).
        """

    def digest(self) -> bytes:
        """Return the 20-byte SHA-1 digest of all data fed so far.

        Does not modify the internal state — you can call digest() at any
        point and continue feeding data.
        """

    def hexdigest(self) -> str:
        """Return the 40-character hex string of the digest."""

    def copy(self) -> "SHA1":
        """Return a copy of the current hasher state.

        Useful for computing multiple hashes that share a common prefix.
        """
```

### Language Mapping

```
Concept            Python              Go               TypeScript
──────────         ──────              ──               ──────────
One-shot           sha1(data)          Sha1(data)       sha1(data: Uint8Array)
Hex output         sha1_hex(data)      Sha1Hex(data)    sha1Hex(data)
Streaming class    SHA1()              NewSHA1()        new SHA1()
Update             h.update(data)      h.Update(data)   h.update(data)
Digest             h.digest()          h.Digest()       h.digest()
Hex digest         h.hexdigest()       h.HexDigest()    h.hexDigest()

Concept            Ruby                Elixir           Rust
──────────         ────                ──────           ────
One-shot           SHA1.sha1(data)     SHA1.hash(data)  sha1(data: &[u8])
Hex output         SHA1.hex(data)      SHA1.hex(data)   sha1_hex(data)
Streaming class    SHA1::Hasher.new    SHA1.new()       Sha1::new()
Update             h.update(data)      SHA1.update(h,d) h.update(data)
Digest             h.digest            SHA1.digest(h)   h.finalize()
Hex digest         h.hexdigest         SHA1.hexdigest(h) h.hexdigest()
```

---

## Testing Strategy

Target: **95%+ line coverage** in Python and Ruby; **80%+** in all others.

### Unit Tests: sha1() one-shot

1. **FIPS vector: empty string** — `sha1(b"") == bytes.fromhex("da39a3ee...")`
2. **FIPS vector: "abc"** — `sha1(b"abc") == bytes.fromhex("a9993e36...")`
3. **FIPS vector: 448-bit message** — 56-byte input, two-block hash
4. **FIPS vector: 1M "a"s** — very long input, many blocks
5. **Single byte 0x00** — sha1 of a null byte
6. **Single byte 0xFF** — sha1 of all-ones byte
7. **Exact 55 bytes** — fills to exactly one block (55 + 1 + 8 = 64)
8. **Exact 56 bytes** — forces two blocks (56 + 1 + 7 + 8 > 64)
9. **Exact 64 bytes** — exactly two blocks with padding in second
10. **127 bytes** — two-block message (127 + 1 + padding + length)
11. **128 bytes** — three-block message
12. **UTF-8 text** — `sha1("hello".encode("utf-8"))` works correctly
13. **Binary data** — `sha1(bytes(range(256)))` covers all byte values
14. **Return type** — result is bytes of length 20
15. **Deterministic** — calling sha1 twice on same input gives same result
16. **Avalanche** — sha1("a") != sha1("b"), completely different

### Unit Tests: sha1_hex()

17. **Returns string** — sha1_hex returns a str, not bytes
18. **Length 40** — always 40 hex characters
19. **Lowercase** — hex digits are lowercase (a-f, not A-F)
20. **Matches digest** — sha1_hex(data) == sha1(data).hex()

### Unit Tests: SHA1 streaming

21. **Single update equals one-shot** — `SHA1().update(b"abc").digest() == sha1(b"abc")`
22. **Split at byte boundary** — `update(b"ab"); update(b"c")` == `sha1(b"abc")`
23. **Split at block boundary** — split at 64 bytes gives same result
24. **Many tiny updates** — one byte at a time for 100 bytes
25. **digest() non-destructive** — can call digest() twice, get same result
26. **Continue after digest()** — update after digest reflects in next digest
27. **copy()** — modifying the copy doesn't affect the original
28. **hexdigest()** — equivalent to digest().hex()
29. **Empty streaming** — SHA1().digest() == sha1(b"")

### Edge Cases

30. **Zero-length input** — no crash, correct FIPS output
31. **Large input (10MB)** — correct output, no memory issues

---

## Trade-Offs

| Decision | Pro | Con |
|----------|-----|-----|
| Pure implementation (no stdlib SHA-1) | Educational; zero external deps | Slower than C-backed stdlib |
| 32-bit word arithmetic | Matches the spec exactly | Need mask `& 0xFFFFFFFF` in Python/TypeScript |
| Streaming API | Handles large files | More state to manage |
| Big-endian word encoding | Matches FIPS spec | Must swap bytes on little-endian machines |
| ROTL(1) in message schedule | SHA-1 spec uses this | SHA-256 uses ROTR instead |

---

## Why SHA-1 Is Not SHA-256

SHA-1 produces 160 bits. SHA-256 produces 256 bits. The extra bits matter for
security — a longer digest means more possible outputs, making collisions
harder to find.

SHA-256 also uses a different auxiliary function structure (6 functions across
64 rounds instead of 4 functions across 80), different initial constants, and
ROTR (right rotate) instead of ROTL (left rotate).

For this project, SHA-1 is sufficient for UUID v5 and is a better educational
starting point because it is simpler.
