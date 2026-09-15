### TypeScript build-tool Dune discovery

- Excluded Dune's exact case-sensitive `_build` output component from
  TypeScript package discovery. Shared and direct regressions prove that
  generated OCaml decoys stay out of the package graph while `_Build` and
  `_build-example` source directories remain discoverable.

