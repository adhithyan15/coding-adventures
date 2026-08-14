# ChaCha20-Poly1305 (Ruby)

A from-scratch implementation of the ChaCha20-Poly1305 AEAD cipher suite
(RFC 8439) and the extended-nonce XChaCha20-Poly1305 construction pinned by
SE04, using only ARX (Add, Rotate, XOR) operations.

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag (uses Ruby's native big integers)
- **AEAD** construction: combined authenticated encryption per RFC 8439
- **HChaCha20 and XChaCha20**: subkey derivation and a 192-bit nonce form

## Usage

```ruby
require "coding_adventures_chacha20_poly1305"

CC = CodingAdventures::Chacha20Poly1305

# Stream cipher
ct = CC.chacha20_encrypt(plaintext, key, nonce, 0)

# One-time MAC
tag = CC.poly1305_mac(message, key)

# Authenticated encryption
ct, tag = CC.aead_encrypt(plaintext, key, nonce, aad)
pt = CC.aead_decrypt(ct, key, nonce, aad, tag)

# SE04 HChaCha20 and raw XChaCha20
subkey = CC.hchacha20_subkey(key, nonce16)
raw_ct = CC.xchacha20_encrypt(plaintext, key, nonce24, counter)

# SE04 XChaCha20-Poly1305 (recommended when a 24-byte nonce is available)
xct, xtag = CC.xchacha20_poly1305_encrypt(plaintext, key, nonce24, aad)
xpt = CC.xchacha20_poly1305_decrypt(xct, key, nonce24, aad, xtag)
```

The HChaCha20 and XChaCha20-Poly1305 vectors come from
`draft-irtf-cfrg-xchacha-03`. That expired Internet-Draft is a pinned
construction reference, not a final IETF standard. A random 24-byte nonce makes
accidental collisions negligible at realistic volumes, but the complete nonce
must still be unique for each key.

Derived subkeys are overwritten on a best-effort basis. Ruby does not guarantee
that copied or moved string buffers are erased from process memory.

This package is educational, hand-written cryptographic code. Prefer a mature,
audited cryptography library for production secrets.

## Building

```bash
bundle install
bundle exec rake test
```

## Portable Conformance

The test suite consumes the versioned
[`se04-xchacha20-poly1305-v1`](../../../specs/fixtures/se04-xchacha20-poly1305-v1/README.md)
fixture shared by all six D18 implementations. It proves byte-identical
HChaCha20, raw XChaCha20, and AEAD outputs plus the common authentication
failure contract. Fixture parsing uses Ruby's standard-library JSON parser.

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
