# ChaCha20-Poly1305 (Elixir)

ChaCha20-Poly1305 authenticated encryption (RFC 8439) implemented in pure Elixir.

## What is ChaCha20-Poly1305?

ChaCha20-Poly1305 is an authenticated encryption scheme combining:

- **ChaCha20**: a stream cipher using only Add, Rotate, XOR (ARX) operations
- **Poly1305**: a one-time message authentication code (MAC)

Together they provide both confidentiality (encryption) and integrity (authentication). Used in TLS 1.3, WireGuard, SSH, and Chrome/Android.

## Usage

```elixir
alias CodingAdventures.ChaCha20Poly1305, as: CC

# ChaCha20 stream cipher
ct = CC.chacha20_encrypt(plaintext, key_32, nonce_12, counter)

# Poly1305 MAC
tag = CC.poly1305_mac(message, key_32)

# AEAD encrypt
{ciphertext, tag} = CC.aead_encrypt(plaintext, key_32, nonce_12, aad)

# AEAD decrypt
{:ok, plaintext} = CC.aead_decrypt(ciphertext, key_32, nonce_12, aad, tag)
{:error, :authentication_failed} = CC.aead_decrypt(ct, key, nonce, aad, bad_tag)
```

## API

| Function | Parameters | Returns |
|----------|-----------|---------|
| `chacha20_encrypt/4` | `(data, key_32, nonce_12, counter)` | binary |
| `poly1305_mac/2` | `(message, key_32)` | 16-byte binary |
| `aead_encrypt/4` | `(plaintext, key_32, nonce_12, aad)` | `{ciphertext, tag}` |
| `aead_decrypt/5` | `(ct, key_32, nonce_12, aad, tag)` | `{:ok, plaintext}` or `{:error, reason}` |

## Running Tests

```sh
mix test --cover
```

## Dependencies

None. Pure Elixir with native arbitrary-precision integers.
