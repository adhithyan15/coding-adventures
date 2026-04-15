# coding_adventures_pbkdf2 (Elixir)

PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018 — implemented from scratch in Elixir.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.

## Usage

```elixir
alias CodingAdventures.Pbkdf2

dk = Pbkdf2.pbkdf2_hmac_sha256(
  "correct horse battery staple",
  :crypto.strong_rand_bytes(16),  # 16 random bytes per user
  600_000,                        # OWASP 2023 minimum for SHA-256
  32
)
```

## API

| Function                    | PRF         | Returns       |
|-----------------------------|-------------|---------------|
| `pbkdf2_hmac_sha1/4`        | HMAC-SHA1   | binary        |
| `pbkdf2_hmac_sha256/4`      | HMAC-SHA256 | binary        |
| `pbkdf2_hmac_sha512/4`      | HMAC-SHA512 | binary        |
| `pbkdf2_hmac_sha1_hex/4`    | HMAC-SHA1   | hex string    |
| `pbkdf2_hmac_sha256_hex/4`  | HMAC-SHA256 | hex string    |
| `pbkdf2_hmac_sha512_hex/4`  | HMAC-SHA512 | hex string    |

## Stack Position

KD01. Depends on `coding_adventures_hmac` (HF05).
