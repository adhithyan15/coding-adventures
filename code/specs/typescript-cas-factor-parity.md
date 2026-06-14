# TypeScript CAS factor parity

## Goal

Port the Python `cas-factor` package to pure TypeScript so browser and Node
CAS paths can factor integer univariate polynomials without native code.

## Scope

This slice provides:

- Integer polynomial helpers over ascending coefficient lists.
- Content extraction and primitive-part normalization.
- Integer-root discovery with multiplicity extraction.
- A Kronecker splitter for non-linear residual factors.
- `factorIntegerPolynomial`, returning content plus factor/multiplicity pairs.
- BigInt arithmetic throughout, so coefficients do not depend on JS safe-number
  limits.

## Follow-up

The Python package also contains the Berlekamp-Zassenhaus-Hensel fallback. This
first TypeScript slice ports the main package surface and Kronecker path; the
BZH fallback is the next deepening step for high-degree modular factoring.
