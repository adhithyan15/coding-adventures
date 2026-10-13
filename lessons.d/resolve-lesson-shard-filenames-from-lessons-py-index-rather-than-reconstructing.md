---
category: Repo policy / workflow reminders
---

# Resolve lesson shard filenames from lessons.py index rather than reconstructing slugs

I copied a long lesson title from the index and reconstructed the expected slug,
but the generator had truncated the filename before the final title words. The
bulk read then failed on the invented path even though the lesson existed. Use
the slug column returned by `lessons.py index`, or resolve the concrete filename
with `rg --files lessons.d`, before opening a lesson shard.
