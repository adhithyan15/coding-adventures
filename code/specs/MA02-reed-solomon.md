# MA02 — Reed-Solomon: Error-Correcting Codes over GF(256)

## Overview

**Reed-Solomon** is a family of block error-correcting codes invented by Irving Reed
and Gustave Solomon in 1960. An RS code adds *redundancy* to a message: extra bytes
computed from the data that allow a decoder to detect and correct corruption even
when some bytes have been damaged or lost in transit.

RS codes are used everywhere reliable data storage or transmission matters:

| System | How RS is Used |
|--------|---------------|
| **QR codes** | Up to 30% of a QR code can be scratched out and still decode. Each version/level combination uses specific RS parameters. |
| **CDs and DVDs** | Two-level RS (CIRC) corrects both random bit flips and burst errors from scratches. |
| **Hard drives** | Firmware uses RS to correct sector-level errors. |
| **Deep-space probes** | Voyager 1 uses RS to send photos across billions of kilometres. |
| **RAID-6** | The two parity drives in RAID-6 are exactly a (n, n-2) RS code over GF(256). |

### Relationship to the MA series

Reed-Solomon builds on the two preceding packages:

```
MA00 polynomial    ← coefficient-array polynomial arithmetic over any field
MA01 gf256         ← GF(2^8) field arithmetic (add = XOR, mul = table lookup)
MA02 reed-solomon  ← RS encoding/decoding: polynomial arithmetic over GF(256)
```

A Reed-Solomon encoder is just polynomial multiplication over GF(256).
A decoder is polynomial GCD (via Berlekamp-Massey) plus evaluation — all operations
already defined in MA00 and MA01, but now composed into a higher-level abstraction.

---

## Code Parameters

An RS code is described by three numbers **[n, k, d]**:

| Symbol | Name | Meaning |
|--------|------|---------|
| `n` | block length | Total symbols in a codeword |
| `k` | message length | Data symbols (bytes you want to send) |
| `n - k` | check symbol count | Redundancy bytes added by the encoder |
| `t` | error correction capacity | Maximum errors that can be corrected: `t = floor((n-k) / 2)` |
| `d = n - k + 1` | minimum distance | Minimum Hamming distance between any two valid codewords |

For GF(256): n ≤ 255 (the field has 256 elements; one is zero, leaving 255 positions).

**This package uses the convention**: `n - k` is even (so `t = (n-k)/2` exactly).
Pass `n_check = n - k` to `encode` and `decode`.

### Capability at a glance

| n_check | t (errors correctable) | Overhead |
|---------|------------------------|---------|
| 2       | 1                      | 2 bytes / k bytes |
| 4       | 2                      | 4 bytes / k bytes |
| 8       | 4                      | 8 bytes / k bytes |
| 16      | 8                      | 16 bytes / k bytes |
| 32      | 16                     | 32 bytes / k bytes |

---

## The Generator Polynomial

The RS encoder and decoder both use a **generator polynomial** `g(x)` whose roots are
consecutive powers of the GF(256) generator `α = 2`:

```
g(x) = (x + α¹)(x + α²)(x + α³) ⋯ (x + α^{n-k})
```

Where:
- `α = 2` (the primitive element of GF(256), same as in MA01)
- All arithmetic on coefficients is in GF(256): addition is XOR, multiplication uses the log/antilog tables
- `x + αⁱ` means `(x - αⁱ)` — in characteristic 2, subtraction equals addition, so `+` and `-` are the same

**Important**: the polynomial product is computed over GF(256). The coefficients of
`g(x)` are GF(256) elements (bytes 0–255), not ordinary integers.

### Generator polynomial construction

Start from `g(x) = [1]` (the constant polynomial 1). Multiply in each linear factor:

```
g = [1]       ← degree 0
for i from 1 to n_check:
    factor = [gf256.multiply(alpha^i, 1), 1]   ← [α^i, 1] = (x + α^i)
    g = poly_multiply_gf256(g, factor)
```

Here `poly_multiply_gf256` is MA00's `multiply` but with GF(256) field arithmetic
for each coefficient product and sum.

### First few generator polynomials

**n_check = 2 (t = 1):** `g(x) = (x + 2)(x + 4)`

```
  [2, 1] × [4, 1]
= x² + (2 XOR 4)x + GF256.multiply(2, 4)
= x² + 6x + 8
```

Coefficient array (little-endian, index = degree): `[8, 6, 1]`

**n_check = 4 (t = 2):** `g(x) = (x+2)(x+4)(x+8)(x+16)`

Continuing from `x² + 6x + 8`, multiply by `(x + 8)`:

```
(x² + 6x + 8)(x + 8)
coefficients computed over GF(256):
  [0] = GF256.mul(8, 8)  = 64
  [1] = GF256.mul(8, 6) XOR GF256.mul(8, 1) = 48 XOR 8 = 56
  [2] = GF256.mul(8, 1) XOR GF256.mul(1, 6) = 8 XOR 6 = 14
  [3] = 1

→ x³ + 14x² + 56x + 64
```

Then multiply by `(x + 16)`:

```
(x³ + 14x² + 56x + 64)(x + 16)
= x⁴ + (16 XOR 14)x³ + (GF256.mul(16,14) XOR 56)x²
       + (GF256.mul(16,56) XOR 64)x + GF256.mul(16,64)
```

(Exact final coefficients are computed by the implementation; the pattern is clear.)

---

## Encoding (Systematic)

**Systematic** means the original message bytes appear unchanged in the output,
followed by the computed check bytes:

```
codeword = [ message bytes | check bytes ]
           ←── k bytes ───→←── n-k bytes ─→
```

### Algorithm

Given message `m = [m₀, m₁, ..., m_{k-1}]` and `n_check`:

1. **Build the generator polynomial** `g(x)` as above (degree = n_check).

2. **Form the shifted message polynomial** by prepending `n_check` zero bytes:
   ```
   shifted = [0, 0, ..., 0, m₀, m₁, ..., m_{k-1}]
              ←n_check zeros→
   ```
   This is `m(x) · x^{n_check}` — shifting the message up to make room for the check bytes.

3. **Divide to get the remainder**:
   ```
   (quotient, remainder) = poly_divmod_gf256(shifted, g)
   ```
   The remainder `r(x)` has degree < n_check (so it has exactly n_check coefficients).

4. **The check bytes are the coefficients of `r(x)`**:
   ```
   check_bytes = [r₀, r₁, ..., r_{n_check - 1}]
   ```

5. **Output**: `message ++ check_bytes`

### Why this works

The codeword polynomial `c(x) = m(x)·x^{n_check} - r(x)` (i.e., `shifted XOR remainder`) is
**exactly divisible by** `g(x)` — because we subtracted the remainder. Since `-r = +r` in
characteristic 2:

```
c(x) = m(x)·x^{n_check} XOR r(x)
c(α^i) = 0  for i = 1, 2, ..., n_check
```

Every valid codeword evaluates to zero at all roots of `g(x)`. The decoder uses
this property to detect and correct errors.

### Encoding worked example

**Parameters**: `n_check = 2`, `message = [4, 3, 2, 1]` (k = 4, n = 6)

**Step 1**: `g(x) = x² + 6x + 8` → coefficients `[8, 6, 1]`

**Step 2**: `shifted = [0, 0, 4, 3, 2, 1]`
(m(x)·x² = 4x² + 3x³ + 2x⁴ + x⁵, represented as `[0, 0, 4, 3, 2, 1]`)

**Step 3**: Polynomial long division over GF(256):
```
Divide [0, 0, 4, 3, 2, 1] by [8, 6, 1]

Iteration 1: leading term x⁵ / x² = x³
  subtract 1·[8, 6, 1]·x³ = [0, 0, 0, 8, 6, 1]
  remainder: [0, 0, 4, 3 XOR 8, 2 XOR 6, 0] = [0, 0, 4, 11, 4]

Iteration 2: leading term 4x⁴ / x² = 4x²
  4·g(x) = [GF256.mul(4,8), GF256.mul(4,6), 4] = [32, 24, 4]
  subtract [0, 0, 32, 24, 4]:
  [0, 0, 4 XOR 32, 11 XOR 24, 4 XOR 4] = [0, 0, 36, 19]

Iteration 3: leading term 19x³ / x² = 19x
  19·g(x) = [GF256.mul(19,8), GF256.mul(19,6), 19] = [152, 106, 19]
  subtract [0, 152, 106, 19]:
  [0, 0 XOR 152, 36 XOR 106, 19 XOR 19] = [0, 152, 78]

Iteration 4: leading term 78x² / x² = 78
  78·g(x) = [GF256.mul(78,8), GF256.mul(78,6), 78]
  Let r₀ = GF256.mul(78,8), r₁ = GF256.mul(78,6)
  subtract [r₀, r₁, 78]:
  [0 XOR r₀, 152 XOR r₁, 78 XOR 78] = [r₀, 152 XOR r₁]

→ remainder = [r₀, 152 XOR r₁]  (2 coefficients, degree < 2 ✓)
```

The exact byte values of `r₀` and `152 XOR r₁` depend on GF(256) multiplication
(use the log/antilog tables from MA01). The reference implementation provides them.
The important structure: a 2-byte remainder that becomes the check bytes.

**Output**: `[4, 3, 2, 1, r₀, 152 XOR r₁]`

---

## Decoding

### High-Level Pipeline

```
received bytes
     │
     ▼
 [1] Compute syndromes S₁ … S_{n_check}
     │
     │ All zero? → no errors, return message bytes
     │
     ▼
 [2] Berlekamp-Massey → error locator polynomial Λ(x)
     │
     ▼
 [3] Chien search → error positions {i₁, i₂, …, iᵥ}
     │
     │ Found more than t positions? → TooManyErrors
     │
     ▼
 [4] Forney algorithm → error magnitudes {e₁, e₂, …, eᵥ}
     │
     ▼
 [5] Correct: received[iₖ] XOR= eₖ for each k
     │
     ▼
 corrected message bytes (strip check bytes)
```

---

### Step 1: Syndrome Computation

The **syndrome** `Sᵢ` is the received polynomial evaluated at the i-th root of `g(x)`:

```
Sᵢ = received(αⁱ)  for i = 1, 2, ..., n_check
```

If the received codeword has no errors, `c(αⁱ) = 0` for all i (because valid codewords
are divisible by `g(x)`). Any non-zero syndrome reveals the presence of errors.

**How to evaluate `received(αⁱ)` using Horner's method:**

Treat the received byte array as a polynomial with the *first byte as the highest-degree term*:

```
received = [r₀, r₁, ..., r_{n-1}]

The polynomial: r(x) = r₀·x^{n-1} + r₁·x^{n-2} + ... + r_{n-1}·x⁰

Horner evaluation at αⁱ:
  acc = 0
  for byte in received:     ← iterating from r₀ to r_{n-1}
      acc = GF256.multiply(acc, alpha_i) XOR byte
  Sᵢ = acc
```

**Note on byte ordering**: The first byte in the array is the *leading* (highest-degree)
coefficient. This is "big-endian" polynomial representation, opposite of MA00's convention.
This is standard for RS implementations; it matches how QR code bytes are laid out.

### Step 2: Berlekamp-Massey Algorithm

Given syndromes `S₁, S₂, ..., S_{2t}`, find the shortest **linear feedback shift register**
(LFSR) that generates the syndrome sequence. The LFSR connection polynomial is `Λ(x)`,
the error locator polynomial.

If errors occurred at positions `i₁, i₂, ..., iᵥ`, the error locators are `Xₖ = αⁱₖ`.
The error locator polynomial is:

```
Λ(x) = ∏ₖ (1 - Xₖ · x)    with  Λ(0) = 1
```

The roots of Λ are `Xₖ⁻¹`, the inverses of the error locators.

**The Berlekamp-Massey algorithm:**

```
Input:  syndromes S[1..2t]   (using 1-based indexing)
Output: error locator Λ(x)

C = [1]       ← current error locator (polynomial with C[0]=1)
B = [1]       ← previous error locator
L = 0         ← current number of errors found
x = 1         ← iterations since last update
b = 1         ← discrepancy at last update

for n from 1 to 2t:

    # Compute the discrepancy at step n
    d = S[n]
    for j from 1 to L:
        d = d XOR GF256.multiply(C[j], S[n - j])

    if d == 0:
        # Syndrome consistent with current Λ — no update needed
        x = x + 1

    elif 2 * L < n:
        # Found more errors than currently modeled — grow Λ
        T    = copy(C)
        scale = GF256.divide(d, b)
        C    = C XOR poly_shift_scale(B, x, scale)
        L    = n - L
        B    = T
        b    = d
        x    = 1

    else:
        # Consistent update — adjust Λ without growing
        scale = GF256.divide(d, b)
        C    = C XOR poly_shift_scale(B, x, scale)
        x    = x + 1

return C     ← Λ(x), length L+1, degree L

# Helper: poly_shift_scale(poly, shift, scalar)
#   Returns scalar · x^shift · poly
#   i.e. [0, 0, ..., 0, scalar·poly[0], scalar·poly[1], ...]
#          ← shift zeros →
```

After the loop, `L` equals the number of errors. If `L > t`, there are too many
errors to correct — return `TooManyErrors`.

### Step 3: Chien Search (Error Positions)

The roots of `Λ(x)` are the inverses of the error locators. To find them, evaluate
`Λ` at every non-zero field element:

```
error_positions = []
for i from 0 to n-1:
    xi_inv = alpha^(255 - i)   ← = GF256.power(2, 255 - i) = GF256.inverse(alpha^i)
    if evaluate_poly_gf256(Λ, xi_inv) == 0:
        error_positions.append(i)
```

If `len(error_positions) != L` (where `L` is the degree of `Λ`), something is
wrong — the received data is too badly corrupted to correct. Return `TooManyErrors`.

**Why `255 - i`**: In GF(256), `αⁱ · α^{255-i} = α²⁵⁵ = 1`, so `α^{255-i}` is
the inverse of `αⁱ`. If `i = 0`, then `α^{255}` = `α⁰` = 1 = inverse of 1.

### Step 4: Forney Algorithm (Error Magnitudes)

Once we know *where* the errors are, we compute *by how much* each symbol was corrupted.

**Step 4a: Compute the error evaluator polynomial Ω(x)**

```
S(x) = S₁ + S₂·x + S₃·x² + ... + S_{2t}·x^{2t-1}   (syndromes as polynomial)

Ω(x) = (S(x) · Λ(x)) mod x^{2t}
```

This uses GF(256) polynomial multiplication (MA00's `multiply` with GF(256) arithmetic),
then keeps only coefficients of degree 0 through 2t-1.

**Step 4b: Compute the formal derivative of Λ**

In characteristic 2, the formal derivative annihilates even-degree terms:

```
if Λ(x) = Λ₀ + Λ₁x + Λ₂x² + Λ₃x³ + Λ₄x⁴ + ...
then Λ'(x) = Λ₁ + Λ₃x² + Λ₅x⁴ + ...
             (only odd-indexed coefficients survive, degree reduced by 1)
```

In code: keep coefficients at odd indices, reduce each index by 1.

```
Λ_prime = []
for i from 1 to degree(Λ) step 2:
    Λ_prime[i - 1] = Λ[i]   ← coefficient of x^(i-1) in Λ'
```

**Step 4c: Compute each error magnitude**

For each error at position `i_k` with locator `X_k = α^{i_k}`:

```
X_k_inv = GF256.power(2, 255 - i_k)   ← X_k⁻¹

e_k = GF256.divide(
    evaluate_poly_gf256(Ω, X_k_inv),
    evaluate_poly_gf256(Λ_prime, X_k_inv)
)
```

### Step 5: Error Correction

```
for k from 0 to len(error_positions) - 1:
    received[error_positions[k]] = received[error_positions[k]] XOR e_k
```

Return the first `k` bytes of the corrected codeword (strip the check bytes).

---

## Why the Forney Formula Works

Think of the received polynomial as `r(x) = c(x) + e(x)` where `c(x)` is the
true codeword and `e(x) = ∑ₖ eₖ · x^{n-1-iₖ}` is the sparse error polynomial.

The syndromes are `Sᵢ = r(αⁱ) = e(αⁱ)` (since `c(αⁱ) = 0`).

The syndrome polynomial `S(x)` relates to the error locator and error evaluator via:

```
S(x) · Λ(x) ≡ Ω(x)  mod  x^{2t}
```

This is the **key identity** in RS decoding. It means `Ω` encodes the magnitudes, and
Forney's formula extracts them by differentiating `Λ` and dividing.

The formal derivative `Λ'(X_k⁻¹)` acts as a "denominator" that isolates the k-th term's
contribution.

---

## Interface Contract

All functions operate on byte arrays. The **polynomial representation** inside the
decoder uses the MA01 GF(256) field for coefficient arithmetic and calls MA00's
polynomial operations with GF(256)-aware multiply/add.

### Types

```
type Message      = [u8]   # arbitrary length (k bytes)
type Codeword     = [u8]   # message ++ check bytes (k + n_check bytes)
type RSError      =
    | TooManyErrors         # > t errors found; data unrecoverable
    | InvalidInput(reason)  # n_check odd, or input too long, etc.
```

### Functions

| Function | Signature | Returns | Throws |
|----------|-----------|---------|--------|
| `build_generator(n_check)` | `(u16) → [GF256]` | Generator polynomial coefficients | n_check = 0 |
| `encode(message, n_check)` | `(&[u8], u16) → [u8]` | `message ++ check_bytes` | InvalidInput |
| `decode(received, n_check)` | `(&[u8], u16) → [u8]` | Corrected message bytes | TooManyErrors, InvalidInput |
| `syndromes(codeword, n_check)` | `(&[u8], u16) → [GF256]` | `[S₁, S₂, ..., S_{n_check}]` | — |
| `error_locator(syndromes)` | `(&[GF256]) → [GF256]` | Λ(x) via Berlekamp-Massey | — |

### Constraints

- `n_check` must be even and ≥ 2
- `len(message) + n_check` must be ≤ 255 (GF(256) block size limit)
- `len(received)` must be ≥ `n_check`

---

## Verification Test Vectors

The following properties must hold for all valid inputs:

### Round-trip property (all languages must verify)

```
decode(encode(message, n_check), n_check) == message
```

for all messages and all even n_check ≥ 2 with `len(message) + n_check ≤ 255`.

### Error correction up to capacity

For any codeword with at most `t = n_check / 2` byte positions corrupted (each to any
byte value), decode must recover the original message:

```
cw = encode([1, 2, 3, 4, 5], n_check=8)   # t = 4
corrupt cw at 4 arbitrary positions with arbitrary values
decode(corrupted_cw, 8) == [1, 2, 3, 4, 5]
```

### Failure beyond capacity

For a codeword with `t + 1` errors, decode must return `TooManyErrors` (not silently
return wrong data):

```
cw = encode([1, 2, 3, 4, 5], n_check=4)   # t = 2
corrupt cw at 3 positions
decode(corrupted_cw, 4) raises/returns TooManyErrors
```

### Syndrome zero on valid codeword

```
cw = encode(message, n_check)
all(s == 0  for s in syndromes(cw, n_check))
```

### Concrete test vectors (cross-validated between all implementations)

Each language implementation must verify the following values are identical:

**Generator polynomial `build_generator(4)`** must equal:
`[gcd_0, gcd_1, gcd_2, gcd_3, 1]` — a degree-4 monic polynomial.
The leading coefficient is always 1. The other 4 coefficients are determined by
GF(256) arithmetic with the 0x11D primitive polynomial.
(*Exact coefficients to be pinned after Rust reference implementation is complete.*)

**Encoding test vector**:
```
encode([32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17, 236, 17, 236], n_check=7)
```
Must produce the exact same 26 bytes as the Rust implementation. This is the
standard QR code Version 1-L message — a well-known test vector against which
implementations can be compared.

---

## Connection to QR Codes (MA03 Preview)

A QR code is essentially a two-dimensional barcode that encodes a string using
RS-protected codewords. Here is how MA02 connects to MA03:

1. **MA03 (QR encoder)** converts a UTF-8 string into a sequence of data *codewords*
   (bytes) according to QR's encoding rules (numeric / alphanumeric / byte modes).

2. MA03 splits those codewords into *blocks* according to the QR version and error
   correction level (L / M / Q / H).

3. For each block, MA03 calls **`MA02.encode(block, n_check)`** to get the RS check bytes.

4. MA03 interleaves blocks, adds format/version information bits, applies a mask,
   and renders the final matrix.

The error correction capacity directly corresponds to the QR levels:

| QR Level | RS check bytes / block | Approx damage tolerance |
|----------|------------------------|------------------------|
| L (Low)  | ~7–20% of block        | ~7% |
| M (Medium) | ~15–35% of block     | ~15% |
| Q (Quality) | ~25–50% of block    | ~25% |
| H (High) | ~30–65% of block       | ~30% |

---

## Implementation Notes

### Polynomial representation inside the codec

Inside `encode` and `decode`, polynomials follow the **big-endian convention** used
in the syndrome evaluation step: the *first* element of the array is the
*highest-degree* coefficient. This is the standard RS/QR convention.

This is the opposite of MA00's little-endian convention (index = degree).
Implementations must be careful about which convention is active in each subroutine.

**Recommendation**: keep a clear boundary. Expose the public API in terms of `&[u8]`
byte slices. Use the big-endian convention internally only where needed (syndrome
evaluation). The BM and Forney steps work in coefficient-array form regardless of
ordering, as long as it is consistent.

### Performance

The Berlekamp-Massey, Chien search, and Forney steps are each O(n·t) or O(n²) in the
worst case. For QR code parameters (n ≤ 255, t ≤ 64), this is fast enough in all
languages without special optimisation. Cache the generator polynomial per `n_check`
value.

### No heap allocation in Rust

The Rust implementation should use fixed-size arrays where possible and avoid
allocating inside the hot path. A `ReedSolomon` struct can cache the generator
polynomial (at most 256 bytes) and syndrome scratch space.

---

## Package Matrix

| Language | Directory | Module |
|----------|-----------|--------|
| Rust | `code/packages/rust/reed-solomon/` | `reed_solomon` |
| TypeScript | `code/packages/typescript/reed-solomon/` | `@coding-adventures/reed-solomon` |
| Python | `code/packages/python/reed-solomon/` | `coding_adventures_reed_solomon` |
| Go | `code/packages/go/reed-solomon/` | `reedsolomon` |
| Ruby | `code/packages/ruby/reed_solomon/` | `CodingAdventures::ReedSolomon` |
| Elixir | `code/packages/elixir/reed_solomon/` | `CodingAdventures.ReedSolomon` |

Rust is the **reference implementation**. All other languages cross-validate
their test vectors against it.

---

## Roadmap

| Spec | Package | Adds |
|------|---------|------|
| MA00 | polynomial | Coefficient-array polynomial arithmetic |
| MA01 | gf256 | GF(2^8) field arithmetic |
| **MA02** | **reed-solomon** | **RS encoding and decoding (this spec)** |
| MA03 | qr-encoder | QR code generation; calls MA02 for error correction codewords |
| MA04 | qr-decoder | Inverse: recognise and decode a QR matrix back to a string |
