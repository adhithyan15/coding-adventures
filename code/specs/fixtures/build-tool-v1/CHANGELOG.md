# Changelog

## 2026-07-29

- Added the process-free bootstrap conformance runner.
- Added standalone result, implementation-inventory, and build-plan schemas.
- Added the 16-lane implementation inventory with 12 present front doors,
  three missing established implementations, and emerging OCaml.
- Added seven representative discovery, resolution, graph, and plan cases.
- Added bounded parsing, two-phase in-memory workspace preflight,
  domain-aware canonical comparison, and fail-closed execution rejection.
- Expanded the corpus from seven cases and four domains to 30 cases covering
  all 11 process-free v1 domains.
- Added a closed pure-domain schema, conservative unknown-path handling,
  framed hash/cache oracles, bounded inline-only Starlark records,
  prerequisite-closed shard verification, normalized BUILD-file validation
  snapshots, the complete toolchain registry including OCaml, and CLI exit
  decisions.
- Added semantic reference, path, hash, cache, Starlark load, shard,
  diagnostic, and toolchain checks while preserving the zero-process,
  zero-materialization bootstrap boundary.
