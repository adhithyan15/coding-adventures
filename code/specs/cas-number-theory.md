# cas-number-theory — Integer Number Theory

> **Status**: New spec. Implements number-theoretic operations as
> generic IR heads: `IsPrime`, `NextPrime`, `PrevPrime`, `FactorInteger`,
> `Divisors`, `Totient`, `MoebiusMu`, `JacobiSymbol`, `ChineseRemainder`.
> Parent: `symbolic-computation.md`. No CAS dependencies beyond
> `symbolic-ir`.

## Why this package exists

Number theory operations appear in every CAS:
`ifactor(n)`, `isprime(n)`, `next_prime(n)`, `totient(n)`, etc.
These are purely integer algorithms — no symbolic IR rewriting is
required, only exact arithmetic on `IRInteger` nodes. They are kept
separate from `cas-simplify` and `cas-factor` (which handle *polynomial*
factoring) to respect the separation between polynomial algebra and
integer number theory.

Note: `Gcd`, `Lcm`, `Mod` are already implemented in `symbolic_vm/cas_handlers.py`
as they are needed for polynomial arithmetic. This package adds the
higher-level number-theoretic operations that build on them.

## Reuse story

| MACSYMA              | Mathematica            | Maple                  | IR head           |
|----------------------|------------------------|------------------------|-------------------|
| `primep(n)`          | `PrimeQ[n]`            | `isprime(n)`           | `IsPrime`         |
| `next_prime(n)`      | `NextPrime[n]`         | `nextprime(n)`         | `NextPrime`       |
| `prev_prime(n)`      | `NextPrime[n, -1]`     | —                      | `PrevPrime`       |
| `ifactor(n)`         | `FactorInteger[n]`     | `ifactor(n)`           | `FactorInteger`   |
| `divisors(n)`        | `Divisors[n]`          | `numtheory[divisors]`  | `Divisors`        |
| `totient(n)`         | `EulerPhi[n]`          | `numtheory[phi]`       | `Totient`         |
| `moebius(n)`         | `MoebiusMu[n]`         | `numtheory[mobius]`    | `MoebiusMu`       |
| `jacobi(a, n)`       | `JacobiSymbol[a, n]`   | `numtheory[jacobi]`    | `JacobiSymbol`    |
| `chinese(r, m)`      | `ChineseRemainder[r,m]`| `mods(r, m)` (CRT)     | `ChineseRemainder`|
| `numdigits(n, b)`    | `IntegerLength[n, b]`  | `length(n, b)`         | `IntegerLength`   |

## Scope

In:

- `IsPrime(n)` — primality test for arbitrary-precision integers.
  Returns `IRSymbol("True")` or `IRSymbol("False")`.
- `NextPrime(n)` — smallest prime strictly greater than `n`.
- `PrevPrime(n)` — largest prime strictly less than `n`.
  Returns unevaluated for `n ≤ 2` (no prime exists below 2).
- `FactorInteger(n)` — prime factorization as a list of `[prime, exponent]`
  pairs: `FactorInteger(12) → [[2, 2], [3, 1]]`.
  In IR: `IRApply(LIST, (IRApply(LIST, (IRInteger(2), IRInteger(2))),
                         IRApply(LIST, (IRInteger(3), IRInteger(1)))))`.
- `Divisors(n)` — sorted list of all positive divisors.
- `Totient(n)` — Euler's phi function: count of integers in `[1, n]`
  coprime to `n`.
- `MoebiusMu(n)` — Möbius function: `0` if `n` has a squared prime
  factor, `(-1)^k` if `n` is a product of `k` distinct primes.
- `JacobiSymbol(a, n)` — generalized Legendre/Jacobi symbol.
- `ChineseRemainder(remainders, moduli)` — CRT reconstruction.
  Both args are `List` IR nodes of equal length.
- `IntegerLength(n)` or `IntegerLength(n, b)` — number of digits of `n`
  in base `b` (default 10).

Out:

- Symbolic/polynomial factoring — that is `cas-factor`.
- Gaussian integer factoring — deferred to `cas-complex` future work.
- Diophantine equations (e.g. Pell's equation) — future package.
- Modular arithmetic beyond CRT (e.g. `PowerMod`, `DiscreteLog`) —
  Phase 2 of this package.

## Public interface

```python
from cas_number_theory import (
    is_prime,                 # int → bool
    next_prime,               # int → int
    prev_prime,               # int → int | None
    factor_integer,           # int → list[tuple[int, int]]
    divisors,                 # int → list[int]
    totient,                  # int → int
    moebius_mu,               # int → int  (-1, 0, 1)
    jacobi_symbol,            # (int, int) → int
    chinese_remainder,        # (list[int], list[int]) → int
    integer_length,           # (int, int=10) → int
    build_number_theory_handler_table,  # () → dict[str, Handler]
)
```

All Python-level functions operate on Python `int`s and are
completely independent of IR. The VM handlers wrap them: extract
`IRInteger.value`, call the function, wrap the result back in
`IRInteger` (or `IRApply(LIST, ...)` for `Divisors`/`FactorInteger`).

## Algorithms

### IsPrime — Miller-Rabin

For `n < 3_215_031_751` (fits in 32 bits): deterministic Miller-Rabin
with witnesses `{2, 3, 5, 7}` — guaranteed correct, no false positives.

For larger `n`: use the first 20 prime witnesses
`{2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71}`.
This is a BPSW-equivalent test and has no known counterexamples.

Python implementation uses the built-in `pow(a, d, n)` for fast
modular exponentiation — no external libraries needed.

### NextPrime / PrevPrime

Sieve a small window above/below `n` using `IsPrime`. Start with an
initial candidate (next odd, skip multiples of 3), advance by 2,
re-test. For `n < 10^6`, use a pre-built sieve (the Sieve of
Eratosthenes up to 10^6 is 125 KB and eliminates most candidates
instantly). Above 10^6, Miller-Rabin per candidate.

### FactorInteger — Trial Division + Pollard's Rho

Phase 1 (small primes): trial-divide by all primes up to min(1000, √n).
Covers the vast majority of inputs a user will type interactively.

Phase 2 (Pollard's Rho): for any remaining cofactor > 1 that is not
prime (tested with Miller-Rabin), apply Floyd's cycle-finding variant of
Pollard's ρ with polynomial `f(x) = x² + c` for a few random `c`. Recursively
factor both pieces.

This gives correct factorizations for all 64-bit integers and
handles 128-bit integers within a reasonable time for interactive use.

### Totient

Given the prime factorization `n = p1^a1 · p2^a2 · ...`:
`φ(n) = n · ∏(1 - 1/pᵢ) = ∏(pᵢ^(aᵢ-1) · (pᵢ-1))`.

Obtain factorization from `factor_integer(n)`, apply the formula
using Python integer arithmetic.

### ChineseRemainder

Iterative CRT using the standard two-moduli formula:
`x ≡ r₁ (mod m₁)`, `x ≡ r₂ (mod m₂)` →
`x = r₁ + m₁ · ((r₂ - r₁) · m₁⁻¹ mod m₂)`.

Extended to `k` moduli by repeated application left-to-right.
Uses Python's `pow(a, -1, m)` (Python ≥ 3.8) for modular inverse.

Returns `None` if moduli are not pairwise coprime.

## Heads added

| Head                 | Arity | Returns                                        |
|----------------------|-------|------------------------------------------------|
| `IsPrime`            | 1     | `True` or `False` (IR symbols)                 |
| `NextPrime`          | 1     | `IRInteger`                                    |
| `PrevPrime`          | 1     | `IRInteger` or unevaluated if `n ≤ 2`          |
| `FactorInteger`      | 1     | `List` of `[prime, exponent]` lists            |
| `Divisors`           | 1     | `List` of `IRInteger`s                         |
| `Totient`            | 1     | `IRInteger`                                    |
| `MoebiusMu`          | 1     | `IRInteger` in `{-1, 0, 1}`                    |
| `JacobiSymbol`       | 2     | `IRInteger` in `{-1, 0, 1}`                    |
| `ChineseRemainder`   | 2     | `IRInteger` or unevaluated (incongruent moduli)|
| `IntegerLength`      | 1–2   | `IRInteger`                                    |

All handlers gracefully return `expr` unevaluated when given non-integer
or negative arguments (where the operation is undefined).

## MACSYMA name table entries

```python
# macsyma_runtime/name_table.py additions
NUMBER_THEORY_NAME_TABLE = {
    "primep":          IRSymbol("IsPrime"),
    "next_prime":      IRSymbol("NextPrime"),
    "prev_prime":      IRSymbol("PrevPrime"),
    "ifactor":         IRSymbol("FactorInteger"),
    "divisors":        IRSymbol("Divisors"),
    "totient":         IRSymbol("Totient"),
    "moebius":         IRSymbol("MoebiusMu"),
    "jacobi":          IRSymbol("JacobiSymbol"),
    "chinese":         IRSymbol("ChineseRemainder"),
    "numdigits":       IRSymbol("IntegerLength"),
}
```

## Test strategy

### IsPrime
- `IsPrime(2) = True`, `IsPrime(3) = True`, `IsPrime(4) = False`.
- `IsPrime(97) = True`, `IsPrime(100) = False`.
- Carmichael number `IsPrime(561) = False` (Miller-Rabin handles this).
- Large prime: `IsPrime(2^31 - 1) = True` (Mersenne prime).
- `IsPrime(0) = False`, `IsPrime(1) = False`.
- Non-integer input → unevaluated.

### NextPrime / PrevPrime
- `NextPrime(10) = 11`.
- `NextPrime(13) = 17`.
- `PrevPrime(10) = 7`.
- `PrevPrime(2)` → unevaluated.

### FactorInteger
- `FactorInteger(1) = []` (empty list).
- `FactorInteger(12) = [[2, 2], [3, 1]]`.
- `FactorInteger(360) = [[2, 3], [3, 2], [5, 1]]`.
- `FactorInteger(p)` for prime `p` = `[[p, 1]]`.
- Large semiprime `FactorInteger(2^32 + 1)` = correct factorization.
- Negative input → unevaluated.

### Divisors
- `Divisors(1) = [1]`.
- `Divisors(12) = [1, 2, 3, 4, 6, 12]`.
- Result is sorted ascending.

### Totient
- `Totient(1) = 1`.
- `Totient(p) = p - 1` for prime `p`.
- `Totient(12) = 4`.

### ChineseRemainder
- `ChineseRemainder([2, 3], [3, 5]) = 8`.
- Non-coprime moduli → unevaluated.

### Stress
- `FactorInteger(n)` for 1000 random `n` in `[1, 10^9]` verified
  against `math.prod(p^e for p,e in factorization) == n`.

Coverage: ≥85%.

## Package layout

```
code/packages/python/cas-number-theory/
  src/cas_number_theory/
    __init__.py
    primality.py          # Miller-Rabin, next/prev prime, sieve
    factorize.py          # trial division + Pollard's rho
    arithmetic.py         # totient, moebius_mu, jacobi_symbol
    crt.py                # ChineseRemainder
    handlers.py           # build_number_theory_handler_table()
    py.typed
  tests/
    test_primality.py
    test_factorize.py
    test_arithmetic.py
    test_crt.py
    test_handlers.py
```

## Dependencies

`coding-adventures-symbolic-ir` only. No other CAS packages needed —
all operations are pure integer arithmetic.

## Future extensions (Phase 2)

- `PowerMod(a, n, m)` — `a^n mod m`.
- `DiscreteLog(a, g, p)` — baby-step giant-step.
- `NthRoot(a, n, m)` — modular nth root (for RSA textbook examples).
- `LegendreSymbol(a, p)` — already implied by `JacobiSymbol` for prime `p`.
- Gaussian integer factoring (joint with `cas-complex`):
  `FactorGaussian(a + b·i)`.
