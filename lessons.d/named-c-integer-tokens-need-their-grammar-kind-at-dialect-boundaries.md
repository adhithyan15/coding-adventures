---
category: Compiler / VM / language pipeline
---

# Named C integer tokens need their grammar kind at dialect boundaries

Extending only the `TokenType::Number` branch did not admit source hex integers:
the compiled C lexer emits named `INT_LIT` tokens with `TokenType::Name`, the
shared lexer's fallback for custom grammar kinds. The earlier plain-decimal
stringize policy had the same source-token mismatch, unnoticed by identifier
fixtures. Inspect the actual lexer token shape and test decimal and hex source
inputs, rather than only constructing generic Number tokens. Admit the named
integer representation explicitly and still validate the spelling; the broad
Name fallback also contains float and punctuation tokens.
