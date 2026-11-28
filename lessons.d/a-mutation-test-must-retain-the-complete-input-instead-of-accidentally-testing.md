---
category: Testing & coverage
---

# A mutation test must retain the complete input instead of accidentally testing a fragment

While testing strict report replay, the helper expected source/needle/replacement
but its callers supplied needle/replacement/source. A missing-needle fallback
silently returned that supposed source: a field fragment instead of the complete
report. Rejection therefore established only that a fragment was invalid, and
could not detect a parser accepting the intended bad field in context.

The helper now replaces one exact needle with a replacement in the original
complete report and asserts that the needle existed. Keep mutation tests on the
full serialized input, parse the resulting complete mutant, and assert both the
changed field and rejection result when testing invalid inputs.
