### Ruby build-tool portable source hashing

- Added OCaml `.ml`, `.mli`, and `.opam` source recognition plus exact
  `.ocamlformat`, `dune`, `dune-project`, and supported BUILD-front handling
  to extension and declared-source collection. Declared mode retains a root
  `.opam` manifest when globs omit it; nested manifests still require a match.
- Framed sorted normalized repository-relative UTF-8 paths and exact raw file
  bytes with unsigned 64-bit lengths, matching hashing v1 and making
  same-content renames observable without hashing absolute checkout paths,
  host locale, decoded text, or metadata.

