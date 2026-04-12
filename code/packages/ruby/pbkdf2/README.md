# coding_adventures_pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Ruby.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```ruby
require "coding_adventures_pbkdf2"

dk = CodingAdventures::PBKDF2.pbkdf2_hmac_sha256(
  "correct horse battery staple",
  SecureRandom.bytes(16),   # 16 random bytes per user
  600_000,                  # OWASP 2023 minimum for SHA-256
  32
)
```

## API

| Method                     | PRF         | Returns        |
|----------------------------|-------------|----------------|
| `pbkdf2_hmac_sha1`         | HMAC-SHA1   | Binary String  |
| `pbkdf2_hmac_sha256`       | HMAC-SHA256 | Binary String  |
| `pbkdf2_hmac_sha512`       | HMAC-SHA512 | Binary String  |
| `pbkdf2_hmac_sha1_hex`     | HMAC-SHA1   | Hex String     |
| `pbkdf2_hmac_sha256_hex`   | HMAC-SHA256 | Hex String     |
| `pbkdf2_hmac_sha512_hex`   | HMAC-SHA512 | Hex String     |

## Stack Position

KD01. Depends on `coding_adventures_hmac` (HF05).
