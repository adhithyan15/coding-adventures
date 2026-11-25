---
category: CI & GitHub Actions
---

# OCAML07 representative packages carry their version in five places, so a partial bump fails validate-evidence

An OCAML07 representative package states its version in five places: the opam file, `PACKAGES` in `ocaml_representative_ci.py`, `manifest.json`, the `0.1.0` literals in `build-ocaml-representative.yml`, and the spec. Bumping some of them fails `validate-evidence` with "required evidence file missing: source-archives/...-0.2.0.tar.gz". The `fetching sources failed: missing checksum` line printed for local pins is a non-fatal opam warning; read past it to the real error. Fixed on #19949 (`e9cf8383ffd`) by keeping 0.1.0; LGE-OCAML-REP-VERSION parameterizes the chain. Grep for the old version across the workflow, script, manifest and spec before bumping.
