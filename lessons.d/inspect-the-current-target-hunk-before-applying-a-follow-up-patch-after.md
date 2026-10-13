---
category: Repo policy / workflow reminders
---

# Inspect the current target hunk before applying a follow-up patch after nearby edits

I prepared a follow-up patch against an assumed nearby test shape, but the actual helper call used a single boundary argument and the hunk did not apply. Re-read the exact current region after every nearby edit, then anchor the patch to text that is present instead of reconstructing the hunk from memory.
