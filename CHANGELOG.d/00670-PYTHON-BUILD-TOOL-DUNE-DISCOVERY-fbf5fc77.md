### Python build-tool Dune discovery

- Excluded Dune's exact case-sensitive `_build` output component from Python
  package discovery while retaining `_Build` and `_build-example` source
  directories.
- Extended the Python shared-registry consumer through the emerging OCaml
  package and program records, with the generated Dune decoy excluded.

