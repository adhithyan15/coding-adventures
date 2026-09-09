# Changelog

## Unreleased

### Added -- the Flutter host answers effects (UI47 §5.4 step 4)

The fourth of five host templates, after Qt, SwiftUI and Compose. The host
gains `effectHandler`, `completeEffect`, `deferEffect` and the same bounded
settle loop; the emitted protocol version moves to `EFFECT_PROTOCOL_VERSION`.

Two things are genuinely different here rather than ported:

- **A Dart isolate is single-threaded.** The other three hosts hold a lock
  across the handler and have to warn against blocking, and Qt and SwiftUI both
  need a way to marshal a deferred answer back. None of that applies: an answer
  cannot arrive concurrently with a settle, only on a later turn of the event
  loop. So there is no lock, no deadlock to document, and nothing to marshal --
  and the acceptance's deferred case answers from a later event-loop turn
  rather than from another thread.
- **Two FFI paths, only one of which can be lenient.** The dynamic constructor
  resolves the seventh symbol through `lookupFunction`, which throws when it is
  absent, so it is wrapped and the slot is nullable -- a protocol-1 runtime
  still loads and only fails if an effect actually arrives. The bundled
  constructor uses `@Native`, which binds against the runtime linked into the
  process; a bundled protocol-1 runtime is a generation-time mismatch rather
  than something to recover from, which is the position the other six symbols
  were already in.

`completeEffect` needs two encoded inputs in one call, which no existing helper
covered, so `_invokeInputs` joins `_invokeInput`. Its allocations are made
inside the `try` and freed only if obtained: allocating first and entering the
`try` afterwards leaks everything already obtained if a later `calloc` throws,
and five allocations make that window widest here.

`_invokeInput`, the pre-existing single-input helper, gets the same treatment.
It had the original shape, and its first allocation is the payload buffer --
the arbitrarily large one of the three. Fixing the new helper and leaving its
twin leaking beside it would have been the wrong half of the job.

`_effectId` refuses a non-finite id. `double.infinity` is the one value that
satisfies the integrality test and still cannot be converted -- infinity equals
its own `roundToDouble()`, and `toInt()` then throws `UnsupportedError` -- and
`jsonDecode` produces it from `1e999` without complaint. A throw there escapes
the round loop, `_settleEffects` and `dispatch`, leaving every id already added
to `_awaiting` in that round with nothing to discharge it, and skipping the
warning that would have said so. `_failOutstanding`, the last-ditch clearing
path, calls it too. Qt and SwiftUI both guard finiteness explicitly and this
port had dropped it; NaN was already refused, because NaN compares unequal to
itself. Found by the security review. Reaching it needs a substituted or
corrupt library, since `EffectId` is a `u64` and serde_json emits integers, so
it is **not** exercised by the acceptance.

`_withPersistenceWarning` prefers the sticky effect warning over
`_persistenceWarning`, the fix Compose needed after review: the effect warning
means persistence is off for the rest of the process, while the other is
cleared by the next successful write, so reading only the latter would announce
that saving recovered while the runtime still refuses to snapshot. Written
correctly here from the start rather than found afterwards.

### Added -- an execution acceptance for the Flutter host

`tests/flutter_effect_completion.rs` emits the host into a temporary Dart
package, resolves `ffi` from the local pub cache with `dart pub get --offline`
so the test never depends on pub.dev, and runs it against the conformance
runtime -- one process per scenario, each with its own state file, because the
host reads `MOSAIC_APP_STATE_PATH` once at load.

Eight scenarios, matching the Compose set: an unanswered await, an answered
one, a fully-answered chaining batch, a partly-answered batch, a throwing
handler, an unconvertible result, a runaway chain, and defer-then-answer-later.
It skips when `dart` is absent or the cache cannot resolve `ffi`.

Mutation-tested: removing the handler-throw guard fails the `throwing` case
with the exception escaping `_settleEffects` into `dispatch`, and making the
round-exhaustion path give up quietly fails "a runaway chain is reported rather
than abandoned quietly".

### Added -- the Compose host answers effects (UI47 §5.4 step 4)

The third of five host templates, after Qt and SwiftUI. `MosaicRuntimeHost`
gains `effectHandler`, `completeEffect(id, result)`, `deferEffect(id)` and the
same bounded settle loop; the JNA interface gains
`mosaic_app_complete_effect`, and the emitted protocol version moves to
`EFFECT_PROTOCOL_VERSION` alongside Qt and SwiftUI.

Unlike SwiftUI, the deferred answer is **not** hopped to a particular thread.
SwiftUI requires state mutation on the main thread; Compose writes
`mutableStateOf` through the snapshot system, which accepts writes from any
thread, so a hop here would impose a rule Compose does not have.

Three defects this found, none of which the crate's text assertions could see:

- **Three missing `kotlinx.serialization` imports.** The emitted host did not
  compile at all. Every existing test asserts on the *text* of the emission and
  passed; the first `kotlinc` invocation failed. Hence the new acceptance below.
- **A handler that throws wedged persistence permanently.** The handler runs
  inside the settle loop, so an escaping exception left the id in `awaiting`
  with nothing left to discharge it -- and the runtime refuses to `snapshot` or
  `restore` while anything is pending. It is not an exotic path:
  `toJsonElement` throws on any value it has no case for, which is what a
  handler returning the `File` a dialog gave it does on its first run. A
  throwing handler is now treated as one that did not answer, so the sweep
  still fails the effect, and the app is told which handler failed and why
  rather than being handed a bare "no host handler answered".
- **Malformed effect entries threw rather than being reported.**
  `JsonElement.jsonObject` and `.jsonArray` throw on a wrong-typed element, and
  two of the three call sites were in `failOutstanding` -- the recovery path,
  where a throw aborts the very sweep that prevents the wedge. Parsing is now
  total, matching Qt and SwiftUI, which already were. This path is defensive:
  the runtime only ever emits well-formed effects, so it is **not** exercised
  by the acceptance below.

### Fixed -- the Compose host discarded its own "state cannot be saved" warning

`effectWarning` was written and never read. It is set in exactly the two places
where the host has lost the ability to persist for the rest of the process --
an effect arrived with an id nothing can answer, or effects were still
outstanding after the drain gave up -- and both messages say so in as many
words. But `withPersistenceWarning` consulted only `persistenceWarning`, so the
message went to a dead field: the user's data quietly stopped being durable
with no indication in the props, the UI, or on stderr.

A port omission rather than a decision -- the SwiftUI host it was derived from
passes `effectWarning ?? persistenceWarning` at every settle site. The effect
warning wins and is sticky, because `persistenceWarning` is cleared by the next
successful write, and announcing that saving recovered while the runtime is
still refusing to snapshot would be worse than saying nothing. It now also
reaches stderr, like every other persistence failure.

Neither condition is reachable through the conformance app, so this is fixed by
inspection against SwiftUI rather than pinned by the acceptance.

### Fixed -- `restore` returned the update that arrived, not the one it stored

Settling answers effects, and answers move the app, so the raw update's props
are the ones from before that happened and its `effects` list names effects
already discharged. A caller rendering the return value would show state that
`props()` disagrees with. Qt and SwiftUI both return the settled update; this
host returned the raw one, and additionally skipped the persistence warning
that `handleEvent` applies. Both now match.

Not exercised by the acceptance: the conformance app's `restore` mints no
effects, so the divergence is unreachable through that fixture. It is a
consistency fix against the two shipped hosts, verified by compilation only.

### Added -- an execution acceptance for the Compose host

`tests/compose_effect_completion.rs` emits the host, compiles it with `kotlinc`
against the real JNA and kotlinx-serialization jars, and runs it against the
conformance runtime -- one JVM per scenario, each with its own state file,
because the host reads `MOSAIC_APP_STATE_PATH` once at load and the JVM cannot
change its own environment. Seven scenarios: an unanswered await, an answered
one, a fully-answered chaining batch, a partly-answered batch, a throwing
handler, an unconvertible result, a runaway chain, and
defer-then-answer-from-another-thread.

The runaway case pins the 64-round settle bound, which is what stops a handler
that answers every effect by minting another from spinning inside a
`@Synchronized` method while holding the monitor. The bound has to both stop
**and** report: giving up quietly would leave the app looking settled while the
runtime still waits.

`consume` now bounds the native length below as well as above. `MosaicSizeT` is
an unsigned `IntegerType`, so a 64-bit `size_t` with the high bit set arrives as
a negative `Long` and sailed past the `<=` test into `getByteArray` with a
negative count. JNA rejects that, so this was never an out-of-bounds read -- but
the guard read as though it checked, and did not.

It skips when `kotlinc`, `java` or the jars are absent, with environment
overrides (`MOSAIC_JNA_JAR`, `MOSAIC_KOTLINX_JSON_JAR`,
`MOSAIC_KOTLINX_CORE_JAR`, `MOSAIC_KOTLIN_STDLIB_JAR`) to point it at them.
The stdlib is resolved separately because `kotlinc` supplies it at compile time
and `java` does not at run time -- a host that compiles cleanly still dies with
`NoClassDefFoundError: kotlin/Result` without it.

### Added -- an `Await` effect can be answered later (#14720)

Both hosts called the effect handler synchronously and failed anything still
unanswered when it returned. A handler therefore had exactly one option: answer
inline, on the settle thread, without blocking. Answering later was rejected as
already completed; blocking on another thread deadlocked against the lock.

That ruled out an asynchronous file dialog -- the motivating case for `await`
effects, and the flow Engram's Anki import needs. The shape could not express
the thing it was built for.

`deferEffect(id)` is the third outcome. A handler that takes ownership leaves
the effect pending instead of having it failed, and answers whenever the work
finishes, from any thread.

Two consequences worth stating:

- **A deferred answer reaches the UI on its own.** It is the return value of no
  call the UI made -- it arrives whenever the dialog closed -- so Qt gained an
  `updated(QVariantMap)` signal, and SwiftUI routes it through the
  `propsChangedHandler` it already had, **on the main thread and outside the
  lock**. Without that the app would advance while the screen kept showing the
  state from before.
- **Answering from another thread has a route on both hosts.** Qt's
  `completeEffect` touches unguarded members and refuses an off-thread call, so
  `answerDeferredEffect` queues onto the host's thread; without it the async
  flow this feature exists for had no working Qt path at all. SwiftUI's lock
  serialises, and the notification hops to main.
- **`snapshot` and `restore` stay refused while an effect is deferred.** That is
  correct rather than a defect: a half-answered import is not a state worth
  restoring. It does mean a handler that defers owes an answer -- abandoning one
  leaves the app waiting for good.

Blocking on another thread from inside the handler is still wrong, and both
templates now say so and say to defer instead -- the Qt header included, which
in the first version of this change still said the feature was impossible.

`deferEffect` refuses an id the runtime is not awaiting. Ids are sequential and
the call is reachable from QML, so an off-by-one would otherwise switch the fail
sweep off for an effect nothing will ever answer -- wedging `snapshot` and
`restore` for the life of the process, which is the failure the sweep exists to
prevent.

Settled before Compose, Flutter and XAML copy the shape, which was the point of
doing it now rather than after.

### Added -- the SwiftUI host answers effects (UI47 §5.4 step 4)

The second of five host templates, after Qt. `MosaicRuntimeHost` gains
`effectHandler`, `completeEffect(_:_:)` and a settle loop; the C shim resolves
`mosaic_app_complete_effect` leniently and reports its absence as a distinct
status, so "this runtime cannot complete effects" is not mistaken for "the
completion failed". The host declares protocol 2, and the assertion pinning the
version pins the capability beside it.

Written with the failure modes the Qt template surfaced already in hand --
accumulate at the write site, discharge on both runaway guards, bound the
nesting -- so those did not have to be rediscovered. Two things differed, and
both are recorded because the difference is the interesting part:

- **`NSRecursiveLock` makes re-entrant completion safe** where Qt needed care:
  a handler answering from inside the callback re-enters the lock on the same
  thread. Answering from another thread is not useful -- that call blocks until
  the settle finishes, by which point the effect has been failed as unanswered.
- **No effect-id validation is needed.** Qt's `completeEffect` is `Q_INVOKABLE`
  and takes a `QVariant`, so QML could hand it `3.5` and `toULongLong` would
  truncate to `4`, answering a *different* outstanding effect. Swift's signature
  is `UInt64`; the same mistake does not compile.

Bugs found by running it, none of which a text-level assertion could see:

- The re-entrant branch was gated on `latestAnswer != nil`, but that starts
  `nil` each frame and is only set *by* that branch -- so it was unreachable
  and every handler's answer was discarded. A direct port of Qt's
  pointer-non-null check into Swift, where the value itself is the Optional.
- **Answering with a `URL` aborted the process.** `JSONSerialization` raises an
  ObjC `NSInvalidArgumentException` for a `URL`, `Date`, `Data`, or a
  non-finite `Double`. Swift cannot catch that, so the surrounding `do/catch`
  was inert and the app terminated with the lock held -- on the first thing a
  real file-dialog handler would write. Now refused with a message.
- A JSON `true` bridges to `NSNumber` and coerced to id **1**, answering effect
  1. Booleans are refused.
- An `await` whose id could not be read was skipped in silence, leaving it
  pending forever. It is now counted and surfaced.
- Either runaway guard returned `["props": [:]]`, blanking every binding in the
  UI until the next event. The error is now attached to the last known good
  update instead of replacing it.
- The warning about abandoned effects was written to `persistenceWarning`,
  which `persistSnapshot()` clears on every successful write -- so it was
  overwritten microseconds later and never reached anyone. It has its own
  field now, because an abandoned effect is a standing condition rather than a
  write failure.

### Known gap -- an `Await` cannot be answered asynchronously (#14720)

Both this host and Qt's call the handler synchronously and fail any `await` not
answered by the time the call returns. Answering later is rejected as already
completed; blocking on another thread deadlocks against the lock. That rules out
an asynchronous file dialog, which is the motivating case for `await` effects --
so **step 5 cannot be completed on this shape**. Both templates now state the
constraint rather than implying a handler may answer whenever it likes.

`tests/swift_effect_completion.rs` emits the host, compiles it with `clang` +
`swiftc`, links the real conformance runtime and runs it -- eleven checks
including a batch where the handler answers both effects with chaining, which is
the shape that caught the equivalent Qt bug.

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
effects they were handed before refusing, and drain what those answers mint for
up to 8 further rounds. That bound can be outrun: an app that keeps minting
replacements past it (measured at 73 chained failures with no handler, or 9 when
the nesting guard fired) leaves effects pending, and persistence is then off for
the rest of the session. It is a fallback of a fallback rather than something
worth an unbounded loop, so the host *states* that terminal condition in its
persistence warning instead of leaving it to be inferred from a later snapshot
quietly failing. Giving up with awaited effects still pending swapped a crash for an app
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
