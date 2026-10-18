---
category: Testing & coverage
---

# Count every marker-prefix occurrence before deriving a validation runner

A runner-derivation guard anticipated two occurrences of a marker prefix, but
the source contained six: two filters and four expected completion identities.
The guard refused before writing a successor or running Cargo. Count the full
source after earlier replacements, require that exact count, then inspect the
successor's complete expected marker set. This derivation assertion is distinct
from a compiler error or a semantic test failure.
