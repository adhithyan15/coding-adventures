# coding_adventures_hmac

HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1 — implemented from scratch in Elixir with no external dependencies beyond the other coding-adventures hash packages.

## What Is HMAC?

HMAC takes a secret key and a message and produces a fixed-size authentication tag that proves:
1. **Integrity** — the message has not been altered
2. **Authenticity** — the sender knows the secret key

It is the foundation of JWT signatures (`HS256`, `HS512`), TLS 1.2 PRF, WPA2 authentication, TOTP/HOTP one-time passwords, and AWS Signature V4.

## Why Not `hash(key || message)`?

A naive approach is vulnerable to **length extension attacks**: anyone who sees `hash(key || message)` can compute `hash(key || message || extra)` without knowing the key, because Merkle-Damgård hash functions resume from the last state.

HMAC defeats this with two nested hash calls using different padded keys:

```
HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
```

An attacker cannot extend the message because they would need to break into the outer hash without knowing `opad_key`.

## Usage

```elixir
alias CodingAdventures.Hmac

# Named variants — most convenient
Hmac.hmac_sha256("my-secret-key", "message to authenticate")
# => <<...32 bytes...>>

Hmac.hmac_sha256_hex("my-secret-key", "message to authenticate")
# => "6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a"

# Available variants
Hmac.hmac_md5(key, msg)     # 16-byte tag (legacy)
Hmac.hmac_sha1(key, msg)    # 20-byte tag (WPA2, SSH)
Hmac.hmac_sha256(key, msg)  # 32-byte tag (modern standard)
Hmac.hmac_sha512(key, msg)  # 64-byte tag (high-security)

# Hex-string versions
Hmac.hmac_md5_hex(key, msg)
Hmac.hmac_sha1_hex(key, msg)
Hmac.hmac_sha256_hex(key, msg)
Hmac.hmac_sha512_hex(key, msg)

# Generic — bring your own hash function
Hmac.hmac(&CodingAdventures.Sha256.sha256/1, 64, key, msg)
```

## Where It Fits

```
D18 Chief of Staff (vault encryption)
         |
         v
      HMAC (this package)
         |
    ┌────┼────┬────────┐
    v    v    v        v
   MD5  SHA1 SHA256  SHA512
```

HMAC is a direct dependency of PBKDF2 (KD01), scrypt (KD02), and HKDF (future), all of which are needed for the vault's key derivation stack.

## Dependencies

- `coding_adventures_md5` — for `hmac_md5`
- `coding_adventures_sha1` — for `hmac_sha1`
- `coding_adventures_sha256` — for `hmac_sha256`
- `coding_adventures_sha512` — for `hmac_sha512`

## Running Tests

```sh
mix deps.get && mix test --cover
```

All 7 RFC 4231 test vectors pass for HMAC-SHA256 and HMAC-SHA512. RFC 2202 vectors pass for HMAC-MD5 and HMAC-SHA1. Coverage: 100%.
