---
category: Testing & coverage
---

# A skip-the-write rule keyed on the revision showing never writes a fresh install's first answer

#16391 made every Mosaic host skip the state-file write when an answer came
back at the revision already *showing* (an environment the app ignored), so a
resize storm would cost no disk writes. Every unit test, driver and
conformance harness passed: each one dispatched an ordinary event, which
saved, before checking that an ignored report then wrote nothing.

The Android emulator gate caught it. On a fresh install, TaskApp's first
answer is to the host's environment report, and the app ignores it. The
revision showing was the revision of the initial update, which had never been
saved. Nothing was written, so `files/task-app/mosaic-state.v1.json` never
appeared.

**Fix:** compare with the revision *saved*, not the one showing. Each host
records the revision of its last successful save (cleared when a save fails),
and starts with none, so the first answer after launch always writes.

**Do instead:** a "nothing changed, skip the write" rule must compare with
what is on disk, never with what is on screen. The two differ before the
first save and after a failed one. Test the skip from a fresh start, not only
after something else has already saved.
