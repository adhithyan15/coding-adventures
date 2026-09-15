---
category: Compiler / VM / language pipeline
---

# Grammar-lexer test helper

: `_tok_type` normalizer is required — non-keyword tokens keep `TokenType` enum values, only promoted keywords use strings. Direct `t.type == "EOF"` comparison fails because `TokenType.EOF != "EOF"`.
