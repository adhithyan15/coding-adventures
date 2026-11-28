# Changelog

## 0.1.0

- Preflight the 65,536 JSON-value and nesting budgets before materializing replay lexer tokens.
- Validate replay workload IDs using the exact ASCII blank set and reject NULs and malformed surrogate escapes.

- Add paired collection and replay, exact JSON integer preservation, the indexed-array CLI, and packaged fixture checks.
- Normalize zero only for the internal xorshift state so default seeded orders shuffle reproducibly; accept integer-token representations of finite `minimum_factor` values during report replay.
- Reject benchmark analysis above 10,000 pairs before ratio construction and reject oversized byte input before UTF-8 decoding.
- Read replay paths in bounded chunks and exercise both native report replay and the 4 MiB refusal in BUILD.
