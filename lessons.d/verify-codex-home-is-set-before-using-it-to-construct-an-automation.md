---
category: Repo policy / workflow reminders
---

# Verify CODEX_HOME is set before using it to construct an automation path

I assumed `CODEX_HOME` was exported and constructed an automation path from it; because it was empty, the command searched `/automations` and failed. Before using an optional environment variable in a filesystem path, verify it is non-empty and otherwise use the known absolute Codex home path or the automation API.
