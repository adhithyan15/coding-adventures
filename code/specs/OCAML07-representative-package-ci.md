# OCAML07 — Representative package execution CI

Status: normative

OCAML07 closes the execution boundary left open by OCAML05 and OCAML06.  The
four representative libraries already have reviewed APIs, package-local tests,
and capability metadata.  This contract requires one commit-pinned workflow to
build, test, install, document, archive, analyze, and consume that dependency
chain on every OCAML03 target.

The workflow is independent of `.github/workflows/build-ocaml.yml`.  OCAML03
owns that workflow's exact three-job solver contract, so representative-package
execution must not widen its job set or weaken its validator.

## 1. Closed authority

The checked authority is:

```text
code/specs/fixtures/ocaml-representative-ci-v1/manifest.json
code/specs/fixtures/ocaml-representative-ci-v1/README.md
code/specs/fixtures/ocaml-representative-downstream-v1/
code/scripts/ocaml_representative_ci.py
code/scripts/tests/test_ocaml_representative_ci.py
.github/workflows/build-ocaml-representative.yml
```

`manifest.json` is a closed schema.  It records the reviewed action commits,
tool versions, opam-repository commit, runner triples, package order and local
dependency edges, minimum coverage, downstream fixture, artifact retention,
and SHA-256 identities for the workflow and governed source trees.  Unknown or
missing keys fail validation.  Every relative path must remain inside the
repository, name a regular file or directory without linked path components,
and use `/` separators.

The repository validator is offline and fail closed.  It validates the
manifest, governed tree identities, exact workflow identity, workflow trigger
surface, job graph, action allowlist, permissions, and required commands.  It
must reject skipped or masked work, mutable action tags, unexpected token or
secret use, `continue-on-error`, and shell success masking such as `|| true`.

## 2. Toolchain and targets

The execution job uses the OCAML03 reviewed identities:

- OCaml 5.2.1;
- opam 2.5.2;
- Dune 3.17.2;
- Alcotest 1.9.0;
- bisect_ppx 2.8.3;
- ocamlformat 0.27.0;
- opam-repository commit `ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff`;
- the OCAML03 commits for checkout, setup-ocaml, and upload-artifact.

OCAML07 additionally pins odoc 3.0.0.  Later odoc releases require a newer Dune
than the reviewed OCAML03 toolchain and therefore are not interchangeable.
Every machine-read version probe, including odoc, yojson, and installed
representative packages, must disable runner-forced color before exact
comparison and before writing retained toolchain evidence.

The matrix is exactly Linux x64 on `ubuntu-24.04`, macOS arm64 on `macos-14`,
and Windows x64 using mingw on `windows-2022`.  The workflow checks the runtime
OS and architecture before package work begins.

## 3. Representative dependency chain

The only representative packages, in leaf-first execution order, are:

1. `logic-gates` (`coding-adventures-logic-gates`);
2. `graph` (`coding-adventures-graph`);
3. `directed-graph` (`coding-adventures-directed-graph`), depending on graph;
4. `state-machine` (`coding-adventures-state-machine`), depending on
   directed-graph and therefore transitively on graph.

Each package is copied to a clean runner-temporary sibling.  `_build`, coverage
files, generated documentation, and archives from the checkout must not become
inputs.  Every step that changes into a runner-temporary directory must capture
the reviewed repository-local switch identity before leaving the checkout and
export that exact switch explicitly.  For every package the workflow must:

1. validate its opam file strictly without an upstream network probe;
2. install only the declared local predecessors needed for resolution;
3. install exact development and test dependencies;
4. run `dune build @fmt`;
5. run the package tests with bisect_ppx instrumentation;
6. require numeric per-production-file coverage of at least 95 percent;
7. build `@install` and `@doc` with warnings treated as errors;
8. create a source archive and verify that every entry is relative, normalized,
   non-linked, inside one package root, and free of `_build`, coverage, VCS, and
   generated artifact paths;
9. install the package into the switch before moving to the next package; and
10. run the OCAML06 analyzer against that package independently and retain its
    successful receipt.

No package may be replaced by a synthetic smoke package, and one successful
package may not stand in for the other three.

## 4. Installed downstream proof

After all four packages are installed, the workflow copies
`ocaml-representative-downstream-v1` to a second clean temporary directory and
runs its format and test aliases.  Its Dune project keeps
`implicit_transitive_deps false`.

The full executable declares and exercises all four public libraries.  The leaf
executable declares only state-machine and exercises its graph-backed reachable
state path.  Both JSON receipts must byte-match the checked expected receipts.
This proves public installation, direct linking, and the declared transitive
dependency chain without access to package source directories.

## 5. Evidence and security

Each target uploads one artifact, retained for seven days, containing:

- per-package numeric coverage summaries;
- generated documentation;
- verified source archives and archive receipts;
- successful analyzer receipts for all four packages;
- the exact installed-package receipt; and
- both downstream execution receipts.

The workflow has only `contents: read`, disables checkout credential
persistence, uses no repository or environment secrets, and has no write token.
It deliberately executes the checked-in package BUILD fronts and tests, including
pull-request code, only on secretless ephemeral hosted runners.  The governed
representative packages declare no network or process capability.  All
automatic-token use is limited to the reviewed setup action input needed to pin
the opam repository commit.

## 6. Validation boundary

Local validation consists of the OCAML07 Python contract suite and repository
validator, OCAML03 and OCAML06 contract suites, the state DAG validator, YAML
parsing, and diff/security checks.  A supported local OCaml switch should also
run the real four-package chain and downstream fixture.  CI remains the
authoritative cross-platform proof for all three reviewed targets.

OCAML may enter an all-language denominator only after this workflow and the
remaining resolver, reporter, and native build-tool owners have merged.  This
contract alone does not promote the lane.
