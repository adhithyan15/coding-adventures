---
category: BUILD files & dependency management
---

# A lane pinned under a shared merged owner item cannot be re-pinned alone

`barcode-layout-1d` for Perl needs a one-line Windows portability fix
(`tempfile()` -> `tempdir()`) and a `BUILD_windows`. Its lane tests pass locally,
so the earlier lesson's blocker ("I cannot run the lane") no longer applies. A
second blocker does: the registry's contract test requires
`entry.verified_revision == owner_item.implementation_revision` and
`entry.adoption_pr == owner_item.pr_number`, and the Perl entry shares its owner
item (`barcode-layout-existing-lanes-v1-conformance`, merged #16343) with six
other languages. Re-pinning Perl alone breaks the equality; re-pinning the shared
item claims the other six lanes were re-verified at a new revision, which nobody
ran.

Do it in its own PR that adds a dedicated owner item for the Perl lane with the
real PR number (so the PR exists first), re-runs the lane, and pins the new tree.
Do not rewrite the shared item, and do not paste a tree hash you did not produce.
