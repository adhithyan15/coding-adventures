---
category: Testing & coverage
---

# A provenance field such as dirty_tree must be captured before the work it describes starts: read after a two-hour run it described the editor, not the run

The VOS-028b capacity probe's results file records `dirty_tree` (was the working tree clean?) and the
`check` gate refuses a committed file whose value is true. The first full run (2 h 17 min, 1,625 rows) built
the metadata AFTER the rows had run, while the author was editing the spec in the same worktree, so a run that
began on a clean commit recorded `dirty_tree: true` and the whole run was unusable: a results file may not be
hand-edited, so the only repair was to run it again.

The field is a statement about the state in which the measurement *started* (the binary under test was built
from it), so it must be read before the first row runs: `cmd_run` now calls `meta::gather` first. The same goes
for any provenance field (a revision, a host, a binary digest): capture it at the start, put it next to the
data, and never let anything that can change during the run feed it afterwards. And when a run takes hours,
write the file incrementally, so that one wrong field does not cost the whole run.
