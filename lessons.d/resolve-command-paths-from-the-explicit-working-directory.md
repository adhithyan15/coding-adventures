---
category: Repo policy / workflow reminders
---

# Resolve command paths from the explicit working directory

When a command runs with a package directory as its working directory, pass
paths relative to that package. Prefixing them with the repository's `code/`
tree silently looks for a second nested `code/packages/...` path. Confirm the
working directory before retrying a failed command.
