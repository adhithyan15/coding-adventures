# CodingAdventures.Sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Elixir.

## What Is SHA-256?

SHA-256 is a member of the SHA-2 family designed by the NSA and published by NIST in 2001. It produces a 256-bit (32-byte) digest and is the workhorse of modern cryptography -- used in TLS, Bitcoin, git, code signing, and password hashing.

## API

### One-shot Functions

```elixir
alias CodingAdventures.Sha256

# Returns 32-byte binary
Sha256.sha256("abc")

# Returns 64-character hex string
Sha256.sha256_hex("abc")
# "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
```

### Streaming Hasher

```elixir
alias CodingAdventures.Sha256.Hasher

h = Hasher.new()
    |> Hasher.update("ab")
    |> Hasher.update("c")

Hasher.hex_digest(h)
# "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

# Branching with copy
base = Hasher.new() |> Hasher.update("common")
h1 = Hasher.copy(base) |> Hasher.update("A")
h2 = Hasher.copy(base) |> Hasher.update("B")
```

## Algorithm

SHA-256 follows the Merkle-Damgard construction with 8 x 32-bit state words, 64 rounds per 64-byte block, and a non-linear message schedule using sigma0/sigma1 rotation functions.

## Dependencies

None. Pure Elixir, no external dependencies.

## How It Fits

Part of the `coding-adventures` monorepo hash function family (MD5, SHA-1, SHA-256).
