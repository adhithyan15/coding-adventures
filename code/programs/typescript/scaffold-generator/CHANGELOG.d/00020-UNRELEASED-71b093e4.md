## [Unreleased]

### Added

- Added OCaml library and program scaffolds with the shared byte-exact OCAML02
  fixtures, exact direct opam/Dune metadata, Alcotest, ocamlformat, bisect_ppx
  coverage, resolved local dependency pins, and capability manifests.

### Fixed

- OCaml dependency pins now scope checksum bypass to local worktree ingestion
  while subsequent registry dependency installs retain strict checksums.
- OCaml metadata rejects `*)` and safely quotes printable descriptions before
  any output is written.
- Dune `%{...}` interpolation openers are rejected before any scaffold output.
- Accepted Unicode descriptions now remain raw UTF-8 with only OCaml/opam
  quote and backslash escaping, matching the Go front door byte-for-byte.
- OCaml dependency discovery reads only the expected opam `depends` field,
  program scaffolds pin package-tree dependencies by resolved relative path,
  and direct/transitive dependency symlinks fail closed.
- Haskell scaffolds now emit the complete shared schema-v1 capability manifest
  and keep their starter library and test source pure.
- Haskell generation rejects control characters before descriptions are
  interpolated into Cabal metadata, and the CLI rejects structural
  block-comment delimiters before any template is written.
- Refreshed the lockfile to use patched PostCSS and its aligned Nano ID
  resolution.
