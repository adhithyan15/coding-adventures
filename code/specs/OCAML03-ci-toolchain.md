# OCAML03 — Cross-platform CI toolchain and solver evidence

Status: in progress

## Purpose

This contract provisions and verifies the first real OCaml execution lane on
Ubuntu, macOS, and Windows. It pins the direct toolchain and opam repository,
preserves one transitive solver lock and installed-package receipt per runner
family, verifies those files before use, and executes both OCaml scaffold
fixtures through their real formatting, test, and coverage commands.

This contract does not add OCaml to canonical build-tool discovery, implement a
representative package, analyze OCaml capabilities, implement the OCaml build
tool, or promote OCaml into the established-language denominator. Hosted-runner
image metadata is diagnostic evidence only; it is not a runner-image TCB
attestation.

## Exact reviewed identities

The CI workflow MUST use:

- OCaml `5.2.1`;
- opam `2.5.2`;
- Dune `3.17.2`, with generated projects using Dune language `3.16`;
- Alcotest `1.9.0`;
- `bisect_ppx` `2.8.3`;
- `ocamlformat` `0.27.0`;
- the reviewed full 40-hex `ocaml/setup-ocaml` action commit;
- the reviewed full 40-hex `actions/checkout` action commit; and
- one reviewed full 40-hex `ocaml/opam-repository` commit.

The workflow MUST fail if a direct installed version differs from the contract.
It MUST pass the full `ocaml-base-compiler.5.2.1` package identity to the setup
action rather than a bare semver value or range that requires live compiler
resolution.
It MUST configure opam with checksum verification and MUST NOT use a moving
repository branch, tag, unpinned action version, `depext`, package pin, network
fallback, cache that can substitute project dependencies, or skip-success path.
Windows uses the setup action's MinGW compiler mode.

The setup action's cache prefix MUST bind the exact source commit, workflow run,
run attempt, target, and phase. Fresh-solve and locked-fixture phases MUST NOT
share or restore a cache from any prior run or rerun, and each phase MUST fail
before dependency installation if Dune, Alcotest, `bisect_ppx`, or
`ocamlformat` is already installed. This prevents a pre-seeded project
dependency solution from influencing either solve.

The setup action currently resolves the newest stable opam below 2.6 rather
than accepting an exact opam version input. The workflow therefore treats the
action commit as reviewed bootstrap code and immediately fails closed unless
`opam --version` is exactly `2.5.2`.

## Closed evidence manifest

`code/specs/fixtures/ocaml-toolchain/toolchain-lock.json` is the authority for
checked-in solver evidence. It is a closed JSON object with:

- `schema_version`, which MUST equal `1`;
- `direct_versions`, a closed map for the six exact versions above;
- `actions`, a closed map of full action commit identities;
- `opam_repository_commit`, a full commit identity;
- `fixture_input_sha256`, the digest shared by the byte-identical library and
  program opam inputs; and
- `targets`, exactly `linux-x64`, `macos-arm64`, and `windows-x64`.

Each target is a closed object containing its GitHub runner label, expected
`RUNNER_OS`, expected `RUNNER_ARCH`, nullable Windows compiler mode, relative
lock and receipt paths, their SHA-256 digests, and diagnostic runner-image
metadata captured when the evidence was generated. Evidence paths MUST be
normalized repository-relative paths inside the fixture directory, MUST name
regular non-symlink files, and MUST NOT traverse outside that directory.

The target evidence lives at:

```text
code/specs/fixtures/ocaml-toolchain/
  README.md
  toolchain-lock.json
  linux-x64/
    coding-adventures-my-pkg.opam.locked
    installed-packages.txt
  macos-arm64/
    coding-adventures-my-pkg.opam.locked
    installed-packages.txt
  windows-x64/
    coding-adventures-my-pkg.opam.locked
    installed-packages.txt
```

The existing scaffold-generator golden trees remain unchanged. Lock evidence is
kept separately because those golden trees are byte-for-byte generator
contracts, while solver output is intentionally platform-specific.

## Evidence generation and verification

The library and program fixture opam files MUST be byte-identical and match the
manifest input digest. Per target, CI performs these phases in isolated copies:

1. create a clean switch from the pinned compiler and repository commit;
2. install the fixture's exact direct dependencies with test and development
   flags, then generate a fresh transitive `.opam.locked` file;
3. write a stable, sorted installed-package receipt;
4. compare the generated lock and receipt byte-for-byte with the checked-in
   evidence whose digests were already verified;
5. create a second clean switch, install from the checked lock, and reassert all
   exact direct versions; and
6. execute both fixture kinds against that locked switch.

Generating a lock from dependencies already installed from that same lock is
circular and MUST NOT count as fresh-solve evidence. The checked receipt is
evidence of the fresh solve, not a claim that packages are portable across
runner families.

## Workflow behavior and security

`.github/workflows/build-ocaml.yml` runs a fail-fast-disabled three-target
matrix. It has read-only repository permissions, uses no repository- or
user-supplied secrets, and uses only commit-pinned actions. The automatic
read-only GitHub token may appear only as the setup action's exact
`github-token` input because the action requires authentication; checkout MUST
NOT persist credentials in Git configuration. The workflow records
`RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, and `ImageVersion` for diagnostics
without treating those mutable labels as an attested host identity.

Both fixture kinds run sequentially on every target. Every nonblank line in the
selected `BUILD` or `BUILD_windows` file is one independent command, matching
the repository build tool's line-oriented behavior. Unix lines execute through
a fresh POSIX shell; Windows lines execute through a fresh `cmd.exe` process
while the setup action's Cygwin/MinGW toolchain remains on `PATH`. CI MUST NOT
use `continue-on-error`, conditional success, `|| true`, or an absent-tool
skip. The run fails unless formatting, Alcotest, and measured `bisect_ppx`
coverage all execute and produce a nonempty coverage artifact.

Before either BUILD file executes, the target's reviewed lock MUST be copied
into that fixture. The line-oriented executor MUST export opam's locked and
checksum-required modes so the BUILD file's dependency-install line cannot
perform an unlocked solve or accept an unchecksummed source.

Pull-request validation may verify this public toolchain and public scaffold
code because it receives no secrets and grants no protected build-tool
execution authority. It MUST NOT be confused with the separately blocked
immutable-runner attestation needed by trusted build-tool invariant probes.

## Static conformance

Repository tests MUST reject:

- missing or unknown manifest keys or targets;
- malformed versions, action commits, repository commits, or SHA-256 digests;
- a workflow identity that differs from the manifest;
- path traversal, symlinks, missing evidence, or digest mismatches;
- non-identical library/program opam inputs;
- lock evidence that omits an exact direct dependency;
- a missing three-platform matrix, weak permissions, mutable action reference,
  cache substitution, secret use, automatic-token use outside the two reviewed
  setup inputs, or skip-success construct; and
- any fixture BUILD file that omits formatting, tests, coverage, or contains a
  blank-success fallback.

The validator is deterministic and offline by default. A separate runtime mode
checks the current target's tool versions and evidence selection without
changing repository files.
