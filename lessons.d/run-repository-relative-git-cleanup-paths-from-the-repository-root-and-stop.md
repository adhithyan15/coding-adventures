---
category: Repo policy / workflow reminders
---

# Run repository-relative Git cleanup paths from the repository root and stop validation when cleanup fails

After installing a TypeScript dependency chain, a lockfile cleanup named paths
relative to the repository root but ran from the leaf package directory. Git
rejected every pathspec. The following test command still ran because the shell
sequence did not stop on that failure, so its green result did not prove the
worktree was clean.

Run repository-relative cleanup from the repository root (or spell paths
relative to the current directory), join cleanup and validation with `&&`, and
inspect the lockfile diff afterward. Pass multiple paths as explicit arguments
or a real shell array: zsh does not split a newline-delimited scalar into
pathspecs merely because its expansion is unquoted. This keeps a valid test
result from being mistaken for successful installer-churn cleanup.
