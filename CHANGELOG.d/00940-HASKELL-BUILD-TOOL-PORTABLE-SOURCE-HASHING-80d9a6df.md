### Haskell build-tool portable source hashing

- Embedded the complete checked 23-language source-input registry, including
  OCaml and all seven selector roles, and exercised both neutral package-local
  collection modes through the production selector.
- Replaced Git SHA-1 and fallback package hashes with local incremental SHA-256
  over Hashing v1 path/content frames, raw UTF-8 ordering, 8 KiB reads, closed
  portable path and glob grammar, and explicit candidate, input, byte, and
  match-work ceilings.
- Added stable root-redacted package-hash failures while keeping repository-
  boundary inputs, dependency/cache hashing, and descriptor-relative native
  snapshot hardening in explicit dependent owners.

