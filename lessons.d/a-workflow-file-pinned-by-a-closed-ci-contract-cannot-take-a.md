---
category: CI & GitHub Actions
---

# A workflow file pinned by a closed CI contract cannot take a repo-wide workflow edit

**What went wrong.** A sweep changed `concurrency.cancel-in-progress` in 13
workflows so main runs are never cancelled. One of them,
`.github/workflows/build-ocaml-representative.yml`, has its exact bytes pinned
by the closed OCAML07 contract: `EXPECTED_WORKFLOW_SHA256` in
`code/scripts/ocaml_representative_ci.py`, plus `workflow_sha256` in its
manifest. CI failed with "workflow digest differs from the closed OCAML07
identity".

**Fix.** Restore that one workflow to main's bytes. Leave it out of the sweep,
and say so where the sweep is recorded.

**Next time.** Before a bulk edit to `.github/workflows/`, grep `code/scripts/`
for the workflow file names and for `sha256`/`digest`. Then run every
`validate-repository`-style contract check locally. A pinned workflow changes
only with a deliberate, reviewed re-pin in its own PR. It never rides along
with a sweep.
