---
category: Mosaic compiler pipeline
---

# Adding an earlier mount of a component renames the later mount's parts (-m2), so tests must match part tags by prefix

**What went wrong.** Journal's `JournalUiTest` (#15971) matched timeline rows
by the toolkit RecordList's part tag, `record-list-title`. Then J4c
(#15975, "On this day") mounted a second `RecordList` ABOVE the timeline in
`JournalApp.mll`. Multi-mount (#15959) keeps the original part names for the
FIRST mount in layout order and suffixes later ones (`-m2`, `-m3`), so the
timeline's rows silently became `record-list-title-m2`. Nothing in #15975
failed: its own tests used text and slots, not tags. The breakage surfaced
on #15971's next CI run, after it merged main: "Expected exactly '1' node
but could not find any".

**The fix.** Match a mounted component's parts by tag PREFIX
(`startsWith("record-list-title")`) in UI tests. Don't use the exact name,
which depends on how many mounts precede it in the layout.

**Do differently.**
- When adding a mount of a component a layout already uses, grep the
  app's UI tests and screenshot harnesses for that component's part names:
  `grep -rn "record-list-\|empty-state" code/packages/rust/*/conformance`.
- Prefer tags on the app's OWN parts (unique, never renamed) for anything a
  test must find, and use a component's parts only by prefix.
