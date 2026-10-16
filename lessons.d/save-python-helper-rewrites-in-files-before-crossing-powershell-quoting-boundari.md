---
category: Cross-platform & Windows BUILD_windows
---

# Save Python helper rewrites in files before crossing PowerShell quoting boundaries

A D2 validator preparation command embedded Python replacement strings inside
PowerShell's quoted python -c argument. Escaped quotes followed Python syntax,
so PowerShell instead parsed a Python set union as its own pipeline and failed
before executing Python or Cargo. No source, lock or target was mutated.

Keep the failed preparation receipt. Write the multiline rewrite as a saved
Python file with exact old-anchor counts, then invoke only that path from
PowerShell. Preserve the already-executed original helper and create a versioned
successor, so existing receipt hashes continue to identify their real code.
