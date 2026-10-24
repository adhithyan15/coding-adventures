---
category: Repo policy / workflow reminders
---

# Use supported timeouts for agent review waits

The collaboration wait interface rejects timeouts below 10 seconds. A
one-second polling attempt therefore provides no status and wastes a tool call.

Use at least the documented 10,000 ms minimum for review waits. Prefer a single
bounded wait over rapid polling, and continue useful local validation while an
independent review is running.
