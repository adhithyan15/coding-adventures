### Added — OCaml Scaffold Infrastructure
- Added the OCAML02 contract, lane README, repository ignores, and shared
  byte-exact library/program fixture trees for the emerging OCaml lane.
- Added matching Go and TypeScript scaffold-generator support with exact direct
  OCaml/opam/Dune/Alcotest/`bisect_ppx`/`ocamlformat` metadata, resolved local
  dependency pins, real formatting/test/coverage build commands, and
  schema-valid capability profiles.
- Added OCaml-specific metadata serialization and `*)` injection hardening
  before any scaffold directory is written.
- Rejected Dune `%{...}` interpolation openers before output generation.
- Added a shared OCaml/opam string encoding contract so accepted Unicode remains
  byte-identical raw UTF-8 across the Go and TypeScript front doors.
- Added dependency-reader regressions for self-name/metadata decoys,
  program-to-library pin paths, and direct/transitive symlink rejection.

