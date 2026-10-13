---
category: Repo policy / workflow reminders
---

# Byte-exact changelog migrations need a scoped blank-at-EOF attribute

`git diff --check origin/main...HEAD` reported blank lines at EOF in 29 newly
generated Japanese changelog owners. Those trailing separators are required to
reconstruct the pre-migration monolith byte-for-byte; removing them would break
the stronger preservation contract. Existing changelog migrations solve this
with a directory-scoped `.gitattributes` rule. Add the matching
`whitespace=-blank-at-eof` rule whenever a new byte-exact changelog shard
directory is introduced, then repeat the branch-wide diff check.
