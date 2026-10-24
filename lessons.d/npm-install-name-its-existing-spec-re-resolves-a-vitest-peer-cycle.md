---
category: TypeScript / JavaScript
---

# npm install name@its-existing-spec re-resolves a vitest peer cycle without touching package.json

**Context:** moving ~500 lockfiles from vitest / `@vitest/mocker` 4.1.11 to
4.1.12 (GHSA-qxg6-89mg-rqv8, GHSA-xp26-3454-mjgp) while every `package.json`
already said `"vitest": "^4.1.11"`, a range 4.1.12 satisfies.

**What went wrong:** `npm update vitest @vitest/coverage-v8` is a no-op, even
under npm 11. The two packages peer-depend on each other at an exact version,
so arborist keeps the locked pair. The earlier sweep (#15903) solved this by
raising the floor in 480 `package.json` files, which is also what
`lessons.d/npm-10-9-7-audit-fix-crashes-on-this-repo-s-vitest.md` recommends.
Raising floors on every advisory turns a lockfile refresh into a manifest
change in every package, and it moves pinned evidence that hashes a package
tree (the barcode-layout-1d lane registry).

**What works instead:** re-request each vitest-family direct dependency at its
*existing* spec:

    npm install 'vitest@^4.1.11' '@vitest/coverage-v8@^4.1.11' \
      --package-lock-only --ignore-scripts --no-audit --no-fund --before=<date>

An explicit request makes arborist resolve that edge fresh, so the pair moves
together to the newest version inside the unchanged range. npm then rewrites
`package.json` with its dependency block re-sorted, so restore the file
byte-for-byte afterwards (copy it aside first). The spec strings are
identical, so the lock's root entry, which records them, still matches and
`npm ci` accepts the result. Follow with `npm update vite source-map-js ...` for
the ordinary transitive bumps.

**Check:** `git status` shows only `package-lock.json` files, the lock root
entry (`packages[""]`) is unchanged, and no `"node_modules/..."` key appears
or disappears except where the new versions legitimately changed the tree.
Exact pins (`"vitest": "4.1.11"`) still need the manifest edited, because no
other version satisfies them.
