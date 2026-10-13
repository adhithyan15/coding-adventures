---
category: Repo policy / workflow reminders
---

# Do not assume CODEX_HOME is exported in local desktop shells; resolve the Codex data directory explicitly before inspecting automation files.

An automation inspection command expanded an unset `CODEX_HOME` to the filesystem root and therefore looked for `/automations/...`, which did not exist. Resolve the Codex data directory from the known desktop location (or verify the variable is non-empty first) before constructing the path. This prevents misleading missing-file errors and avoids passing unresolved environment state into filesystem commands.
