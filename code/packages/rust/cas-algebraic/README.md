# cas-algebraic

`cas-algebraic` factors integer univariate polynomials over quadratic
extensions `Q[sqrt(d)]`.

This Rust port covers the same first slice as the Python package:

- monic quadratics whose discriminant is a rational square times `d`;
- depressed monic quartics `x^4 + p*x^2 + q`;
- integer-polynomial pre-factoring through `cas-factor`;
- lightweight symbolic-IR helpers for `AlgFactor(poly, Sqrt(d))`.

Polynomials use ascending coefficient order: `[-2, 0, 1]` is `x^2 - 2`.
