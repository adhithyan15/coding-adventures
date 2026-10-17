---
category: Testing & coverage
---

# JSON payload equality does not prove a selected numeric operand preserves its bits

CLOC40's right-selected native-shape loop compared complete JSON payloads after
removing only the root CV. Its NumericLiteral fixture contains negative zero.
The separate literal-side table checked typed bits for retained left numbers,
but selected an Identifier on the right. It therefore could not prove the right
NumericLiteral preserved its sign bit. Independent review withheld test-quality
acceptance before production was changed.

Capture the right fixture's typed value.to_bits() and raw spelling before it is
moved, then require a NumericLiteral result with those exact bits and spelling.
Keep the existing complete JSON nested-identity/payload comparison and exact
returned-root ancestry checks as separate assertions. Never infer a numeric-bit
guarantee for one operand direction from another direction's tests or from JSON
number equality. Preserve the earlier withheld report rather than relabeling it.
