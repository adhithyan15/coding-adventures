# cas-complex (Rust)

Complex number arithmetic over symbolic IR expressions, including
normalization to canonical `a + b·I` form, part extraction, conjugation,
modulus, argument, and integer powers via De Moivre's theorem.

## Operations

| Function | Description |
|---|---|
| `complex_normalize(z)` | Rewrite `z` into canonical `a + b·I` form |
| `real_part(z)` | Extract real part `Re(z)` |
| `imag_part(z)` | Extract imaginary part `Im(z)` (coefficient of `I`) |
| `conjugate(z)` | Complex conjugate: `a + b·I → a − b·I` |
| `modulus(z)` | Modulus `|z| = √(a² + b²)` → `IRFloat` |
| `argument(z)` | Argument `arg(z) = atan2(b, a)` in `(−π, π]` → `IRFloat` |
| `complex_pow(z, n)` | Integer power via De Moivre's theorem |

## Normalization

`complex_normalize` handles:
- `Add`, `Sub`, `Mul`, `Neg` with full complex arithmetic
  (`(a+bi)·(c+di) = (ac−bd) + (ad+bc)·i`)
- `Pow(I, n)` cycles through `1, i, -1, -i` for any integer `n`
- Numeric literals (Integer, Float, Rational) are treated as pure real
- Symbolic atoms other than `I` are treated as opaque reals

Zero parts are suppressed: `0 + 2·I → 2·I`, `3 + 0·I → 3`.

## Power computation

`complex_pow(z, n)` for integer `n`:

```text
z^n = r^n · (cos(n·θ) + i·sin(n·θ))
```

where `r = |z|` and `θ = arg(z)`. Near-integer float results are snapped
back to exact integers (within 1e-9) for clean symbolic output.

Special cases handled exactly:
- `z^0 = 1`
- `z^1 = z`
- `z^(-1) = conj(z) / |z|²`

## Usage

```rust
use cas_complex::{complex_normalize, complex_pow, modulus, real_part, imag_part};
use symbolic_ir::{apply, int, sym, ADD, MUL, POW};

// Normalize: (1 + I)*(1 - I) = 2
let a = apply(sym(ADD), vec![int(1), sym("I")]);
let b = apply(sym(ADD), vec![int(1), apply(sym("Neg"), vec![sym("I")])]);
let product = apply(sym(MUL), vec![a, b]);
let r = complex_normalize(&product);
assert_eq!(r, int(2));

// Power: (1 + I)^2 = 2*I
let w = apply(sym(ADD), vec![int(1), sym("I")]);
let result = complex_pow(&w, &int(2));
assert_eq!(real_part(&result), int(0));
assert_eq!(imag_part(&result), int(2));

// I^4 = 1
assert_eq!(complex_pow(&sym("I"), &int(4)), int(1));
```

## Stack position

```
symbolic-ir  ←  cas-complex
```
