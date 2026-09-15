### OCaml process-free build substrate

- Added the OCAML04 contract and wired OCaml into canonical package/program
  discovery, field-aware opam and Dune dependency resolution, generated-output
  safe hashing, prerequisite validation, shard costing, and `needs_ocaml` CI
  detection. A language-neutral resolver fixture pins the boundary without
  granting process execution, package-manager, or native build authority.

