# HKDF (Elixir)

HMAC-based Extract-and-Expand Key Derivation Function, implementing [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869).

## What is HKDF?

HKDF derives cryptographic keys from input keying material (IKM) in two stages:

1. **Extract** — condenses IKM into a fixed-length pseudorandom key (PRK) using HMAC
2. **Expand** — stretches the PRK into output keying material (OKM) of any desired length

HKDF is used in TLS 1.3, Signal Protocol, WireGuard, and many other protocols.

## API

```elixir
alias CodingAdventures.Hkdf

# Full HKDF (extract + expand combined)
okm = Hkdf.hkdf(salt, ikm, info, length, :sha256)

# Separate stages
prk = Hkdf.extract(salt, ikm, :sha256)
okm = Hkdf.expand(prk, info, length, :sha256)

# Hex-output variants
hex = Hkdf.hkdf_hex(salt, ikm, info, length, :sha256)
hex = Hkdf.extract_hex(salt, ikm, :sha256)
hex = Hkdf.expand_hex(prk, info, length, :sha256)
```

All functions accept `:sha256` (default) or `:sha512` as the hash algorithm.

## Dependencies

- `coding_adventures_hmac` (which depends on SHA-256, SHA-512, MD5, SHA-1)

## Running Tests

```bash
mix deps.get --quiet && mix test --cover
```
