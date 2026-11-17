---
category: CI & GitHub Actions
---

# Git auto-maintenance races test walks of fixture repos

Newer git (the ubuntu-26.04 runner's) runs `git maintenance run --auto` after
commits, creating and deleting `.git/objects/maintenance.lock` (and auto-gc may
rewrite objects) in a process that can outlive the command. A test that builds a
template repository and then copies or walks it saw the lock in readdir and
failed on the lstat (`no such file or directory`) - a flake that never shows on
a developer's older git (kiln D29).

Fix the race, do not tolerate it: every git a fixture helper starts gets
`maintenance.auto=false`, `gc.auto=0`, `gc.autoDetach=false` (GIT_CONFIG_COUNT
env, and persisted in the template's config). A fixture repo has nothing to
maintain. Treat a vanished `.git/**/*.lock` in a walk as a belt-and-braces skip
only. When a CI-only ENOENT appears in a tree walk, suspect a background
process touching the tree before suspecting the test logic.
