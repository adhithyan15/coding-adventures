---
category: Testing & coverage
---

# Enumerate every hash-bearing receipt input before publication

Native32 publisher v1 was withheld before execution because its fresh physical
union omitted the core receipt's pins. The v2 proposal added that union, but
independent review found two further omissions: core.helper_sha256 and the
documentation receipt's helper_sha256 were explicit receipt inputs outside
those pin lists. Counting pins did not prove identity coverage.

Keep both unexecuted refusals and their reports unchanged. Enumerate every
hash-bearing prerequisite field, form a conflict-checked path/hash union,
verify the actual review binds current receipt digests, then independently
replay the complete physical snapshot. Only reviewed v3 actually executed its
ordinary fast-forward push and passed postpublication checks. Its acceptance
does not retroactively validate v1 or v2.
