---
category: Testing & coverage
---

# Parenthesize a Go composite literal before calling its method in an if condition

The BF03 native test initially used `if IrText{"note"}.String() != "note"`.
Go parses the literal brace ambiguously in a control clause and rejects the
test before execution. Write `if (IrText{"note"}).String() != "note"` instead,
and run gofmt before interpreting a test failure as a semantic regression.
The corrected package suite then measured99.6% production statement coverage.
