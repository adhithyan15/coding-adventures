# OCAML04 — Process-free canonical build substrate

Status: in progress

## Purpose

This contract admits the emerging OCaml lane to the repository-owned,
process-free parts of the canonical Go build tool. It owns package and program
discovery, local dependency resolution, source hashing, BUILD validation,
affected/prerequisite propagation, shard cost, toolchain detection, and the
main CI workflow's `needs_ocaml` marker.

It does not run opam or Dune. Command execution, opam-switch serialization,
trusted-execution conformance, a native OCaml build-tool implementation,
representative packages, promotion, and entry into the established-language
denominator remain separate roadmap items.

## Discovery and identity

`code/packages/ocaml/<name>` and `code/programs/ocaml/<name>` roots with a
selected BUILD file are discovered as `ocaml/<name>` and
`ocaml/programs/<name>`. The existing specification and scaffold fixture trees
and exact Dune `_build` output trees remain excluded from package discovery
even though they contain BUILD files.
Duplicate normalized graph identities fail closed under the shared discovery
contract.

## Local dependency resolution

The resolver uses only checked-in data inside the declaring package root. It
does not execute opam or Dune, resolve external packages, follow pins, read
switch state, or inspect referenced dependency directories.

Directory aliases are the lower-case root basename, its
`coding-adventures-<basename>` opam form, and its
`coding_adventures_<basename-with-underscores>` Dune form. When both a package
and program expose the same alias, the package is authoritative. A dependency
that resolves to the declaring graph identity is ignored rather than creating
a self-edge.

An OCaml root contributes manifest aliases and opam dependency candidates only
when it has exactly one regular, non-directory `.opam` file directly in the
root. Multiple manifests are ambiguous and contribute neither. The manifest's
top-level `name:` string and filename stem are aliases. Dependency candidates
come only from quoted strings in the top-level `depends: [ ... ]` field.
Comments, filters, constraints, build commands, descriptions, pins, other
fields, malformed or unterminated fields, and external names create no edge.
Two library packages claiming the same declared alias make that alias
ambiguous and unusable; a library still takes precedence over a same-named
program.

Dune dependency candidates come only from `(libraries ...)` fields in the
fixed regular root-local files `dune`, `src/dune`, `bin/dune`, and `test/dune`.
Comments, strings outside that field, library/executable names, public names,
preprocess/instrumentation fields, nested expressions, variables, external
libraries, and all other files or forms create no edge. Duplicate candidates
collapse to one edge. The opam and Dune candidate sets are unioned and sorted.

## Source hashing

Legacy shell BUILD packages hash selected BUILD files plus `.ml`, `.mli`, and
`.opam` files and the exact special filenames `dune`, `dune-project`, and
`.ocamlformat`. Relative-path sorting, raw-byte hashing, file boundaries,
dependency-hash propagation, and generated-directory exclusion follow the
shared build-tool contract. README, changelog, `_build`, and host opam-switch
state do not enter the package hash.

## Validation, affected work, and sharding

OCaml is an isolated-build language. Every transitive local dependency
inferred from opam or Dune metadata must therefore appear in the selected BUILD
recipe's explicit local pin/bootstrap paths, unless the BUILD is an existing
reviewed intentional skip. Undeclared sibling references and missing
standalone prerequisites fail the shared validator.

Generic graph selection propagates a changed OCaml prerequisite to its
dependents, and a selected dependent expands back to its complete prerequisite
closure. Shards remain prerequisite-closed. OCaml uses the same base cost as
other compiler/package-manager lanes (`dotnet`, Haskell, Swift, and
TypeScript), plus its BUILD command count. Its shard toolchain key is `ocaml`.

## CI marker

`ocaml` is present in the closed Go toolchain registry. Incremental detection
emits `needs_ocaml=true` only for affected OCaml packages or an OCaml-scoped CI
workflow change. Force or unknown-diff mode emits it as true with every other
known toolchain. The main CI detect job binds
`needs_ocaml: ${{ steps.toolchains.outputs.needs_ocaml }}` and explicitly
normalizes `needs_ocaml=true` on the forced main-build path.

The generic CI job does not install or execute OCaml in this tranche. The
separate commit-pinned `build-ocaml.yml` workflow remains the OCAML03 execution
authority until execution integration is reviewed.

## Required conformance

Tests must cover package/program identities and fixture exclusion; opam and
Dune field isolation; comments, aliases, ambiguity, duplicate candidates, and
self-edge safety; source and metadata hash invalidation; BUILD prerequisite
validation; affected and prerequisite propagation; OCaml shard cost and
toolchain output; `needs_ocaml` incremental and force behavior; CI marker
classification; and the exact main-workflow binding and normalization.
