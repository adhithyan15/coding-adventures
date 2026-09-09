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
now pins the capability beside it.

Scope of that bump, stated accurately: a v2 host running a v1 **app** is fine --
no `await` is ever emitted and the completion path goes unused, which the
existing `venture-browser` Qt acceptance (a v1 app) confirms. A v2 host against
an older **runtime** that only accepts protocol 1 is not: it is rejected at
`mosaic_app_create`. Every runtime in this repository accepts both, so nothing
here breaks, but a package pairing a newly emitted host with a stale
`libmosaic_app` would.

Five bugs found only by running it, three of them by the security review after
the first version of this change. None would have been visible to any assertion
on the emitted text:

- **Persistence ran before settling.** The runtime refuses to snapshot while an
  effect is outstanding -- correctly, since a half-answered effect is not a
  state worth restoring -- so every effect produced a spurious "could not
  persist Mosaic state" warning. Settling now happens first.
- **A handler answering during the emit left the caller with a stale update.**
  `completeEffect` called re-entrantly now hands its update back to the settle
  loop already running instead of starting a second one, and the loop adopts
  it.
- **A partly-answered batch dropped effects, twice over.** First, adoption and
  the fail loop were alternatives, so a batch where a handler answered one
  effect and ignored another left the ignored one neither answered nor cleared.
  Then, once that was fixed, the fail loop still *overwrote* the adopted
  update -- and an `Update` carries only the effects produced by the call that
  returned it, so a completion that produced a further effect (the "import
  needing a second dialog" this code describes) had it dropped instead.
  Effects now accumulate **at the write site** rather than being read back from
  a slot: `completeEffect` appends to the round's list, so N answers in one
  round contribute N effect lists. Accumulating at the read site was a third
  version of the same bug -- it survived one answer per round and dropped every
  earlier answer's effects when a handler answered two.

  Both are permanent: the runtime gates `snapshot` **and** `restore` on nothing
  being pending, so one dropped effect disables persistence for the rest of the
  process. The conformance fixture gained a two-effect batch event, because no
  single-effect test can reach either.
- **A handler calling back into the host recursed until the stack gave out.**
  The 64-round bound bounds iterations within a frame, not frames; a handler
  calling `handleEvent` rather than `completeEffect` re-entered one level
  deeper, and the review reproduced a SIGSEGV at roughly 1600 levels. Nesting is
  now bounded at 8 with a message.
- **`toULongLong` was not validation.** It accepts `-1` (wrapping to 2^64-1),
  `3.5` (truncating to **4**), and `1e30` (saturating) -- and `completeEffect`
  is `Q_INVOKABLE`, so QML, where every number is a double, is the expected
  caller. An id of 3.5 would have answered a *different* outstanding effect with
  the wrong result. Ids are now range- and integrality-checked before
  conversion.

A handler may also delete the host mid-emit, which the review reproduced as a
use-after-free under ASan. Liveness is carried across the `settleEffects` call
boundary -- the first attempt gated only *inside* it, and every caller then went
on to touch members of a freed object, which ASan reproduced again. The depth
guard no longer writes through freed memory, and the header says to use
`deleteLater()`.

Thread affinity is a **refusal**, not only an assert: `Q_ASSERT_X` compiles to
nothing under `QT_NO_DEBUG`, which is what a release build of a generated app
defines, so the check was absent in every shipped app. It matters more than a
stale result now that the re-entrancy slots are pointers into `settleEffects`'s
stack frame -- a completion arriving on another thread would write into a live
frame belonging to a different one.

Both runaway guards -- the nesting bound and the round bound -- answer the
effects they were handed before refusing, and drain what those answers mint in
turn. Giving up with awaited effects still pending swapped a crash for an app
that can never snapshot again. The error path out of the fail loop discharges
what the round accumulated for the same reason.

The thread refusal covers `handleEvent` and `restore` as well as
`completeEffect`. Those two are `Q_INVOKABLE` and both install the frame
pointers, so guarding only the completion left the hazard the guard exists for
reachable through the other entry points.

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
