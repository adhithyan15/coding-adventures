---
category: TypeScript / JavaScript
---

# npm install --package-lock=false can fail on local file dependencies with a null edgesOut error

`human-language-data` has local `file:` dependencies and a postinstall script
that installs them in dependency order. In a fresh worktree, running
`npm install --package-lock=false` failed immediately inside npm's dependency
tree construction with `Cannot read properties of null (reading 'edgesOut')`.
The flag intended to protect the tracked lockfile changed npm's local-file
resolution path instead of merely suppressing a write.

Use the repository's ordinary `npm install` command for this package, then
inspect `git diff -- package-lock.json`. If the current npm version rewrites the
lock, restore only that known installer-owned diff after installation rather
than changing install semantics. The lockfile check belongs after the install;
it cannot safely be moved into npm's resolver flags for a package with local
file dependencies.
