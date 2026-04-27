# CodingAdventures.ChaCha20Poly1305

ChaCha20-Poly1305 Authenticated Encryption with Associated Data (AEAD) — Elixir implementation of RFC 8439.

## What It Does

ChaCha20-Poly1305 combines two algorithms into a single AEAD construction:

- **ChaCha20** — a stream cipher based on the Salsa20 family. It generates a pseudorandom keystream from a 256-bit key, 96-bit nonce, and 32-bit counter, then XORs the keystream with the plaintext to produce ciphertext.

- **Poly1305** — a one-time MAC that evaluates a polynomial over the prime field GF(2^130 - 5). It authenticates both the ciphertext and any additional data (AAD), ensuring an attacker cannot tamper with either.

Together, they provide confidentiality, integrity, and authentication in a single pass — the standard definition of AEAD.

## Where It Fits

ChaCha20-Poly1305 is the primary symmetric cipher in TLS 1.3 (RFC 8446), WireGuard, OpenSSH, and many other modern protocols. It was designed as an alternative to AES-GCM that:

1. Performs well on CPUs without AES-NI hardware acceleration (mobile, embedded, IoT).
2. Is immune to cache-timing side-channel attacks (pure arithmetic, no table lookups).
3. Is simpler to implement correctly — AES-GCM misuse (nonce reuse, truncated tags) is catastrophically common.

## Usage

```elixir
alias CodingAdventures.ChaCha20Poly1305, as: CC

# Generate a key (once, stored securely) and a fresh nonce per message
key   = :crypto.strong_rand_bytes(32)   # 256-bit key
nonce = :crypto.strong_rand_bytes(12)   # 96-bit nonce — NEVER REUSE per key!

# Encrypt with optional associated data (authenticated, not encrypted)
aad = "v1:packet-header"
{ciphertext, tag} = CC.aead_encrypt("Hello, secret!", key, nonce, aad)

# Decrypt — returns {:ok, plaintext} or {:error, :authentication_failed}
case CC.aead_decrypt(ciphertext, key, nonce, aad, tag) do
  {:ok, plaintext}                  -> IO.puts("Decrypted: #{plaintext}")
  {:error, :authentication_failed} -> IO.puts("Authentication failed!")
end
```

### Low-Level API

```elixir
# Generate a single 64-byte ChaCha20 keystream block
block = CC.chacha20_block(key, counter, nonce)

# Encrypt/decrypt raw data (no authentication — use AEAD in production!)
ciphertext = CC.chacha20_encrypt(plaintext, key, nonce)
plaintext  = CC.chacha20_encrypt(ciphertext, key, nonce)  # XOR twice = identity

# Compute a Poly1305 MAC tag
tag = CC.poly1305_mac(message, poly_key)
```

## Security Notes

- **Never reuse a (key, nonce) pair.** Nonce reuse with Poly1305 leaks the authentication key and breaks confidentiality. For random nonces, 96 bits provides a birthday bound collision probability of ~1/(2^48) per 2^32 messages — acceptable for most use cases.
- **This package is for education.** For production Elixir, use `:crypto.crypto_one_time_aead(:chacha20_poly1305, key, nonce, plaintext, aad, true)`.
- Tag verification uses constant-time comparison to prevent timing oracle attacks.

## Algorithm Overview

```
Key (256-bit) + Nonce (96-bit) + Counter (32-bit)
                    │
            chacha20_block()
           /                \
   counter=0                counter=1,2,...
   (derive Poly1305 key)    (encrypt plaintext)
          │                         │
     poly_key (256-bit)        ciphertext
          │                         │
   poly1305_mac(pad16(aad) || pad16(ct) || len64(aad) || len64(ct))
          │
        tag (128-bit)
```

## Testing

```
mix test --cover
```

All tests verified against RFC 8439 official test vectors and Erlang OTP `:crypto` reference implementation.

## References

- [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439) — ChaCha20 and Poly1305 for IETF Protocols
- [Bernstein, 2008](https://cr.yp.to/chacha/chacha-20080128.pdf) — ChaCha, a variant of Salsa20
- [Bernstein, 2005](https://cr.yp.to/mac/poly1305-20050329.pdf) — The Poly1305-AES message-authentication code
