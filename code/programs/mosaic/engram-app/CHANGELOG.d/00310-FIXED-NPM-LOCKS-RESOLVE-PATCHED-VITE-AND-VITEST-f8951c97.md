### Fixed — npm locks resolve patched vite and vitest

`npm/web/package-lock.json` and `npm/electron/package-lock.json` were
regenerated with `scripts/build-web.sh --update-lock` and
`scripts/build-electron.sh --update-lock` after the Mosaic emitter moved its
pins to `vite` 7.3.7 and `vitest` 4.1.12. The old locks resolved vite 7.3.6
(GHSA-587h-7wmj-h5pm and four related advisories) and vitest 3.2.7, whose
`@vitest/mocker` and `tinypool` carry advisories with no 3.x fix. The scripts'
seven-day `--before` floor still applies.

`--update-lock` now refuses to run under npm older than 11 and says why: npm
10's arborist crashes resolving vitest 4's peer graph from scratch
(`Cannot read properties of null (reading 'edgesOut')`). `npm ci` from the
committed lock is unaffected and still works under npm 10.
