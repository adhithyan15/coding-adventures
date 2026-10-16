---
category: Repo policy / workflow reminders
---

# replace an existing file with one update patch rather than delete and add operations

One `apply_patch` attempted to delete and add each of four existing files in
the same patch. The patch engine rejects multiple operations that target one
path, so none of the rewrite applied. Replace an existing file with an Update
operation, or split deletion and addition into separate patch calls when a
whole-file rewrite is truly necessary.

This recurred during the D3a shape-gate rewrite: Delete+Add for its existing
scaffold was rejected atomically. Reading the actual scaffold and applying one
Update hunk installed the intended replacement without assuming partial edits.
