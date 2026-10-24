---
category: Repo policy / workflow reminders
---

# Agent waits have a ten-second minimum

A collaboration wait requested a one-second timeout and the tool rejected it
before waiting. Use at least ten seconds for agent wait calls, and use the
minimum only when a short progress poll is actually useful.
