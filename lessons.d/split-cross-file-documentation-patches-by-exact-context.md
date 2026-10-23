---
category: Repo policy / workflow reminders
---

# Split cross-file documentation patches by exact context

A broad patch spanning manifests, changelogs, READMEs, and source comments
failed atomically because one wrapped prose fragment did not match the current
README. Re-read each prose file immediately before editing and use small,
file-local patches with exact surrounding text so one context mismatch cannot
delay or obscure the independent version and boundary updates.
