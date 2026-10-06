### Fixed — the desktop installer packages an Electron runtime verified against the lockfile

- `build-electron.sh --package` let electron-builder download the Electron
  runtime zip itself, outside the npm lockfile. It now runs Electron's
  `install.js` first, which verifies the zip's SHA-256 against
  `checksums.json` shipped inside the lock-pinned `electron` tarball, and sets `build.electronDist` to
  `node_modules/electron/dist` so electron-builder copies that verified runtime.
- install.js runs through a Node wrapper that first deletes, case-insensitively
  (Windows environment names are), the variables that would make it fetch
  checksums from the network (`electron_use_remote_checksums`) or another
  platform's runtime (`ELECTRON_INSTALL_PLATFORM`/`ARCH`, `npm_config_platform`/
  `arch`). Afterwards `dist/version` must equal the pinned electron version.
- A dropped download is retried up to three times; a checksum mismatch fails
  every attempt. Locally the packaged `engram` binary is byte-identical to the
  verified one, and a tampered checksum (fresh or cached download) is refused.
