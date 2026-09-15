---
category: Workspace & package metadata
---

# Haskell `cabal.project` must list every transitive local package

Cabal does not discover sibling deps from a sibling's own `cabal.project`. Single-package validation: plain `cabal test` (NOT `cabal test all`, which builds the whole universe).
