---
category: Rust
---

# Borrow a paint scene while comparing its boxes and glyphs

The first Railroad glyph regression borrowed rectangle instructions and then
tried to move the same scene's instruction vector into a glyph filter. Rust
correctly rejected it before the rendering oracle ran. Iterate by reference
and clone only the expected/actual glyph traces for comparison. Keep the
compiler-failure receipt distinct from the later executed geometry failure.
