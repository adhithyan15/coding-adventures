# Changelog — cas-number-theory (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-number-theory` package.
- `arithmetic` module:
  - `gcd(a, b)` — Euclidean algorithm on `|a|`, `|b|`.
  - `lcm(a, b)` — `|a·b| / gcd(a, b)`.
  - `extended_gcd(a, b)` — recursive Bézout coefficients `(g, s, t)`.
  - `totient(n)` — Euler's φ via trial division.
  - `mod_inverse(a, m)` — modular inverse via `extended_gcd`; `None` when
    `gcd(a, m) ≠ 1`.
  - `mod_pow(base, exp, modulus)` — fast modular exponentiation by repeated
    squaring in O(log exp).
- `primality` module:
  - `is_prime(n)` — trial division with `6k±1` step optimisation.
  - `primes_up_to(limit)` — Sieve of Eratosthenes up to `limit`.
  - `next_prime(n)` — smallest prime strictly greater than `n`.
  - `nth_prime(k)` — the k-th prime (1-indexed).
- `factorize` module:
  - `factor_integer(n)` — trial division factorization;
    returns `Vec<(prime, exponent)>` in ascending prime order.
  - `factorize_ir(expr)` — wraps an `IRNode::Integer` as a product of prime
    powers (`Mul(Pow(p, e), …)`) in symbolic IR; primes and units returned
    unchanged.
- `crt` module:
  - `crt(remainders, moduli)` — iterative pairwise Chinese Remainder Theorem;
    handles non-pairwise-coprime moduli when congruences are consistent;
    uses `i128` intermediates to avoid overflow; returns `None` on conflict.
- 46 integration tests + 14 doc-tests; all passing.
