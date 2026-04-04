# CodingAdventures::Polynomial

Single-variable polynomial arithmetic over the real numbers, implemented in
Pure Perl as part of the coding-adventures monorepo.

## What It Does

This package provides the core polynomial operations you need to implement
error-correcting codes (Reed-Solomon), GF(2⁸) arithmetic, and polynomial-based
cryptography. It represents polynomials as Perl array references where index `i`
holds the coefficient of `x^i`.

```
3x² + 2x + 1  →  [1, 2, 3]
```

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::Polynomial qw(
    zero one degree normalize
    add subtract multiply divmod_poly divide modulo
    evaluate gcd_poly
);

my $p = [1, 2, 3];       # 3x^2 + 2x + 1
my $q = [5, 4];          # 4x + 5

say degree($p);           # 2
say evaluate($p, 2);      # 17  (= 3*4 + 2*2 + 1)

my $sum = add($p, $q);   # 3x^2 + 6x + 6 = [6, 6, 3]

my ($quot, $rem) = divmod_poly($p, $q);

my $g = gcd_poly([-1,0,1], [-1,1]);  # x-1
```

## Functions

| Function | Description |
|---|---|
| `normalize($p)` | Remove trailing near-zero coefficients |
| `degree($p)` | Highest power with non-zero coefficient |
| `zero()` | The zero polynomial `[0]` |
| `one()` | The unit polynomial `[1]` |
| `add($a, $b)` | Add two polynomials |
| `subtract($a, $b)` | Subtract $b from $a |
| `multiply($a, $b)` | Multiply two polynomials |
| `divmod_poly($a, $b)` | Long division: returns (quotient, remainder) |
| `divide($a, $b)` | Quotient only |
| `modulo($a, $b)` | Remainder only |
| `evaluate($p, $x)` | Evaluate at $x using Horner's method |
| `gcd_poly($a, $b)` | Monic GCD via the Euclidean algorithm |

## How It Fits in the Stack

`Polynomial` is a foundational mathematics package (layer MA00). It is used
by:
- `GF256` — which performs polynomial arithmetic modulo the irreducible
  polynomial x⁸ + x⁴ + x³ + x + 1 over GF(2)
- Future Reed-Solomon packages

## Running Tests

```bash
prove -l -v t/
```

## License

MIT
