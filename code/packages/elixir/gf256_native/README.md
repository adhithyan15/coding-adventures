# CodingAdventures.GF256Native

Elixir NIF wrapping the Rust `gf256` crate for GF(256) Galois Field arithmetic.
Uses the zero-dependency `erl-nif-bridge` — no Rustler, no `erl_nif.h` required.

## Where it fits

```
CodingAdventures.GF256Native    (this package — Elixir NIF)
         │
         └── erl-nif-bridge (Rust)  ──── Erlang/OTP NIF C API
         └── gf256 (Rust)           ──── core arithmetic (log/antilog tables)
```

## Usage

```elixir
alias CodingAdventures.GF256Native, as: GF

GF.add(83, 202)          # => 153  (XOR)
GF.multiply(2, 16)       # => 32
GF.divide(4, 2)          # => 2
GF.power(2, 8)           # => 29   (reduced mod 0x11D)
GF.inverse(83)           # => ?    (such that 83 * ? = 1 in GF256)
```

## Building

```bash
bash BUILD
```
