---
category: CI & GitHub Actions
---

# Editing the OCaml capability analyzer changes a governed tree digest pinned in the OCAML07 manifest

`code/specs/fixtures/ocaml-representative-ci-v1/manifest.json` pins a SHA-256
of the whole `code/packages/ocaml/ca-capability-analyzer` tree under
`governed_trees`. Any change to that package's files fails the "Validate
OCAML07 representative CI contract" check with
`governed tree digest mismatch`. That includes a source edit, a test, a
fixture mirror or a CHANGELOG line. The local `contracts-capability-cage` and
`test_ocaml_capability_gate.py` runs do not exercise this check.

**Do instead:** after touching any governed tree (the list is in
`governed_trees`), recompute and update its digest, then validate:

```bash
cd code/scripts && python3 -c "import ocaml_representative_ci as m; from pathlib import Path; print(m._tree_sha256(Path('../packages/ocaml/ca-capability-analyzer').resolve()))"
cd ../.. && python3 code/scripts/ocaml_representative_ci.py validate-repository
```
