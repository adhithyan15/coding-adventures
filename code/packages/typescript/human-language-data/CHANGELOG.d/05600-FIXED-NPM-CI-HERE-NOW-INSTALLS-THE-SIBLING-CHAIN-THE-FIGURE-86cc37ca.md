### Fixed — npm ci here now installs the sibling chain the figure tests need
`tests/figure.test.ts` and `tests/figure-cli.test.ts` failed on every fresh
checkout with `Cannot find package '@coding-adventures/paint-vm'`. The package
was never missing: `file:` dependencies are symlinks, the siblings ship
TypeScript source, so resolution starts from the sibling's REAL directory and
never looks in this package's `node_modules`. Each sibling needs its own
install, leaf first — which the BUILD file did and `npm ci` did not, so CI was
green while the figure suite could not execute locally. A `postinstall`
(`local-deps.mjs`) now walks the `file:` closure from the manifests and installs
only what is not already linked. Set `HUMAN_LANGUAGE_DATA_LOCAL_DEPS=skip` to
opt out. It refuses to leave
`code/packages/typescript/`, so a `file:` path pointing anywhere else fails
instead of running `npm` — and lifecycle scripts — in a directory of somebody
else's choosing; it also refuses a sibling with no lockfile rather than falling
back to an unpinned `npm install`.
