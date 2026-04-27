# cas-number-theory

Integer number theory operations for the symbolic computation substrate.

Implements: `IsPrime`, `NextPrime`, `PrevPrime`, `FactorInteger`, `Divisors`,
`Totient`, `MoebiusMu`, `JacobiSymbol`, `ChineseRemainder`, `IntegerLength`.

All operations are language-neutral IR heads installed on `SymbolicBackend`,
inherited by any CAS frontend (MACSYMA, Maple, Mathematica, …).

## Algorithms

- **Primality**: Sieve of Eratosthenes for n < 1,000,000; deterministic
  Miller-Rabin for n < 3,215,031,751; BPSW-equivalent (20 prime witnesses)
  for larger n.
- **Factoring**: Trial division (small primes) + Pollard's rho (large cofactors).
- **CRT**: Iterative two-moduli formula using `pow(m, -1, m_i)`.
