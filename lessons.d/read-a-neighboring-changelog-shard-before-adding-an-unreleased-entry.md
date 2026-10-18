---
category: Repo policy / workflow reminders
---

# Read a neighboring changelog shard before adding an unreleased entry

The retained-corpus documentation carrier added a changelog shard with a level-one title. The real doc-shard checker rejected it because entry shards start with level-two headings; `_meta.md` owns the document preamble. Lessons and whitespace checks do not validate this shard protocol.

Preserve the failed checker exit1 and raw error with its exact source and guards. Read an adjacent shard before authoring another entry, repair only the heading level, then run the actual unchanged checker on the repaired source. Keep accepted native receipts separate from a failed Markdown carrier.
