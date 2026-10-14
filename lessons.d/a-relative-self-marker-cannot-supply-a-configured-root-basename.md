---
category: Haskell
---

# A relative self marker cannot supply a configured root basename

The registry repair classified discovery roots relative to the configured code
root, then also took the basename from that relative path. When the configured
root itself held BUILD, `makeRelative` returned `.` and the new identity became
`unknown/.`. The source review caught this before publication; a native RED
example reproduced it. Keep bucket/origin classification relative, but take
the basename from the actual current directory. Preserve configured-root
membership unless the governing contract explicitly excludes it.
