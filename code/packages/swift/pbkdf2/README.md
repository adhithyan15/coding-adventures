# swift/pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```swift
import PBKDF2

let dk = try pbkdf2HmacSHA256(
    password: Data("correct horse battery staple".utf8),
    salt: Data((0..<16).map { _ in UInt8.random(in: 0...255) }),
    iterations: 600_000,   // OWASP 2023 minimum for SHA-256
    keyLength: 32
)
```

## API

| Function                | PRF         | Returns  |
|-------------------------|-------------|----------|
| `pbkdf2HmacSHA1`        | HMAC-SHA1   | `Data`   |
| `pbkdf2HmacSHA256`      | HMAC-SHA256 | `Data`   |
| `pbkdf2HmacSHA512`      | HMAC-SHA512 | `Data`   |
| `pbkdf2HmacSHA1Hex`     | HMAC-SHA1   | `String` |
| `pbkdf2HmacSHA256Hex`   | HMAC-SHA256 | `String` |
| `pbkdf2HmacSHA512Hex`   | HMAC-SHA512 | `String` |

All functions throw `PBKDF2Error` for invalid inputs.

## Stack Position

KD01. Depends on `swift/hmac` (HF05).
