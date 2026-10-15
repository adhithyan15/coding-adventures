---
category: Testing & coverage
---

# Keep style sanitizer admission distinct from tokenizer value syntax

The first Venn hardening red run had five expected unsafe-value failures and
one extra positive fixture failure: an unquoted percent sign in a color function
was rejected by the tokenizer before style parsing. A sanitizer admitting that
character does not imply the source grammar admits it unquoted.

Retain the actual one-pass/six-failure receipt. Quote positive color values so
the existing string-token route reaches the intended validator, preserving the
complete expected semantic objects. Do not widen the tokenizer or call this
setup failure evidence of the new style validator. Re-run before importing the
new boundary; the five independent unsafe-value regressions must still fail.
