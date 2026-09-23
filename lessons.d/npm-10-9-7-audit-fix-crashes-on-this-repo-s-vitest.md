---
category: TypeScript / JavaScript
---

# npm 10.9.7 audit fix crashes on this repo's vitest peer graph; npm 11 is required

**Context:** clearing ~600 Dependabot alerts, all of them npm, spread across the
repo's 490 `package-lock.json` files.

**What went wrong:** every mutating npm command died instantly in every
TypeScript package:

    $ npm audit fix
    npm error Cannot read properties of null (reading 'edgesOut')

The stack is entirely inside arborist's peer resolver:

    at #loadPeerSet (@npmcli/arborist/lib/arborist/build-ideal-tree.js:1289:38)
    at async #loadPeerSet (...:1297:11)          <- recursing
    at async Arborist.buildIdealTree (...:181:7)

The verbose log shows the last two manifests fetched before the crash:

    silly fetch manifest vitest@*
    silly fetch manifest @vitest/browser-playwright@5.0.1

`@vitest/coverage-v8` declares a peer on `vitest`, so arborist walks `vitest@*`,
lands on the 5.x line, and recurses into its **optional** peers
(`@vitest/browser-playwright` and friends). npm 10.9.7 dereferences a node in
that optional-peer set before checking it for null. This is a bug in the
resolver, not in the lockfiles — `npm audit`, which only reads the lockfile and
never builds an ideal tree, works fine on every one of the 490 packages.

**The trap:** the obvious workaround makes things quietly worse.
`--legacy-peer-deps` skips peer resolution, so it doesn't crash, and
`npm audit` afterwards reports `found 0 vulnerabilities` — which looks like
success. It isn't. Because peers are no longer auto-installed, they are
**dropped from the lockfile**. On `macsyma-browser-repl` that single run
deleted `@testing-library/dom` (a peer of `@testing-library/react`) along with
`@babel/code-frame`, `dom-accessibility-api`, `pretty-format`, `react-is`,
`ansi-regex`, `ansi-styles`, `js-tokens`, `lz-string` and `@types/aria-query`.
A later `npm ci` would then reify a tree with no `@testing-library/dom` in it
and the component tests would fail to import. Diff stat is the tell: the
`--legacy-peer-deps` run was 160 insertions / 197 deletions, the correct run
was 11 / 9.

**What to do instead:** run the fix under npm 11, which resolves the same graph
without crashing and without shedding peers:

    npm install --silent npm@11 --prefix /tmp/npm11
    NPM=/tmp/npm11/node_modules/.bin/npm
    $NPM install --package-lock-only --no-audit --no-fund
    $NPM audit fix --package-lock-only

npm 11 writes the same `lockfileVersion: 3`, so the result is readable by the
npm 10 that CI ships with; only the resolver differs.

**Check the diff shape, not just the audit result.** After any bulk lockfile
operation, confirm that no `"node_modules/..."` key was added or removed:

    git diff -U0 -- '*/package-lock.json' | grep -E '^[-+]\s+"node_modules'

A clean dependency bump changes `version`, `resolved` and `integrity` lines and
touches no package keys at all. If that grep prints anything, packages moved in
or out of the tree and the change needs explaining before it is pushed.

**Second-order lesson:** `npm audit fix` cannot break a peer cycle on its own.
`vitest` and `@vitest/coverage-v8` peer-depend on each other at an exact
version (`"peerDependencies": {"vitest": "4.1.8"}`), so neither can move unless
both move. npm 11's `audit fix` fixed `nanoid` and `postcss` and left the
vitest advisory in place, and `npm update vitest @vitest/coverage-v8` was a
no-op for the same reason. The fix is to raise the floor in `package.json`
(`^4.1.0` -> `^4.1.11`) so the locked 4.1.8 no longer satisfies the range, then
re-resolve. Do that by editing the version string in place rather than by
`npm install vitest@4.1.11`, which rewrites `package.json` and re-sorts the
whole `dependencies` block into an unrelated diff.
