---
category: Compiler / VM / language pipeline
---

# Conservative call-by-name analysis must track lexical procedure shadowing

A nested procedure shadowing a known read-only one can write through a by-name formal while the outer one stays marked read-only.
