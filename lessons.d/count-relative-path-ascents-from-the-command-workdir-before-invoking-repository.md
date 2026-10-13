---
category: Repo policy / workflow reminders
---

# Count relative path ascents from the command workdir before invoking repository scripts; package roots nested under code need three ascents to code/scripts, not four.

From `code/packages/typescript/human-language-data`, a command used `../../../../scripts/lessons.py`; four ascents reach the repository root, where `scripts/lessons.py` does not exist. The validator lives under `code/scripts`, reached as `../../../scripts/lessons.py` from that package. Prefer running repository-wide validators from the worktree root with their canonical path, or verify each ascent before using a package-local workdir.
