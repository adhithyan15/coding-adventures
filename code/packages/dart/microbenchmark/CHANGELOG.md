# Changelog

## 0.1.0

- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add paired collection and replay, exact JSON integer preservation, the indexed-array CLI, and packaged fixture checks.
- Normalize zero only for the internal xorshift state so default seeded orders shuffle reproducibly; accept integer-token representations of finite `minimum_factor` values during report replay.
