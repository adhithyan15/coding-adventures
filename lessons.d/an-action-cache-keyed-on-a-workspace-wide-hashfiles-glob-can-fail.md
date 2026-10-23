---
category: CI & GitHub Actions
---

# An action cache keyed on a workspace-wide hashFiles glob can fail a green job in its post step

**What went wrong.** On #17972, a ~500-lockfile npm bump that built ~490
packages, `build (macos-latest)` built and tested every package green. It then
failed in "Post Set up Flutter" with
`hashFiles('**/pubspec.lock') couldn't finish within 120 seconds`.
`subosito/flutter-action` keys its pub cache on that glob. The cache is saved,
and the key re-evaluated, only after a successful job. By that point the
workspace holds every built package's `node_modules`, so the walk is slow. The
same step took 26 s at job start on a clean checkout. Small PRs never notice.
A red job that already failed earlier never reaches the save, so the failure
only appears once everything else is fixed.

**Fix.** In `ci.yml`, give the Flutter step `pub-cache: false` next to
`cache: true`. The SDK cache's key has no `hashFiles`, so it stays. Without the
pub cache, `pub get` downloads dependencies again, which is cheap.

**Do differently.**
- Before trusting an action's `cache: true`, read its `action.yaml` for
  `hashFiles('**/…')` keys.
- Remember that the post step runs against the workspace as the job left it,
  not as it was checked out.
- A `ci.yml` edit like this one does not force a full rebuild on a PR. The
  build tool prints "keeping package-scoped PR selection", and with no package
  changes it selects nothing (`build-tool -dry-run -emit-plan` shows 0
  affected).
