---
category: Repo policy / workflow reminders
---

# Include the automation kind discriminator when updating a heartbeat

I tried to update the long-running heartbeat with only its identifier, mode, and
prompt. The automation API rejected the request because updates still require
the `kind` discriminator. I also guessed a lesson category that the repository
does not define before retrying with the listed workflow category. For heartbeat
updates, provide `kind: heartbeat` plus the complete existing schedule, status,
name, and destination fields; when creating a lesson, select a category from the
validator's known list rather than inventing a new label.
