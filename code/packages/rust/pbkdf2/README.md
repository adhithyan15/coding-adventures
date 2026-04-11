# rust/pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Rust.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```rust
use coding_adventures_pbkdf2::pbkdf2_hmac_sha256;

let dk = pbkdf2_hmac_sha256(
    b"correct horse battery staple",
    b"\xde\xad\xbe\xef\xde\xad\xbe\xef\xde\xad\xbe\xef\xde\xad\xbe\xef",
    600_000,  // OWASP 2023 minimum for SHA-256
    32,
).unwrap();
```

## API

| Function                    | PRF         | Returns                        |
|-----------------------------|-------------|-------------------------------|
| `pbkdf2_hmac_sha1`          | HMAC-SHA1   | `Result<Vec<u8>, Pbkdf2Error>` |
| `pbkdf2_hmac_sha256`        | HMAC-SHA256 | `Result<Vec<u8>, Pbkdf2Error>` |
| `pbkdf2_hmac_sha512`        | HMAC-SHA512 | `Result<Vec<u8>, Pbkdf2Error>` |
| `pbkdf2_hmac_sha1_hex`      | HMAC-SHA1   | `Result<String, Pbkdf2Error>`  |
| `pbkdf2_hmac_sha256_hex`    | HMAC-SHA256 | `Result<String, Pbkdf2Error>`  |
| `pbkdf2_hmac_sha512_hex`    | HMAC-SHA512 | `Result<String, Pbkdf2Error>`  |

## Errors

- `Pbkdf2Error::EmptyPassword` — password is empty
- `Pbkdf2Error::InvalidIterations` — iterations is zero
- `Pbkdf2Error::InvalidKeyLength` — key_length is zero

## Stack Position

KD01. Depends on `rust/hmac` (HF05).
