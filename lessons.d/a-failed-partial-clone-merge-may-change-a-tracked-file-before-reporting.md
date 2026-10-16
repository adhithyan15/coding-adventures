---
category: Repo policy / workflow reminders
---

# A failed partial-clone merge may change a tracked file before reporting a missing object

The real CV04b current-main merge failed before creating MERGE_HEAD: the promisor
remote refused a requested object as not-our-ref. Git had already removed one
tracked human-language JSON file although the index and HEAD were unchanged.
The checkout was clean before this controlled attempt; restore only that known
merge-caused deletion from HEAD, then verify the worktree before retrying.

The initial lesson title guessed an ignored lockfile cause before reading the
fatal output; that guess was wrong. Inspect the actual failure and distinguish
missing-object recovery from content-conflict resolution. Preserve the failed
log and never claim that the expected fourteen-conflict merge state was reached.
