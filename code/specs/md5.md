# MD5 — Message Digest 5

## Overview

MD5 takes any sequence of bytes as input and produces a fixed-size 16-byte
(128-bit) digest. Like SHA-1, it is deterministic, fixed-size, and one-way.
Unlike SHA-1, MD5 is **cryptographically broken** — but it remains widely
used for non-security purposes: checksums, UUID v3, and legacy systems.

This package implements MD5 from scratch in all six supported languages with
no external dependencies.

---

## Where It Fits

```
Application / UUID v3
      |
      v
  md5(data) --> 16-byte digest
      |
      v
  [This package — no dependencies below this line]
```

**Depends on:** nothing (pure algorithm, stdlib only)

**Used by:**
- uuid — v3 UUID generation (`MD5(namespace || name)`)
- Any package needing a checksums or legacy-compatible digests

---

## The MD (Message Digest) Family

MD5 is the fifth in a series of hash functions designed by Ron Rivest at MIT:

```
Name   Year   Output     Status
────   ────   ──────     ──────
MD2    1989   128 bits   Completely broken, deprecated
MD4    1990   128 bits   Broken (practical collision attacks)
MD5    1991   128 bits   Broken (chosen-prefix collisions in hours)
MD6    2008   variable   Submitted to SHA-3 competition, withdrawn
```

The MD in "MD5" stands for **Message Digest** — exactly what it produces: a
digest (summary) of a message.

**Why is this history interesting?**

Each version was broken faster than the previous one. MD4 was broken by Bert
den Boer in 1991 — the same year Rivest published MD5. MD5 was designed to
fix MD4's weaknesses. The attacks on MD5 found by Wang et al. in 2004 were
more sophisticated than MD4's breakage, requiring clever analysis of how bit
differences propagate through the compression function.

This arms race between hash function designers and cryptanalysts is a core
part of the history of modern cryptography.

---

## How MD5 Differs from SHA-1

Both MD5 and SHA-1 use the Merkle-Damgård construction (see sha1.md for a
detailed explanation of that structure). The key differences:

```
Property           MD5                     SHA-1
────────           ───                     ─────
Output size        128 bits (16 bytes)     160 bits (20 bytes)
Output in hex      32 characters           40 characters
State words        4 × 32-bit (A,B,C,D)   5 × 32-bit (H₀..H₄)
Block size         512 bits (64 bytes)     512 bits (64 bytes)
Rounds             64 (4 stages × 16)      80 (4 stages × 20)
Word endianness    LITTLE-endian           Big-endian
Length field       LITTLE-endian 64-bit    Big-endian 64-bit
Shift amounts      Variable per round      Constant per stage
Security           Broken (since 2004)     Weakened (since 2017)
```

**The most confusing difference: little-endian output.**

SHA-1 stores everything in big-endian (most significant byte first, like
ordinary number notation). MD5 stores its state words in little-endian (least
significant byte first, like how x86 CPUs store integers).

This means the bytes of each state word appear in reversed order in the output:

```
State word A = 0x01234567

Big-endian (SHA-1 style):
  Memory: 01 23 45 67   ← natural left-to-right order

Little-endian (MD5 style):
  Memory: 67 45 23 01   ← bytes reversed
```

This catches nearly every first-time implementer. The algorithm runs
correctly, but the final output looks completely wrong.

---

## Initialization Constants

MD5 starts with the same "nothing up my sleeve" constants as SHA-1:

```
A = 0x67452301
B = 0xEFCDAB89
C = 0x98BADCFE
D = 0x10325476
```

The state is the same four words as SHA-1's H₀..H₃ — no coincidence; both
were influenced by the same design philosophy from the early 1990s.

---

## Step 1: Padding

MD5 padding follows the same Merkle-Damgård rule as SHA-1, with one crucial
difference: the length is appended in **little-endian** instead of big-endian.

```
Rule:
  1. Append byte 0x80
  2. Append zeros until message is 56 bytes mod 64
  3. Append original bit length as 64-bit LITTLE-endian integer
```

**Worked example: padding "abc" (3 bytes = 24 bits)**

```
Same padding bytes as SHA-1:
  61 62 63 80 00 00 00 00  (... 52 zero bytes ...)  [length field]

SHA-1 length field (big-endian):
  00 00 00 00 00 00 00 18

MD5 length field (little-endian):
  18 00 00 00 00 00 00 00   ← reversed!
```

**Why little-endian?**

MD5 was designed by Ron Rivest, who was optimizing for speed on the dominant
architecture of 1991: the Intel x86. x86 is a little-endian architecture.
By treating the message as an array of little-endian 32-bit words, MD5 could
run without any byte-swapping on x86 — a significant speed advantage at the
time.

SHA-1 was designed at the NSA for use in cryptographic protocols, where
big-endian (network byte order) was standard. Different design goals, different
choices.

---

## The T Table (64 Sine-Based Constants)

MD5 uses 64 precomputed constants, one per round:

```
T[i] = floor(abs(sin(i + 1)) × 2³²)  for i = 0, 1, ..., 63
```

**Why the sine function?**

Sine is a transcendental function — its output has no algebraic relationship
to its input. The bits of `sin(1)`, `sin(2)`, etc. are essentially random
from the perspective of anyone trying to find a pattern.

This makes T[i] "nothing up my sleeve" constants: if Rivest had published
arbitrary hex values, people would worry he chose them to introduce a backdoor.
By deriving them from a well-known mathematical function, the design is
transparent.

**Computing T[1]:**
```
sin(1 radian) ≈ 0.8414709848078965
abs(0.8414709848078965) = 0.8414709848078965
× 2³² = 0.8414709848078965 × 4294967296 ≈ 3614090360.0
floor(3614090360.0) = 3614090360 = 0xD76AA478
```

So T[1] = 0xD76AA478. You can verify this matches RFC 1321 Appendix A.

**All 64 T values (for implementation reference):**

```
Round  T[i]        Round  T[i]        Round  T[i]        Round  T[i]
─────  ─────────   ─────  ─────────   ─────  ─────────   ─────  ─────────
  0    0xD76AA478    16   0xF61E2562    32   0xFFFA3942    48   0xF4292244
  1    0xE8C7B756    17   0xC040B340    33   0x8771F681    49   0x432AFF97
  2    0x242070DB    18   0x265E5A51    34   0x6D9D6122    50   0xAB9423A7
  3    0xC1BDCEEE    19   0xE9B6C7AA    35   0xFDE5380C    51   0xFC93A039
  4    0xF57C0FAF    20   0xD62F105D    36   0xA4BEEA44    52   0x655B59C3
  5    0x4787C62A    21   0x02441453    37   0x4BDECFA9    53   0x8F0CCC92
  6    0xA8304613    22   0xD8A1E681    38   0xF6BB4B60    54   0xFFEFF47D
  7    0xFD469501    23   0xE7D3FBC8    39   0xBEBFBC70    55   0x85845DD1
  8    0x698098D8    24   0x21E1CDE6    40   0x289B7EC6    56   0x6FA87E4F
  9    0x8B44F7AF    25   0xC33707D6    41   0xEAA127FA    57   0xFE2CE6E0
 10    0xFFFF5BB1    26   0xF4D50D87    42   0xD4EF3085    58   0xA3014314
 11    0x895CD7BE    27   0x455A14ED    43   0x04881D05    59   0x4E0811A1
 12    0x6B901122    28   0xA9E3E905    44   0xD9D4D039    60   0xF7537E82
 13    0xFD987193    29   0xFCEFA3F8    45   0xE6DB99E5    61   0xBD3AF235
 14    0xA679438E    30   0x676F02D9    46   0x1FA27CF8    62   0x2AD7D2BB
 15    0x49B40821    31   0x8D2A4C8A    47   0xC4AC5665    63   0xEB86D391
```

---

## Shift Amounts

Each of the 64 rounds also has a per-round left-rotation amount:

```
s = [7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  # round 1
     5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  # round 2
     4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  # round 3
     6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21]  # round 4
```

The varying shift amounts were chosen by Rivest to maximize diffusion —
different bits of the state word are rotated into different positions each
round, spreading input bits as widely as possible.

---

## The 64-Round Compression Function

For each 512-bit block, MD5 runs 64 rounds with four auxiliary functions:

```
Stage    Rounds    f(b, c, d)                   g (message word index)
─────    ──────    ──────────────────────────   ─────────────────────
  F      0–15     (b AND c) OR (NOT b AND d)   g = i
  G     16–31     (d AND b) OR (NOT d AND c)   g = (5i + 1) mod 16
  H     32–47     b XOR c XOR d               g = (3i + 5) mod 16
  I     48–63     c XOR (b OR NOT d)           g = (7i) mod 16
```

**F: Selector** (same idea as SHA-1 stage 1)

F(b, c, d) = (b AND c) OR (NOT b AND d)

If b=1, output c. If b=0, output d. b acts as a control bit.

**G: Reverse Selector**

G(b, c, d) = (d AND b) OR (NOT d AND c)

If d=1, output b. If d=0, output c. Same idea, different control bit.

**H: Parity** (same as SHA-1 stages 2 and 4)

H(b, c, d) = b XOR c XOR d

**I: Non-linear function**

I(b, c, d) = c XOR (b OR NOT d)

This is the most complex of the four. NOT d flips all bits of d. OR with b
gives 1 for most positions. XOR with c flips those bits where c is 1.

Truth table for I:
```
b  c  d  │  I(b,c,d) = c XOR (b OR NOT d)
──────────┼───────────────────────────────
0  0  0   │  NOT 0 = 1, 0 OR 1 = 1, 0 XOR 1 = 1
0  0  1   │  NOT 1 = 0, 0 OR 0 = 0, 0 XOR 0 = 0
0  1  0   │  NOT 0 = 1, 0 OR 1 = 1, 1 XOR 1 = 0
0  1  1   │  NOT 1 = 0, 0 OR 0 = 0, 1 XOR 0 = 1
1  0  0   │  NOT 0 = 1, 1 OR 1 = 1, 0 XOR 1 = 1
1  0  1   │  NOT 1 = 0, 1 OR 0 = 1, 0 XOR 1 = 1
1  1  0   │  NOT 0 = 1, 1 OR 1 = 1, 1 XOR 1 = 0
1  1  1   │  NOT 1 = 0, 1 OR 0 = 1, 1 XOR 1 = 0
```

**Each round:**

```
F = f(b, c, d)            -- auxiliary function for this stage
val = A + F + M[g] + T[i] -- add state word, aux, message word, constant
val = ROTL(s[i], val)     -- rotate by this round's shift amount
A, B, C, D = D, B + val, B, C   -- rotate state words, inject new B
```

Compare with SHA-1, where A was always the "hot" word. In MD5, B is the
primary output word of each round — the mixing result is added to the old B.

**The message word index g** changes every round:
- Stage F: g = i (words 0, 1, 2, ..., 15 in order)
- Stage G: g = (5i+1) mod 16 (words 1, 6, 11, 0, 5, 10, ...)
- Stage H: g = (3i+5) mod 16 (words 5, 8, 11, 14, 1, 4, ...)
- Stage I: g = 7i mod 16 (words 0, 7, 14, 5, 12, 3, 10, 1, ...)

This non-sequential access pattern ensures each message word influences the
state from multiple angles across the 64 rounds.

---

## Finalization

After all 64 rounds for a block:

```
A₀ = (A₀ + A) mod 2³²
B₀ = (B₀ + B) mod 2³²
C₀ = (C₀ + C) mod 2³²
D₀ = (D₀ + D) mod 2³²
```

The final digest is A₀ ∥ B₀ ∥ C₀ ∥ D₀, each word in **little-endian**:

```
State word 0xDEADBEEF in the final output:
  Little-endian bytes: EF BE AD DE
                       ^^ most of the mistakes happen right here
```

**Full example: md5("abc")**

The expected output is `900150983cd24fb0d6963f7d28e17f72`.

After computing all rounds and adding back the initial state, if you get
something different, check your little-endian word serialization first.

---

## Why MD5 Is Broken

In 2004, Xiaoyun Wang and Hongbo Yu published a paper showing how to find
two different messages M and M' such that MD5(M) == MD5(M'). This is called
a **collision**.

By 2008, Marc Stevens demonstrated **chosen-prefix collisions**: given two
arbitrary prefixes P₁ and P₂, attackers can find suffixes S₁ and S₂ such
that MD5(P₁ ∥ S₁) == MD5(P₂ ∥ S₂). This is far more dangerous because an
attacker can embed a meaningful document in P₁ and a malicious document in P₂.

**The Flame malware (2012)** exploited this to forge a Microsoft code-signing
certificate. It created two CSR documents with the same MD5 hash, got one
legitimately signed, and used the signature on the other.

**Why doesn't the round function prevent this?**

The MD5 round function, while non-linear, does not mix bits well enough.
Wang's attack exploits specific "differential paths" — carefully chosen
differences between two messages that cancel out through the compression
function. The four-word state (128 bits) combined with the relatively small
number of rounds provides insufficient "diffusion" to prevent this.

SHA-1 is also weakened (the SHAttered attack, 2017), but it required vastly
more computation than breaking MD5. SHA-256 and SHA-3 are currently
considered secure.

**For this project:** MD5 is used only for UUID v3, where collision resistance
is irrelevant. UUID v3 is about generating a stable identifier from a
(namespace, name) pair, not about security. The UUID standard explicitly
recommends v5 (SHA-1) over v3 for new systems.

---

## Test Vectors (RFC 1321 Appendix A)

```
Input                                              Expected digest
────────────────────────────────────────────────── ────────────────────────────────
""                                                 d41d8cd98f00b204e9800998ecf8427e
"a"                                                0cc175b9c0f1b6a831c399e269772661
"abc"                                              900150983cd24fb0d6963f7d28e17f72
"message digest"                                   f96b697d7cb7938d525a2f31aaf161d0
"abcdefghijklmnopqrstuvwxyz"                       c3fcd3d76192e4007dfb496cca67e13b
"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstu  d174ab98d277d9f5a5611c2c9f419d9f
 vwxyz0123456789"
"12345678901234567890123456789012345678901234567    57edf4a22be3c955ac49da2e2107b67a
 890123456789012345678901234567890"
```

---

## Public API

```python
# === One-shot hashing ===

def md5(data: bytes) -> bytes:
    """Compute the MD5 digest of data.

    Returns 16 bytes (128 bits). Suitable for checksums and UUID v3.
    NOT suitable for password hashing, digital signatures, or any
    security-critical application.

    Example:
        >>> md5(b"abc").hex()
        '900150983cd24fb0d6963f7d28e17f72'
    """


def md5_hex(data: bytes) -> str:
    """Compute MD5 and return the 32-character lowercase hex string.

    Equivalent to md5(data).hex().

    Example:
        >>> md5_hex(b"abc")
        '900150983cd24fb0d6963f7d28e17f72'
    """


# === Streaming hashing ===

class MD5:
    """MD5 hasher that accepts data in multiple chunks.

    Same interface as the SHA1 streaming hasher.

    Example:
        >>> h = MD5()
        >>> h.update(b"ab")
        >>> h.update(b"c")
        >>> h.digest().hex()
        '900150983cd24fb0d6963f7d28e17f72'
    """

    def __init__(self) -> None: ...
    def update(self, data: bytes) -> None: ...
    def digest(self) -> bytes: ...
    def hexdigest(self) -> str: ...
    def copy(self) -> "MD5": ...
```

Language mapping is identical to the SHA-1 API mapping — same method names,
same structure, just `md5` instead of `sha1` everywhere.

---

## Testing Strategy

Target: **95%+ line coverage** in Python and Ruby; **80%+** in all others.

### Unit Tests: md5()

1. **RFC 1321 vector: empty string** — `md5(b"") == bytes.fromhex("d41d8cd9...")`
2. **RFC 1321 vector: "a"**
3. **RFC 1321 vector: "abc"**
4. **RFC 1321 vector: "message digest"**
5. **RFC 1321 vector: lowercase alphabet**
6. **RFC 1321 vector: alphanumeric**
7. **RFC 1321 vector: 80-char string**
8. **Single zero byte** — md5(b"\x00")
9. **All-ones byte** — md5(b"\xff")
10. **Exact 55 bytes** — one block with padding
11. **Exact 56 bytes** — forces two blocks
12. **Exact 64 bytes** — two blocks with padding in second
13. **127 bytes** — two-block message
14. **128 bytes** — three-block message
15. **UTF-8 text** — works on encoded strings
16. **Binary data** — bytes(range(256))
17. **Return type** — bytes of length 16 (not 20!)
18. **Deterministic** — same input, same output
19. **Little-endian check** — md5(b"abc") first byte is 0x90, not 0x00
20. **Avalanche** — md5("a") != md5("b")

### Unit Tests: md5_hex()

21. **Returns string** — not bytes
22. **Length 32** — always 32 hex characters (not 40!)
23. **Lowercase** — hex digits are lowercase
24. **Matches digest** — md5_hex(data) == md5(data).hex()

### Unit Tests: MD5 streaming

25. **Single update equals one-shot**
26. **Split at byte boundary**
27. **Split at 64-byte block boundary**
28. **Many tiny updates** — one byte at a time
29. **digest() non-destructive**
30. **copy()** — copy is independent

---

## Trade-Offs

| Decision | Pro | Con |
|----------|-----|-----|
| Pure implementation | Educational; no deps | Slower than C-backed stdlib |
| Little-endian explicit | Matches spec exactly | Easy to get wrong |
| Include T table as constants | No runtime computation | 64 constant values in source |
| Non-sequential g indexing | Matches RFC 1321 exactly | Confusing to read at first |

---

## MD5 vs SHA-1 Cheat Sheet

```
                      MD5                     SHA-1
                      ───                     ─────
State words           A, B, C, D              H₀, H₁, H₂, H₃, H₄
Init values           0x67452301...           0x67452301...  (same!)
Block length          64 bytes                64 bytes       (same!)
Padding length        little-endian           big-endian     (different!)
Word parsing          little-endian           big-endian     (different!)
Output length         16 bytes                20 bytes       (different!)
Rounds                64                      80             (different!)
Round function        F/G/H/I                 Ch/Parity/Maj  (similar ideas)
Constants             T[i] from sin(i+1)      K (4 fixed)    (different!)
"Active" state word   B (mixed result to B)   A (temp → A)   (different!)
```
