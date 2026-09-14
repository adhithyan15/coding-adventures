# The agent scratchpad is shared across concurrently running agents, including ones in other worktrees

Mid-task, my `scratchpad/measure.mjs` was overwritten by a different agent working in a
different worktree (`es-a1-t4`) on unrelated Spanish content. Same scratchpad path, same
generic filename. My measurement script became theirs; a re-run would have silently
measured something else, or failed in a way that looked like my own bug.

**Give scratchpad files a task-scoped subdirectory** (`scratchpad/<issue-or-slug>/`) and
copy anything you intend to diff against later into it immediately. Generic names —
`measure.mjs`, `out.json`, `tmp.txt` — are the ones that collide.
