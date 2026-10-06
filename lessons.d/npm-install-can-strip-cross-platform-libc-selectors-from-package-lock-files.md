---
category: BUILD files & dependency management
---

# npm install can strip cross-platform libc selectors from package-lock files

On macOS, running `npm install` after adding a dependency can silently remove
the `libc` selectors from optional Linux native-package entries in an existing
lockfile. Those removals are unrelated churn and weaken the lock's
cross-platform metadata. Inspect the lockfile diff immediately after npm runs,
restore every unrelated selector, and retain only the root dependency plus its
actual transitive lock entries. A successful local install is not evidence that
the generated lock diff is portable.
