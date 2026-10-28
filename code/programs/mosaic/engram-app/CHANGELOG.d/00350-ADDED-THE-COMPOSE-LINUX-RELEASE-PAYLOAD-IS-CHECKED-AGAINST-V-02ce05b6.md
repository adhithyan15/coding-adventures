### Added — the Compose Linux release payload is checked against v0.4.0 state

`release-engram.yml` now launches the Compose Desktop Linux payload against
the `fixtures/release-upgrade-v0.4.0.json` fixture. This is the first native
Engram bundle the release lane runs on old state. It uses the shared gate
(`code/specs/mosaic-release-state-gate-v1.md`, step 3 of "How an app adopts
the gate").

The new step extracts the zip that was just archived and launches it from `/`
on a virtual display (`run-under-xvfb.sh`). Neither `MOSAIC_APP_LIBRARY` nor
`MOSAIC_APP_STATE_PATH` is set, so the app can only use the runtime inside the
payload, and only the state path a user's machine would use:

```
$XDG_DATA_HOME/engram-app/mosaic-state.v1.json
```

That path comes from the generated Compose host's `stateFile()`. Its
directory is the package name, which the emitter passes in as the application
id. `XDG_DATA_HOME` only moves the root to a scratch directory. A local macOS
build of the same distribution confirmed the code path: its `apple` branch
wrote `<home>/Library/Application Support/engram-app/mosaic-state.v1.json`,
re-saved the v1 fixture as v3, and moved unreadable bytes to `.corrupt`.

The step has two halves:

- Upgrade. `materialize` writes v0.4.0's state, the app runs on it for 15
  seconds, and `verify` requires the deck and card to survive with no
  `.corrupt` file beside them.
- Recovery. The state is replaced with unreadable bytes and the app runs
  again. `verify-quarantine` then requires those bytes at
  `mosaic-state.v1.json.corrupt`, unchanged, and no longer at the live path.

A launch must still be running when `timeout` fires (exit 124). Any other
exit fails the step and prints its log. That includes the app refusing to
start because the Rust runtime did not load.

`test_engram_release.py` pins the step's paths, its order, and its environment.
The lane's `paths:` filter now also covers `run-under-xvfb.sh` and the
generated Compose host template, because the step depends on both.

The macOS and Windows Compose payloads, and the Flutter, Qt, SwiftUI and
XAML payloads, are not yet launched against the fixture.
