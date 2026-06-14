# TypeScript CAS algebraic parity

## Goal

Port Python `cas-algebraic` to pure TypeScript so browser CAS paths can factor
integer univariate polynomials over `Q[sqrt(d)]`.

## Scope

This package builds on `@coding-adventures/cas-factor` and exposes:

- exact rational algebraic coefficients `a + b*sqrt(d)`;
- algebraic polynomial coefficient lists in ascending degree order;
- monic quadratic splitting when `discriminant / d` is a rational square;
- depressed monic quartic splitting for `x^4 + p*x^2 + q`;
- top-level `factorOverExtension` after integer factorization over `Z`;
- symbolic IR helpers for `AlgFactor(poly, Sqrt(d))` style callers.

## Follow-up

VM handler registration and MACSYMA surface syntax should wire into these pure
helpers once the TypeScript CAS package graph has landed.
