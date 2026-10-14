---
category: Haskell
---

# Cabal test options split Hspec filters at spaces

Cabal parsed `--test-options=--match=actual configured-root basename` into
separate test arguments, so Hspec rejected `configured-root` before running an
example. Use a unique space-free substring such as `--match=configured-root`,
or run the compiled test binary directly with a correctly separated argument
list. The direct native run reproduced one configured-root failure. A setup or
argument error is not a tests-first RED result; report the executed example
count and assertion failure instead.
