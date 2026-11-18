---
category: Repo policy / workflow reminders
---

# Use an installed GitHub client when the project mise config is untrusted

Invoking `gh` inside a fresh sibling worktree can select the mise shim before
an installed GitHub client. If that worktree's project configuration has not
been trusted, mise rejects the command before it reaches GitHub. Check which
client will run before publishing; when the shim is blocked, invoke the
already-installed GitHub client by its explicit path. Do not weaken the local
trust boundary merely to publish a branch or pull request.
