---
category: Repo policy / workflow reminders
---

# Set the command worktree explicitly after creating it before invoking its repository tools

A chained command created a fresh audit worktree and then invoked a repository
script while the shell still ran in the main checkout. Creating a worktree does
not change the current process directory, so the command looked for the script
on the main checkout's older dirty branch and failed. Split worktree creation
from tool execution, then set the second command's working directory explicitly
to the new worktree. The same rule applies even when every path argument passed
to the tool names the desired revision: the tool binary itself still comes from
the current working tree.
