---
category: Repo policy / workflow reminders
---

# Patch long roadmap sections with exact nearby context

A combined state-and-roadmap patch used a paraphrased capitalization for the
roadmap anchor, so `apply_patch` correctly rejected the whole atomic change.
Inspect the exact tail or numbered lines first, then patch independent files
with the smallest exact context so one stale anchor cannot hide another valid
edit.
