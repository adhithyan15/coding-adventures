# @coding-adventures/hkdf

HKDF (HMAC-based Extract-and-Expand Key Derivation Function, RFC 5869) implemented from scratch in TypeScript.

## What It Does

HKDF transforms raw input keying material (from Diffie-Hellman exchanges, passwords, etc.) into cryptographically strong keys. It operates in two phases:

1. **Extract** -- compress non-uniform input into a fixed-size pseudorandom key (PRK)
2. **Expand** -- stretch the PRK into any number of output bytes, with domain separation via an "info" string

HKDF is the key derivation function used in TLS 1.3, Signal Protocol, WireGuard, and the Web Crypto API.

## Usage

```typescript
import { hkdf, hkdfExtract, hkdfExpand } from "@coding-adventures/hkdf";

// Full HKDF: extract-then-expand in one call
const key = hkdf(salt, inputKeyingMaterial, info, 32); // 32-byte key

// Or use the phases separately for multiple derived keys
const prk = hkdfExtract(salt, inputKeyingMaterial);
const encKey = hkdfExpand(prk, new TextEncoder().encode("enc"), 32);
const macKey = hkdfExpand(prk, new TextEncoder().encode("mac"), 32);

// SHA-512 variant
const key512 = hkdf(salt, ikm, info, 64, "sha512");
```

## How It Fits

This package builds on `@coding-adventures/hmac`, which in turn uses the SHA-256 and SHA-512 packages. It sits in the cryptographic stack between HMAC (authentication) and higher-level protocols like TLS key schedules and PBKDF2.

## API

- `hkdfExtract(salt, ikm, hash?)` -- Extract phase. Returns PRK (HashLen bytes).
- `hkdfExpand(prk, info, length, hash?)` -- Expand phase. Returns OKM (length bytes).
- `hkdf(salt, ikm, info, length, hash?)` -- Combined extract-then-expand.

Hash defaults to `"sha256"`. Also accepts `"sha512"`.
