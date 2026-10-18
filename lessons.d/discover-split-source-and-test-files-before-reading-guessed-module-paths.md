---
category: Repo policy / workflow reminders
---

# Discover split source and test files before reading guessed module paths

During the Timeline wrapping audit I guessed `timeline.rs` and
`timeline_size_layout_tests.rs`; both reads failed because those modules do not
exist. No source or build output was changed by either read. `rg --files` found
the actual painter root and `cv04_timeline_font_size.rs`. Discover paths before
reading them, especially after a source file has been split into test modules.

The same audit later guessed flat grammar and parser module paths and repeated
the read failure. Their discovered paths are `code/grammars/mermaid/timeline.grammar`
and the parser's `src/lib.rs`. Apply the inventory step to each new subtree;
remembering that a prior read failed does not discover the next path.
