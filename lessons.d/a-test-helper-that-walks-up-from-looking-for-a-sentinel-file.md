---
category: Haskell
---

# A test helper that walks up from `"."` looking for a sentinel file needs an ABSOLUTE starting directory, not the literal string `"."`

— `takeDirectory "."` returns `"."` again, so a loop like `findRepoRoot` that stops when `takeDirectory dir == dir` returns immediately without ever climbing real directories. The production entry point (`Main.hs`) got this right (`cwd <- getCurrentDirectory; findRepoRoot cwd`), but a test that shortcuts to `findRepoRoot "."` silently no-ops and then fails downstream with a confusing "file does not exist" instead of the real problem. Always resolve `getCurrentDirectory` first in tests too.
