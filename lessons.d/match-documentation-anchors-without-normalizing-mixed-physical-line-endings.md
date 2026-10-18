---
category: Python
---

# Match documentation anchors without normalizing mixed physical line endings

The minima docs writer inferred one newline convention from a mixed file.
The backlog prefix had CRLF followed by LF, so its multi-line anchor matched
zero times. The writer refused at its first prepared body, before any
repository write; the empty partial candidate and actual exit1 are retained.

Use a uniquely checked heading-only anchor. Preserve every old physical byte
outside the exact replacement instead of normalizing the file. Keep complete
preimages and after-bodies, verify clean source after refusal, and review
the bounded successor before retrying. This is a preparation failure, not
a compiled semantic red or failed native test.
