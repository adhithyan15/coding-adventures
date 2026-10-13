---
category: Repo policy / workflow reminders
---

# Run repository-root scripts from the repository root even when package validation runs from a nested directory

I invoked `code/scripts/lessons.py` while the shell was still in a nested TypeScript package, so Python resolved the repository-relative path beneath that package and failed to find the script. Run package-local commands from the package directory, then switch the command working directory back to the repository root before invoking repository-relative scripts.
