---
category: Compiler / VM / language pipeline
---

# Builtin punctuation token kinds do not need a type-name override

A new JavaScript division-goal test expected `type_name == Some("SLASH")`.
The lexer correctly emitted the builtin `TokenType::Slash` with `type_name`
absent. The test failed despite correct tokenization. Assert the actual builtin
kind and its canonical absence of an override; reserve name checks for custom
grammar tokens such as `REGEX`. Inspect emitted tokens before assuming every
grammar terminal is represented by an effective-type override. Also inspect the
selected edition's grammar: the earliest JavaScript editions have no `REGEX`
rule, so a preservation probe for that token belongs only to editions that
actually provide it. The initial all-edition regex assertion failed for exactly
that reason; the all-edition division acceptance probe remains unconditional.
