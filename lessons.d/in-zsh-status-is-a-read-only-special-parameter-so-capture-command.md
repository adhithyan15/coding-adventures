---
category: Repo policy / workflow reminders
---

# In zsh status is a read-only special parameter so capture command exits in a task-specific variable

A tests-first validation wrapper temporarily disabled `set -e`, ran the expected
failing Rust test, and then tried to save `$?` in `status`. The repository shell
is zsh, where `status` is a read-only special parameter, so assignment aborted
the wrapper before it could inspect the compiler error or remove its temporary
log. Use a task-specific name such as `test_exit` whenever a zsh command exit
must be retained, and clean any already-created temporary artifact before
rerunning the wrapper.
