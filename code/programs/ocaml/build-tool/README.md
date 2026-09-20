# OCaml Build-Tool Graph and Diff Core

This directory is the process-free beginning of the native OCaml build tool.
It answers two questions from caller-supplied values:

1. In which deterministic prerequisite-first levels may packages run?
2. Which changed packages, dependent packages, and prerequisite-only packages
   follow from a bounded changed-path snapshot?

It is intentionally not a CLI or complete build tool. It does not discover
packages, read BUILD files, invoke Git, hash checkout files, plan work, cache
results, or execute commands. The later OCaml build-tool owners add those
boundaries without widening this core.

## Native API

`Coding_adventures_build_tool.evaluate_graph` accepts declared package names
and `[prerequisite, dependent]` edges. It returns canonical edges and complete
parallel levels, or a typed stable error such as `Graph_cycle`.

`Coding_adventures_build_tool.evaluate_diff_selection` accepts package roots,
source modes, the same edges, forced seeds, changed paths, an unknown-path
policy, and an optional trusted, prevalidated inert repository-boundary
registry. Independent shape, string-size, scope, and authorization guards run
before digesting or projecting that registry. The operation returns sorted
changed, affected, and prerequisite-only package sets. Structural, digest,
unknown-path, and match-work failures cannot carry partial output because the
public API uses OCaml's `result` type.

The implementation follows Unicode 17 NFC, full case folding, full uppercase,
numeric scalar ordering, strict portable globs, and the fixed 50,000,000-unit
match-work ceiling. SHA-256 boundary validation uses the pure
`digestif.ocaml` implementation.

## Development

Use the repository's reviewed OCaml 5.2.1 switch. Local graph dependencies are
pinned in leaf-to-root order before installing the program dependencies:

```text
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-graph ../../../packages/ocaml/graph
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-directed-graph ../../../packages/ocaml/directed-graph
opam install . --deps-only --with-test --with-dev-setup -y --require-checksums
opam exec -- dune build @fmt @all @install
opam exec -- dune runtest --force
```

The native suite discovers and evaluates all eight graph and eleven
diff-selection fixtures under `code/specs/fixtures/build-tool-v1`. Test-only
filesystem read/list capabilities are declared honestly; production source has
no filesystem or other ambient authority. The literal `BUILD` and
`BUILD_windows` fronts additionally enforce at least 95 percent measured
coverage for the production module.

See `code/specs/OCAML08-build-tool-graph-diff-core.md` and
`code/specs/build-tool-conformance.md` for the normative contract.
