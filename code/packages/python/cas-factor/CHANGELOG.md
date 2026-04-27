# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase 1.

- ``content`` extraction (integer GCD of coefficients).
- Rational-root test for linear factors over Q.
- ``factor_integer_polynomial(coeffs)`` — orchestrates content +
  rational-root iteration; returns
  ``(content, [(factor_coeffs, multiplicity), ...])``.
- ``FACTOR``, ``IRREDUCIBLE`` head sentinels.

Deferred to Phase 2: Berlekamp factorization mod p, Hensel lifting,
Zassenhaus recombination.
