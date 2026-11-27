---
category: CI & GitHub Actions
---

# Recompute OCAML07 governed tree digests from a clean checkout: the digest walks the working tree and includes git-ignored build outputs

**What went wrong.** LGE-OCAML-HARDEN (#20153) recomputed the
`code/packages/ocaml/logic-gates` digest in `ocaml-representative-ci-v1/manifest.json`
after a local `dune build`. `_tree_sha256` in `code/scripts/ocaml_representative_ci.py`
walks the *working tree* (`rglob`), skipping only `.git`, `_build`, `__pycache__`,
`.pytest_cache` and `*.coverage` — so the git-ignored, dune-generated
`coding-adventures-logic-gates.install` was hashed in. `validate-repository` passed
locally (same dirty tree) and failed in CI's clean checkout:
"governed tree digest mismatch: code/packages/ocaml/logic-gates".

**Fix.** Recompute the digest over a clean export
(`git archive HEAD code/packages/ocaml | tar -x -C <tmp>`, then `_tree_sha256(<tmp>/<root>)`),
update the manifest, and delete the generated `.install` file before re-validating.

**Do differently.** Before committing any OCAML07 digest, run `git clean -ndX
code/packages/ocaml/<pkg>` (or validate on a `git archive` export); a local
`validate-repository` pass on a built tree proves nothing.
