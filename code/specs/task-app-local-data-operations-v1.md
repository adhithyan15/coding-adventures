# TaskApp local-data operations v1

Issue: [#13614](https://github.com/adhithyan15/coding-adventures/issues/13614)

## Stable identity and exact live-state paths

Every released desktop host passes the same Mosaic application identifier,
`task-app`, to the Rust runtime. `org.codingadventures.trestle` is macOS bundle
and release provenance; it does not replace the persistence identity.

| Released bundle | Live snapshot |
| --- | --- |
| Qt / Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/TaskApp/task-app/mosaic-state.v1.json` |
| Flutter / Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/task-app/mosaic-state.v1.json` |
| Compose Desktop / Linux | `${XDG_DATA_HOME:-$HOME/.local/share}/task-app/mosaic-state.v1.json` |
| SwiftUI / macOS | `~/Library/Application Support/task-app/mosaic-state.v1.json` |
| WinUI / Windows | `%LOCALAPPDATA%\task-app\mosaic-state.v1.json` |

Qt includes the generated executable name `TaskApp` because
`QStandardPaths::AppDataLocation` contributes that directory before Mosaic
appends the stable application identifier. The other hosts select their
platform data root directly. This difference is pinned by the release gate; an
upgrade must reuse the path for its own backend rather than migrate data between
backend directories implicitly.

Each released archive carries a `LOCAL-DATA.txt` beside its install guidance so
these operations remain available offline. The exact artifact and runtime claims
are defined by the [Linux bundle](task-app-linux-release-bundles-v1.md),
[macOS application](task-app-macos-application-v1.md), and
[Windows application](task-app-windows-application-v1.md) contracts.

## Install, launch, and upgrade

These are portable, unsigned development bundles rather than package-manager
installers.

- Linux: extract one `.tar.gz` and launch its `launch-trestle` entry point. Keep
  the extracted application tree together.
- macOS: unzip `Trestle.app`, optionally move it to `/Applications`, and use the
  explicit first-open approval described in `INSTALL.txt`.
- Windows: extract the complete Trestle directory and run `Trestle.exe`; do not
  separate the executable from its runtime and WinUI resources.

For an upgrade, close every Trestle process, make a backup, extract the new
version to a new application directory, and launch it. Do not move or rename the
live-state directory. Linux's `launch-trestle` also makes one non-overwriting,
version-and-backend-specific copy before launch under:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/task-app/backups/
  pre-v<VERSION>-<BACKEND>.json
```

macOS and Windows do not yet have a release launcher that can create that copy,
so their pre-upgrade backup is manual. A future installer or updater must not
claim automatic rollback until its own lifecycle is verified.

## Backup and restore

The live file is one JSON envelope containing the TaskApp Rust snapshot. Close
Trestle before copying it; a running host can atomically replace the file after
an event.

1. Locate the exact live path in the table.
2. Close all Trestle processes.
3. Copy `mosaic-state.v1.json` to a user-chosen backup location. Record the app
   version and backend with the copy.
4. To restore, keep the current live file as a second backup, then copy a
   known-good snapshot back to the same live path.
5. Launch the same backend and confirm the expected tasks and Rust schedule.

Do not edit the snapshot byte array by hand. Cross-backend restore is supported
only when the destination host uses the same `task-mosaic-app/state` schema and
snapshot version; the release gate exercises that shared contract, but it does
not silently search another backend's state directory.

## Uninstall with data, uninstall and purge

To uninstall while retaining tasks, close Trestle and delete only the extracted
Linux/Windows application directory or `Trestle.app`. Reinstalling the same
backend later reuses its live-state path.

To purge local data, first make any wanted backup, then remove the application
and that backend's live `mosaic-state.v1.json`, `mosaic-state.v1.json.tmp`, and
`mosaic-state.v1.json.corrupt`. On Linux, also remove the explicitly selected
`task-app/backups` directory if its pre-upgrade copies are no longer wanted.
Delete only the exact backend paths from the table; do not recursively remove a
broad platform data root.

## Corrupt-state quarantine and manual recovery

Every generated host parses the JSON envelope and asks the Rust runtime to
restore it. If either step rejects the file, the host preserves the rejected
bytes as `mosaic-state.v1.json.corrupt`, reports a persistence warning, and
starts a fresh workspace. A newer failure replaces an older `.corrupt` sibling,
so copy a damaged file elsewhere before retrying repairs.

Recovery is deliberately explicit:

1. Close Trestle and copy the `.corrupt` file somewhere safe.
2. Prefer restoring the most recent known-good manual or Linux pre-upgrade
   backup to the exact live path.
3. Keep the rejected file for forensic/manual extraction; never overwrite the
   only copy while experimenting.
4. Launch Trestle and verify task names, due dates, and generated schedule before
   deleting any recovery copy.

There is no supported partial JSON editor or cloud recovery service in v1.

## Automated upgrade and recovery gate

`fixtures/release-upgrade-v0.1.0.json` is a committed semantic snapshot from the
first TaskApp release. The release helper materializes it into the exact
`task-mosaic-app/state` host envelope. The Rust adapter unit test restores it and
checks its task, due date, schedule, and Full CPM setting.

The pull-request release matrix then seeds that fixture at each platform's
normal per-user path and launches the current extracted bundle without moving
the file. It rejects any unexpected `.corrupt` sibling and verifies the sentinel
task remains. The same matrix seeds invalid bytes, launches the real bundle
again, and requires byte-preserving `.corrupt` quarantine. Windows retains its
separate UI Automation lifecycle; macOS retains replacement-runtime conformance;
Linux retains its one-time pre-upgrade backup comparison.
