---
category: Testing & coverage
---

# Keep style sanitizer admission distinct from tokenizer value syntax

The first Venn hardening red run had five expected unsafe-value failures and
one extra positive fixture failure: a percent sign in a color function was
rejected by the tokenizer before style parsing. Quoting the value also failed.
A sanitizer admitting a character does not imply this source grammar admits it.

Retain both actual one-pass/six-failure receipts. Use admitted numeric function
spelling for positive lexical-helper controls and preserve both original percent
forms as exact tokenizer-error tests. Do not widen the tokenizer or call these
setup failures evidence of the new style validator. Re-run before importing the
new boundary; the five independent unsafe-value regressions must still fail.
