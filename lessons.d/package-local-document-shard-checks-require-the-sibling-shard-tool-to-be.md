---
category: TypeScript / JavaScript
---

# Package-local document shard checks require the sibling shard tool to be built in a fresh worktree

`script-ductus` exposes `npm run check:doc-shards`, but the command deliberately
executes `../human-language-data/dist/doc-shard-cli.js`; installing Script
Ductus dependencies does not create that sibling build output. In a fresh
worktree the first check therefore failed with `MODULE_NOT_FOUND` even though
the changed shard was valid. Before running a package script that calls a
sibling repository tool by relative `dist/` path, install and build that sibling
from its own lockfile. Treat the consumer package install and the tool package
build as separate prerequisites.
