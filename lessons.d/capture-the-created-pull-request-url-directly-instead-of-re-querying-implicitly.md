---
category: Repo policy / workflow reminders
---

# Capture the created pull request URL directly instead of re-querying implicitly

After `gh pr create` printed the new URL, a follow-up command tried
`gh pr view --repo ...` without an explicit PR target. In this invocation form
GitHub CLI requires a number, URL, or branch and returned an argument error,
leaving the shell variable empty. Capture `gh pr create` stdout directly, or
pass the known head branch explicitly to every later `gh pr view`/`merge`
command; do not depend on implicit current-branch lookup when `--repo` is set.
