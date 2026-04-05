# coding-adventures-reed-solomon (Lua)

Reed-Solomon error-correcting codes over GF(256) — part of the `coding-adventures`
math stack (layer MA02). Builds on MA01 (gf256) to provide systematic encoding,
syndrome computation, and full decoding with Berlekamp-Massey + Chien + Forney.

## What It Does

Reed-Solomon (RS) is a block error-correcting code invented in 1960. It adds
redundancy bytes to a message so that even if some bytes are corrupted in transit,
the original can be recovered.

Real-world uses:

- **QR codes** — Up to 30% of the symbol can be scratched and still decoded.
- **CDs / DVDs** — CIRC two-level RS corrects scratches and burst errors.
- **Hard drives** — Firmware sector-level error correction.
- **Voyager probes** — Images sent across 20+ billion km of lossy radio.
- **RAID-6** — The two parity drives ARE an (n, n-2) RS code over GF(256).

## Stack Position

```
MA03 — qr-encoder       (uses reed-solomon)
MA02 — reed-solomon     ← you are here
MA01 — gf256            (GF(2^8) field arithmetic)
MA00 — polynomial       (conceptual foundation)
```

## Quick Start

```lua
local rs = require("coding_adventures.reed_solomon")

-- Encode a 4-byte message with 2 check bytes (can correct 1 error)
local msg      = {4, 3, 2, 1}
local codeword = rs.encode(msg, 2)
-- codeword = {4, 3, 2, 1, check0, check1}  (6 bytes)

-- Corrupt one byte
codeword[2] = 0xFF

-- Decode and recover original
local recovered = rs.decode(codeword, 2)
-- recovered = {4, 3, 2, 1}

-- Inspect syndromes of a valid codeword (all zeros means no errors)
local clean = rs.encode({10, 20, 30}, 4)
local s = rs.syndromes(clean, 4)
-- s = {0, 0, 0, 0}

-- Build the generator polynomial for n_check=2
local g = rs.build_generator(2)
-- g = {8, 6, 1}  (little-endian: g(x) = x² + 6x + 8)
```

## API Reference

| Function | Description |
|----------|-------------|
| `encode(message, n_check)` | Systematic RS encoding. Returns table of `#message + n_check` bytes. |
| `decode(received, n_check)` | Decode and correct up to `t = n_check/2` byte errors. Returns message bytes. |
| `syndromes(received, n_check)` | Compute `n_check` syndrome values. All zero = no errors. |
| `build_generator(n_check)` | Monic generator polynomial in little-endian form, length `n_check+1`. |
| `error_locator(syndromes)` | Berlekamp-Massey algorithm. Returns Λ(x) in little-endian form, Λ[1]=1. |

**Constants:**

| Name | Value | Meaning |
|------|-------|---------|
| `VERSION` | `"0.1.0"` | Package version |

### Error conventions

| Error string prefix | Meaning |
|--------------------|---------|
| `"TooManyErrors: ..."` | More than `t = n_check/2` errors present; unrecoverable. |
| `"InvalidInput: ..."` | Bad `n_check` (0 or odd), or total length > 255, or `received` too short. |

## Correction Capacity

| n_check | t (errors correctable) | Overhead |
|---------|------------------------|---------|
| 2       | 1                      | 2 bytes per k bytes |
| 4       | 2                      | 4 bytes per k bytes |
| 8       | 4                      | 8 bytes per k bytes |
| 16      | 8                      | 16 bytes per k bytes |
| 32      | 16                     | 32 bytes per k bytes |

## Polynomial Conventions

There are two polynomial representations used internally:

1. **Codewords (big-endian):** index 1 = highest-degree coefficient.
   `[c_{n-1}, c_{n-2}, ..., c_1, c_0]`

2. **Internal polynomials (little-endian):** index 1 = constant term.
   `[p_0, p_1, p_2, ...]` where `p(x) = p_0 + p_1·x + p_2·x² + ...`

The generator, error locator Λ(x), and error evaluator Ω(x) all use
little-endian form. The codeword byte arrays are big-endian.

## Cross-Language Test Vector

`build_generator(2)` must return `{8, 6, 1}` in all language implementations:

```
g(x) = (x + α¹)(x + α²) = (x + 2)(x + 4) = x² + 6x + 8
LE: [8, 6, 1]
```

## How the Decoder Works

```
received bytes
     │
     ▼  Step 1: Syndromes  S_j = received(α^j) for j=1..n_check
     │          All zero → return message (no errors)
     │
     ▼  Step 2: Berlekamp-Massey → error locator Λ(x)
     │          deg(Λ) > t → TooManyErrors
     │
     ▼  Step 3: Chien search → error positions
     │          |positions| ≠ deg(Λ) → TooManyErrors
     │
     ▼  Step 4: Forney algorithm → error magnitudes
     │          e_p = Ω(X_p⁻¹) / Λ'(X_p⁻¹)
     │
     ▼  Step 5: XOR magnitudes at error positions
     │
     ▼  Return first k bytes (strip check bytes)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

With mise:

```bash
~/.local/share/mise/shims/busted tests/ --verbose --pattern=test_
```

## Dependencies

- Lua ≥ 5.4 (uses `//` for integer division, `~` for XOR, `<<` for bit shift)
- `coding-adventures-gf256` (MA01) — GF(256) field arithmetic

## License

MIT
