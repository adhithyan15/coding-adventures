## 0.85.0 — 2026-08-11 — nested procedure visibility

Nested procedure signatures and declaration ASTs now leave compiler lookup
tables with their lexical block. Emitted sibling names remain reserved so two
disjoint blocks cannot collide. This restores undeclared and standard-function
lookup after scope exit while preserving calls already emitted inside the block.

