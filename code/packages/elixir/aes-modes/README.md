# CodingAdventures.AesModes (Elixir)

AES modes of operation: ECB, CBC, CTR, and GCM. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## What This Package Does

AES operates on fixed 16-byte blocks. This package provides four modes of operation that extend AES to handle arbitrary-length messages:

| Mode | Security | Use Case |
|------|----------|----------|
| ECB  | INSECURE | Educational only — identical blocks produce identical ciphertext |
| CBC  | Legacy   | XOR-chains blocks; vulnerable to padding oracle attacks |
| CTR  | Modern   | Turns block cipher into stream cipher; parallelizable |
| GCM  | Best     | CTR + authentication tag; used in TLS 1.3 |

## Dependencies

- `coding_adventures_aes` — AES block cipher (aes_encrypt_block / aes_decrypt_block)

## Usage

```elixir
alias CodingAdventures.AesModes

# ECB (INSECURE — educational only)
ct = AesModes.ecb_encrypt(plaintext, key)
pt = AesModes.ecb_decrypt(ct, key)

# CBC
ct = AesModes.cbc_encrypt(plaintext, key, iv_16)
pt = AesModes.cbc_decrypt(ct, key, iv_16)

# CTR (no padding needed)
ct = AesModes.ctr_encrypt(plaintext, key, nonce_12)
pt = AesModes.ctr_decrypt(ct, key, nonce_12)

# GCM (authenticated encryption)
{ct, tag} = AesModes.gcm_encrypt(plaintext, key, iv_12, aad)
{:ok, pt} = AesModes.gcm_decrypt(ct, key, iv_12, aad, tag)
```

## Running Tests

```bash
mix deps.get --quiet && mix test --cover
```
