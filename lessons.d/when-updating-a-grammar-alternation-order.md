---
category: Mosaic compiler pipeline
---

# When updating a grammar alternation order:

edit both the `.grammar` source file in `code/grammars/` AND the corresponding `_grammar.rs` (the embedded Rust representation). They must stay in sync; the CI grammar-tools pipeline validates the `.grammar` but the Rust parser uses `_grammar.rs`.
