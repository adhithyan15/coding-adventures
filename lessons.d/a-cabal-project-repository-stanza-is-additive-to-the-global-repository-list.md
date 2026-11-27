---
category: CI & GitHub Actions
---

# A Cabal project repository stanza is additive to the global repository list

A project-level `repository hackage.haskell.org` entry does not replace the
same-named repository from the user's global Cabal configuration. Cabal merges
the project and global repository lists; repository entries with the same name
but different URLs remain distinct, and the global entry can still be tried
first. A build that depends on project precedence may therefore keep using a
stale HTTP mirror even when `cabal.project` declares HTTPS.

When CI must choose one repository URL, pass a dedicated config to Cabal or
export it with `CABAL_CONFIG` for subsequent steps. Keep Cabal's secure mode,
shipped trust roots, and threshold intact. Do not edit the user's/global config
or replace signed mirror metadata.