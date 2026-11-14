---
category: BUILD files & dependency management
---

# Run scaffold-generator from its owning program directory because it resolves repository layout from the current working directory

Invoking a separately built scaffold-generator binary from the repository root
made it search for `scaffold-generator.json` in the repository's parent
directory and fail before generating files. The program derives repository
paths from its process working directory rather than from the executable path.
Build and run it from `code/programs/go/scaffold-generator`, including dry-run
and real generation, so its relative climb reaches the actual repository root.
