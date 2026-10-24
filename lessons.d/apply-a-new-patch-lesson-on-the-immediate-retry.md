---
category: Repo policy / workflow reminders
---

# Apply a new patch lesson on the immediate retry

After recording that broad prose patches should be split, the immediate retry
still attempted one large canonical-spec patch and failed on another wrapped
line. A lesson is only useful when it changes the next action: after a context
failure, retry with one small section or hunk at a time, verify it, and only
then proceed to the next section.
