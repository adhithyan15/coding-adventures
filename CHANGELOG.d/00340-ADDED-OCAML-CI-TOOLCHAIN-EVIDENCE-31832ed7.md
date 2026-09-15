### Added — OCaml CI Toolchain Evidence
- Added OCAML03, a closed evidence manifest, and digest-checked transitive opam
  solver locks plus installed-package receipts for Ubuntu x64, macOS arm64, and
  Windows x64.
- Added a commit-pinned, read-only three-platform workflow that fresh-solves
  against one reviewed opam-repository commit, compares checked evidence, then
  performs a separate locked install.
- Added real line-oriented execution of the library and program scaffold
  `BUILD`/`BUILD_windows` contracts, including formatting, Alcotest, and
  measured `bisect_ppx` coverage on every runner family.
- Added an offline validator and CI unit tests for closed keys, action/repository
  identities, safe evidence paths, digests, direct versions, workflow security,
  and platform command-shell dispatch.

