---
category: Repo policy / workflow reminders
---

# Reconstruct semantic merge regions instead of assuming identical conflict grouping

The seven-unit/display-label merge committed ce283d14, but its first full-tree
proof stopped at DG04. The worktree ort merge had automatically retained five
incoming label-contract lines outside its conflict, while `git merge-file`
grouped those lines inside a wider conflict. Choosing the owned side in both
tools therefore produced different expected files. No complete-tree acceptance
was recorded from the failed proof.

Preserve the diagnostic diff. Reconstruct this region explicitly from the owned
bounded circular-curve qualification plus the exact five incoming label-contract
lines. Verify the entire resulting document, all other overlap bytes, every
pure incoming object and all retained objects. Do not discard incoming prose
or weaken equality merely because conflict boundaries differ between tools.
