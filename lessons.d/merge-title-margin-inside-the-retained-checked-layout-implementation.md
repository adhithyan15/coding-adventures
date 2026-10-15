---
category: Rust
---

# Merge title margin inside the retained checked layout implementation

The incoming title-margin producer changed the old public layout body, while
the owned branch had moved arithmetic behind a fallible checked boundary.
The actual merge conflicted at that split. Retain the checked wrapper and all
constraint indexes/guards, then apply only the new title-height expression
inside its supported implementation. Verify the entire resolved file equals
the owned file with that sole incoming expression replaced, and independently
reconstruct every other overlap and pure incoming object.

Do not inspect or edit an in-progress merge as if its working tree were final.
Wait for its actual terminal result before resolving conflicts. Audit both
spaced and compact config literals for the new default None field; preserve
all existing values/assertions and run typed-admission red controls before
adding a new caller-owned field to the guard.
