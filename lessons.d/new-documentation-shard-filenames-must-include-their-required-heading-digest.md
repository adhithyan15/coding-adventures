---
category: Repo policy / workflow reminders
---

# New documentation shard filenames must include their required heading digest

Three new CHANGELOG shards omitted the required lowercase eight-hex heading
digest. The native suites passed, but actual documentation validation rejected
the malformed filenames. Do not treat code acceptance as documentation success.

Use the existing docSlug, headingDigest and docShardFilename functions to build
the basename from the real section heading, preserving bodies and chronological
rank. Keep the original failure immutable and rerun the unchanged shard validator
on the corrected documentation head; do not grandfather new malformed names.

The checked-Ishikawa carrier repeated this mistake with a lowercase descriptive
basename lacking rank/digest. Independent review withheld e983 and the actual
current-source doc checker exited1 on that exact file. Preserve both receipts.
Derive the full basename with the real current docSlug/headingDigest/
docShardFilename functions; rename with body SHA256 unchanged. Validate the
unchanged checker against the complete checkout after correction. A Markdown
file living beside valid shards is still subject to this grammar, even when it
is a new authored entry rather than a migrated aggregate section.
