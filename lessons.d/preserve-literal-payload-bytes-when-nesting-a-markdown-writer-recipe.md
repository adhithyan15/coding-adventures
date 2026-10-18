---
category: CI & GitHub Actions
---

# Preserve literal payload bytes when nesting a Markdown writer recipe

Event-height documentation at 158c9835 failed the actual docs gate after native
source d90aa531 had independently passed. A generator indented every source line
to nest a recipe, including text inside triple-quoted payloads. That inserted one
space on blank payload lines and left a final space after the last newline. The
shard join concatenates bytes without adding separators, so the dangling space
prefixed the following heading and made 115 shards rebuild as 114 sections.
Actual doc-shards exited 1 and git diff --check exited 2; lessons exited 0.
All physical proof guards stayed true. Neither native acceptance nor failed
documentation is relabeled. Preserve the failed head, receipt and raw logs.

When nesting code, preserve literal string bytes separately from code indentation.
Replay the complete recipe from immutable preimages, assert exact old/new bytes,
remove only the introduced payload indentation, and require a final LF without
whitespace-only lines or an extra blank EOF. Re-run the actual shard join and
diff check on the repaired carrier, then obtain independent exact-head review
before pushing. Inventory the lesson category list before selecting a category:
the first attempted Build & CI category was invalid and created no file; the
CLI-provided CI & GitHub Actions category succeeded.
