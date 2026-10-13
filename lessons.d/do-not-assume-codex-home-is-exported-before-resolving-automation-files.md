---
category: Repo policy / workflow reminders
---

# do not assume CODEX_HOME is exported before resolving automation files

I expanded `$CODEX_HOME/automations` without first checking whether the desktop
environment exported `CODEX_HOME`; because it was empty, the read-only search
targeted `/automations` and failed. Use the app's automation view/update tool
first. If a local automation file really must be inspected, resolve the known
Codex configuration directory explicitly only after verifying that location,
rather than allowing an unset variable to become a root-relative path.
