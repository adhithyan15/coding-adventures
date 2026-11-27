---
category: Security boundaries
---

# A Git blob can be a symlink, not a regular file

Git records a symbolic link as a blob too, with mode `120000`. Checking only
`git cat-file -t` proves that the object contains bytes; it does not prove that
the repository path is a regular file. A mode-only change can keep the same
blob bytes, so comparing file contents before checking the tree mode can skip a
security decision even though a checkout follows the link to different data.

When a policy consumes repository files as data, inspect the exact `git ls-tree`
entry before reading the blob. Require an ordinary file mode (`100644` or
`100755`) for manifests, schemas, approvals, signatures, and trusted helpers.
Test mode-only symlink changes with `git update-index --cacheinfo 120000,...`;
that reproduces Git's tree semantics without relying on symlink privileges in
the test environment.
