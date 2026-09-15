### TypeScript build-tool portable source hashing

- Added OCaml `.ml`, `.mli`, and `.opam` source recognition plus exact
  `.ocamlformat`, `dune`, and `dune-project` metadata recognition to extension
  and declared-source collection.
- Framed each sorted normalized repository-relative UTF-8 path with unsigned
  64-bit lengths and exact raw file bytes, matching hashing v1 and making
  same-content renames observable without hashing absolute checkout paths,
  host locale, decoded text, or metadata.

