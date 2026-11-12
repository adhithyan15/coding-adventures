---
category: Repo policy / workflow reminders
---

# Lesson shards live directly under lessons.d rather than category subdirectories

`lessons.py index` prints a category and slug, but the category is front-matter
metadata rather than a filesystem directory. Building a path from both fields
causes avoidable missing-file errors. Resolve a relevant slug with
`rg --files lessons.d | rg '<slug>'`, then read the returned shard directly
under `lessons.d/`.
