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
