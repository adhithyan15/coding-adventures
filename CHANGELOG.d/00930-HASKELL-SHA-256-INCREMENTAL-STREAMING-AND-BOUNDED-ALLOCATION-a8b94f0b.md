### Haskell SHA-256 incremental streaming and bounded allocation

- Added a strict immutable streaming context to the Haskell SHA-256 package so
  future build-tool source hashing can process bounded byte chunks without
  retaining complete files. Existing list-based SHA-256, HMAC, HKDF, and
  PBKDF2 consumers remain compatible.
- Added independent FIPS and binary vectors across SHA-256 padding and 8 KiB
  file-chunk boundaries, repeatable finalization, immutable branching, a real
  Windows BUILD test front door, and the missing Haskell HF03 package-matrix
  entry.
- Enforced the `< 2^64`-bit FIPS message domain with checked counter arithmetic
  and made the Windows front skip cleanly when Cabal is genuinely absent while
  still propagating real test failures.
- Replaced per-block boxed list expansion with a reusable 16-word unboxed
  rolling schedule and bounded bridge/remainder copying. The optimized
  million-byte allocation gate covers one-chunk and 8 KiB streaming modes with
  a 128 MiB ceiling, reducing the measured local allocation from approximately
  929 MB to 58 MB while preserving every digest vector and public API.

