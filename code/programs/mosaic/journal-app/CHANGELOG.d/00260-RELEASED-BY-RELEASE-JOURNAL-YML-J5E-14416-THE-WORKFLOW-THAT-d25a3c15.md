- **Released by `release-journal.yml` (J5e, #14416).** The workflow that
  used to package the old TypeScript Electron app now releases this
  package as `journal-v<semver>`. It publishes only from a
  `workflow_dispatch` on `main`; a pull request that touches the lane runs
  it as a dry run that publishes nothing. A release carries
  `journal-web-v<V>.zip`, `journal-qt-linux-v<V>.zip`,
  `journal-compose-linux-v<V>.zip` and `SHA256SUMS`. These are the three
  hosts CI launches, and each is launched again from its extracted archive
  before it ships. To publish, this changelog's section must be dated
  (`## <V> — YYYY-MM-DD`) and must match `mosaic-package.toml`'s version;
  `code/scripts/journal_release.py` checks both. SwiftUI, Flutter desktop,
  XAML and mobile are not released yet: `journal-mosaic-app.md` says what
  each one waits on.
