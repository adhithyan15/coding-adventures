---
category: Testing & coverage
---

# Adjacent instruction names do not imply the same operand width

An initial PDP-10 implementation treated `IDIV` like the adjacent double-word
`DIV` family and constructed a 72-bit dividend from AC and AC+1. The primary
manual defines `IDIV` as a single-word signed dividend in AC; AC+1 receives only
the remainder. A superficially plausible multiply-then-divide test exposed the
mistake as quotient overflow.

When an ISA offers related short, long, integer, or floating instruction
families, transcribe each family's operand sources, result destinations, and
fault limits independently from the primary architecture manual. Include a
test whose high companion register is deliberately nonzero: for a short form,
changing that register must not change the result.
