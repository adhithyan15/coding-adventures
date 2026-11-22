---
category: Repo policy / workflow reminders
---

# Stable patch IDs belong to git patch-id, not git diff

I passed `--stable` to `git diff`, which does not own that option and printed
usage instead of a patch. Generate the ordinary bounded diff first and pipe it
to `git patch-id --stable`; verify both commands succeed before recording the
patch identity used for protected merge verification.
