---
category: Repo policy / workflow reminders
---

# Verify optional search roots before passing them to ripgrep

A capability-test discovery command passed `code/tests` to `rg` without first
checking that the directory exists. The piped search still found the relevant
test under `code/scripts/tests`, but emitted an avoidable missing-path error.
Use `rg --files` to discover candidate roots or guard optional paths with
`test -d` before including them in multi-root searches.
