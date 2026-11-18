---
category: Repo policy / workflow reminders
---

# Run lesson tooling with an installed Python when the repository mise runtime is incomplete

Invoking `python3 code/scripts/lessons.py ...` inside a repository checkout can
select the project-pinned mise interpreter. If that interpreter is only
partially installed, even the read-only lesson index fails while mise tries to
repair the runtime. Check which interpreter will run before using the lesson
tooling; when the pinned runtime is unavailable, invoke the script with an
already-installed explicit Python path. Keep that choice limited to the
repository-owned lesson script rather than changing the repository toolchain.
