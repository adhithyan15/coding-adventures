---
category: Repo policy / workflow reminders
---

# Never hand-edit `_grammar.rs` files

Edit the `.tokens` or `.grammar` source files in `code/grammars/` and re-run the grammar-tools pipeline to regenerate. Hand-edits diverge from the grammar source of truth and break the single-source-of-truth invariant.
