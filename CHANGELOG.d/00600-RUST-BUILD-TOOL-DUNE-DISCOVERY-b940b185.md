### Rust build-tool Dune discovery

- Excluded Dune's exact case-sensitive `_build` output component from Rust
  package discovery before BUILD-file membership is tested. Shared and direct
  regressions prove that generated OCaml decoys stay out of the package graph
  while `_Build` and `_build-example` source directories remain discoverable.

