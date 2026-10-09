### Security — Dependabot sweep and the external dependency alert backlog

- **Every Dependabot alert a safe bump can close is closed.**
  - `npm audit fix --package-lock-only` (npm 11.13.0, as CI pins) was run over
    the 486 `package-lock.json` files that audited dirty. The main fix is
    `source-map-js` 1.2.1 → 1.2.2 (GHSA-68fv-2mgg-jv7q, high) in 479
    lockfiles.
  - The three Electron apps also move `electron` 43.4 → 43.7.9, `undici`,
    `brace-expansion`, `fast-uri`, `http-cache-semantics` and the
    electron-builder 26.17 chain.
  - The five jest 29 packages hit `handlebars` (critical) through istanbul.
- **`trig`, `wave`, `matrix`, `gradient-descent` and `loss-functions` move to
  jest 30.** `braces` (GHSA-vfj7-8cjw-p6xm) has no patched release, so
  dropping it from the tree is the only fix.
- **Engram's generated npm locks** move to `electron` 42.11.10, `vitest`
  4.1.11 and `electron-builder` 26.17.0. The pins are changed in the Mosaic
  emitters, not the locks.
- **New `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md`** records every
  third-party package that has ever raised a Dependabot alert here (36
  entries), what is still open and why, and which in-house replacement would
  remove each family of alerts.
- Cargo, Go, Dart and Hex dependencies were audited and have no open
  advisories.
