# OCAML02 — Scaffold and metadata infrastructure

Status: in progress

## Purpose

This contract makes OCaml a scaffoldable emerging lane without promoting it
into the established-language denominator. It owns deterministic Go and
TypeScript front doors, checked-in opam and Dune metadata, starter tests,
formatting and coverage commands, capability metadata, repository ignores, and
the lane README. Canonical build-tool discovery, CI toolchain provisioning,
representative packages, capability analysis, the native build tool, and lane
promotion remain separate roadmap items.

## Exact direct tool metadata

Generated metadata uses these exact reviewed versions:

- OCaml `5.2.1`;
- opam `2.5.2` for CI/toolchain provisioning;
- Dune `3.17.2` (while emitting the stable Dune language `3.16`);
- Alcotest `1.9.0`;
- `bisect_ppx` `2.8.3`; and
- `ocamlformat` `0.27.0`.

No generated file may contain a network pin, bootstrap download, `depext`,
install/remove hook, or user-controlled command.

These are exact direct constraints, not a transitive opam solver lock. The
repository snapshot, switch state, and generated `.opam.locked`/equivalent
proof are owned by the separately tracked `ocaml-ci-toolchain` item before
three-platform CI can claim reproducibility.

## Names and dependencies

The input directory name is validated kebab case. For `my-pkg`:

- opam and Dune project/package name: `coding-adventures-my-pkg`;
- private library and OCaml module basename: `coding_adventures_my_pkg`; and
- local dependency `graph`: opam package `coding-adventures-graph`, with a pin
  path resolved relative to the generated target (`../graph` for package
  siblings or `../../../packages/ocaml/graph` from a program).

Only validated repository package/program directories may become local
dependencies. Symlinked or repository-escaping dependency paths fail closed.
The checked-in opam file lists direct dependencies. `BUILD` and
`BUILD_windows` pin the complete leaf-first local closure before installing the
current package dependencies. The generator dependency reader recognizes only
`"coding-adventures-<kebab>"` entries from the `depends` list.

## Exact output trees

Both front doors MUST match the shared byte-for-byte golden trees under
`code/specs/fixtures/scaffold-generator/ocaml-library` and
`ocaml-program`. Common `README.md` and `CHANGELOG.md` files remain governed by
the existing scaffold contract and are intentionally outside the golden tree.

Library:

```text
.gitignore
.ocamlformat
BUILD
BUILD_windows
coding-adventures-my-pkg.opam
dune-project
required_capabilities.json
src/coding_adventures_my_pkg.ml
src/coding_adventures_my_pkg.mli
src/dune
test/dune
test/test_my_pkg.ml
```

Program adds `bin/dune` and `bin/main.ml` to the same testable library core.
Its capability manifest declares only `stdout:write` to `*`; the library
manifest is the explicit empty pure-computation profile.

## Build behavior

Both platform build files contain real fixed commands and no skip-success path.
They pin local siblings without running them, install the exact direct
metadata constraints with test and development flags, verify formatting with
`dune build @fmt`, run Alcotest through `dune runtest`, and emit a
`bisect-ppx-report` coverage summary. Instrumentation is test-only.

## Input and serialization safety

All validation completes before the target directory is created. Package and
dependency names reject traversal, separators, dot segments, uppercase, and
invalid opam names. Descriptions reject controls, NUL, U+0085, U+2028, U+2029,
`*/`, OCaml comment termination `*)`, and Dune interpolation opener `%{`.
Printable descriptions, including quotes, backslashes, hashes, parentheses,
semicolons, backticks, and shell-like text, are encoded as data in Dune/opam
strings and never enter a build command.
Accepted Unicode remains identical raw UTF-8 across both front doors; only
quotes and backslashes receive OCaml/opam-compatible escapes. Dry-run writes
nothing; existing or symlink targets fail closed.

## Conformance

Tests MUST cover language registration, library and program generation, empty
and local dependency cases, byte identity against both shared golden trees,
dependency parsing, dry-run, overwrite/symlink refusal, invalid names,
adversarial description serialization, capability-schema validation, fixed
build commands, and absence of dangerous starter constructs.
