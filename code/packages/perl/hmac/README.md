# CodingAdventures::HMAC (Perl)

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Perl.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** and **authenticity**. Used in TLS, JWT, WPA2, TOTP, and AWS Signature V4.

## API

```perl
use lib 'lib';
use CodingAdventures::HMAC qw(
    hmac_sha256_hex hmac_sha512_hex
    hmac_md5_hex    hmac_sha1_hex
    hmac_sha256     hmac_sha512
);

my $key = "\x0b" x 20;

# Hex strings (most common)
hmac_sha256_hex($key, "Hi There");
# "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

# Raw bytes (array-ref of integers 0-255)
my $tag = hmac_sha256($key, "Hi There");   # [ 176, 52, 76, ... ]

# All four variants
hmac_md5($key, $msg);    # 16-element arrayref
hmac_sha1($key, $msg);   # 20-element arrayref
hmac_sha256($key, $msg); # 32-element arrayref
hmac_sha512($key, $msg); # 64-element arrayref
```

## Dependencies

- `CodingAdventures::Md5`
- `CodingAdventures::Sha1`
- `CodingAdventures::SHA256`
- `CodingAdventures::Sha512`

## How It Fits

```
Md5 / Sha1 / SHA256 / Sha512
         ↓
       HMAC  ← you are here
         ↓
    PBKDF2 / HKDF  (next)
```
