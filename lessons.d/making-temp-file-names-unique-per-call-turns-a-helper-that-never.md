---
category: Testing & coverage
---

# Making temp-file names unique per call turns a helper that never deletes its files into an unbounded leak

**What happened.** For #18485, about 120 test helpers in the `semantic-ir-to-*`
crates got a per-call `unique_suffix()` (process id plus atomic counter) in
their temp-file names, so parallel tests could no longer share a file.  The
suites passed.  But 23 of the C helpers had never deleted their `.c` and
executable: with a pid-and-source-hash name each run overwrote the last
one, and with a unique name each run added a new pair.  Two runs of the C
suite left about 270 MB in `/tmp` before this was noticed.  Twelve
counter-named C helpers had been leaking that way all along.  A first
cleanup guard also deleted the executable inside three `compile_and_link`
helpers that return it to the caller, and 35 tests failed.

**Fix.** Each helper holds its files in a `RemoveOnDrop` guard.  Where a
helper returns the executable, it drops the `.c` once linked, and each
caller guards the executable it runs.  Check: count the files in the temp
directory before and after two runs of the suite; the counts must match.

**Do differently.** Treat "unique name" and "removed on every path" as one
change.  Before making a name unique, grep the helper for `remove_file`.  An
RAII guard covers every return, `?` and panic, but it must live in the scope
that last uses the file, not in a helper that hands the path back.
