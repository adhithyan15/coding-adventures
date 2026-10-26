### Changed — the desktop lock moves to Electron 43.8.0

The Mosaic emitter now pins Electron 43.8.0 (was 42.5.0), so
`npm/electron/package-lock.json` was regenerated with
`scripts/build-electron.sh --update-lock` under npm 11. The `--before` floor
held every resolved version to one published at least seven days ago. Besides
Electron, transitive packages moved within their existing ranges: undici
7.30.1, follow-redirects 1.16.1, ws 8.22.0, rollup 4.64.8, @swc/core 1.16.18,
postcss 8.5.30 and a few smaller ones. npm 11 also records the `libc` selectors
on the glibc/musl platform packages, which the previous npm 10 lock had dropped.
The `--package` comment that traces the runtime zip back to the lock now names
43.8.0.
