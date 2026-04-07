# coding-adventures/go/hmac

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Go.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag proving both
**message integrity** (the message was not altered) and **authenticity** (the sender knows the key).
It is used in TLS 1.2/1.3, JWT (HS256/HS512), WPA2, TOTP/HOTP, and AWS Signature V4.

## Why Not `hash(key || message)`?

Naively prepending the key is vulnerable to **length extension attacks** on Merkle-Damgård hashes
(MD5, SHA-1, SHA-256, SHA-512). An attacker who knows `hash(key || message)` can compute
`hash(key || message || padding || extra)` without knowing `key`.

HMAC defeats this with two nested hash calls under different padded keys:

```
HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
```

where `ipad = 0x36` and `opad = 0x5C`, repeated to the hash function's block size.

## API

```go
import hmac "github.com/adhithyan15/coding-adventures/code/packages/go/hmac"
import "bytes"

key := bytes.Repeat([]byte{0x0b}, 20)

// Named variants return []byte
tag := hmac.HmacSHA256(key, []byte("Hi There"))

// Hex variants return string
hex := hmac.HmacSHA256Hex(key, []byte("Hi There"))
// "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

// MD5, SHA-1, SHA-512 variants
hmac.HmacMD5(key, msg)    // 16 bytes
hmac.HmacSHA1(key, msg)   // 20 bytes
hmac.HmacSHA256(key, msg) // 32 bytes
hmac.HmacSHA512(key, msg) // 64 bytes

// Generic function: bring your own hash
tag = hmac.HMAC(myHashFn, 64, key, msg)
```

## Algorithm (RFC 2104 §2)

```
1. If len(key) > block_size:  key = H(key)
2. Pad key with 0x00 to exactly block_size bytes
3. inner_key = key XOR (0x36 × block_size)
4. outer_key = key XOR (0x5C × block_size)
5. inner     = H(inner_key || message)
6. return      H(outer_key || inner)
```

## Block and Digest Sizes

| Hash    | Block (bytes) | Digest (bytes) |
|---------|--------------|----------------|
| MD5     | 64           | 16             |
| SHA-1   | 64           | 20             |
| SHA-256 | 64           | 32             |
| SHA-512 | 128          | 64             |

## Dependencies

- `github.com/adhithyan15/coding-adventures/code/packages/go/md5`
- `github.com/adhithyan15/coding-adventures/code/packages/go/sha1`
- `github.com/adhithyan15/coding-adventures/code/packages/go/sha256`
- `github.com/adhithyan15/coding-adventures/code/packages/go/sha512`

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
