---
category: Repo policy / workflow reminders
---

# Use the lessons tool to derive length-capped shard filenames

Lesson shard identities are not the unlimited kebab-case form of their title.
The repository slugger strips markup, keeps at most twelve words, and caps the
stem at eighty characters on a word boundary. Use `lessons.py new` or call its
slugger before creating a shard by hand; otherwise validation rejects an
otherwise well-formed lesson because its filename is not derivable.
