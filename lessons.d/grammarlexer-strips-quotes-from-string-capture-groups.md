---
category: Compiler / VM / language pipeline
---

# GrammarLexer strips quotes from string capture groups

`STRING = /"([^"\\]|\\.)*"/` makes the value `hello`, not `"hello"`. Fix tests.
