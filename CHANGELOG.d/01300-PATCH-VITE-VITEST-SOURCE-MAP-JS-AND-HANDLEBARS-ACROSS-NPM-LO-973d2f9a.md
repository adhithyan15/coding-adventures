### Patch vite, vitest, source-map-js and handlebars across npm lockfiles

- Moved every npm lockfile off the versions named by the open Dependabot
  alerts: `vite` (GHSA-587h-7wmj-h5pm, GHSA-698v-4r3x-3vcv,
  GHSA-r825-w8mm-5pxw, GHSA-rq7h-c2jc-7f22, GHSA-9jrq-w75r-8gcw,
  GHSA-vfpm-58rq-9qcg) to 8.3.4 / 7.3.7 / 6.4.4 by major line; `vitest` and
  `@vitest/*` (GHSA-82fw-gwwq-j7x9, GHSA-qxg6-89mg-rqv8, GHSA-xp26-3454-mjgp)
  to 4.1.12, or 5.0.5 for the two vitest-5 packages; `source-map-js`
  (GHSA-68fv-2mgg-jv7q) to 1.2.2; `handlebars` (GHSA-8r5x-fm3f-whwj,
  GHSA-p8wg-vrv2-v86f, GHSA-xw65-4hp5-5hc7) to 4.7.10.
- Lockfile-only: no `package.json` range moved except
  `forme-plugin-runner-conformance`, whose exact `4.1.11` vitest pins became
  `4.1.12`. vitest and `@vitest/coverage-v8` peer-depend on each other at an
  exact version, so `npm update` cannot move them; each was re-requested at
  its existing spec (`npm install vitest@^4.1.11 ...`) and `package.json`
  restored byte-for-byte. See
  `lessons.d/npm-install-name-its-existing-spec-re-resolves-a-vitest-peer-cycle.md`.
- Resolved with npm 11 and `--before=2026-10-16`, so nothing newer than about a
  week is locked: vite 8.3.5 and 6.4.5 (published 2026-10-21) were skipped in
  favour of 8.3.4 and 6.4.4, which already carry every fix above.
- Diff shape, checked mechanically across all changed lockfiles: no lock root
  entry changed (apart from the one exact pin above); package keys moved only
  where the new versions changed the tree — rolldown 1.2.14 dropped its
  optional `wasm32-wasi` binding (and the `@emnapi/*`, `@napi-rs/wasm-runtime`,
  `@tybys/wasm-util`, `tslib` chain under it) and added
  `binding-android-arm-eabi`, and vitest 5.0.5 no longer pulls
  `siginfo`/`stackback`. npm 11 also added missing `libc` selectors to existing
  Linux-only optional binaries; none was removed. Every `resolved` URL is
  `https://registry.npmjs.org/` with a sha512 integrity, and no package gained
  an install script.
- Two lockfiles (`vite-plugin-lattice`, `code39-visualizer`) also record the
  full trees of their `file:` link targets; their nested vitest-family entries
  were moved to the same 4.1.12 metadata as the root's.
