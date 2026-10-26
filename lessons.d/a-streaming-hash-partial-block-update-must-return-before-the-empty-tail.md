---
category: Cryptography & security review
---

# A streaming hash partial-block update must return before the empty-tail path resets the buffer

The first SHA-384 streaming update filled only part of the retained block, then
fell through to the common direct-block tail path. Because no caller bytes
remained, that path reset `buffer_len` to zero and silently discarded the
partial message. One-shot vectors passed while partitioned updates and
non-destructive snapshots failed.

When an incremental hash begins with a partial retained block, either fill and
compress it or commit the new total length and return immediately. Add
partition tests that cross every padding and block boundary; one-shot
known-answer tests alone cannot exercise this state transition.
