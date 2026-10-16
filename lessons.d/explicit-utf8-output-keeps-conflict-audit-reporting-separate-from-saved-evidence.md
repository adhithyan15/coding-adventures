---
category: Cross-platform & Windows BUILD_windows
---

# Explicit UTF8 output keeps conflict-audit reporting separate from saved evidence

The read-only conflict inspector saved all fourteen merged-tree hunk groups as
UTF8 JSON, then failed while printing an arrow through Windows cp1252 stdout.
The saved audit was complete; the reporting failure supplied no merge or test
acceptance. Preserve that artifact and failed helper, then use a separate reader
with stdout.reconfigure(encoding='utf-8') instead of rerunning one-time writes.
