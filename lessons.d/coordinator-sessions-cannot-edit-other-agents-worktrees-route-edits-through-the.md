---
category: Repo policy / workflow reminders
---

# Coordinator sessions cannot edit other agents' worktrees; route edits through the owning agent

A coordinator session cannot edit files in another agent's worktree (isolation blocks it), and working around that by switching branches in a shared checkout disturbs other agents. Route every edit through the owning agent (SendMessage to it, or a fresh agent in a worktree on the branch) and keep the ready-to-push queue naming each branch, its worktree and its HEAD so the owner is easy to find.
