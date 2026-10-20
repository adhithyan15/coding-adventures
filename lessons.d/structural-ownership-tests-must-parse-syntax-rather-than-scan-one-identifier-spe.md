---
category: Testing & coverage
---

# Structural ownership tests must parse syntax rather than scan one identifier spelling

The first Sanskrit A1 aggregate-owner regression searched raw source text for
expressions spelled with a variable named `coverage`. A feature test could
therefore restore the same shared total under a renamed variable, a
destructuring binding, bracket notation, or another matcher and remain green.
Comments and string literals could also create false owners.

When the contract is about program structure, parse the language syntax and
test the semantic shapes that violate the contract. Prefer a capability
boundary over an open-ended taint analysis: expose a point-only helper to
feature tests, then use the AST to forbid imports of the aggregate measurement
capability outside its census owner. Include adversarial fixtures for renamed
and namespace imports from both a defining module and its public barrel before
relying on the guard. Text search is appropriate for exact generated tokens;
it is not an ownership boundary when ordinary refactoring changes how the same
behavior is spelled.
