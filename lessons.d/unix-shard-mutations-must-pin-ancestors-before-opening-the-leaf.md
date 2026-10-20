---
category: Security boundaries
---

# Unix shard mutations must pin ancestors before opening the leaf

Checking each ledger pathname with `lstat` and then opening its final directory
with `O_NOFOLLOW` leaves an ancestor-swap race: the final flag does not reject
a symlink installed at an earlier track component. An outside shard can then
be replaced or removed even though the shard filename itself is never followed.

Open each descendant directory relative to its already-open parent with
`O_DIRECTORY` and `O_NOFOLLOW`. Check the selected shard relative to the final
descriptor, and create the temporary file, replace the target, or unlink it
through that same descriptor. Deterministic write and remove tests should swap
an ancestor immediately after pathname prevalidation and prove an outside
sentinel remains unchanged. Preserve static symlink rejection and replacement
mode behavior; a second pathname check is not an anchoring substitute.
