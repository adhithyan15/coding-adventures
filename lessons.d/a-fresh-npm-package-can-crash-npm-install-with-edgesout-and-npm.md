---
category: BUILD files & dependency management
---

# A fresh npm package can crash npm install with edgesOut, and npm install drops the libc fields from a lock

`npm install` in a brand-new TypeScript package (devDependencies `vitest` 4,
`@vitest/coverage-v8`, `typescript`, `@types/node`) died with
`TypeError: Cannot read properties of null (reading 'edgesOut')` from arborist's
`#loadPeerSet` on npm 10.9.9, writing nothing. Re-running does not help.

What worked: copy the `package-lock.json` of a sibling package with the same
devDependency set (`actor` here), rename its root entry, and run `npm ci`. When
the new package also has to be linked into another package's lock as a `file:`
dependency, do not let `npm install` rewrite that lock: it silently strips every
`"libc": [...]` field, which makes the lock differ for glibc and musl installs
and churns ~30 unrelated lines. Add the three entries by script instead (the
root `devDependencies` line, the `"../pkg"` entry and the
`node_modules/<name>` link) and check with `npm ci`.

Do differently: after any `npm install`, read `git diff --stat` on the lock
before committing it, and prefer a seeded lock plus `npm ci` for a new package.
