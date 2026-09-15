---
category: Cross-platform & Windows BUILD_windows
---

# uv workspace membership on Windows

creates the venv at the workspace root, sharing it across parallel package builds (race condition wipes pytest). Don't add new packages to `[tool.uv.workspace]` members unless intentional. Fix unresolvable workspace deps by removing the offending member, not by adding the missing dep to the workspace.
