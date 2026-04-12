# coding_adventures_des

DES (Data Encryption Standard) and Triple DES block cipher — FIPS 46-3 / NIST SP 800-67.

## What Is DES?

DES was published by NIST in 1977. It uses a 56-bit key and is completely broken by modern hardware. Despite this, it remains a vital educational subject for understanding Feistel networks, S-boxes, key schedules, and why 3DES was needed.

## Usage

```ruby
require 'coding_adventures_des'

key   = ["133457799BBCDFF1"].pack("H*")
plain = ["0123456789ABCDEF"].pack("H*")

cipher = CodingAdventures::Des.des_encrypt_block(plain, key)
puts cipher.unpack1("H*").upcase  # → "85E813540F0AB405"

recovered = CodingAdventures::Des.des_decrypt_block(cipher, key)
puts recovered.unpack1("H*").upcase  # → "0123456789ABCDEF"
```

### ECB Mode

```ruby
key   = ["0133457799BBCDFF"].pack("H*")
plain = "Hello, DES!"

ct = CodingAdventures::Des.des_ecb_encrypt(plain, key)
pt = CodingAdventures::Des.des_ecb_decrypt(ct, key)
puts pt  # → "Hello, DES!"
```

### Triple DES (3DES / TDEA)

```ruby
k1 = ["0123456789ABCDEF"].pack("H*")
k2 = ["23456789ABCDEF01"].pack("H*")
k3 = ["456789ABCDEF0123"].pack("H*")
plain = ["6BC1BEE22E409F96"].pack("H*")

cipher = CodingAdventures::Des.tdea_encrypt_block(plain, k1, k2, k3)
# cipher.unpack1("H*").upcase → "3B6423D418DEFC23"
```

## API

| Method | Description |
|---|---|
| `Des.expand_key(key)` | Derive 16 round subkeys |
| `Des.des_encrypt_block(block, key)` | Encrypt one 8-byte block |
| `Des.des_decrypt_block(block, key)` | Decrypt one 8-byte block |
| `Des.des_ecb_encrypt(plain, key)` | ECB mode with PKCS#7 padding |
| `Des.des_ecb_decrypt(cipher, key)` | ECB mode decryption |
| `Des.tdea_encrypt_block(block, k1, k2, k3)` | 3DES EDE encrypt |
| `Des.tdea_decrypt_block(block, k1, k2, k3)` | 3DES EDE decrypt |

## Security Warning

**Do not use DES or 3DES to protect real data.** DES is broken (56-bit key). 3DES was deprecated by NIST in 2017 and disallowed in 2023.

## References

- FIPS 46-3 (withdrawn 2005)
- NIST SP 800-67: Triple Data Encryption Algorithm
