# VisiCalc Electron eMudhra advisory remediation

Issue #18270 tracks Dependabot alert #7877 / GHSA-jr5w-w6x2-84p9. The
`visicalc-electron` program directly loads Electron at runtime. Electron 43.8.0
is inside the advisory's vulnerable range (`>=43.0.0-alpha.1, <43.8.1`), so a
caret declaration that still permits 43.8.0 is not sufficient even if GitHub's
alert state changes independently of the checked-in lockfile.

## Required state

- `package.json` must require Electron `^43.8.1` or a later compatible patched
  release on the reviewed 43.x line. A major-version transition must fail
  closed until its separate advisory threshold is reviewed.
- `package-lock.json` must resolve the direct Electron package to 43.8.1 or a
  later release outside the vulnerable range.
- The update must retain the existing dependency tree shape: no package keys
  may be added or removed merely to refresh Electron.
- Application behavior and the renderer security boundary remain unchanged.

## Regression and validation

The package test suite must read both manifests and reject either a dependency
floor below 43.8.1, a locked Electron release below 43.8.1, or an unreviewed
major transition. This matters because the advisory has distinct patched
thresholds per Electron major; 44.0.0 and 44.9.0 are not made safe merely by
sorting above 43.8.1. Validation also includes a clean `npm ci`, the existing
main-process smoke test, a production dependency audit, and review of the
lockfile diff for unrelated package-key or cross-platform metadata churn.
