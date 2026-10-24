---
category: Repo policy / workflow reminders
---

# Inspect a generated lesson shard before replacing its placeholder

An `apply_patch` intended to fill a freshly generated lesson assumed an older
placeholder sentence, so verification failed even though the target file was
correct. Generated scaffolds are allowed to evolve; a remembered placeholder
is not a stable editing contract.

Read the generated shard before patching it, then match the exact current
placeholder. This keeps the required immediate lesson update small and avoids
turning documentation of one mistake into another failed command.
