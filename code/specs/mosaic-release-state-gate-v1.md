# Mosaic release state gate v1

Generalises: [TaskApp local-data operations v1 — Automated upgrade and recovery
gate](task-app-local-data-operations-v1.md#automated-upgrade-and-recovery-gate)

Implementation: `code/scripts/mosaic_release_state_gate.py` (standard library
only), tested by `code/scripts/tests/test_mosaic_release_state_gate.py`.
Payloads are launched through `code/scripts/mosaic-release-launch.sh` (macOS)
and `code/scripts/mosaic-release-launch.ps1` (Windows); see
[Launching a payload](#launching-a-payload).

## Why this exists

Every Mosaic desktop app persists the same way. The generated host asks the
Rust runtime for a `Snapshot` and writes it to a per-user file:

```json
{"bytes":[123,34,...],"schema":"engram-mosaic-app","version":3}
```

`schema` and `version` name the payload's shape; `bytes` is the payload itself,
serde's encoding of a `Vec<u8>` (one JSON integer per byte). The payload is
whatever the app's adapter chose to write — in every app so far, UTF-8 JSON.

A release that cannot read the previous release's file loses the user's data
on the first launch after an upgrade. A host that cannot recognise a damaged
file must not silently overwrite it either; it moves it aside as
`<state>.corrupt` and starts empty. TaskApp's release lane proved both
properties against real extracted bundles from its first release onward. None
of that proof was TaskApp-specific except the final "is my task still there?"
question, so this spec lifts the rest into a gate any app can adopt: Engram
first, then Journal and VisiCalc.

The gate has three halves, each a CLI subcommand:

| Subcommand | Runs | Proves |
| --- | --- | --- |
| `materialize` | before the new build launches | an older release's state is on disk, byte-shaped exactly as that release's host wrote it |
| `verify` | after the new build ran on it | the new build kept it: no quarantine, a compatible schema and version, the expected values |
| `verify-quarantine` | after the new build ran on garbage | the garbage was moved to `.corrupt` byte for byte, not deleted or repaired |

## The fixture

A fixture is a committed JSON file, conventionally
`code/programs/mosaic/<app>/fixtures/release-upgrade-v<version>.json`.

```json
{
  "fixtureSchema": "mosaic/release-upgrade-fixture",
  "fixtureVersion": 1,
  "sourceApp": "engram",
  "sourceVersion": "0.4.0",
  "provenance": "How the state was captured, in prose.",
  "snapshotSchema": "engram-mosaic-app",
  "snapshotVersion": 1,
  "acceptedLiveVersions": [1, 3],
  "expectations": [
    {"liveVersions": [1], "pointer": "/decks/0/name", "equals": "Upgrade fixture deck"},
    {"liveVersions": [3], "pointer": "/session/state/decks/0/name", "equals": "Upgrade fixture deck"}
  ],
  "state": {"decks": [{"name": "Upgrade fixture deck"}]}
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `fixtureSchema` | string | Exactly `mosaic/release-upgrade-fixture`. |
| `fixtureVersion` | integer | Exactly `1`. A later shape gets a new number, never a silent reinterpretation. |
| `sourceApp` | string | The product, `^[a-z0-9][a-z0-9-]*$` (for example `engram`, `task-app`). Used in error messages. |
| `sourceVersion` | string | The release the state came from. Strict SemVer 2.0.0, no `v` prefix. |
| `provenance` | string, optional | How the state was produced. JSON has no comments; this is the comment. |
| `snapshotSchema` | string | The envelope `schema` that release wrote. Non-empty. |
| `snapshotVersion` | integer | The envelope `version` that release wrote. Non-negative. |
| `acceptedLiveVersions` | list of integers | Every envelope `version` the live file may carry after the new build ran. Non-empty, unique, non-negative. Need not contain `snapshotVersion`. |
| `expectations` | list | Non-empty. Each entry is described below. |
| `state` | object | The semantic payload: the JSON the old release encoded as `bytes`. |

Unknown top-level or expectation keys are rejected, so a typo such as
`expectation` fails loudly instead of disabling a check.

A fixture file must be a regular, non-symlinked file of at most 4 MiB.

### Why `acceptedLiveVersions` is a list

TaskApp's original gate required the live `version` to equal the fixture's.
That holds only while an app never bumps its payload version. Engram's v0.4.0
wrote version 1; the current adapter restores 1 through 3 and *writes* 3. A host
that re-persists after launching therefore turns a v1 file into a v3 file, and a
host that does not leaves it at v1. Both are correct upgrades. Version 2 is
neither: the current adapter never writes it, so a live v2 file after this
upgrade would mean something unexplained happened, and the gate says so.

The list is therefore *what the current code can prove*, not every version the
adapter can read.

### Expectations

```json
{"liveVersions": [3], "pointer": "/session/state/cards/0/front", "equals": "Upgrade fixture front"}
```

- `pointer` is an [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) JSON
  Pointer evaluated against the **decoded live payload** (not the envelope, and
  not the fixture's `state`). `~1` is `/`, `~0` is `~`, and array indexes are
  decimal without leading zeros. `-` (the past-the-end index) never resolves.
  The empty pointer `""` names the whole document.
- `equals` is any JSON value. Comparison is type-strict: `true` is not `1`, and
  `1` is not `"1"`. Integers and floats compare by numeric value, as JSON does
  not distinguish them.
- `liveVersions` is optional. When present it must be a non-empty subset of
  `acceptedLiveVersions`, and the expectation applies only when the live
  envelope carries one of those versions. When absent the expectation applies
  to every accepted version.

A payload version change usually moves the data — Engram v3 nests the old
collection under `/session/state` — so one pointer cannot name the same value in
both shapes. Scoping by version lets one fixture say where the deck is in each.

**No vacuous pass.** Loading a fixture fails unless *every* accepted live version
has at least one applicable expectation. Without that rule, accepting a new
version while forgetting to describe it would make the gate check nothing for
exactly the case it was extended to cover.

### The state must be captured, never written by hand

`state` must be the real output of the source release's adapter, captured by
running that release's code (for example a throwaway test in a worktree at the
release tag that seeds data through the public API and prints `snapshot()`'s
bytes). A hand-written state proves only that the new build reads what the
author believed the old build wrote. `provenance` records how it was captured.

## `materialize --fixture F --output S`

1. Load and validate `F`.
2. Encode `state` as compact JSON (`,` and `:` separators, sorted keys, UTF-8,
   non-ASCII left unescaped). This is serde_json's own output for a
   `serde_json::Value` (whose maps are sorted), which is what every current
   adapter writes. For Engram v0.4.0 it reproduces the release's bytes exactly.
3. Write `{"bytes":[...],"schema":snapshotSchema,"version":snapshotVersion}`
   (compact, sorted keys) plus one `\n`, as bytes — no platform newline
   translation — to `S`, creating parent directories.
   The file is created with mode `0600`, as it stands in for one person's
   private data.

`S` must not be a symlink or any non-regular file; the gate never writes through
a link into a location the caller did not name.

## `verify --fixture F --state S`

Fails, with a message naming `sourceApp`, unless all of these hold in order:

1. Nothing exists at any quarantine name: `S.corrupt`, `S.corrupt.1` …
   `S.corrupt.9` (a dangling symlink counts). A host moves a rejected file to
   the first free one of these ten and never replaces an earlier quarantine,
   so a refusal on a machine that already holds `.corrupt` lands in
   `.corrupt.1`. Any of them means the new build rejected a compatible file. This is checked
   first because it is the most specific diagnosis: a host that quarantined
   the fixture and then wrote an empty state, or nothing, would otherwise be
   reported as a missing value or a missing file.
2. `S` exists, is a regular file and is not a symlink.
3. `S` is at most 16 MiB, is UTF-8 JSON, and is an object.
4. Its `schema` equals `snapshotSchema`.
5. Its `version` is an integer in `acceptedLiveVersions`.
6. Its `bytes` is a list of integers each in `0..=255` (booleans are not
   integers here, even though Python's `bool` is one).
7. The bytes decode as UTF-8 JSON to an object.
8. Every expectation applicable to that version resolves and equals its value.

## `verify-quarantine --state S --probe P [--app NAME]`

Used after writing invalid bytes to `S`, copying them to `P` and relaunching:

1. `S.corrupt` exists, is a regular file and not a symlink. A release runner
   starts with no state, so the first free quarantine name is `.corrupt`.
2. Its bytes equal `P`'s exactly.
3. If `S` exists, its bytes differ from `P`'s. The host must *move* the rejected
   file aside; a copy left at the live path is quarantined again on every
   launch and never recovers.

`P` is subject to the same regular-file and size rules. No fixture is needed,
because nothing about garbage is app-specific; `--app` only names the product in
error messages.

## Failure modes, summarised

| Symptom | Meaning |
| --- | --- |
| state does not exist | the host wrote elsewhere, or deleted the file |
| `.corrupt` or `.corrupt.1`…`.9` present on `verify` | the new build rejected a compatible file — the upgrade lost data |
| schema mismatch | the host or adapter renamed the payload |
| version not accepted | an unbumped payload change, or an unexplained rewrite |
| invalid bytes | the envelope is malformed (not a 0..255 integer list) |
| pointer missing / value differs | the payload was restored but lost or altered data |
| `.corrupt` missing on `verify-quarantine` | the host deleted or repaired damaged data instead of preserving it |
| `.corrupt` differs from probe | the host rewrote the damaged bytes before preserving them |
| live state equals probe | the host copied instead of moving |
| symlink or non-regular file | refused outright; the gate does not follow links |
| file over the size cap | refused; fixture-seeded state is small, so a huge file is itself a fault |

The CLI exits `0` on success and `1` with a one-line `error: ...` on any
failure. It never evaluates fixture content as code.

## How an app adopts the gate

1. Capture a fixture from the app's latest published release, as above, with
   `acceptedLiveVersions` set to what the current adapter can leave on disk.
2. Add a Rust test to the app's adapter that `include_str!`s the fixture,
   restores its `state` at `snapshotVersion`, asserts the data survives, and
   checks every expectation against the bytes of a fresh `snapshot()` — that is
   what proves the fixture's pointers describe the real current payload rather
   than a guess.
3. In the release workflow, per desktop backend: `materialize` into the
   backend's real live-state path, launch the extracted bundle, `verify`; then
   write invalid bytes, copy them to a probe, relaunch, `verify-quarantine`.
4. Run `test_mosaic_release_state_gate.py` in that workflow and add
   `code/scripts/mosaic_release_state_gate.py` and its test to its `paths:`.

## Launching a payload

Step 3 launches what a user downloads: the archive the release lane just
wrote, extracted to a fresh directory, never the build tree. Every launch
unsets `MOSAIC_APP_LIBRARY` (so only the runtime inside the payload can load)
and `MOSAIC_APP_STATE_PATH` (so the host resolves the path a user's machine
would resolve). A launch passes only if the app is **still running** when its
time is up. An app that exits early has crashed, or refused to start without
its runtime, and the step fails with its log.

One launcher per runner OS, shared by every app's release lane, so the rule
above cannot drift between apps:

| Runner | Launcher | Why this one |
| --- | --- | --- |
| Linux | `code/scripts/run-under-xvfb.sh timeout <N>s <exe>`, run from `/`; exit `124` required | Needs a virtual display. `timeout` killing a live app is the pass. |
| macOS | `code/scripts/mosaic-release-launch.sh <N> <log> <exe-or-.app>` | macOS runners have no GNU `timeout`. |
| Windows | `code/scripts/mosaic-release-launch.ps1 -ExePath <exe> -Seconds <N> -Label <name>` | A WinUI or Flutter GUI exe has no console. `Start-Process` with a hidden window is what TaskApp's WinUI gate proved on `windows-latest`. |

`mosaic-release-launch.sh` runs the target from `/` with its output in
`<log>`. It checks once a second that the process is still alive, then stops it
with `TERM` (then `KILL` after 10 s) and prints the log. A target ending in
`.app` is resolved through `Contents/Info.plist`'s `CFBundleExecutable`, which
must be a bare file name.

| Exit | Meaning |
| --- | --- |
| `0` | alive for the whole window, stopped, and the log has no `Mosaic Rust runtime unavailable` |
| `1` | usage error, an unresolvable target, or a log reporting `Mosaic Rust runtime unavailable` |
| `2` | the app exited before its time was up |

The separate `2` lets a caller run crash diagnostics only for a crash, as
TaskApp's SwiftUI gate does with `lldb`.

`mosaic-release-launch.ps1` starts the exe with `Start-Process -WindowStyle
Hidden -PassThru`, with its working directory set to the drive root. It checks
`HasExited` once a second, throws (exit `1`) naming the exit code if the app
quit early, and stops the app with `Stop-Process -Force` in a `finally`. It
cannot read a GUI exe's output, so the runtime-unavailable report is checked
the other way: Compose's `Main.kt` exits when the runtime does not load, which
fails the liveness check.

TaskApp's macOS and Windows gates and Engram's use these two scripts.
`code/scripts/tests/test_mosaic_release_launch.py` pins their behaviour (the
bash one against stub executables) and pins that both workflows call them.

## Engram's release lane

`release-engram.yml` gates every payload it builds whose host persists the
envelope above, with fixture
`code/programs/mosaic/engram-app/fixtures/release-upgrade-v0.4.0.json`. Each
gate is one step that runs after the archive step and before the upload. That
way a payload that fails is never uploaded, so the publish job's
payload-set check fails too.

The live path is where the shipped host really looks. The application id is
the package name, `engram-app`. On Linux, `XDG_DATA_HOME` moves only the root,
to a scratch directory. On macOS and Windows the gate uses the runner's real
per-user directory, and clears the state and every quarantine name before and
after.

| Payload | Job (leg) | Launcher, window | Live state file |
| --- | --- | --- | --- |
| Compose, Linux | `build-compose` (linux) | xvfb + `timeout 15s`, `*/bin/engram_app` | `$XDG_DATA_HOME/engram-app/mosaic-state.v1.json` |
| Compose, macOS | `build-compose` (macos) | `mosaic-release-launch.sh 15`, the `.app` | `~/Library/Application Support/engram-app/mosaic-state.v1.json` |
| Compose, Windows | `build-compose` (windows) | `mosaic-release-launch.ps1 -Seconds 15`, `engram_app.exe` | `%LOCALAPPDATA%\engram-app\mosaic-state.v1.json` |
| SwiftUI, macOS | `build-swiftui` | `mosaic-release-launch.sh 10`, the `.app` | `~/Library/Application Support/engram-app/mosaic-state.v1.json` |
| Flutter, Linux | `build-flutter` (linux) | xvfb + `timeout 15s`, the one executable in the bundle root | `$XDG_DATA_HOME/engram-app/mosaic-state.v1.json` |
| Flutter, macOS | `build-flutter` (macos) | `mosaic-release-launch.sh 15`, the `.app` | `~/Library/Containers/<CFBundleIdentifier>/Data/Library/Application Support/engram-app/mosaic-state.v1.json` when the app is sandboxed; otherwise as SwiftUI |
| Flutter, Windows | `build-flutter` (windows) | `mosaic-release-launch.ps1 -Seconds 15`, the one `.exe` in the bundle root | `%LOCALAPPDATA%\engram-app\mosaic-state.v1.json` |
| Qt, macOS | `build-qt` | `mosaic-release-launch.sh 10`, the `.app` | `~/Library/Application Support/Engram/engram-app/mosaic-state.v1.json` |

Each runs both halves: `materialize` → launch → `verify`, then invalid bytes
→ probe → relaunch → `verify-quarantine --app engram`. The recovery half also
proves the host reads that path at all. A host that looks elsewhere leaves the
v1 file untouched, which `verify` alone would accept, but then nothing is
quarantined and `verify-quarantine` fails. So a wrong path in this table fails
the step loudly rather than passing it.

Three of these paths are not the obvious one:

- **Flutter on macOS is sandboxed.** `flutter create` writes
  `macos/Runner/Release.entitlements` with `com.apple.security.app-sandbox`,
  and `build-native.sh` keeps it. A sandboxed process gets its container as
  `HOME`, and the generated Flutter host roots its state at `HOME`. The gate
  reads the bundle id from the extracted `Info.plist` and the entitlement from
  `codesign -d --entitlements -`. If the app is sandboxed, the gate first
  launches it once with no state, so that macOS creates the container. A
  directory created by hand has no container metadata. The gate then seeds the
  container.
- **Qt appends the application name.** The generated Qt host roots its state
  at `QStandardPaths::AppDataLocation`. On macOS that is `~/Library/Application
  Support/<organization>/<application>`. Nothing sets an organization, and the
  application name is the bundle's `CFBundleName`, `Engram`.
- **Compose and Flutter on Windows** use `%LOCALAPPDATA%`, not `%APPDATA%`.

### Gaps (not gated, by name)

| Payload | What is missing | Why |
| --- | --- | --- |
| XAML, Windows (`build-xaml`, not published) | **upgrade half** | Engram's XAML build still ships a hand-written host (`host/xaml/MosaicHost.cs`, a `[host_assets]` override). It persists engram-capi's own snapshot JSON at `%USERPROFILE%\.engram\mosaic-snapshot.v1.json`, not the envelope this gate materializes. No XAML payload has ever been published, so there is no previous release's XAML state to upgrade from. This closes when XAML moves onto the standard runtime, or when a release publishes it and a fixture of that format is captured. |
| XAML, Windows | (recovery half **is** gated) | `mosaic-release-launch.ps1` launches the archived publish output's apphost (the exe named by the root's `*.runtimeconfig.json`). `verify-quarantine` then runs against `%USERPROFILE%\.engram\mosaic-snapshot.v1.json`. The host copies the bad file to `.corrupt` and then saves the demo deck over it, so the live file differs from the probe, as the gate requires. The job is `continue-on-error`, so this step informs and does not block, like the rest of the job. |
| Electron desktop, all three (`build-desktop`) | **both halves** | `host/electron/host.js` persists the same engram-capi format at `~/.engram/mosaic-snapshot.v1.json`, so the upgrade half needs a fixture captured in that format from v0.4.0's Electron build, and a gate mode that writes it. The recovery half would need its own launch work: the Linux AppImage needs FUSE or `--appimage-extract-and-run` and Chromium's sandbox settled, and the Windows portable exe unpacks itself and starts a second process, so the PID it returns is not the app's. |
| Web (`build-web`) | both halves | The browser build persists to `localStorage`, not a file. The gate's file checks do not apply. |

The XAML recovery gate runs on `windows-latest`, and none of the launches can
run anywhere else. Their first real evidence is a run of this workflow, which
runs on every pull request that touches it.

## TaskApp's legacy fixture

TaskApp's `fixtures/release-upgrade-v0.1.0.json` predates this spec and uses
`fixtureSchema: task-app/release-upgrade-fixture`, a single `snapshotVersion`,
and a TaskApp-only `expectedTask` block. It stays readable **through an adapter
in `taskapp_release.py`**, not by converting the file:

- The file is a published contract. `task-mosaic-app`'s Rust test reads it and
  asserts its `fixtureSchema`, and five release-lane steps name it; converting
  it would churn all of them for no behavioural gain.
- `expectedTask` is stronger than a pointer. It requires exactly one task with
  that name anywhere in the workspace, independent of task or project ids; a
  pointer would pin an id that TaskApp is free to change.

The adapter validates the legacy shape exactly as before, builds the gate's
in-memory fixture from it (`acceptedLiveVersions = [snapshotVersion]`, no
pointer expectations), delegates every generic check to the shared module, and
then runs the `expectedTask` walk. Its CLI subcommands
(`materialize-upgrade-fixture`, `verify-upgrade-state`) are unchanged. The
non-empty-expectations rule applies to fixture *files* of the generic schema;
the legacy adapter supplies its own app-specific check in its place.

One behavioural difference: the shared writer emits `\n` as a byte on every
platform, where TaskApp's previous writer used text mode and so wrote `\r\n` on
Windows. Both are the same JSON to every host.
