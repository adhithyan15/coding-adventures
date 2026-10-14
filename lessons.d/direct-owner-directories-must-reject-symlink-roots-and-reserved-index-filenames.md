---
category: Security boundaries
---

# Direct-owner directories must reject symlink roots and reserved index filenames

The first provenance-owner gate validated each child with `lstat`, but called
`readdir` on the root without validating the root itself. A symlinked owner
directory would therefore be followed. Its strict filename grammar also
accepted lowercase `readme.md`, `index.md`, and `manifest.md`, allowing a new
shared index under an otherwise valid owner name. Fail closed at both layers:
`lstat` and require a real directory before discovery, then reserve the common
index/manifest basenames in addition to enforcing the owner slug grammar. Add
negative fixtures for the root symlink and every reserved name.
