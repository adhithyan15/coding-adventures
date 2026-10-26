### Fixed — an unreadable snapshot is set aside, not overwritten

Every hand-written Engram host loads a saved snapshot at startup. If that
load failed, the host reset to the demo deck and saved it over the
snapshot. A snapshot written by a newer build, or one damaged by a crash
mid-write, was destroyed along with the study history in it. Four host
files had this bug:

- `host/electron/host.js`, the shipped desktop app. It stores the snapshot
  at `ENGRAM_SNAPSHOT_PATH`, by default `~/.engram/mosaic-snapshot.v1.json`.
- `host/xaml/MosaicHost.cs`, which uses the same file.
- `host/web/engram-host.mjs` (html and webcomponent) and
  `host/web/engram-host.ts` (react). These store it in
  `localStorage["engram.snapshot.v1"]`.

Now each host first sets the snapshot it could not load aside under the
`.corrupt` name that every generated Mosaic host uses, replacing any
earlier quarantine:

- The file hosts move it to `<snapshot>.corrupt`.
- The web hosts copy it to `engram.snapshot.v1.corrupt`.

Only then does the host save the demo deck. If setting the snapshot aside
fails, the host saves nothing for the rest of the session. An unsaved
session can be recovered; an overwritten snapshot cannot.

`tests/snapshot_quarantine.rs` runs the real `host.js` and
`engram-host.mjs` under Node, with stand-ins only for `electron`, the
wasm engine and the browser `window`. It covers three cases for each
host: the snapshot loads, setting it aside succeeds, and setting it aside
fails. With the old host files, the four set-aside tests fail. The XAML
host needs WinUI and the react host is TypeScript, so `package_compiles`
pins their quarantine source instead.
