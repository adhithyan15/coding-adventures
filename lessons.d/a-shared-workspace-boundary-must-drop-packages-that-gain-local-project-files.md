---
category: CI & GitHub Actions
---

# A shared workspace boundary must drop packages that gain local project files

The repo-wide metadata CI job failed because ten Haskell packages had gained
package-local `cabal.project` files, while the checked-in repository source
boundary still claimed they inherited the shared project. The canonical-root
test caught the mismatch. When adding a local project file, remove that package
from the shared boundary in the same PR and refresh the boundary digest in its
corpus cases and pinned summary test.
