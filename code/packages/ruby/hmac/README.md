# coding_adventures_hmac (Ruby)

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Ruby.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** and **authenticity**. Used in TLS, JWT, WPA2, TOTP, and AWS Signature V4.

## API

```ruby
require "coding_adventures_hmac"
HMAC = CodingAdventures::Hmac

key = "\x0b" * 20

# Hex strings (most common)
HMAC.hmac_sha256_hex(key, "Hi There")
# => "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

# Raw binary strings
tag = HMAC.hmac_sha256(key, "Hi There")   # 32-byte binary String

# All four variants
HMAC.hmac_md5(key, msg)    # 16 bytes
HMAC.hmac_sha1(key, msg)   # 20 bytes
HMAC.hmac_sha256(key, msg) # 32 bytes
HMAC.hmac_sha512(key, msg) # 64 bytes

# Generic function — bring your own hash
tag = HMAC.hmac(->(d) { CodingAdventures::Sha256.sha256(d) }, 64, key, msg)
```

## Dependencies

- `coding_adventures_md5`
- `coding_adventures_sha1`
- `coding_adventures_sha256`
- `coding_adventures_sha512`

## How It Fits

```
md5 / sha1 / sha256 / sha512
         ↓
       hmac  ← you are here
         ↓
    pbkdf2 / hkdf  (next)
```
