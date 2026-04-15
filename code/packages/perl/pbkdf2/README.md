# CodingAdventures::PBKDF2 (Perl)

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Perl.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```perl
use CodingAdventures::PBKDF2 qw(pbkdf2_hmac_sha256);

my $dk = pbkdf2_hmac_sha256(
    "correct horse battery staple",
    "\xde\xad\xbe\xef" x 4,  # 16 random bytes per user
    600_000,                  # OWASP 2023 minimum for SHA-256
    32
);
```

## API

| Function                  | PRF         | Returns       |
|---------------------------|-------------|---------------|
| `pbkdf2_hmac_sha1`        | HMAC-SHA1   | binary string |
| `pbkdf2_hmac_sha256`      | HMAC-SHA256 | binary string |
| `pbkdf2_hmac_sha512`      | HMAC-SHA512 | binary string |
| `pbkdf2_hmac_sha1_hex`    | HMAC-SHA1   | hex string    |
| `pbkdf2_hmac_sha256_hex`  | HMAC-SHA256 | hex string    |
| `pbkdf2_hmac_sha512_hex`  | HMAC-SHA512 | hex string    |

## Stack Position

KD01. Depends on `CodingAdventures::HMAC` (HF05).
