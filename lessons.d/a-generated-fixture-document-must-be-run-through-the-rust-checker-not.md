---
category: Testing & coverage
---

# A generated fixture document must be run through the Rust checker, not only the oracle

S11a committed `data_forms_v1.warrant` after only the Python oracle had lowered it. The oracle typed a `None` from its sibling branch (`unwrap_or(if c then at(d, i) else None, 0u8)`), which the Rust checker, the tool the generator slice reads the document with, rejects (W0301). The oracle was more lenient than the checker on `None`, `Option` lifting and tuple lifting, so the parity fixture was unreadable by its second consumer.

Fix: the oracle's `_assignable`/`_unify` now mirror `checker.rs` `is_assignable_to`/`common_type`, and the whole fixture was run through `check_warrant` (a small example in `warrant-grammar`, sparse-adding its path deps). Before committing a document that another tool must read, run that tool's checker over every line, and make the oracle's typing relations a copy of the checker's, not a looser reading of the spec.
