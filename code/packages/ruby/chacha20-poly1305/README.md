# ChaCha20-Poly1305 (Ruby)

A from-scratch implementation of the ChaCha20-Poly1305 AEAD cipher suite
(RFC 8439), using only ARX (Add, Rotate, XOR) operations.

## What's Inside

- **ChaCha20** stream cipher: 256-bit key, 96-bit nonce, 32-bit counter
- **Poly1305** one-time MAC: 16-byte authentication tag (uses Ruby's native big integers)
- **AEAD** construction: combined authenticated encryption per RFC 8439

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
```

## Building

```bash
bundle install
bundle exec rake test
```

## Part Of

[coding-adventures](https://github.com/adhithyan15/coding-adventures) -- a
monorepo of from-scratch implementations for learning.
