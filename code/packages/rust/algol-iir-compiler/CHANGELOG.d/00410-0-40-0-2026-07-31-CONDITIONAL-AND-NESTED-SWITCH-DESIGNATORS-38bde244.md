## 0.40.0 — 2026-07-31 — conditional and nested switch designators

Switch-list elements now retain their complete designational expression until
the corresponding `goto s[index]` runs. An element may choose a label with
`if` or dispatch through another switch, so conditions observe their current
values and nested indices resolve at the computed-goto site. Cyclic switch
elements receive a deterministic compiler error instead of recursively growing
the generated dispatch chain.

