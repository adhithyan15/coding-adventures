### Fixed — the web and desktop builds install pinned npm dependencies from committed lockfiles

- `scripts/build-web.sh --build` and `scripts/build-electron.sh --build`/
  `--package` used to run a bare `npm install` in the freshly emitted project.
  The emitter pins direct dependencies, but the several hundred transitive
  packages Vite, Electron and electron-builder pull in were resolved fresh on
  every release build. They now install with `npm ci --ignore-scripts` from
  `npm/web/package-lock.json` and `npm/electron/package-lock.json`, which pin
  every package to one version and a sha512 integrity hash, all resolved from
  registry.npmjs.org. No install script runs; none is needed (esbuild and swc
  load their binaries from the locked platform packages). The shared helper is
  `scripts/npm-lock.sh`.
- `build-electron.sh` no longer fetches its packaging tools with `npx --yes`.
  `electron-builder` (was `^25.1.8`) and `@electron/asar` (was unpinned) are
  exact devDependencies (`25.1.8`, `4.3.0`) and run with `npx --no-install`.
- Both scripts take `--update-lock`, which emits the project and regenerates
  its lockfile with `npm install --package-lock-only --ignore-scripts`, from
  versions published at least seven days earlier (`--before`), skipping the wasm
  build and installing nothing.
- New `tests/npm_lockfiles.rs` fails when either lock no longer matches the
  emitted package.json (plus the packaging tools for Electron), when a locked
  package is not its own registry tarball (`<name>/-/<name>-<version>.tgz`) or
  lacks a sha512 hash, when an entry is unreachable from the project's
  dependencies (npm ci would still install it), and when a package outside
  `@swc/core`, `esbuild` and `fsevents` declares an install script.
