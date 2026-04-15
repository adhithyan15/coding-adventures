# coding_adventures_aes_modes

AES modes of operation --- ECB, CBC, CTR, and GCM --- implemented from scratch in Ruby for educational purposes.

## What Are Modes of Operation?

AES is a **block cipher**: it encrypts exactly 16 bytes at a time. A **mode of operation** defines how to use the block cipher for messages of arbitrary length.

| Mode | Security | Description |
|------|----------|-------------|
| ECB  | BROKEN   | Each block encrypted independently. Patterns leak. |
| CBC  | Legacy   | Blocks chained via XOR. Vulnerable to padding oracles. |
| CTR  | Good     | Stream cipher mode. No padding. Parallelizable. |
| GCM  | Best     | Authenticated encryption. TLS 1.3 standard. |

## Usage

```ruby
require "coding_adventures_aes_modes"

key = ["2b7e151628aed2a6abf7158809cf4f3c"].pack("H*")

# ECB (INSECURE)
ct = CodingAdventures::AesModes.ecb_encrypt("Hello!", key)
pt = CodingAdventures::AesModes.ecb_decrypt(ct, key)

# CBC
iv = "\x00" * 16
ct = CodingAdventures::AesModes.cbc_encrypt("Hello!", key, iv)
pt = CodingAdventures::AesModes.cbc_decrypt(ct, key, iv)

# CTR
nonce = "\x00" * 12
ct = CodingAdventures::AesModes.ctr_encrypt("Hello!", key, nonce)
pt = CodingAdventures::AesModes.ctr_decrypt(ct, key, nonce)

# GCM
ct, tag = CodingAdventures::AesModes.gcm_encrypt("Secret!", key, nonce, "metadata")
pt = CodingAdventures::AesModes.gcm_decrypt(ct, key, nonce, "metadata", tag)
```

## Dependencies

- `coding_adventures_aes` --- AES block cipher

## Testing

```bash
bundle exec rake test
```

Tests use NIST SP 800-38A vectors (ECB, CBC, CTR) and NIST GCM specification vectors.
