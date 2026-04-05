# SE02 — AES Modes of Operation

## Overview

AES operates on fixed 128-bit blocks. To encrypt messages of arbitrary
length, you need a **mode of operation** that defines how to chain
multiple block cipher calls together. The choice of mode critically
affects security — ECB is broken, CBC is vulnerable to padding oracles,
while CTR and GCM are the modern standards.

This package implements multiple modes to teach their differences and
why some are dangerous.

## Modes

### ECB (Electronic Codebook) — INSECURE, for education only

Each block is encrypted independently.

```
C_i = AES_encrypt(P_i, K)
```

**Problem:** Identical plaintext blocks produce identical ciphertext
blocks. The famous "ECB penguin" demonstrates this — encrypting an
image in ECB mode reveals the image structure in the ciphertext.

### CBC (Cipher Block Chaining) — Legacy

Each plaintext block is XOR'd with the previous ciphertext block before
encryption.

```
C_0 = IV
C_i = AES_encrypt(P_i XOR C_{i-1}, K)
```

Requires an unpredictable IV (not just unique). Vulnerable to padding
oracle attacks (POODLE, Lucky 13) if padding errors are observable.

### CTR (Counter Mode) — Recommended

Turns a block cipher into a stream cipher. Encrypts a counter, XORs
the keystream with plaintext.

```
keystream_i = AES_encrypt(nonce || counter_i, K)
C_i = P_i XOR keystream_i
```

Parallelizable, no padding needed, random access. Requires a unique
nonce per message (reuse is catastrophic — XOR of two ciphertexts
reveals XOR of plaintexts).

### GCM (Galois/Counter Mode) — Recommended with Authentication

CTR mode + GHASH authentication tag. Provides both encryption and
integrity (authenticated encryption).

```
Ciphertext = CTR_encrypt(plaintext, K, IV)
Tag = GHASH(AAD, ciphertext, K, IV)
```

The gold standard for TLS 1.3. Requires a unique IV per message.

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `ecb_encrypt` | `(plaintext, key) -> ciphertext` | ECB mode (INSECURE). |
| `ecb_decrypt` | `(ciphertext, key) -> plaintext` | ECB mode. |
| `cbc_encrypt` | `(plaintext, key, iv) -> ciphertext` | CBC mode. |
| `cbc_decrypt` | `(ciphertext, key, iv) -> plaintext` | CBC mode. |
| `ctr_encrypt` | `(plaintext, key, nonce) -> ciphertext` | CTR mode. |
| `ctr_decrypt` | `(ciphertext, key, nonce) -> plaintext` | CTR mode (same as encrypt). |
| `gcm_encrypt` | `(plaintext, key, iv, aad) -> (ciphertext, tag)` | GCM authenticated encryption. |
| `gcm_decrypt` | `(ciphertext, key, iv, aad, tag) -> plaintext` | GCM authenticated decryption (fails if tag invalid). |

### Padding (for ECB and CBC)
PKCS#7 padding: pad with N bytes of value N (1-16).

## Educational Demos

### ECB Penguin
- Encrypt a BMP image in ECB mode
- Show that the image structure is visible in the ciphertext
- Compare with CBC/CTR where ciphertext looks random

### Nonce Reuse Attack (CTR)
- Encrypt two messages with same nonce
- XOR ciphertexts → get XOR of plaintexts
- Demonstrate why nonce reuse is catastrophic

### Padding Oracle Attack (CBC)
- Show how observable padding errors leak plaintext byte-by-byte
- Step-by-step: modify last ciphertext byte, observe error
- Why TLS moved from CBC to GCM

## Package Matrix

Same 9 languages, in `aes-modes/` directories.

**Dependencies:** SE01 (AES core block cipher).
