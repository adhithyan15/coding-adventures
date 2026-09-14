---
category: Compiler / VM / language pipeline
---

# Skip-pattern ordering affects NEWLINE emission

If `\n` is in a grammar's WHITESPACE skip pattern, no NEWLINE tokens will be emitted. Update downstream lexer-wrapper tests when changing the lexer's main loop.
