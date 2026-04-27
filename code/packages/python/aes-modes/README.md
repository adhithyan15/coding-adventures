# coding-adventures-aes-modes

AES modes of operation --- ECB, CBC, CTR, and GCM --- implemented from scratch for educational purposes.

## What Are Modes of Operation?

AES is a **block cipher**: it encrypts exactly 16 bytes at a time. Real messages are rarely exactly 16 bytes, so a **mode of operation** defines how to use the block cipher for arbitrary-length messages.

The choice of mode is critical for security:

| Mode | Security | Use Case |
|------|----------|----------|
| ECB  | BROKEN   | Never use. Educational only. |
| CBC  | Legacy   | TLS 1.0-1.2 (being phased out). Vulnerable to padding oracles. |
| CTR  | Good     | Stream cipher mode. No padding. Parallelizable. |
| GCM  | Best     | Authenticated encryption. TLS 1.3 standard. Detects tampering. |

## Usage

```python
from coding_adventures_aes_modes import (
    ecb_encrypt, ecb_decrypt,
    cbc_encrypt, cbc_decrypt,
    ctr_encrypt, ctr_decrypt,
    gcm_encrypt, gcm_decrypt,
)

key = bytes.fromhex("2b7e151628aed2a6abf7158809cf4f3c")  # 16 bytes for AES-128

# ECB (INSECURE --- educational only)
ciphertext = ecb_encrypt(b"Hello, world!!!!", key)
plaintext = ecb_decrypt(ciphertext, key)

# CBC (requires random IV)
iv = bytes(16)  # In production, use os.urandom(16)
ciphertext = cbc_encrypt(b"Hello, CBC mode!", key, iv)
plaintext = cbc_decrypt(ciphertext, key, iv)

# CTR (requires unique nonce)
nonce = bytes(12)  # In production, use os.urandom(12)
ciphertext = ctr_encrypt(b"Hello, CTR mode!", key, nonce)
plaintext = ctr_decrypt(ciphertext, key, nonce)

# GCM (authenticated encryption)
ciphertext, tag = gcm_encrypt(b"Secret!", key, nonce, aad=b"metadata")
plaintext = gcm_decrypt(ciphertext, key, nonce, aad=b"metadata", tag=tag)
```

## Dependencies

- `coding-adventures-aes` --- provides `aes_encrypt_block` and `aes_decrypt_block`
- `coding-adventures-gf256` --- GF(2^8) arithmetic used by the AES package

## Installation

```bash
pip install coding-adventures-aes-modes
```

## Testing

```bash
uv pip install -e ".[dev]"
pytest tests/ -v
```

Tests use NIST SP 800-38A vectors (ECB, CBC, CTR) and NIST GCM specification vectors.
