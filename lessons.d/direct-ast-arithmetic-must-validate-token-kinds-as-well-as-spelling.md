---
category: Compiler / VM / language pipeline
---

# Direct AST arithmetic must validate token kinds as well as spelling

The native Python IIR compiler accepted caller-supplied unary and binary
operator tokens solely by their text. Its grouping path ignored both delimiter
tokens. A forged string token spelling `+` therefore lowered as arithmetic;
the new adversarial test failed against that behavior before the repair.

Public AST entry points bypass the lexer. Validate syntax token kind, effective
type override and exact spelling together, and require tokens rather than nodes
at operator and delimiter positions. Exercise each field independently in
standalone expressions and every call argument. Keep source conformance and
resource-limit tests alongside forged-AST cases; parser-only fixtures cannot
prove the caller-supplied tree boundary.
