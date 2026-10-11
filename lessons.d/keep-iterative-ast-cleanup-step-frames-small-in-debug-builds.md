---
category: Rust
---

# Keep iterative AST cleanup step frames small in debug builds

The first iterative cleanup put every node destructure in one large match function. Its debug-build stack frame alone overflowed a 128 KiB caller, even though no recursion remained. Extract one small step function per node type; the dispatch loop then calls only one bounded step at a time. Verify the actual debug build on a small stack, not only the absence of recursive source calls. When rewriting functions programmatically, do not mutate text while iterating offsets captured from its previous version: collect replacements or operate on independent function segments. Leaf steps need no unused work-vector parameter; removing it also satisfies strict lint.
