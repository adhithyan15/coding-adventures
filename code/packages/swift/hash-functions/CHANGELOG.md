# Changelog — HashFunctions (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- Implement FNV-1a 32/64, DJB2, MurmurHash3-32, and keyed SipHash-2-4.
- Add the repository's polynomial-rolling compatibility helper with full-width
  modular multiplication.
- Add a common `HashFunction` protocol and six concrete strategy values.
- Add deterministic avalanche and distribution analysis over caller-owned data.
- Pin the algorithms to corrected DT17 vectors and all 64 upstream SipHash
  reference vectors.
- Declare an empty capability profile and provide native Unix/macOS and Windows
  build fronts.
