---
category: Repo policy / workflow reminders
---

# Resolve exact commit hashes with git rev-parse instead of expanding abbreviations

An abbreviated commit printed by `git commit` was manually expanded when
requesting an exact-head security review, producing a nonexistent hash even
though the correct object was available locally. That makes an otherwise
precise review request ambiguous and can invalidate later delivery checks.

Before sending, recording, comparing, or monitoring an exact head, obtain it
with `git rev-parse HEAD` (or the exact remote ref) and copy that complete
output. Never infer the remaining hexadecimal digits from an abbreviation;
correct any mistaken review request before the reviewer begins.
