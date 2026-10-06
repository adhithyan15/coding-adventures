# task-mosaic-app

`task-mosaic-app` is the standard Mosaic application adapter for TaskApp. It owns
portable presentation state, delegates domain mutations and projections to the pure
`task-core` engine, and exports the fixed `mosaic-app-capi` ABI consumed by generated
native hosts.

The adapter is deliberately not a second task engine. `task-core` remains the single
trust boundary for task, project, scheduling, label, and note invariants. This crate
only maps TaskApp's MIL slots and semantic events to that typed API, giving every
generated backend the same complete initial prop snapshot, event behavior, and
versioned persistence format.

Build the native runtime library with:

```bash
cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app
```

Pass the resulting platform library to `mosaic-compile --runtime-library` when
emitting `code/programs/mosaic/task-app`. Use the `native-complete` profile on
all five supported native backends; TaskApp currently emits with zero capability
degradations on Qt, Flutter, Compose Desktop, SwiftUI, and XAML. Existing style
drops are reported separately and ratcheted per property by the shared TaskApp
contract: fixes may reduce them, while a new drop or an increased occurrence
count fails CI (#16182) on the route to #12022's zero-drop hard fail.

The `conformance/{qt,flutter,compose,swiftui,xaml}` fixtures are task-specific
functional gates. CI combines each one with the complete generated TaskApp and its
standard binding, then drives create, scheduling, complete/reopen, delete,
invalid-input atomicity, and persisted restart restoration against the real Rust
adapter. `code/scripts/taskapp_native_control_contract.py` separately rejects
generated sources with inert controls, sample fallbacks, missing runtime wiring,
or native style-degradation growth.

The phones have their own UI tests (UI89 §4.4), run on the CI emulator and
simulator against the installed app.
`conformance/compose-android/TrestleAndroidUiTest.kt` is compiled into the
Android test APK and run by `code/scripts/mosaic-android-ui-test.sh`.
`conformance/swiftui-ios/TrestleUiTests.swift` reaches the iOS project through
`mosaic-compile pkg --ios-ui-test` and runs on an iPad simulator. Each one goes
through the generated controls in two cold launches. The first adds a task,
completes it, reopens it, deletes it, and then adds the task that must
survive. The second finds that task restored and deletes it.

The crate also consumes TaskApp's shared
`fixtures/presentation-contract-v1.json`. Its checkpoint test and the web host's
real-WASM test assert the same canonical task/project state and user-visible core
slots after each lifecycle step; intentional theme-storage and locale-copy
differences are documented in `code/specs/task-app-presentation-contract-v1.md`.
