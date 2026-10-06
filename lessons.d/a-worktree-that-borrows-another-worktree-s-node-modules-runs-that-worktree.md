---
category: TypeScript / JavaScript
---

# A worktree that borrows another worktree's node_modules runs that worktree's sibling-package source

**Context:** adding Tamil vowel-sign filmstrips in a second git worktree whose
`code/packages/typescript/*/node_modules` were symlinks into a first
worktree, to save disk.

**What happened:** `script-ductus`'s `tests/filmstrip-ledger.test.ts` imports
`@coding-adventures/human-language-data/src/figure-targets.ts`. Inside
`script-ductus/node_modules`, that package is a RELATIVE link
(`../../../human-language-data`). Because the whole `node_modules` directory
was itself a symlink into the other worktree, the relative link resolved
there. `generate:filmstrip-ledger` would have built the ledger from the other
worktree's `figure-targets.ts`, which did not know the new written-order
composer, and dropped the new sign entries without any error.

**Why:** a relative link is resolved from where its directory really lives,
not from the path you reached it through. Borrowing `node_modules` borrows
every `file:` sibling dependency with it.

**Fix:** run the script-ductus suite through a scratch Vitest config that
merges the package's own config and adds a `resolve.alias` mapping
`@coding-adventures/human-language-data/` to this worktree's package. For
`language-ladder`, build a real `node_modules` directory whose external
entries are symlinks but whose `@coding-adventures/*` entries point at this
worktree.

**Do differently:** in a worktree with borrowed `node_modules`, check
`readlink -f node_modules/@coding-adventures/<sibling>` before running any
generator that imports a sibling package's source. If it leaves the
worktree, alias it back before trusting the output.
