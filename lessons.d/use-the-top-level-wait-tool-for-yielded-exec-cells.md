---
category: Repo policy / workflow reminders
---

# Use the top-level wait tool for yielded exec cells

When `functions.exec` yields a cell ID, resume it with the top-level
`functions.wait` tool. The nested `tools` object inside a new `functions.exec`
is for the tools explicitly exposed to that JavaScript isolate and does not
include `wait`; attempting `tools.wait(...)` fails before polling the original
cell.
