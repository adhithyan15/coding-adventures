---
category: Compiler / VM / language pipeline
---

# Shorthand slot-binding props (`slot: label`) require an alternation in the grammar

If a `.mll` grammar rule only has `prop = NAME COLON prop_value`, then `slot: label` fails at parse time because `slot` is a KEYWORD token, not a NAME. Add `| KEYWORD COLON NAME` as an alternative and handle it in the semantic analyzer. LL(1) is preserved because KEYWORD ≠ NAME are disjoint token types.
