---
category: Testing & coverage
---

# Untracked documents join Git diff checks after staging

The first timeline documentation carrier e9d85917 passed doc-shard and lesson
validation but failed committed git diff --check on one new blank line at the
end of its new changelog shard. The earlier unstaged diff had omitted that
untracked file, so it did not establish the complete candidate's whitespace.
The failed gate remains false, with all six streams and a local backup ref.

Write new documents with exactly one final newline. Stage the complete explicit
path list before checking the staged or committed diff, including added files.
Fix only the EOF and add this lesson; preserve the executed Rust source and
rerun the three documentation gates on the replacement carrier.
