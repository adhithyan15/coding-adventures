### Swift build-tool Dune discovery

- Excluded Dune's exact case-sensitive `_build` output component from Swift
  package discovery and source hashing. Shared and direct fixture coverage
  verifies that `_Build` and `_build-example` remain discoverable and hashed.

