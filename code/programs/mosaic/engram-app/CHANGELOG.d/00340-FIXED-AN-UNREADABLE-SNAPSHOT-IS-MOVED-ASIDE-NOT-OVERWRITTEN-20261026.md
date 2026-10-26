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

Now each host first copies the snapshot it could not load aside. It uses
the first free name in a fixed sequence:

- The file hosts use `<snapshot>.corrupt` (the name every generated Mosaic
  host uses), then `.corrupt.1` and so on up to `.corrupt.9`.
- The web hosts use the same sequence of `localStorage` keys, starting
  with `engram.snapshot.v1.corrupt`.

A copy never replaces an earlier quarantine, because the earlier one may
be the real history. For example, startup rejects the snapshot, and then
the reload after an `.apkg` import rejects the merged result too. Only
once the copy is made does the host save the demo deck. If the copy
fails, or all ten names are taken, the host saves nothing for the rest of
the session. An unsaved session can be recovered; an overwritten snapshot
cannot.

`tests/snapshot_quarantine.rs` runs the real `host.js` and
`engram-host.mjs` under Node, with stand-ins only for `electron`, the
wasm engine and the browser `window`. It covers these cases: the snapshot loads; the copy goes to the first
free name; an earlier quarantine is never replaced; a snapshot already
held is not copied twice (web); and nothing is saved when the copy fails
or all ten names are taken. Against main's host files, the quarantine
tests fail, and against a single-slot `.corrupt` they fail too. The XAML
host needs WinUI and the react host is TypeScript, so `package_compiles`
pins their quarantine source instead.
