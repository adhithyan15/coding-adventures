---
category: TypeScript / JavaScript
---

# A derived Set must preserve the iteration order of the public Set it replaces

HL47 projected `SEPARATE_LETTER_SCRIPTS` by filtering the composition array.
Membership stayed identical, so every membership test passed, but the public
Set's insertion order changed. JavaScript Set iteration is observable through
spread, `for...of`, and serialization by consumers; a refactor that preserves
only membership can still break callers.

When a public Set is rebuilt from direct owners, preserve its historical order
as owner metadata and assert the full iteration sequence. Do not infer one
public collection's order from an unrelated composition order.
