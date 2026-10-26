---
category: Repo policy / workflow reminders
---

# Sparse checkout additions must be staged explicitly before publication

The native Rust package was created outside the sparse path list. A combined
PowerShell invocation continued after `git add` warned that those files were
not staged, publishing only the scripts/docs commit before the package itself.
No PR was opened for that incomplete branch.

Add the new package directory with `git sparse-checkout add` before staging,
check the staging command exit code, and inspect `git diff --cached --stat`
against the intended source inventory before committing or pushing. Preserve
the published history and follow with the missing validated package; never
assume a successful later command proves every earlier command succeeded.
