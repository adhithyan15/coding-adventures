# des — DES and 3DES Block Cipher (Go)

A pure-Go educational implementation of the Data Encryption Standard (DES)
and Triple DES (3TDEA / 3DES), conforming to FIPS 46-3 and NIST SP 800-67.

**Do not use this for protecting real data.** DES's 56-bit key is completely
broken by modern hardware. Use AES-GCM from the Go standard library instead.

## What It Implements

- `ExpandKey` — derive the 16 round subkeys from an 8-byte DES key
- `EncryptBlock` / `DecryptBlock` — single 8-byte block cipher
- `ECBEncrypt` / `ECBDecrypt` — ECB mode with PKCS#7 padding (for arbitrary-length data)
- `TDEAEncryptBlock` / `TDEADecryptBlock` — Triple DES EDE (Encrypt-Decrypt-Encrypt)

## How It Fits in the Stack

This package sits at layer SE01 (symmetric encryption primitives) in the
coding-adventures security stack. It depends on nothing (pure Go, no external
imports). AES (go/aes) is the companion modern cipher.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/des"

key := []byte{0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1}
plain := []byte{0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF}

ct, err := des.EncryptBlock(plain, key)
// ct = 0x85E813540F0AB405

pt, err := des.DecryptBlock(ct, key)
// pt == plain

// Variable-length ECB (PKCS#7 padding)
ciphertext, err := des.ECBEncrypt([]byte("Hello!"), key)
plaintext, err  := des.ECBDecrypt(ciphertext, key)

// Triple DES EDE: C = E_K1(D_K2(E_K3(P)))
ct, err = des.TDEAEncryptBlock(plain, k1, k2, k3)
```

## Algorithm Notes

DES is a Feistel network with 16 rounds. Each round applies:

1. **E** — 32-bit right half expanded to 48 bits
2. **XOR** with the 48-bit round subkey
3. **S-boxes** — 8 × (6 bits → 4 bits), the only non-linear step
4. **P** — 32-bit diffusion permutation

Decryption is identical but with subkeys reversed (K16..K1) — the
self-inverse property of Feistel networks.

**Triple DES (EDE):** `C = E_K1(D_K2(E_K3(P)))` — when K1=K2=K3, reduces
to single DES for backward compatibility.

## Test Vectors

| Key                | Plaintext          | Ciphertext         |
|--------------------|--------------------|--------------------|
| `133457799BBCDFF1` | `0123456789ABCDEF` | `85E813540F0AB405` |
| `0101010101010101` | `95F8A5E5DD31D900` | `8000000000000000` |
| `8001010101010101` | `0000000000000000` | `95A8D72813DAA94D` |

## Running Tests

```bash
go test ./... -v -cover
```
