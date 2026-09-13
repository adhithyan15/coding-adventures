# `os.remove()`/`os.replace()` on a file another handle still has open: fine on POSIX, `PermissionError`/`WinError 32` on Windows — check every early-return path, not just the obvious one

`storage-sqlite`'s `Pager._recover()` opened the journal file for reading
(`with open(self._journal_path, "rb") as j:`) and, on two of its three exit
paths, called `self._drop_journal()` (an `os.remove()`) from **inside** that
`with` block — while `j` was still open. POSIX unlinks an open file just fine
(the inode survives until the last handle closes); Windows refuses
(`PermissionError: [WinError 32] ... being used by another process`).

The third exit path (successful replay) already called `_drop_journal()`
**after** the `with` block closed `j`, so it never had this bug — which is
exactly what made the two early-return call sites easy to miss on a read-through
that only checks "does this function eventually close its handles," rather than
"does *every* removal of a path happen after every handle to that path is
closed." **When auditing a function for this class of bug, don't stop at
finding one instance that does it correctly — the correct and incorrect
versions can coexist in the same function, on different branches, and a partial
read makes the whole function look safe.**

**Fix:** collapse the three early-return branches into one control-flow path
that always exits the `with` block before removing the file — a `header = None`
sentinel plus a combined boolean condition, rather than three separate
`self._drop_journal(); return` call sites. One removal call site, reached after
every branch, is easier to audit than three copies of the same call scattered
across early returns — and this class of bug is specifically about a removal
call's *position relative to a `with` block*, so collapsing to one call site is
the fix, not just a refactor.
