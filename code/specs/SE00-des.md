# SE00 — DES (Data Encryption Standard)

## Overview

DES (Data Encryption Standard) was the world's first openly standardized encryption
algorithm. Published by NIST in 1977 (FIPS 46), it became the backbone of banking,
government communications, and commercial software for two decades.

It is now **completely broken** — a 56-bit key can be exhausted in under 24 hours
on consumer hardware. DES is included in this project as a historical study: a case
study in what happens when key size is chosen for political reasons rather than
cryptographic ones, and as the foundation for understanding why AES was designed
the way it was.

**Do not use DES for anything new.** NIST withdrew the standard in 2005. Its
only surviving application is Triple DES (3DES), which itself was deprecated by
NIST in 2023.

---

## Where It Fits

```
The Cryptography Timeline

1977  DES (56-bit key, 64-bit block)        ← this package
1999  EFF Deep Crack breaks DES in 22 hours
2001  AES replaces DES (128/192/256-bit key) ← SE01
2003  3DES officially deprecated for new systems
2023  NIST formally retires 3DES

The chain of reasoning:
DES → why 56 bits isn't enough → brute force → key size matters
DES → Feistel networks → how block cipher structure works → AES

Understanding DES makes AES legible.
```

**Depends on:** Nothing. A Feistel network is self-contained.

**Used by:**
- 3DES / TDEA — three sequential DES operations with two or three keys
- Educational comparisons with AES
- Legacy system support (read-only — don't generate new DES ciphertext)

---

## History: The IBM Cipher That Became a Standard

In 1972, the National Bureau of Standards (NBS, now NIST) issued a public call for
a national encryption standard. The goal was to allow government agencies and banks
to share encrypted data interoperably — a serious problem at the time, when every
vendor had a proprietary scheme.

In 1974, IBM submitted a cipher called **Lucifer**, designed by Horst Feistel and
Don Coppersmith. Lucifer used a 128-bit key and was based on a novel structure
called a **Feistel network** (see below). The NBS accepted Lucifer as the basis
for the standard — but with modifications requested by the NSA.

The two most controversial changes:
1. **Key size reduced from 128 bits to 56 bits.** The NSA's stated reason: 56 bits
   was sufficient for non-military use. The unstated reason, suspected widely:
   56 bits was breakable by the NSA's own hardware, giving them a surveillance
   advantage.
2. **The S-boxes were redesigned by the NSA.** IBM's original S-boxes were replaced
   with ones that the NSA did not explain. This caused immediate suspicion of
   backdoors.

In 1977, DES became FIPS 46.

**Vindication on the S-boxes:** In 1990, Biham and Shamir published "differential
cryptanalysis" — a powerful new technique. They showed that DES's S-boxes were
*specifically resistant* to differential cryptanalysis, suggesting the NSA knew
about this attack two decades before the public. The S-boxes were not backdoored;
they were hardened against a then-secret attack. The NSA had been ahead.

**Failure on key size:** The 56-bit key was always too small.

```
Year   Event
────   ─────
1977   DES published (56-bit key, ~72 quadrillion possible keys)
1993   Michael Wiener designs a $1M machine that could break DES in 3.5 hours
1997   RSA Security's DES Challenge I: 96 days to crack one key
1998   DES Challenge II-1: 39 days
1998   DES Challenge II-2: 56 hours
1999   DES Challenge III: EFF's Deep Crack + distributed.net = 22 hours 15 minutes
2004   AES has been standard for 3 years; DES is only legacy
2005   NIST withdraws FIPS 46-3 (the last DES standard)
```

The EFF's Deep Crack was a $250,000 custom FPGA machine built in 1998. It could try
90 billion keys per second. A modern GPU cluster can try trillions per second.
A 56-bit keyspace has only 72 quadrillion keys (2^56 ≈ 7.2 × 10^16). At one trillion
keys per second, exhaustive search takes 72,000 seconds — about 20 hours.

---

## The Feistel Network

The core innovation of DES is not the key schedule or the S-boxes — it is the
**Feistel structure**, which allows decryption with almost the same circuit as
encryption.

A Feistel network splits the data into two halves (L = left, R = right) and applies
16 rounds of the following transformation:

```
Round i:
  L_i = R_{i-1}
  R_i = L_{i-1} XOR f(R_{i-1}, K_i)

where:
  f  = the Feistel round function (see below)
  K_i = the i-th 48-bit subkey
```

**Why this is clever:**

Notice that decryption is identical to encryption — you just apply the subkeys in
reverse order (K_16, K_15, ..., K_1 instead of K_1, K_2, ..., K_16). The function
`f` never needs to be inverted.

```
ENCRYPTION                        DECRYPTION
──────────                        ──────────
Plaintext                         Ciphertext
    │                                 │
    ├─► L₀ (32 bits)                  ├─► L₁₆ (32 bits)
    └─► R₀ (32 bits)                  └─► R₁₆ (32 bits)
         │                                 │
  Round 1 (K₁)                      Round 1 (K₁₆)
         │                                 │
  Round 2 (K₂)                      Round 2 (K₁₅)
         │                                 │
      ...                              ...
         │                                 │
  Round 16 (K₁₆)                    Round 16 (K₁)
         │                                 │
    Ciphertext                         Plaintext
```

This means DES encryption and decryption share the same hardware — crucial when
implementing in 1970s chips where die area was expensive.

---

## Algorithm

### Step 1: Initial Permutation (IP)

The 64-bit plaintext block is permuted according to a fixed table before any
round computation begins. This was designed to scatter bit positions for efficient
implementation on the 8-bit parallel bus architectures of the 1970s.

```
IP table (position in input → position in output):
58 50 42 34 26 18 10  2
60 52 44 36 28 20 12  4
62 54 46 38 30 22 14  6
64 56 48 40 32 24 16  8
57 49 41 33 25 17  9  1
59 51 43 35 27 19 11  3
61 53 45 37 29 21 13  5
63 55 47 39 31 23 15  7
```

After IP, the 64 bits are split: L₀ = bits 1–32, R₀ = bits 33–64.

### Step 2: Key Schedule — 16 Subkeys from One 64-bit Key

The 64-bit input key has 8 parity bits (bits 8, 16, 24, 32, 40, 48, 56, 64).
The actual key material is 56 bits. The key schedule produces sixteen 48-bit
subkeys K_1 through K_16.

```
PC-1 (Permuted Choice 1): 64 bits → 56 bits (drop parity)
Split into C₀ (28 bits) and D₀ (28 bits)

For each round i = 1 to 16:
  C_i = LS(C_{i-1}, shift[i])
  D_i = LS(D_{i-1}, shift[i])
  K_i = PC-2(C_i ∥ D_i)    (56 bits → 48 bits)

Left-shift amounts (total across 16 rounds = 28, one full rotation):
  Round: 1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
  Shift: 1  1  2  2  2  2  2  2  1  2  2  2  2  2  2  1
```

### Step 3: The Round Function f(R, K)

This is where DES's security lives. It takes a 32-bit half-block and a 48-bit
subkey, and produces a 32-bit output.

```
R (32 bits)
    │
    ▼
E (Expansion): 32 bits → 48 bits
    │   (each 4-bit group of R expands to 6 bits by copying border bits)
    │
XOR with K_i (48 bits)
    │
    ▼
S-Boxes: 48 bits → 32 bits
    │   (8 S-boxes, each takes 6 bits → 4 bits)
    │
    ▼
P-Box (Permutation): 32 bits → 32 bits
    │
    ▼
Output (32 bits) — XOR'd with L to produce new R
```

### The S-Boxes: DES's Core Non-Linearity

This is the most critical part of DES. Eight substitution boxes (S-boxes), each
taking 6 bits of input and producing 4 bits of output. Without S-boxes, DES
would be entirely linear and breakable with simple algebra.

**How to read an S-box:**

Given 6 input bits b₁b₂b₃b₄b₅b₆:
- Row = 2×b₁ + b₆  (outer bits, 0–3)
- Col = 8×b₂ + 4×b₃ + 2×b₄ + b₅  (inner bits, 0–15)
- Output = S[row][col]

**S-Box 1 (example):**

```
        Column
Row  0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15
 0  14  4 13  1  2 15 11  8  3 10  6 12  5  9  0  7
 1   0 15  7  4 14  2 13  1 10  6 12 11  9  5  3  8
 2   4  1 14  8 13  6  2 11 15 12  9  7  3 10  5  0
 3  15 12  8  2  4  9  1  7  5 11  3 14 10  0  6 13
```

**Reading S1 with input 011011:**
- b₁=0, b₆=1 → row = 2×0 + 1 = 1
- b₂b₃b₄b₅ = 1101 = 13 → col = 13
- S1[1][13] = 5 = 0101

**Why S-boxes provide security:**

An S-box is a non-linear mapping — you cannot express it as a set of linear
equations over GF(2). This non-linearity prevents the "just solve the linear
system" attack that would break purely XOR-based ciphers.

The NSA-redesigned S-boxes in DES satisfy additional properties discovered
later:
1. No output bit is a linear function of the input bits
2. Changing 1 input bit changes at least 2 output bits
3. For any non-zero 6-bit difference Δ, at most 8 of 64 input pairs share
   that difference and produce the same output difference

These are exactly the properties needed to resist differential cryptanalysis.

### Step 4: Final Permutation (FP = IP⁻¹)

After 16 rounds, apply IP⁻¹ (the inverse of the initial permutation) to get the
64-bit ciphertext. FP is the bitwise inverse of IP.

```
FP table:
40  8 48 16 56 24 64 32
39  7 47 15 55 23 63 31
38  6 46 14 54 22 62 30
37  5 45 13 53 21 61 29
36  4 44 12 52 20 60 28
35  3 43 11 51 19 59 27
34  2 42 10 50 18 58 26
33  1 41  9 49 17 57 25
```

---

## Why 56 Bits Is Not Enough

### The Birthday Bound Is Not The Problem Here

For hash functions, the security concern is the birthday paradox: you can find
collisions in √(2^n) operations. For a 128-bit hash, that's 2^64 — still infeasible.

DES is different. DES is a block cipher, and the threat model is **key recovery**,
not collisions. To recover the key, you must try all possible keys until you find
the right one. With 56 bits, that's 2^56 ≈ 72 quadrillion tries.

**Is 2^56 hard?**

```
Year   Technology                  DES keys/second    Time to break
────   ──────────                  ───────────────    ─────────────
1977   DES published               —                  "sufficient"
1993   Wiener's design (paper)     1.6 × 10^9        3.5 hours
1997   RSA DES Challenge I (net)   1 × 10^8          96 days
1998   EFF Deep Crack              9 × 10^10          ~22 hours
2008   COPACOBANA (FPGAs, $10k)    2 × 10^9          1 week
2022   Modern GPU cluster          ~10^12             ~20 hours
```

The NSA knew in 1977 that their hardware could break 56-bit keys. They designed
the standard to be breakable by themselves but not by academic researchers or
other nations — a deliberate security-through-hardware-cost strategy.

This strategy failed as hardware got cheaper.

### The Mathematical Guarantee of AES

AES-128 has a 128-bit key: 2^128 possible keys.

```
2^56  = 72,057,594,037,927,936           (DES key space)
2^128 = 340,282,366,920,938,463,463,374,607,431,768,211,456  (AES-128 key space)
```

AES-128's keyspace is 2^72 = ~4.7 sextillion times larger than DES. Even if you
had hardware a trillion times faster than the GPU cluster above, AES-128 would
still take longer than the age of the universe.

AES was designed with this margin deliberately. NIST required that the AES
finalist algorithms support at least 128-bit keys.

---

## Triple DES (3DES / TDEA)

When DES's weakness became apparent in the 1990s, the obvious stopgap was to
apply DES three times with different keys. This is 3DES, standardized as ANSI X9.52.

### 2TDEA (Two-key 3DES): EDE with K1≠K2, K3=K1

```
Ciphertext = DES_encrypt(K1, DES_decrypt(K2, DES_encrypt(K1, plaintext)))
```

Effective key: 112 bits. But vulnerable to a **meet-in-the-middle attack** that
reduces security to about 80 bits in practice.

### 3TDEA (Three-key 3DES): EDE with K1, K2, K3 all different

```
Ciphertext = DES_encrypt(K3, DES_decrypt(K2, DES_encrypt(K1, plaintext)))
```

Effective key: 168 bits. Meet-in-the-middle reduces to about 112 bits.

**Why EDE (Encrypt-Decrypt-Encrypt) and not EEE?**

EDE is backward compatible: if K1=K2=K3, 3DES reduces to single DES. The decrypt
step in the middle cancels with one of the outer encryptions.

### Why 3DES Is Also Retired

1. **Slow:** Three sequential DES operations. AES is faster by a factor of 3–6×.
2. **64-bit blocks:** DES and 3DES use 64-bit blocks. With a fixed key, a 64-bit
   block cipher becomes vulnerable to birthday attacks after ~2^32 blocks
   (32 GB) of data encrypted under the same key — the **SWEET32** attack (2016).
3. **Meet-in-the-middle:** 2TDEA has less security than its key length suggests.

NIST deprecated 3DES for new applications in 2017 and disallowed it entirely in 2023.

---

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `des_encrypt_block` | `(block: 8 bytes, key: 8 bytes) -> 8 bytes` | Encrypt one 64-bit block. |
| `des_decrypt_block` | `(block: 8 bytes, key: 8 bytes) -> 8 bytes` | Decrypt one 64-bit block. |
| `expand_key` | `(key: 8 bytes) -> [K1..K16: 6 bytes each]` | Generate 16 subkeys. |
| `des_ecb_encrypt` | `(plaintext: bytes, key: 8 bytes) -> bytes` | ECB mode (educational). |
| `des_ecb_decrypt` | `(ciphertext: bytes, key: 8 bytes) -> bytes` | ECB mode. |
| `tdea_encrypt_block` | `(block: 8 bytes, k1, k2, k3: 8 bytes each) -> 8 bytes` | 3DES encrypt. |
| `tdea_decrypt_block` | `(block: 8 bytes, k1, k2, k3: 8 bytes each) -> 8 bytes` | 3DES decrypt. |

**Notes:**
- Key bits 8, 16, 24, 32, 40, 48, 56, 64 are parity bits — implementations
  should accept any 8-byte key (ignoring parity) to match real-world usage.
- ECB mode is provided for compatibility with historical data. Never use ECB
  for new encryption (same block → same ciphertext, no diffusion across blocks).

---

## Test Vectors (NIST FIPS 81 and SP 800-20)

```
# Single-block DES encryption
Key:       0133457799BBCDFF
Plaintext: 0123456789ABCDEF
Ciphertext: 85E813540F0AB405

# Zero plaintext with single non-zero key byte
Key:       0000000000000080
Plaintext: 0000000000000000
Ciphertext: 9295B59BB384736E

# Known-answer test (NIST SP 800-20 Table 1)
Key:       0101010101010101
Plaintext: 95F8A5E5DD31D900
Ciphertext: 8000000000000000

# 3DES (3TDEA) test vector (NIST SP 800-67)
Key1:      0123456789ABCDEF
Key2:      23456789ABCDEF01
Key3:      456789ABCDEF0123
Plaintext: 6BC1BEE22E409F96
Ciphertext: 06EDE3D82884090A
```

---

## Educational Demos

### Demo 1: Brute-Force Attack (56-bit Key Exhaustion)

Given a known plaintext–ciphertext pair, search all 2^56 keys to find the right
one. This is the fundamental attack that breaks DES.

```
known_plaintext  = 0x0123456789ABCDEF
known_ciphertext = 0x85E813540F0AB405

for key in range(0, 2**56):
    if des_encrypt_block(known_plaintext, key) == known_ciphertext:
        print(f"Found key: {key:016x}")
        break
```

In an educational implementation, run this for a small number of rounds to
demonstrate the concept, then extrapolate: at 10^12 guesses/second, 2^56 takes
~20 hours.

**Compare with AES-128:**
At the same rate, 2^128 takes ~10^22 years (the universe is 1.38 × 10^10 years old).
The gap between 56-bit and 128-bit security is not 2×; it is 2^72 ≈ 4.7 × 10^21 ×.

### Demo 2: Meet-in-the-Middle Attack on Double DES

Why isn't "Double DES" (DES twice with two different keys) as secure as 112 bits?
Because of the meet-in-the-middle attack:

```
Double DES:  C = DES_K2(DES_K1(P))

Attack:
1. Encrypt P forward with all 2^56 possible K1 values → table T_enc
2. Decrypt C backward with all 2^56 possible K2 values → table T_dec
3. Find any value that appears in BOTH tables
   → that value is DES_K1(P) = DES_K2^{-1}(C)
   → the matching (K1, K2) pair is the key

Cost: 2 × 2^56 operations and 2^56 storage
Security: ~2^57, not 2^112
```

This is why 2DES was never standardized — it provides almost no security over
single DES at double the cost. 3DES's EDE structure was designed specifically
to defeat this attack while maintaining backward compatibility.

### Demo 3: The ECB Penguin

Encrypt a grayscale bitmap image in ECB mode (one 8-byte block at a time with
the same key, no chaining). The resulting image will visually resemble the
original — the structure of the image is preserved in the ciphertext because
identical 8-byte blocks of pixels always encrypt to the same ciphertext block.

This demonstrates why ECB mode is insecure: identical plaintext blocks reveal
their equality in the ciphertext, leaking information about the data structure.

CBC mode (which chains each block with the previous ciphertext) produces
indistinguishable-from-random output that reveals no image structure.

---

## DES vs AES Comparison

```
Property           DES (1977)               AES (2001)
────────           ──────────               ──────────
Block size         64 bits                  128 bits
Key sizes          56 bits (effective)      128, 192, 256 bits
Structure          Feistel network          Substitution-permutation
Rounds             16                       10 / 12 / 14
Round function     Feistel f (S-boxes)      SubBytes, ShiftRows, MixColumns
Key schedule       Left rotations + PC-2    AES KeyExpansion (word-based)
S-boxes            8 × (6-bit → 4-bit)      1 × (8-bit → 8-bit), GF(2^8)
Speed (software)   ~25 MB/s                 ~200 MB/s (no AES-NI)
Speed (AES-NI)     N/A                      ~10 GB/s
Status             Retired (2005)           Current standard
Meet-in-middle     Breaks double-DES        Not applicable (single key)
Brute force        22 hours (1999 hardware) Infeasible (> age of universe)
```

---

## Design Notes

- **S-box representation:** Hardcode all 8 S-boxes as constant tables. Each
  S-box is a 4×16 matrix of 4-bit values. Total: 8 × 64 = 512 nibbles = 256 bytes.
- **Parity bits:** Accept keys with incorrect parity — real-world keys often have
  wrong parity and the algorithm ignores those bits anyway.
- **Performance:** For educational clarity, implement IP, FP, E, P as explicit
  table lookups rather than optimized bit-manipulation code. Production DES
  implementations merge IP, FP, and the round permutations into combined tables
  (a technique called "the bitslice").
- **Constant-time:** Not applicable for educational DES — this algorithm should
  never be used in production. Note in the implementation that timing side-channel
  defenses are omitted deliberately.

---

## Package Matrix

Same 9 languages, in `des/` directories.

**Dependencies:** None. The Feistel network, IP/FP tables, S-boxes, and key
schedule are entirely self-contained.

**Note:** Implementation is deferred — this spec is committed first as
documentation of what DES is and why we study it, in the same spirit as the
MD5 spec which explains a broken hash function for educational purposes. The
implementation will follow after the active encryption primitives (AES, ChaCha20).
