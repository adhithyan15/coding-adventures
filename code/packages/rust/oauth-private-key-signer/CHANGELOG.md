# Changelog

## Unreleased

- Added a zero-external-dependency, audit-first, non-exporting OAuth signing
  boundary with provider/opaque-key/trace binding.
- Kept JOSE algorithm selection as validated provider data so injected HSM,
  operating-system, and future repository-owned authorities share one contract.
