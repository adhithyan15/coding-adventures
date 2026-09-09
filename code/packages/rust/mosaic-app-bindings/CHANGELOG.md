# Changelog

## Unreleased

### Added -- the Qt host answers effects (UI47 §5.4 step 4)

`MosaicHost` resolves `mosaic_app_complete_effect`, drains `update.effects`
after every create / dispatch / restore / completion, and declares **protocol
2**.

Before this, `Effect` rode the wire and was read by no native host at all: an
`await` was dropped and the app waited forever, with nothing anywhere reporting
it. The Qt host is the first to close that.

- `effectRequested(id, kind, payload, delivery)` is emitted per effect;
  a handler connected directly may call `completeEffect(id, result)` from
  inside the emit to answer it.
- An `await` **no handler answered** is completed as
  `failed: no host handler answered effect N` rather than dropped. A missing
  handler becomes a visible error instead of a hang.
- `notify` needs no answer and is never failed.
- Completion can produce further effects -- an import needing a second dialog
  is an ordinary flow -- so settling drains rather than sweeping once, with a
  64-round runaway guard that reports rather than looping.

Protocol 2 is declared because the host now *implements* completion. Declaring
it without that is the harmful direction, so the assertion that pins the version
now pins the capability beside it. Per UI47 §5.3 the bump is safe in both
directions, and the existing Qt app acceptance (`venture-browser`, a v1 app)
still passes: a v2 host running a v1 app simply never sees an `await`.

Two bugs found only by running it, which no amount of asserting on the emitted
text would have surfaced:

- **Persistence ran before settling.** The runtime refuses to snapshot while an
  effect is outstanding -- correctly, since a half-answered effect is not a
  state worth restoring -- so every effect produced a spurious "could not
  persist Mosaic state" warning. Settling now happens first.
- **A handler answering during the emit left the caller with a stale update.**
  `completeEffect` called re-entrantly now hands its update back to the settle
  loop already running instead of starting a second one, and the loop adopts
  it.

### Added -- an execution acceptance for the emitted Qt host

`tests/qt_effect_completion.rs` emits the host, compiles it against Qt Core with
CMake, links the real conformance runtime, and runs it. The other tests in this
crate assert on the *text* of the emitted host, which cannot establish that it
compiles, let alone that it behaves.

It skips only when CMake cannot find Qt6 **Core** -- probed by configuring a
throwaway project, not by looking for `qmake` on `PATH`. A Qt tool being present
does not mean CMake can find the component this build needs, and conflating them
turns "unavailable" into a red test rather than a skip.

- When an application's prop envelope declares `storage-warning`, propagate the
  native host's existing persistence warning into that prop as well as the
  diagnostic top-level field. Generated UI can now show corrupt-state or write
  failures persistently instead of leaving them only in logs/status chrome.
- Preserve JSON strings as Kotlin strings in the Compose/JNA host even when
  their contents look numeric or boolean, while keeping actual JSON numbers
  and booleans mapped to native Kotlin primitives.
- Accept both standard `{name, payload}` events and generated flat
  `{event, ...payload}` envelopes in the Compose/JNA host, matching Flutter and
  allowing emitted Compose controls to dispatch into the Rust engine.
- Persist opaque Mosaic snapshots in every application-scoped Compose,
  SwiftUI, XAML, Flutter, and Qt host. Generated applications restore before
  first render, atomically replace state after successful dispatches, quarantine
  corrupt or incompatible state, and surface a recoverable warning. Native CI
  launches each binding twice to prove process-restart restoration.
- Resolve Flutter's selected Rust engine through Dart's bundled code-asset
  contract after the explicit environment override, so generated native apps
  need no platform-specific loader path or global library installation.
- Resolve SwiftUI's selected Rust engine from its SwiftPM `Runtime` resource
  bundle after the explicit environment override, so generated apps need no
  global dylib installation.
- Resolve XAML's conventional `mosaic_app.dll` from `AppContext.BaseDirectory`
  after the explicit environment override and before global lookup.
- Resolve Qt's conventional Mosaic application library beside the native
  executable after the explicit environment override and before global lookup.
- Resolve Compose's conventional Mosaic application library from the installed
  native-distribution resources after explicit property/environment overrides
  and before falling back to a global library name.
- Add direct required-runtime and required-prop APIs to the standard Qt binding.
  Strict shells now reject missing Rust libraries and incomplete prop envelopes,
  while consistently mapping MIL slot names onto generated QML properties.
- Add direct required-runtime APIs to the standard XAML binding. Strict WinUI
  shells now fail explicitly when Rust is unavailable, reject missing required
  props and invalid values, and revalidate props after every event.
- Add `MosaicRuntimeHost.loadRequired()` for strict SwiftUI shells that must
  fail explicitly when the Rust application runtime cannot be loaded.
- Add `MosaicHost.loadRequired()` for strict Flutter shells that must fail
  explicitly when the Rust application runtime cannot be loaded.
- Execute the generated Compose/JNA host against the shared Rust conformance
  library in Linux CI, covering startup, dispatch, snapshot/restore,
  notification, buffer ownership, and teardown on the JVM.
- Expose the generated Compose ABI carrier constructors to JNA reflection;
  Kotlin file-private `size_t` and structure classes compiled but failed when
  the standard host first attempted to load a real Rust library.
- Execute the generated Flutter/Dart FFI host against the shared Rust
  conformance library in Linux CI, covering startup, dispatch,
  snapshot/restore, notification, buffer ownership, and teardown.
- Execute the generated Qt/QML host against the shared Rust conformance library
  in headless Linux CI, covering startup, dispatch, snapshot/restore, buffer
  ownership, and teardown through the real ABI.
- Execute the generated SwiftUI binding and C loader against the shared Rust
  conformance dylib in macOS CI, covering startup, dispatch, snapshot/restore,
  prop-change notification, buffer ownership, and teardown through the real ABI.
- Make the XAML Rust-runtime conformance executable independent of WinUI desktop
  initialization after the complete generated TaskApp has compiled against the
  real Windows App SDK, and bound the CI execution with an explicit timeout.
- Execute the generated XAML binding against the shared Rust conformance DLL in
  Windows CI, covering startup, initial props, dispatch, revised props, Rust
  buffer release, and teardown through the real ABI.
- Make the hosted-runner WinUI launch smoke test tolerate transient runtime
  startup failures while printing Windows application events when every real
  launch attempt fails.
- Add the package-independent Qt/QML binding using Qt Core dynamic loading,
  JSON, variants, and QObject invocation.
- Add the package-independent Flutter/Dart FFI binding and preserve the public
  injectable `MosaicHost` contract for tests and specialized packages.
- Add the package-independent XAML/.NET binding using built-in native loading
  and JSON support.
- Add the package-independent SwiftUI/Foundation binding and C dynamic loader.
- Own Swift application startup, successful event sequencing, snapshots, Rust
  buffers, and teardown through the same fixed C ABI as Compose.

## 0.1.0

- Add the package-independent Compose/JNA binding for the Mosaic C ABI.
- Manage native app handles and Rust-owned output buffers without app glue.
- Generate startup and event envelopes with the shared protocol version.
- Decode complete runtime updates into the generated Compose host contract.
