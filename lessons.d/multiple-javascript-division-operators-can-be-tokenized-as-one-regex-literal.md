---
category: Compiler / VM / language pipeline
---

# Multiple JavaScript division operators can be tokenized as one regex literal

A three-argument console fixture combined `1 / 0` and `0 / 0` to display
Infinity and NaN. The current source lexer instead recognized `/ 0, 0 /` as
a regex literal, so parsing failed before the native builtin ran. The same
fixture succeeds in Node; number-display correctness alone does not establish
source acceptance.

Record the exact rejected source and the division/regex lexical-goal repair
in LANG78. For a bounded console-arity change, check special numbers in separate
source programs that each contain one division expression. Keep Node as a
separate oracle, and do not claim that this fixture workaround repairs the lexer.
