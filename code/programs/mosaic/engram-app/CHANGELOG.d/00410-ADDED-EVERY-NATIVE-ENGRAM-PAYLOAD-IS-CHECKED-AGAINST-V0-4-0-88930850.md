### Added — every native Engram payload is checked against v0.4.0 state

`release-engram.yml` used to run the shared upgrade and corrupt-recovery gate
only on the Compose Linux payload. Now it runs on every payload whose host
writes the Mosaic state envelope. A regression on any of the other hosts used
to ship unnoticed. It now fails before its payload is uploaded. Details are in
`code/specs/mosaic-release-state-gate-v1.md`, "Engram's release lane":

| Payload | Launched through | Live state file |
| --- | --- | --- |
| Compose, macOS | `mosaic-release-launch.sh`, 15 s | `~/Library/Application Support/engram-app/` |
| Compose, Windows | `mosaic-release-launch.ps1`, 15 s | `%LOCALAPPDATA%\engram-app\` |
| SwiftUI, macOS | `mosaic-release-launch.sh`, 10 s | `~/Library/Application Support/engram-app/` |
| Flutter, Linux | xvfb + `timeout 15s` | `$XDG_DATA_HOME/engram-app/` |
| Flutter, macOS | `mosaic-release-launch.sh`, 15 s | the app's sandbox container |
| Flutter, Windows | `mosaic-release-launch.ps1`, 15 s | `%LOCALAPPDATA%\engram-app\` |
| Qt, macOS | `mosaic-release-launch.sh`, 10 s | `~/Library/Application Support/Engram/engram-app/` |

Each payload runs both halves of the gate, as Compose Linux does. The v0.4.0
fixture is materialized, the extracted payload is launched, and `verify` runs.
Then invalid bytes go in, a probe is copied, the payload is relaunched, and
`verify-quarantine` runs. Two of the paths need explaining:

- The Flutter macOS build is sandboxed. `flutter create`'s
  `Release.entitlements` turns the sandbox on, so the host's `HOME` is the
  app's container. The gate reads the bundle id and the entitlement from the
  extracted app. It also launches the app once before writing to the
  container, so that macOS creates the container itself.
- Qt's `AppDataLocation` adds the application name, `Engram`.

The XAML payload (`build-xaml`, not published) runs only the recovery half. Its
hand-written host stores engram-capi's own snapshot format at
`~/.engram/mosaic-snapshot.v1.json`. That is not the envelope, and no XAML
release exists to upgrade from. The Electron and web payloads are not gated.
The spec lists all three as named gaps and gives the reasons.

The launchers are new and shared with TaskApp. macOS runners have no GNU
`timeout`, so `code/scripts/mosaic-release-launch.sh` keeps the app up for N
seconds, then stops it. It exits 2 if the app quits early, so TaskApp's
SwiftUI gate still collects its crash reports and an lldb backtrace only on a
crash. `code/scripts/mosaic-release-launch.ps1` is TaskApp's hidden-window
`Start-Process` check, moved out of the workflow. TaskApp's macOS and WinUI
upgrade and recovery launches now use both. That also fixes TaskApp's macOS
runtime-unavailable check: it was a `! grep` that was not the last command of
its function, so it never failed the step.

`engram_release.py payload-name --backend B --platform P --version V` names
each native archive, so the gate steps do not spell the published names in
shell. The lane's `paths:` filter now covers both launchers, their tests, and
the Flutter, Qt and SwiftUI host templates as well as Compose's.
`test_engram_release.py` pins every gate step: its order, its launcher, its
state path, and that the gated set matches the spec table.
`test_mosaic_release_launch.py` checks the bash launcher against stub
executables. That suite runs in CI's always-on contracts job and in both
release lanes.
