---
category: Cross-platform & Windows BUILD_windows
---

# Read Unicode repository documents with explicit UTF-8 before rewriting them

A PREP01 doc-sync helper used `Path.read_text()` without an encoding on Windows,
then wrote UTF-8. The default read decoded Unicode through a legacy code page,
turning em dashes and arrows into mojibake across unrelated backlog and README
lines. The commit stat exposed the accidental 1,284-line backlog diff before
review or push. Restore the UTF-8 originals from a known commit, decode Git
output explicitly as UTF-8, and reapply only the intended documentation edits.
Always specify the encoding on both reads and writes; verify the complete diff,
not only the helper's exit status or `git diff --check`.
