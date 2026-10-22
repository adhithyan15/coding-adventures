---
category: Repo policy / workflow reminders
---

# Read lesson categories before creating a shard

The first lesson retry guessed `Workflow` as a category, but categories are a
closed set declared in `lessons.d/_meta.md`. Read the allowed categories (or
use the CLI's rejection output) before creating a shard, then pass the exact
declared category string.
