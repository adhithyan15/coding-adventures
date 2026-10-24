---
category: BUILD files & dependency management
---

# Run the diff-based build tool after committing the intended changes

The repository build tool completed discovery and orphan validation but
reported `Git diff: no package files changed` because the implementation was
still only in the working tree. Its affected-package selector compares
committed history with `-diff-base`; it does not include unstaged or staged
content. After focused tests and diff review, commit only the intended files,
then run the current-source build tool against the exact branch base. Treat a
zero-change plan before that commit as an invalid affected-closure check, not
as proof that no packages need validation.
