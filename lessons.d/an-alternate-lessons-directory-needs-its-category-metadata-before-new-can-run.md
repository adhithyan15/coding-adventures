---
category: Repo policy / workflow reminders
---

# An alternate lessons directory needs its category metadata before new can run

During an exact-head review, lessons were prepared outside the frozen checkout using `lessons.py --dir`. The first command failed because `new` reads `_meta.md` to validate its category before creating a shard. Copy the real category metadata into that alternate directory first, then use the exact filename returned by `new` and fill its body.

When filling a generated lesson, preserve both category front matter and its `# Title` heading. Replacing the whole post-front-matter body discarded that required heading; validation rejected all three pending shards until it was restored.

Read the generated body before replacing its placeholder. A guessed multi-line
placeholder failed to match the actual single `TODO:` line and the patch tool
rejected the edit without modifying the lesson.
