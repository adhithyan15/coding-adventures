---
category: Testing & coverage
---

# A token parser that repeats its final token must reject missing and embedded EOF at ingress

The Lisp direct-token entrypoint accepted a symbol or opening parenthesis
without EOF. The shared parser returns the last token after exhaustion, so
its repeated expression rule could match the same atom forever or recurse
beyond the nesting prescan. Source-string fuzzing did not exercise this
boundary because the lexer always appends EOF.

Validate that each nonempty caller token stream has exactly one EOF at the
end before invoking the engine. Reject embedded EOF rather than silently
discarding trailing input. Keep the documented empty-stream behavior and
test both the throwing and nullable APIs with missing, repeated and embedded
EOF, plus correctly terminated lexer output.
