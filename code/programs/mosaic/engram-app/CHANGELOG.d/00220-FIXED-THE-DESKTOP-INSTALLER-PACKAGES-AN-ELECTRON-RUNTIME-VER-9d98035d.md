### Fixed — the desktop installer packages an Electron runtime verified against the lockfile

- `build-electron.sh --package` let electron-builder download the Electron
  runtime zip itself, outside the npm lockfile. It now runs Electron's
  `install.js` first, which verifies the zip's SHA-256 against
  `checksums.json` shipped inside the lock-pinned `electron` tarball (with the
  two remote-checksum overrides cleared), and sets `build.electronDist` to
  `node_modules/electron/dist` so electron-builder copies that verified runtime.
- A dropped download is retried up to three times; a checksum mismatch fails
  every attempt. Locally the packaged `engram` binary is byte-identical to the
  verified one, and a tampered checksum (fresh or cached download) is refused.
