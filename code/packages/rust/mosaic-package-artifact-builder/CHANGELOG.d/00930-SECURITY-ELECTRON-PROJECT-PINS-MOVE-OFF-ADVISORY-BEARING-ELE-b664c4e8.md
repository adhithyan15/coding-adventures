### Security — Electron project pins move off advisory-bearing electron and vitest

- **The Electron project's pinned devDependencies move off versions with open
  Dependabot advisories.** `electron` goes from `42.5.0` to `42.11.10`. That
  closes GHSA-hq2x-r82h-9wj4, GHSA-gr2m-v5gq-v685, GHSA-j84w-jfhq-vhvj,
  GHSA-9qh4-3jw8-366w, GHSA-qmv3-fv6v-rmhq and GHSA-r4w5-6pfg-jxp5 (popup
  sandbox inheritance, cross-origin reads, `<webview>` Node integration,
  preload code-cache poisoning). `vitest` goes from `3.2.7` to `4.1.11`. That
  closes GHSA-82fw-gwwq-j7x9 and drops `tinypool`, which carried two critical
  prototype-pollution advisories (GHSA-5gmw-xhrv-c9v3, GHSA-85c8-ppgw-ccpr).
  The 3.x line has no fix for either.
- `concurrently` stays at `10.0.4` for now. The fix for `shell-quote`
  (GHSA-pqg4-j6r4-53mv) only arrives with `concurrently` 10.0.6. That release
  is younger than the seven-day cooldown `engram-app`'s `--update-lock`
  enforces, so it is tracked in
  `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md` instead.
