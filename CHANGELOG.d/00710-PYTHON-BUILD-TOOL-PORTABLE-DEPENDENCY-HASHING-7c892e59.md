### Python build-tool portable dependency hashing

- Framed each sorted transitive dependency identity with its UTF-8 byte length
  and decoded 32-byte SHA-256 digest, matching hashing v1 without hashing hex
  presentation text or permitting package-boundary ambiguity.
- Added the portable combined cache digest over the raw package and dependency
  digests, direct shared-fixture regressions, and closed validation for missing,
  malformed, uppercase, short, and oversized dependency digests.
- Precomputed every package digest before dependency hashing in both normal and
  plan-file CLI flows, so lexical discovery order cannot make a dependent miss
  a prerequisite digest.

