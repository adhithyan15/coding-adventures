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
test the semantic shapes that violate the contract. Follow direct calls,
import aliases, variable aliases, and later assignments, and allow only the
specific narrow access pattern the non-owner needs. Include adversarial
fixtures for inline calls, aggregate computations through an otherwise safe
property, and obvious alternate spellings before relying on the guard. Text
search is appropriate for exact generated tokens; it is not an ownership
boundary when ordinary refactoring changes how the same behavior is spelled.
