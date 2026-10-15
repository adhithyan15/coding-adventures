---
category: Testing & coverage
---

# Cabal main-is names the generated Haskell source for hsc2hs

The first real offline GHC9.4.8/Cabal3.16 probe rejected main-is: Main.hsc before compilation. Cabal requires main-is: Main.hs even when the authored source is Main.hsc. Preserve the actual9e5d6bae failure and executable portable red before correcting the field. The initial substring assertion also matched Main.hsc because Main.hs is its prefix; compare the entire field line, never mere token presence. The failed repair helper stopped before production mutation when its expected red did not materialize. Parser rendering alone does not prove program-default consumption or actual macOS linking; require effective-route controls and real compile/link/run evidence.
