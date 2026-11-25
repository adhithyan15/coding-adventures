---
category: BUILD files & dependency management
---

# Run repository scripts from the repository root or make their path package-relative

# Run repository scripts from the repository root or make their path package-relative

When a command is launched from a package working directory, a repository-root
path such as `code/scripts/lessons.py` resolves beneath the package and fails.
Either run the command from the repository root or use a path relative to the
known package directory. Keep BUILD commands package-relative; use repository
scripts with a calculated or documented root path.

This occurred in the final lesson validation after acceptance commands ran
from the Mosaic package directory. The validation itself had passed from the
repository root earlier.
