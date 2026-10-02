### Fixed — the web and desktop builds install pinned npm dependencies from committed lockfiles

- `scripts/build-web.sh --build` and `scripts/build-electron.sh --build`/
  `--package` used to run a bare `npm install` in the freshly emitted project.
  The emitter pins direct dependencies, but the several hundred transitive
  packages Vite, Electron and electron-builder pull in were resolved fresh on
  every release build. They now install with `npm ci` from
  `npm/web/package-lock.json` and `npm/electron/package-lock.json`, which pin
  every package to one version and a sha512 integrity hash, all resolved from
  registry.npmjs.org. The shared helper is `scripts/npm-lock.sh`.
- `build-electron.sh` no longer fetches its packaging tools with `npx --yes`.
  `electron-builder` (was `^25.1.8`) and `@electron/asar` (was unpinned) are
  exact devDependencies (`25.1.8`, `4.3.1`) and run with `npx --no-install`.
- Both scripts take `--update-lock`, which emits the project and regenerates
  its lockfile with `npm install --package-lock-only --ignore-scripts`, skipping
  the wasm build and installing nothing.
- New `tests/npm_lockfiles.rs` fails when either lock no longer matches the
  emitted package.json (plus the packaging tools for Electron), and when any
  locked package resolves outside registry.npmjs.org or lacks a sha512 hash.
