---
category: Repo policy / workflow reminders
---

# Session-map shard filenames normalize case, but row IDs do not

The filename uses an uppercase slug for collision-resistant ownership, while
the Markdown row must preserve the lesson ID's exact authored case. Derive the
row ID from lesson frontmatter, not from the normalized filename.
