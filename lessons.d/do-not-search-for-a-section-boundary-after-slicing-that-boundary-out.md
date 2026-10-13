---
category: Testing & coverage
---

# Do not search for a section boundary after slicing that boundary out

The C1 mock test first sliced the confidential listening master at the
`## Speaking` heading, then tried to end the final listening part at that same
heading inside the already-truncated string. The delimiter could never be
found, so correct content would fail. When a parent slice removes its closing
sentinel, let the final child slice run to the parent's end or retain the
sentinel in the parent slice. Audit every final child of the same parent rather
than correcting only the first instance reported by a reviewer or test.
