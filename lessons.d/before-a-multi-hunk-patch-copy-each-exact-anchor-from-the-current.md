---
category: Repo policy / workflow reminders
---

# Before a multi-hunk patch, copy each exact anchor from the current file

A four-hunk expansion patch for a long mock paper trusted remembered line
wrapping in one narrative anchor. That one mismatch rejected the whole patch.
Read the exact nearby lines from the current file before composing each hunk,
and apply long-document expansions in small independently verifiable patches so
one stale anchor cannot discard unrelated additions.

The same failure recurred when a two-hunk patch used an anchor whose first
sentence actually began on the preceding line. Treat even two distant edits as
separate patches in long prose files; `rg -n -A/-B` output is the source of
truth for the complete anchor, not the visually memorable sentence fragment.

A third rejection showed that even a single hunk fails when one remembered
line has been reflowed. Always run the anchor search immediately before the
patch and paste only the shortest exact adjacent lines; do not reconstruct a
long paragraph from memory.

A fourth rejection occurred even after an anchor search because the patch
still included a distant heading as context. Anchor a prose insertion to one
single exact line only; the insertion itself supplies enough locality.
