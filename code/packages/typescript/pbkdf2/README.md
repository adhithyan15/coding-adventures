# @coding-adventures/pbkdf2

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in TypeScript.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```typescript
import { pbkdf2HmacSHA256 } from "@coding-adventures/pbkdf2";

const dk = pbkdf2HmacSHA256(
  new TextEncoder().encode("correct horse battery staple"),
  crypto.getRandomValues(new Uint8Array(16)),  // 16 random bytes per user
  600_000,  // OWASP 2023 minimum for SHA-256
  32,
);
```

## API

| Function               | PRF         | Returns      |
|------------------------|-------------|--------------|
| `pbkdf2HmacSHA1`       | HMAC-SHA1   | `Uint8Array` |
| `pbkdf2HmacSHA256`     | HMAC-SHA256 | `Uint8Array` |
| `pbkdf2HmacSHA512`     | HMAC-SHA512 | `Uint8Array` |
| `pbkdf2HmacSHA1Hex`    | HMAC-SHA1   | `string`     |
| `pbkdf2HmacSHA256Hex`  | HMAC-SHA256 | `string`     |
| `pbkdf2HmacSHA512Hex`  | HMAC-SHA512 | `string`     |

## Stack Position

KD01. Depends on `@coding-adventures/hmac` (HF05).
