---
category: Repo policy / workflow reminders
---

# Match command path prefixes to the selected workdir before renaming repository files or invoking repository tools

A rename command used repository-root-relative paths while its `workdir` was
already the package directory, so the source path was doubled and did not
exist. The immediate lesson-recording retry then guessed the repository tool's
relative depth and also missed it. For cross-directory operations, either run
from the repository root with repository-relative paths or use package-relative
paths consistently; do not mix the two, and prefer the already verified root
invocation for repository tools.
