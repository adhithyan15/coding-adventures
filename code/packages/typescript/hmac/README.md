# @coding-adventures/hmac

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in TypeScript.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** (the message was not altered) and **authenticity** (the sender knows the key).
It is used in TLS 1.2/1.3, JWT (HS256/HS512), WPA2, TOTP/HOTP, and AWS Signature V4.

## Why Not `hash(key || message)`?

Naively prepending the key is vulnerable to **length extension attacks** on Merkle-Damgård hashes
(MD5, SHA-1, SHA-256, SHA-512). HMAC defeats this with two nested hash calls:

```
HMAC(K, M) = H((K' ⊕ opad) || H((K' ⊕ ipad) || M))
```

## API

```ts
import {
  hmacSHA256Hex, hmacSHA512Hex,
  hmacMD5Hex, hmacSHA1Hex,
  hmacSHA256, hmacSHA512,
} from "@coding-adventures/hmac";

const enc = new TextEncoder();
const key = new Uint8Array(20).fill(0x0b);

// Hex strings (most common use case)
hmacSHA256Hex(key, enc.encode("Hi There"));
// "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

// Raw bytes
const tag: Uint8Array = hmacSHA256(key, enc.encode("Hi There"));

// Generic function — bring your own hash
import { sha256 } from "@coding-adventures/sha256";
import { hmac } from "@coding-adventures/hmac";
const tag2 = hmac(sha256, 64, key, enc.encode("Hi There"));
```

## Block and Digest Sizes

| Hash    | Block (bytes) | Digest (bytes) | Hex length |
|---------|--------------|----------------|-----------|
| MD5     | 64           | 16             | 32        |
| SHA-1   | 64           | 20             | 40        |
| SHA-256 | 64           | 32             | 64        |
| SHA-512 | 128          | 64             | 128       |

## Dependencies

- `@coding-adventures/md5`
- `@coding-adventures/sha1`
- `@coding-adventures/sha256`
- `@coding-adventures/sha512`

## How It Fits

```
md5 / sha1 / sha256 / sha512
         ↓
       hmac  ← you are here
         ↓
    pbkdf2 / hkdf  (next)
         ↓
     vault / jwt / totp
```
