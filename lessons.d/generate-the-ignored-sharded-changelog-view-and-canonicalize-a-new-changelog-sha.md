---
category: Repo policy / workflow reminders
---

# Generate the ignored sharded changelog view and canonicalize a new changelog shard before full tests

The first full package run started immediately after adding a hand-named
`CHANGELOG.d` fragment. The real-document guards failed because the ignored
rendered `CHANGELOG.md` had not been rebuilt and the fresh-shard check expects
the canonical heading-derived filename. After adding an entry, use the package
document-shard CLI to canonicalize the shard and regenerate the ignored view
before running the full suite; do not infer that an arbitrary readable suffix
is sufficient for every real-document invariant.
