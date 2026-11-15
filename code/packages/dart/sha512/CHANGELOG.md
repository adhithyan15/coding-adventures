# Changelog

## 0.1.0

- Initial pure Dart implementation of SHA-512 (FIPS 180-4) and `HF04-sha512` contract.
- Support for one-shot hashing (`sha512`, `sha512Hex`) and incremental streaming (`Sha512Hasher`).
- Strict 64-bit pair representation (`Word64`) for cross-platform fidelity on Dart VM and Web.
- 128-bit message length validation and FIPS domain enforcement.
- Standing Rule #20 error message isolation in `Sha512Messages`.
