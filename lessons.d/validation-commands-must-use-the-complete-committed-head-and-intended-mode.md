---
category: Testing & coverage
---

# Validation commands must use the complete committed head and intended mode

Two accidental CLOC38 invocations supplied placeholder heads, first with
native-red mode after implementation and then with focus mode after the first
lesson commit. Both exact-head assertions rejected before creating receipts or
launching Cargo. Neither refusal is a compiled regression, a passing check or
a failure of the implementation. The second attempt exposed the same command
construction mistake: a dependent command was prepared before reading the
commit result.

Read the complete committed HEAD and choose the intended phase explicitly
before constructing each validation command; use a separate tool call after
the commit result, or resolve HEAD into a task-local shell variable. Keep the head assertion as the
first guard, ahead of receipt and target writes. Preserve rejected preflight
attempts separately; run the corrected focus/full command against the new
committed head and consume its real terminal result before claiming evidence.
