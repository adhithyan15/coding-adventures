---
category: Testing & coverage
---

# Windows source rewrites must decode UTF8 explicitly before encoding it

A CLOC38 test repair used Path.read_text() without an encoding on Windows, then
wrote UTF-8. The default cp1252 decoder changed the existing astral Unicode
truthiness witness and its expected label to matching mojibake. Full tests still
passed, but independent source review correctly withheld acceptance because the
original witness was lost. Preserve that exact-head green receipt as execution
evidence with source acceptance withheld, never as final successor acceptance.

Restore the exact fixture bytes from the previous immutable Git blob, compare
the complete changed line against that blob, and verify every other byte is
retained. Use explicit encoding='utf-8' for text reads and writes; prefer byte
replacement for an exact restoration. Review the entire successor diff before
claiming prior assertions or non-ASCII witnesses were preserved, then rerun the
affected suites on the actual corrected head.
