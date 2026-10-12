---
category: Repo policy / workflow reminders
---

# Inspect a repository CLI's help before assuming it supports filters

I guessed that the lessons index accepted a category filter, but the subcommand
only supports the options shown by its own help and rejected the invocation.
Run the CLI's help before composing an unfamiliar command, then filter its
ordinary output with a separate read-only tool when the CLI has no such flag.
