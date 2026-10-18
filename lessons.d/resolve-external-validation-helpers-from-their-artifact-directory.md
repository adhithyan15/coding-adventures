---
category: Testing & coverage
---

# Resolve external validation helpers from their artifact directory

A read combined a repository test module with an external validation helper
under the repository working directory. The test module was read, but PowerShell
reported the helper absent because it lives in the task artifact directory.
Use the recorded absolute artifact path, or a separate command with that
directory as its working directory. A missing-path read is not a validation
failure and provides no evidence about the helper's contents or execution.

