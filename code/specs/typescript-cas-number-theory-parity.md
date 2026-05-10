# TypeScript CAS number theory parity

## Goal

Port Python/Rust `cas-number-theory` to pure TypeScript so browser-side CAS
paths can evaluate integer number-theory helpers without native code.

## Scope

This slice mirrors the Rust package surface:

- `gcd`, `lcm`, `extendedGcd`
- `totient`, `modInverse`, `modPow`
- `isPrime`, `primesUpTo`, `nextPrime`, `nthPrime`
- `factorInteger`
- `factorizeIr`
- `crt`

All integer arithmetic uses `bigint`. Number and string inputs are accepted as
ergonomic wrappers but normalized internally before computation.
