# Check the candidate delta against incoming main during reconciliation

During the October 11 Closure source-span reconciliation, `git diff --cached
--check` compared the merge index to the old candidate and reported upstream
Markdown hard breaks and unrelated changelog spacing. That output is retained
as an upstream finding, not repaired by changing unrelated files.

Use `git diff --cached --check origin/main` to check the candidate delta against
the incoming base before committing a resolved merge, then `git diff --check
origin/main...HEAD` for the final PR delta. Preserve upstream content and record
actual failures; do not let a broad check silently expand the change's scope.
