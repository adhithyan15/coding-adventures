# cas-number-theory (Rust)

Number theory operations: GCD, LCM, Euler's totient, primality testing,
integer factorization, and the Chinese Remainder Theorem.

## Operations

| Function | Description |
|---|---|
| `gcd(a, b)` | Greatest common divisor |
| `lcm(a, b)` | Least common multiple |
| `extended_gcd(a, b)` | Bézout coefficients: `(g, s, t)` where `a*s + b*t = g` |
| `totient(n)` | Euler's totient φ(n) |
| `mod_inverse(a, m)` | Modular inverse: `a*x ≡ 1 (mod m)` |
| `mod_pow(base, exp, m)` | Fast modular exponentiation |
| `is_prime(n)` | Primality test (trial division) |
| `primes_up_to(limit)` | Sieve of Eratosthenes |
| `next_prime(n)` | Smallest prime > n |
| `nth_prime(k)` | k-th prime (1-indexed) |
| `factor_integer(n)` | Prime factorization: `Vec<(prime, exponent)>` |
| `factorize_ir(expr)` | Express an integer as a Mul of prime powers in IR |
| `crt(remainders, moduli)` | Chinese Remainder Theorem |

## Usage

```rust
use cas_number_theory::{gcd, lcm, is_prime, factor_integer, crt, factorize_ir};
use symbolic_ir::int;

// Basic arithmetic
assert_eq!(gcd(12, 8), 4);
assert_eq!(lcm(4, 6), 12);

// Primality
assert!(is_prime(97));
assert!(!is_prime(100));

// Factorization
assert_eq!(factor_integer(360), vec![(2, 3), (3, 2), (5, 1)]);

// Symbolic IR factorization
// 12 → Mul(Pow(2, 2), 3)
let ir = factorize_ir(&int(12));

// Chinese Remainder Theorem
// x ≡ 2 (mod 3), x ≡ 3 (mod 5), x ≡ 2 (mod 7) → x = 23
assert_eq!(crt(&[2, 3, 2], &[3, 5, 7]), Some(23));
```

## Stack position

```
symbolic-ir  ←  cas-number-theory
```
