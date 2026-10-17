---
category: Testing & coverage
---

# Validation commands must use the complete committed head and intended mode

An accidental CLOC38 invocation supplied a placeholder head and native-red
mode after the implementation commit. The validator's exact-head assertion
rejected it before creating a receipt or launching Cargo. That refusal is not
a compiled regression, a passing check or a failure of the implementation.

Read the complete committed HEAD and choose the intended phase explicitly
before constructing each validation command. Keep the head assertion as the
first guard, ahead of receipt and target writes. Preserve rejected preflight
attempts separately; run the corrected focus/full command against the new
committed head and consume its real terminal result before claiming evidence.
