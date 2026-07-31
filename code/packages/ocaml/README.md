# OCaml implementation lane

OCaml is an `emerging_implementation` lane. Its packages are visible to the
package-parity inventory but remain outside the established 15-language
denominator until the reviewed promotion gates pass.

## Exact direct conventions

- OCaml `5.2.1`
- opam `2.5.2`
- Dune `3.17.2` (Dune language `3.16`)
- Alcotest `1.9.0`
- `bisect_ppx` `2.8.3`
- `ocamlformat` `0.27.0`

Package directories use kebab case. A directory such as `my-pkg` publishes the
local opam/Dune name `coding-adventures-my-pkg`; OCaml compilation units use
the `Coding_adventures_my_pkg` prefix. Direct local dependencies are exact
`0.1.0` opam dependencies, while build scripts pin the complete leaf-first
sibling closure before installing the current package.

The direct constraints are backed by the reviewed OCAML03 CI contract:
Ubuntu x64, macOS arm64, and Windows x64 each preserve a digest-checked
transitive solver lock and installed-package receipt from one pinned
opam-repository commit. CI regenerates and compares that evidence before a
separate locked install executes both scaffold kinds. Mutable hosted-runner
image metadata is diagnostic only and is not an immutable host attestation.

Every package checks in `dune-project`, its `.opam` file, `.ocamlformat`,
Alcotest tests, `BUILD`, `BUILD_windows`, a capability manifest, README, and
changelog. Both build files must run real formatting, tests, and
`bisect_ppx` coverage commands—skip-success placeholders are not acceptable.

Generate a starter through either reviewed front door:

```sh
scaffold-generator my-pkg --language ocaml --description "A pure OCaml package"
```

The exact library and program contracts live in
[`OCAML02-scaffold-infrastructure.md`](../../specs/OCAML02-scaffold-infrastructure.md).
The solver and execution gate lives in
[`OCAML03-ci-toolchain.md`](../../specs/OCAML03-ci-toolchain.md). Canonical
build-tool integration, representative packages, capability analysis, the
native OCaml build tool, and denominator promotion remain explicitly tracked
roadmap items.
