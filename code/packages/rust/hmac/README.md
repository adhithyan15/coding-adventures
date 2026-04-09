# coding_adventures_hmac

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Rust.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** (the message was not altered) and **authenticity** (the sender knows the key).
It is used in TLS 1.2/1.3, JWT (HS256/HS512), WPA2, TOTP/HOTP, and AWS Signature V4.

## Why Not `hash(key || message)`?

Naively prepending the key is vulnerable to **length extension attacks** on Merkle-Damgård hashes
(MD5, SHA-1, SHA-256, SHA-512). An attacker who knows `hash(key || message)` can compute
`hash(key || message || padding || extra)` without knowing `key`, because the hash function's
internal state is fully recoverable from the digest.

HMAC defeats this with two nested hash calls under different padded keys:

```
HMAC(K, M) = H((K' ⊕ opad) || H((K' ⊕ ipad) || M))
```

where `ipad = 0x36` and `opad = 0x5C`, each repeated to the hash function's block size.

## API

### Named Variants (bytes)

```rust
use coding_adventures_hmac::{hmac_md5, hmac_sha1, hmac_sha256, hmac_sha512};

let key = b"secret";
let msg = b"hello";

let tag_md5    = hmac_md5(key, msg);    // [u8; 16]
let tag_sha1   = hmac_sha1(key, msg);   // [u8; 20]
let tag_sha256 = hmac_sha256(key, msg); // [u8; 32]
let tag_sha512 = hmac_sha512(key, msg); // [u8; 64]
```

### Hex Variants

```rust
use coding_adventures_hmac::hmac_sha256_hex;

let hex = hmac_sha256_hex(b"secret", b"hello");
// "88aab3ede8d3adf94d26ab90d3bafd4a2083070c3bcce9c014ee04a443847c0b"
```

### Generic Function

```rust
use coding_adventures_hmac::hmac;
use coding_adventures_sha256::sha256;

// Wrap sha256 to convert [u8; 32] → Vec<u8>
let tag = hmac(|d| sha256(d).to_vec(), 64, b"key", b"message");
```

## RFC 4231 Test Vector (TC1, HMAC-SHA256)

```rust
use coding_adventures_hmac::hmac_sha256_hex;

let key = vec![0x0bu8; 20];
assert_eq!(
    hmac_sha256_hex(&key, b"Hi There"),
    "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
);
```

## Algorithm (RFC 2104 §2)

```
1. If len(key) > block_size:  key = H(key)
2. Pad key with 0x00 bytes to exactly block_size bytes
3. inner_key = key ⊕ (ipad × block_size)   [each byte XOR 0x36]
4. outer_key = key ⊕ (opad × block_size)   [each byte XOR 0x5C]
5. inner     = H(inner_key || message)
6. return      H(outer_key || inner)
```

## Block and Digest Sizes

| Hash   | Block (bytes) | Digest (bytes) |
|--------|--------------|----------------|
| MD5    | 64           | 16             |
| SHA-1  | 64           | 20             |
| SHA-256| 64           | 32             |
| SHA-512| 128          | 64             |

## Dependencies

- `coding_adventures_md5`
- `coding_adventures_sha1`
- `coding_adventures_sha256`
- `coding_adventures_sha512`

All are pure-Rust, zero-external-dependency implementations in this monorepo.

## How It Fits

HMAC is the next layer above the raw hash functions in the `coding-adventures` cryptography stack:

```
MD5 / SHA-1 / SHA-256 / SHA-512
         ↓
       HMAC  ← you are here
         ↓
    PBKDF2 / HKDF  (next)
         ↓
     Vault / JWT / TOTP
```
