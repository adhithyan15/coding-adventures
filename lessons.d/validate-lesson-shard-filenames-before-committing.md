---
category: Repo policy / workflow reminders
---

# Validate lesson shard filenames before committing

The lesson filename is derived from its title, including language-specific
normalization and leading articles. A manually named shard can look readable
but still fail `lessons.py validate`. Create it with the repository helper,
fill the generated file, and run validation before staging lesson changes.
