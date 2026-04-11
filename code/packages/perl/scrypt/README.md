# CodingAdventures::Scrypt (Perl)

scrypt (RFC 7914) memory-hard password-based key derivation — implemented from scratch in Perl.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## What Is scrypt?

scrypt derives a cryptographic key from a password in a way that is simultaneously hard on CPU **and memory**. Unlike PBKDF2 (which is only CPU-hard), scrypt's memory-hard design means that parallelising a brute-force attack requires proportionally more RAM — an adversary building 1,000 cores also needs 1,000× the RAM, which is far more expensive than 1,000× the CPU.

The algorithm was designed by Colin Percival (2009) and standardised in RFC 7914 (2016).

## Algorithm

```
scrypt(P, S, N, r, p, dkLen):
  B        = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)  # initial keying material
  B[i]     = scryptROMix(B[i], N, r)   for i=0..p-1  # memory-hard mix
  DK       = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)    # final key stretch
```

The core primitive is **BlockMix** (Salsa20/8 based) wrapped in **ROMix**, which fills an N-slot scratchpad and reads it in a data-dependent order.

## Usage

```perl
use CodingAdventures::Scrypt qw(scrypt scrypt_hex);

# Interactive login (16 MB, ~100 ms on modern hardware)
my $dk = scrypt(
    "correct horse battery staple",   # password
    "\xde\xad\xbe\xef" x 4,          # 16 random bytes per user
    16384,                             # N — CPU/memory cost
    8,                                 # r — block size
    1,                                 # p — parallelism
    32                                 # dk_len — output bytes
);

# Offline/bulk key derivation (1 GB, several seconds)
my $hex = scrypt_hex($password, $salt, 1048576, 8, 1, 64);
```

## API

| Function     | Returns       |
|--------------|---------------|
| `scrypt`     | binary string |
| `scrypt_hex` | hex string    |

### Parameters

| Name     | Type | Meaning                                           |
|----------|------|---------------------------------------------------|
| password | str  | Secret to derive from. May be empty.              |
| salt     | str  | Per-credential random bytes. May be empty.        |
| N        | int  | Cost factor. Power of 2, 2..2^20.                 |
| r        | int  | Block size factor. >= 1. Typically 8.             |
| p        | int  | Parallelism. >= 1. Typically 1.                   |
| dk_len   | int  | Output bytes. 1..2^20.                            |

Memory requirement: `128 * r * N` bytes (e.g. 16 MB for N=16384, r=8).

## Stack Position

KD02. Depends on `CodingAdventures::HMAC` (HF05).

Implements: RFC 7914 (scrypt), using RFC 7914 § 11 test vectors for validation.
