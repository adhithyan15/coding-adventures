---
category: Repo policy / workflow reminders
---

# A disk floor with no headroom means npm ci into tmpfs and symlink node_modules, then delete the symlink before git add

With the disk 40 MB above the agreed floor, `npm ci` in the package (about
70 MB) would have crossed it. Install into a directory on tmpfs
(`/dev/shm/<name>`, with a copy of `package.json` and the lock; for a `file:`
dependency, a sibling symlink so the relative path still resolves), symlink
`node_modules` into the package, and run the real BUILD lines. The symlink is not
matched by a `node_modules/` ignore pattern, so it shows as untracked: stage
explicit paths, never `git add -A`, and delete the symlink and the tmpfs
directory at the end. Python needs no venv when system `pytest` and `ruff`
exist: `PYTHONPATH=src` is enough.
