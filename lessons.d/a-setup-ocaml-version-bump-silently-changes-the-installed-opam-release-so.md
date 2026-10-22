---
category: Supply chain & CI pinning
---

# A setup-ocaml version bump silently changes the installed opam release, so the OCaml toolchain lock must keep its reviewed action commit

**What went wrong.** A Dependabot group PR (#15723) moved `ocaml/setup-ocaml`
from v3.7.0 (`15d66000…`) to v3.9.0 (`93303b62…`) and `actions/upload-artifact`
from v7.0.1 to v7.0.2 in every workflow. Four checks failed:

- "OCaml capability gate" in `ci.yml` printed `opam-version 2.6.1` and then
  failed `test "$(opam --version)" = "2.5.2"`. setup-ocaml has no exact opam
  version input. It installs the newest stable opam inside a range that is
  hard-coded in each release: `<2.6.0` in v3.7.0 and `<2.7.0` in v3.9.0. A
  minor action bump is therefore an opam upgrade.
- "Validate OCAML03 contract" and "Repo-wide metadata contracts" failed with
  "workflow actions must equal the reviewed action allowlist" and "generic CI
  OCaml setup must equal the reviewed identity". `ocaml_toolchain_lock.py` and
  `toolchain-lock.json` pin both action commits.
- "Validate OCAML07 representative CI contract" failed with "workflow digest
  differs from the closed OCAML07 identity". `build-ocaml-representative.yml`
  is pinned byte for byte.

**Fix.** setup-ocaml stays at the reviewed v3.7.0 commit in all three
workflows. Moving to v3.9.0 means a reviewed opam 2.6 upgrade of the OCAML03
lock (versions, evidence, and the spec's "below 2.6" statement), which does
not belong in an action bump. The upload-artifact v7.0.2 change (only an
`@actions/artifact` dependency bump, with no new inputs) was re-pinned
deliberately. That meant changing `ACTIONS` in both contract scripts, both
fixture manifests, and the OCAML07 workflow digest in the manifest and in
`EXPECTED_WORKFLOW_SHA256`. The test in `test_ci_gate_registry.py` that pins
the `download-artifact` commit in the Forme release verdict job also moved
to v8.0.2.

**Next time.** For an action bump PR, run
`python3 code/scripts/ocaml_toolchain_lock.py validate-repository`,
`python3 code/scripts/ocaml_representative_ci.py validate-repository`, and
the `test_ocaml_*` and `test_ci_gate_registry` suites before anything else.
Also grep `code/scripts` and `code/specs/fixtures` for every old commit SHA
in the diff. For setup-ocaml, read `OPAM_STABLE_VERSION_RANGE` in the new
release's `packages/setup-ocaml/src/opam.ts`. If the range changed, the bump
is a toolchain upgrade, not a pin refresh.
