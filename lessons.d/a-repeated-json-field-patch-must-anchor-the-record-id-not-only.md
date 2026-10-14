---
category: Testing & coverage
---

# A repeated JSON field patch must anchor the record id, not only the common field

An exam-inventory edit changed a generic `"probe": null` hunk and a unique
note in the same patch. The note landed on the intended point, but the probe
hunk matched the first null probe in the file and silently attached future
atoms to the unrelated reflexive-pronoun point.

Inspect the diff immediately after any repeated-field edit. More importantly,
include the record's unique `id` and the intervening keys in every patch hunk,
even when another hunk in the same patch has unique context. Patch matching is
local to each hunk; uniqueness elsewhere in the patch does not constrain it.
