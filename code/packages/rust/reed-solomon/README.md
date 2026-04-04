# reed-solomon

Reed-Solomon error-correcting codes over GF(256).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## What It Does

Reed-Solomon is a **block error-correcting code**: you add `n_check` redundancy
bytes to a message, and the decoder can recover the original data even if up to
`t = n_check / 2` bytes are corrupted in transit.

```
[  message bytes  |  check bytes  ]
 ←──── k bytes ───→←── n_check ──→
         ↑ systematic: message bytes are unchanged
```

**Correction capacity**:

| `n_check` | `t` (errors correctable) |
|-----------|--------------------------|
| 2 | 1 |
| 4 | 2 |
| 8 | 4 |
| 16 | 8 |
| 32 | 16 |

## Where RS Is Used

| System | How RS Helps |
|--------|-------------|
| QR codes | Up to 30% of a QR symbol can be scratched/obscured and still decode |
| CDs / DVDs | CIRC double-layer RS corrects bursts from scratches |
| Hard drives | Firmware error correction for sector-level faults |
| Voyager probes | Transmit images across 20+ billion km with near-zero error rates |
| RAID-6 | The two parity drives are exactly an (n, n-2) RS code |
| DSL / LTE | Outer RS code + inner convolutional code for reliable links |

## Quick Start

```rust
use reed_solomon::{encode, decode};

let message = b"hello world";
let n_check = 8;  // t = 4 errors correctable

// Encode: appends 8 check bytes
let mut codeword = encode(message, n_check).unwrap();
assert_eq!(codeword.len(), message.len() + n_check);
assert_eq!(&codeword[..message.len()], message);  // systematic

// Simulate 4 byte errors
codeword[0] ^= 0xFF;
codeword[3] ^= 0xAA;
codeword[7] ^= 0x55;
codeword[10] ^= 0x0F;

// Decode: recovers original message despite 4 corruptions
let recovered = decode(&codeword, n_check).unwrap();
assert_eq!(recovered, message);
```

## API

```rust
/// Encode `message` with `n_check` redundancy bytes.
/// Returns codeword = message || check_bytes.
pub fn encode(message: &[u8], n_check: usize) -> Result<Vec<u8>, RSError>

/// Decode a (possibly corrupted) codeword, correcting up to t = n_check/2 errors.
/// Returns the recovered message bytes.
pub fn decode(received: &[u8], n_check: usize) -> Result<Vec<u8>, RSError>

/// Compute the n_check syndromes of a codeword.
/// All-zero → no errors; any non-zero → errors present.
pub fn syndromes(received: &[u8], n_check: usize) -> Vec<u8>

/// Build the RS generator polynomial g(x) = ∏(x + αⁱ) for i=1..n_check.
pub fn build_generator(n_check: usize) -> Result<Vec<u8>, RSError>

/// Compute the error locator polynomial via Berlekamp-Massey.
pub fn error_locator(syndromes: &[u8]) -> Vec<u8>
```

## Error Handling

```rust
pub enum RSError {
    TooManyErrors,          // > t errors in the codeword; unrecoverable
    InvalidInput(String),   // bad n_check or codeword too long
}
```

## How It Works

### The Math Stack

```
MA00  polynomial   — f64 polynomial arithmetic
MA01  gf256        — GF(2^8) field arithmetic (add=XOR, multiply=table lookup)
MA02  reed-solomon — RS codes built on top of gf256
```

### Encoding

1. **Build generator** `g(x) = (x+α¹)(x+α²)…(x+α^{n_check})` — degree `n_check`, monic, coefficients in GF(256).
2. **Left-shift** message: `m(x) · x^{n_check}` (prepend `n_check` zero bytes).
3. **Divide**: remainder `r(x) = m(x)·x^{n_check} mod g(x)`.
4. **Codeword**: `c = message || r` (the remainder fills the trailing slots).

The codeword `c(x)` is divisible by `g(x)`, so it evaluates to zero at all roots `α¹, …, α^{n_check}`. This is the zero-syndrome condition the decoder checks.

### Decoding (5-step pipeline)

```
received
  │
  ▼  Step 1: Compute syndromes Sᵢ = r(αⁱ)
  │          All zero? → return message bytes directly (no errors)
  │
  ▼  Step 2: Berlekamp-Massey → error locator Λ(x)
  │          deg(Λ) > t? → TooManyErrors
  │
  ▼  Step 3: Chien search → error positions {i₁ … iᵥ}
  │          found ≠ deg(Λ)? → TooManyErrors
  │
  ▼  Step 4: Forney algorithm → error magnitudes {e₁ … eᵥ}
  │
  ▼  Step 5: Correct: received[iₖ] XOR= eₖ
  │
  ▼  Return first k bytes (strip check bytes)
```

**Berlekamp-Massey**: treats syndromes as an LFSR output sequence and finds the
shortest LFSR that generates it. The connection polynomial is Λ(x).

**Chien search**: evaluates Λ(α⁻ⁱ) = Λ(α^{255-i}) for every position `i`. A root
means position `i` is an error location.

**Forney algorithm**: computes correction bytes via
`eₖ = Ω(Xₖ⁻¹) / Λ'(Xₖ⁻¹)` where Ω = (S · Λ) mod x^{2t} and Λ' is the formal
derivative of Λ.

## Constraints

- `n_check` must be **even** and **≥ 2** (odd values → no integer `t`).
- `message.len() + n_check ≤ 255` (GF(256) operates over 255-element field; the
  all-zero element is excluded from the codeword alphabet).

## Dependencies

- `gf256` (MA01) — all coefficient arithmetic is GF(256) field operations.

## Specification

See `code/specs/MA02-reed-solomon.md` for the full specification including
worked examples, algorithm derivations, and test vectors.
