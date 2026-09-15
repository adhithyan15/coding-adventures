### Go build-tool portable source hashing

- Unified extension and declared-source collection around the exact
  case-sensitive 26-component generated-directory registry, all five BUILD
  fronts, OCaml source/metadata recognition, and stable link/reparse pruning.
- Framed each sorted normalized repository-relative UTF-8 path and exact raw
  file body with unsigned 64-bit big-endian lengths before SHA-256, matching
  hashing v1 and making same-content renames and binary bytes observable
  without hashing absolute checkout paths, host metadata, or locale.
- Made source collection and hashing fail closed before package-root or nested
  link/reparse traversal. Unstable inputs now return a checked error and a
  stable root-redacted CLI diagnostic with a control-safe quoted package
  identity instead of a sentinel digest, forged log line, or panic.

