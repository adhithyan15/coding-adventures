---
category: Security boundaries
---

# Containment checks must canonicalize both filesystem paths

**What went wrong.** An installed-package smoke compared a real path returned by
Node's resolver with a temporary-directory path kept in its original spelling.
On macOS, `/var/...` and `/private/var/...` can name the same directory. A
lexical `path.relative` therefore classified an installed package as outside
its consumer and failed the check.

**Fix.** Resolve both the parent and candidate through `realpath` before doing
a containment comparison. Check that a map target exists before resolving it,
then compare its canonical target and derive its package-relative path from that
canonical value. Keep the check that the installed path is inside the external
consumer and outside the source tree; canonicalization fixes aliases without
widening either boundary.

For cleanup, first inspect the exact lexical target with `lstat`, reject a
symlink as the owned root, verify its marker, and only then use canonical paths
for containment. If tests create a directory symlink or junction to exercise
alias handling, remove only that exact link non-recursively and verify its
target still exists. Do not recursively remove a resolved path that could name
outside the owned temporary root.

**Next time.** Treat path spelling and filesystem identity as different
things. Test a real directory alias, a real sibling outside the consumer, and a
source-tree alias; assert the same inside/outside policy after canonicalization.
