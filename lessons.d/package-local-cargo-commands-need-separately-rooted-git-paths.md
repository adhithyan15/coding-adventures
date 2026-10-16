---
category: Repo policy / workflow reminders
---

# Package local Cargo commands need separately rooted Git paths

The first D3a scaffold command ran Cargo from the standalone compiler directory
but reused a repository-relative path in git add. Git refused the doubled path,
so its following commit did not happen; Cargo nevertheless compiled and hit the
expected unknown-field assertion. That dirty-source run is not the frozen-head
receipt. Commit from the repository root, then use a saved wrapper with explicit
and separate Git and Cargo working directories, clean-head checks and actual
exit capture. Shell command lists do not stop automatically after an earlier
Git failure. Read the actual generated lesson template before filling it: an
assumed placeholder also caused an apply_patch guard refusal before mutation.
