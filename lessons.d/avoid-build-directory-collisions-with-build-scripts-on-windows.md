---
category: Cross-platform & Windows BUILD_windows
---

# Avoid build directory collisions with build scripts on Windows

Windows treats names that differ only by case as the same path. A generated
`build/` directory therefore prevents creating the repository's conventional
`BUILD` script in the same package. Keep compiler outputs under task `work/`
and remove only verified generated artifacts before adding the script.
