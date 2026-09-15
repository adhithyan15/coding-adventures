## 0.78.0 — 2026-08-11 — block-scoped label shadowing

Labels now receive stable internal names within their lexical block. Nested
blocks may shadow an outer label, nearest-binding lookup applies to forward and
backward gotos, and leaving the block restores the outer binding. Switch-list
label designators retain the declaration block's binding when expanded later.
Duplicate labels in one block remain an error.

