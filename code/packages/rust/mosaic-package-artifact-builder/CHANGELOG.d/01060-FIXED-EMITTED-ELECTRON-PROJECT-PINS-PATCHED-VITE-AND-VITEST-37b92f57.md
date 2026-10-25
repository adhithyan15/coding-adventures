### Fixed — emitted Electron project pins patched vite and vitest

- The emitted Electron `package.json` now pins `vitest` 4.1.12 instead of
  3.2.7, and inherits `vite` 7.3.7 (was 7.3.6) from `mosaic-emit-react`'s
  `EmitOptions`. vite 7.3.6 is affected by GHSA-587h-7wmj-h5pm,
  GHSA-698v-4r3x-3vcv, GHSA-r825-w8mm-5pxw, GHSA-rq7h-c2jc-7f22 and
  GHSA-9jrq-w75r-8gcw (fixed in 7.3.7); vitest 3.x bundles `@vitest/mocker`
  3.x, affected by GHSA-82fw-gwwq-j7x9 and GHSA-qxg6-89mg-rqv8, which have no
  fix in the 3.x line. The emitted `vitest run --environment jsdom` script is
  unchanged and works on vitest 4, which accepts vite `^6 || ^7 || ^8`.
