---
category: Testing & coverage
---

# A Mosaic host harness run without MOSAIC_APP_STATE_PATH persists into the real per-user data directory

**What went wrong.** While verifying the UI48 ENV4 hardening locally, the XAML
conformance harness was run once with `MOSAIC_APP_STATE_PATH` unset, to cover
its "skipped" branch. The binding was generated *for an application*
(`xaml_runtime_binding_for_application`), so persistence was on, and with no
explicit path the host fell back to its default location -- on Linux,
.NET's `LocalApplicationData`, i.e. `~/.local/share/<app-id>/mosaic-state.v1.json`.
The run wrote a real state file into the user's home, outside the scratch
directory, and a second run of the same case then *restored* it and failed
`initial count`, which looked like a regression and was not.

Every application-scoped host does the same: Compose and Flutter use
`$XDG_DATA_HOME` or `~/.local/share`, SwiftUI Application Support, Qt
`AppDataLocation` (see `mosaic-app-bindings/README.md`, "Persistence").

**The fix.** The stray file (and the directory the run created) were removed.
Runs of the no-state-path case now set `XDG_DATA_HOME` to a scratch directory,
which both .NET and the Dart host honour on Linux, and clear it between runs.

**Do differently.** When a harness runs an application-scoped host, always pin
where it may write: set `MOSAIC_APP_STATE_PATH` to a scratch file, or -- to
exercise the unset branch -- point the platform's data root (`XDG_DATA_HOME`
on Linux, `LOCALAPPDATA` on Windows) at a fresh scratch directory. A second
run against a leftover default state file restores it and fails on counts.
