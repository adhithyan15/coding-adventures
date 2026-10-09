### Security — npm lockfiles regenerated past the Dependabot advisories

- **`npm/web/package-lock.json` and `npm/electron/package-lock.json`
  regenerated** with `scripts/build-web.sh --update-lock` and
  `scripts/build-electron.sh --update-lock`. The cooldown still applies:
  nothing published in the last seven days was taken. The emitter now pins
  `electron` 42.11.10 and `vitest` 4.1.11, and `build-electron.sh` moves
  `electron-builder` from 25.1.8 to 26.17.0.
- This closes the critical `tinypool` advisories and the moderate `vitest`
  advisory in both locks. In the Electron lock it also closes the six
  Electron 42 advisories, the `app-builder-lib` and `builder-util-runtime`
  advisories (GHSA-7g7r-gx96-252g, GHSA-p2f4-r6v6-j797), and twelve `tar`
  advisories.
- **Still open in `npm/electron` until the cooldown passes:**
  - `shell-quote` (critical, via `concurrently` 10.0.4; fixed in 10.0.6).
  - `http-cache-semantics` (high; fixed in 4.3.0).
  - `sprintf-js` (moderate; no patched release exists).
  
  All three are recorded in `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md`.
- **`electron-winstaller` added to `INSTALL_SCRIPT_PACKAGES`** in
  `tests/npm_lockfiles.rs`. electron-builder 26 brings it in as a peer of
  `electron-builder-squirrel-windows`. Its install script (reviewed at 5.4.0)
  only copies its own bundled 7-Zip binary for the host arch. It never runs
  here anyway, because the build installs with `npm ci --ignore-scripts`.
