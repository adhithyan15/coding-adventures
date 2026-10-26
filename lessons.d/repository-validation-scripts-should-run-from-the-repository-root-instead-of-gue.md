---
category: Repo policy / workflow reminders
---

# Repository validation scripts should run from the repository root instead of guessed relative package paths

While validating the Cortex-M4 package from `code/packages/rust`, a chained
command guessed that `../../../scripts/lessons.py` reached `code/scripts`.
It actually reached a nonexistent repository-level `scripts` directory, so
all Rust gates passed and only the lessons invocation failed to start.

Run repository policy scripts from the repository root, where their documented
paths are stable. If package-local commands need their own working directory,
separate those invocations rather than embedding fragile `../` arithmetic in a
long validation chain.
