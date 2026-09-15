### Python build-tool portable source hashing

- Added OCaml `.ml`, `.mli`, and `.opam` source recognition plus exact
  `.ocamlformat`, `dune`, and `dune-project` metadata recognition to extension
  and declared-source collection, including applicable `.opam` manifests that
  declared source globs omit.
- Framed each normalized repository-relative UTF-8 path with its unsigned
  64-bit content length and exact raw bytes, matching the language-neutral
  hashing-v1 oracle and making same-content renames observable without hashing
  absolute checkout paths, host metadata, or decoded text.
- Closed ancestor replacement races by retaining no-follow component handles,
  rejecting Windows reparses through the package root, and binding the opened
  source handle to its exact lexical path before streaming.

