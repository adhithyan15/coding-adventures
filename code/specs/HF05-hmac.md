# HF05 — HMAC (Hash-based Message Authentication Code)

## Overview

HMAC is a construction for computing a message authentication code (MAC)
using a cryptographic hash function. Defined in RFC 2104 (1997), HMAC
provides both integrity and authentication — proving that a message
hasn't been tampered with AND was created by someone who knows the secret
key.

HMAC is generic over the hash function: HMAC-MD5, HMAC-SHA1, HMAC-SHA256,
HMAC-SHA512 are all instances of the same construction. This makes it a
natural package that depends on HF01-HF04.

### Why HMAC Instead of hash(key + message)?

Naively computing `hash(key || message)` is vulnerable to **length
extension attacks** (demonstrated in HF01 breaking demos). HMAC defeats
this by using two nested hash calls with different padded keys:

```
HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
```

Where:
- K' = key padded to hash block size (or hashed first if too long)
- ipad = 0x36 repeated to block size
- opad = 0x5C repeated to block size

The double hashing with XOR'd keys prevents state extension.

## Algorithm

```
1. If len(key) > block_size: key = hash(key)
2. If len(key) < block_size: pad key with zeros to block_size
3. inner_key = key XOR ipad (each byte XOR 0x36)
4. outer_key = key XOR opad (each byte XOR 0x5C)
5. inner_hash = hash(inner_key || message)
6. result = hash(outer_key || inner_hash)
```

Block sizes: MD5/SHA-1/SHA-256 = 64 bytes, SHA-512 = 128 bytes.

## Interface Contract

| Function | Signature | Description |
|----------|-----------|-------------|
| `hmac_md5` | `(key: bytes, message: bytes) -> bytes` | HMAC using MD5. |
| `hmac_sha1` | `(key: bytes, message: bytes) -> bytes` | HMAC using SHA-1. |
| `hmac_sha256` | `(key: bytes, message: bytes) -> bytes` | HMAC using SHA-256. |
| `hmac_sha512` | `(key: bytes, message: bytes) -> bytes` | HMAC using SHA-512. |
| `hmac` | `(hash_fn, block_size, key, message) -> bytes` | Generic HMAC with any hash. |
| `*_hex` variants | Same + `-> string` | Hex-encoded versions. |

## Test Vectors (RFC 4231)

```
# Test Case 1
Key:  0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b (20 bytes)
Data: "Hi There"
HMAC-SHA256: b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7

# Test Case 2
Key:  "Jefe"
Data: "what do ya want for nothing?"
HMAC-SHA256: 5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843

# Test Case 3 (key longer than block size)
Key:  aaaa...aa (131 bytes of 0xaa)
Data: "Test Using Larger Than Block-Size Key - Hash Key First"
HMAC-SHA256: 60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54
```

## Package Matrix

Same 9 languages, in `hmac/` directories.

**Dependencies:** HF01 (MD5), HF02 (SHA-1), HF03 (SHA-256), HF04 (SHA-512).
