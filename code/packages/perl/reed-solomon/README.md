# CodingAdventures::ReedSolomon

Reed-Solomon error-correcting codes over GF(2^8) — implemented in Pure Perl,
part of the coding-adventures monorepo (MA02).

## What Is Reed-Solomon?

Reed-Solomon (RS) is a block error-correcting code invented by Irving Reed and
Gustave Solomon in 1960.  Given a message of k bytes, we produce a codeword of
n = k + n_check bytes.  The decoder can recover the original k bytes even when
up to t = n_check/2 bytes have been corrupted.

### Where RS codes appear

- **QR codes** — up to 30% of the symbol can be obscured and still decoded
- **CDs / DVDs** — CIRC two-level RS corrects burst scratches
- **Hard drives** — sector-level error correction in firmware
- **Voyager probes** — images sent across 20+ billion kilometres
- **RAID-6** — the two parity drives are an (n, n-2) RS code over GF(256)

## How It Fits in the Stack

```
MA00  Polynomial       — polynomial arithmetic over arbitrary fields
MA01  GF256            — GF(2^8) field: add=XOR, mul=LOG/ALOG tables
MA02  ReedSolomon      — RS encode/decode (THIS MODULE)
```

## Polynomial Conventions

- **Codewords** are big-endian: `codeword[0]` is the highest-degree coefficient.
- **Internal polynomials** (generator, Λ, Ω) are little-endian: `index = degree`.

## Installation

```bash
# Install GF256 dependency first
cpanm ../gf256/

# Install ReedSolomon
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::ReedSolomon qw(encode decode syndromes build_generator error_locator);

# Encode 5 bytes with 4 check bytes (t=2: corrects up to 2 errors)
my $codeword = encode([1, 2, 3, 4, 5], 4);
# => 9-element arrayref: [1, 2, 3, 4, 5, c0, c1, c2, c3]

# Introduce up to 2 errors
my @corrupted = @$codeword;
$corrupted[0] ^= 0xFF;
$corrupted[3] ^= 0xAB;

# Decode: recover original message
my $recovered = decode(\@corrupted, 4);
# => [1, 2, 3, 4, 5]

# Syndromes: all zero for a valid codeword, nonzero if corrupted
my $s = syndromes($codeword, 4);
# => [0, 0, 0, 0]

# Generator polynomial (little-endian)
my $g = build_generator(2);
# => [8, 6, 1]   — cross-language test vector

# Error locator polynomial from syndromes
my $lam = error_locator($s);
# => [1]  (no errors)
```

## Public API

| Function | Description |
|---|---|
| `encode(\@message, $n_check)` | Systematic encode; returns arrayref of length `k + n_check` |
| `decode(\@received, $n_check)` | Decode and correct errors; returns message arrayref |
| `syndromes(\@received, $n_check)` | Returns arrayref of n_check syndrome values |
| `build_generator($n_check)` | Returns LE monic generator poly (length n_check+1) |
| `error_locator(\@syndromes)` | Runs Berlekamp-Massey; returns LE Λ(x) with Λ[0]=1 |

### Error conditions

- `encode` / `decode` die with `"InvalidInput: ..."` if n_check is 0, odd, or
  the total codeword length would exceed 255.
- `decode` dies with `"TooManyErrors: ..."` if the number of corrupted bytes
  exceeds the correction capacity t = n_check/2.

## The Five-Step Decode Pipeline

1. **Syndromes** — evaluate received(α^j) for j=1..n_check
2. **Berlekamp-Massey** — find error locator polynomial Λ(x)
3. **Chien search** — evaluate Λ at all X_p⁻¹ to find error positions
4. **Forney algorithm** — compute error magnitudes from Ω(x) and Λ'(x)
5. **Apply corrections** — XOR each magnitude into the corrupted byte

## Running Tests

```bash
# From the package directory
cpanm --installdeps --quiet .
prove -l -v t/
```

## License

MIT
