---
category: Repo policy / workflow reminders
---

# Materialise every BUILD dependency path before executing a package BUILD in a sparse worktree

The first local `forme-plugin-host` BUILD attempt in the FM-B075 sparse
worktree reached several `cd ../dependency` lines before those sibling paths
had been materialised. The BUILD file deliberately declares its complete
leaf-to-root installation chain, but sparse checkout can hide a valid declared
dependency just as completely as a missing package.

Before executing a package BUILD in a sparse worktree, read every referenced
path and add the complete dependency closure to the sparse checkout. A partial
checkout is not evidence that the BUILD declaration is wrong, and allowing a
non-`set -e` BUILD to continue after missing-directory diagnostics can obscure
which validation actually ran.
