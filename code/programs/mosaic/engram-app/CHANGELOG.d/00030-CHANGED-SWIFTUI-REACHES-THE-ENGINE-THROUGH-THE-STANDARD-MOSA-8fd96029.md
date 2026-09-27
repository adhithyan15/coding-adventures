### Changed — SwiftUI reaches the engine through the standard Mosaic runtime

The second backend to make this migration, after Qt (#13728). Engram's SwiftUI
project no longer overrides the generated `MosaicHost`: props, events, snapshot
and restore go through `engram-mosaic-app` and the standard binding, and the
only SwiftUI-specific file left is `host/swiftui/EngramEffects.swift`, which
answers the Anki import and export effects with `NSOpenPanel` and `NSSavePanel`.

It emits with **`nativeComplete: true`, zero degradations, and an empty
`replacedGeneratedFiles`**, and the emitted project builds — the same
definition-of-done UI47 §5.5.5 sets for Qt.

#### Deferred, not inline — where SwiftUI parts company with Qt

The Qt handler answers effects in place under `Qt::DirectConnection`, and
mirroring that here would have been wrong twice over. `settleEffects` is not
guaranteed to run on the main thread and `runModal()` off-main is invalid; and
the host's lock is **held across the handler call**, so a modal panel run inline
would block `applyProps()` — which SwiftUI calls every frame — for as long as
the dialog stayed open. The documented escape, `DispatchQueue.main.sync`, is
precisely the wedge the host warns against.

So each dialog takes ownership with `deferEffect`, which keeps the effect out of
the fail sweep, and answers from the main thread when the person is done. Qt is
the outlier here, not the template: SwiftUI, Compose, Flutter and XAML all hold
a lock across the handler and all expose `deferEffect` for exactly this case.

That inverts the risk, so the code is shaped for it. A deferred effect has left
the fail sweep, so a path that forgets to answer no longer degrades to "failed"
— it wedges the app permanently, because the runtime gates snapshot and restore
on nothing being pending. The dialog functions therefore *return* an outcome
rather than answering, so every path funnels to exactly one `completeEffect`.

#### Twenty-six substring assertions retired with the file

They read the `engram-capi` binding — `eg_engram_app_props`, `hydrateSession`,
`ENGRAM_SNAPSHOT_PATH` and the rest. None of that ships for SwiftUI now, so
they were testing text nothing builds, exactly as Qt's sixteen were.

What replaces them is narrower because the file is. They cover all three
outcomes (a handler that only ever answers `ok` leaves a cancelled dialog
looking like a hang), the deferral pair, the weak capture, and the two checks
below. The manifest test also asserts SwiftUI declares **no** `[host_assets]`
override — asserted absent rather than merely deleted, for the same reason the
CI lane pins `replacedGeneratedFiles` to `[]`.

#### Two checks that came from the Qt handler's history

**Absolute regex anchors.** The extension filter uses `\A`/`\z`, not `^`/`$`.
ICU's `$` concedes a trailing line terminator — LF, CRLF, CR, U+2028, U+2029 and
NEL — so `^...$` would accept `"apkg\n"`, and
`UTType(filenameExtension: "apkg\n")` does not return nil but a *dynamic* type
matching no file. The panel would open with a filter hiding everything and
report no reason. The Qt handler had this exact bug; it is fixed there too.

**A zip-signature check after base64 decoding.** The first version of this was
an is-it-empty check, and its comment was wrong about why it was needed —
security review caught it. In strict mode the only input decoding to zero bytes
is `""`, which the preceding guard already rejects, so that check was redundant.

The case that actually slips through is padding-only input: `"===="` decodes
*successfully* to a single zero byte (measured), clearing both a nil check and
an empty check, and would be written out as a real `.apkg` reported `ok` — the
person then hands Anki a file it cannot open, with nothing pointing back here.
That is the same argument the strict-decode guard already makes for itself, and
the same silent-success shape as Qt's `QFile` flush-on-destruction case.

So the check is now `PK\x03\x04`, which an `.apkg` always starts with: cheap,
unambiguous, and it catches every degenerate payload rather than the one shape
that happened to be enumerated.

#### A symlinked package is readable again

`attributesOfItem` is `lstat`-based — it reports a symlink as a symlink and does
not follow it — while `Data(contentsOf:)` does follow. So the first version
inspected one file and read another, and rejected a symlink pointing at a
perfectly good `.apkg` with "that is not a regular file", which the retired host
read without complaint. The path is now resolved once and used for both, which
removes the mismatch and restores the behaviour. It does not weaken the check:
after resolution the stat describes the target, so a link pointing at a fifo
still reports a fifo and is still refused.

#### The release script had to move with it

`scripts/build-native.sh` built the SwiftUI payload for the old architecture and
broke the moment the override came off — caught by CI on this branch, not by
reasoning. Three things were wrong, and all three were verifying a shape that no
longer exists:

1. **It wired a `CEngram` system library.** The block built `engram-capi` as a
   static archive, wrote a module map, and patched the emitted `Package.swift`
   to link it, because `MosaicHost.swift` opened with `import CEngram`. Its own
   comment said to delete it rather than generalise it "once #13728 moves the
   adapters onto the standard runtime" — so this is that deletion.
2. **It asserted on linked `_eg_` symbols.** After the migration a *correct*
   build has zero of them, so the check failed the exact configuration it
   existed to protect. It now verifies the standard runtime landed in the
   bundle, is byte-identical to the library just built, and exports
   `mosaic_app_*` — the same question one layer out.
3. **The `.app` shipped without an engine.** Bundling copied only the
   executable, which was sufficient when `engram-capi` was statically linked
   *into* it. The runtime is now a resource, so `App_App.bundle` has to come
   too — precisely the "second chance to lose the engine" the assertion beside
   it warns about, which the Compose backend once shipped for real.

   It goes at the **`.app` root**, not in `Contents/Resources`, and the first
   version of this put it in the wrong place. SwiftPM's generated accessor
   resolves `Bundle.main.bundleURL/App_App.bundle` — which for a packaged app
   is `Engram.app` itself — and otherwise falls back to an *absolute
   build-machine path* baked in at compile time. So the wrong placement runs
   perfectly on the machine that built it and fatal-errors everywhere else
   (`could not load resource bundle`, SIGTRAP, exit 133). Security review
   caught it by building both layouts and running them.

   Placing it correctly also removes that build-machine path from the shipped
   binary's startup search, rather than leaving a released app that `dlopen`s
   from a directory which happened to exist on a CI runner.

4. **The release archiver had its own copy of the same stale gate.**
   `engram_release.py` independently scans the packaged executable for defined
   `eg_*` symbols, so the publish step would have failed even with
   `build-native.sh` fixed. It now parses the *bundled runtime* for
   `mosaic_app_*` instead — same question, same deliberate
   parse-the-Mach-O-rather-than-shell-out-to-`nm` method, different file.

The emit step also now passes `--profile native-complete --runtime-library`,
without which the generated host falls back to a reflection bridge that is not
there.

#### Verification

Built locally end to end, not only asserted over text: the emitted project
compiles with `swift build`, `mosaic-degradations.json` reports
`nativeComplete: true` with no degradations and no replaced files, and the
install lands at `App.swift:294`, immediately after the bridge assignment at
`:292` — which is the `loadRequired(libraryPath:)` form the bundled-runtime
rewrite produces, and the reason the emitter anchors on `self.bridge = ` rather
than the whole call.

The release script's output was launched, not merely built — and launched in a
way that could actually fail. The first attempt ran the `.app` in place, which
proves nothing here: the resource accessor's baked-in `.build` fallback still
resolved, so a bundle in the wrong place ran anyway.

The check that means something is to copy the `.app` elsewhere and delete the
build directory first. Done both ways: the shipped layout runs for five seconds
with no runtime error, and the layout this initially produced dies immediately
with `could not load resource bundle` and exit 133. Both assertions were
tightened from a depth-agnostic `find` to the exact path, because the glob
accepted the crashing layout — the "assertion that accepts the failure it exists
to catch" shape this script argues against elsewhere and had reintroduced here.

A new CI step runs the same sequence on macOS and asserts the install sits
inside `MosaicHostState`, after the bridge assignment. "Inside" is checked by
finding the nearest type declaration *above* the install — the first version
compared indices against the class declaration instead, which review pointed out
was near-vacuous, since the class is declared once far up the file and the
ordering holds even when the call lands in a later type. Demonstrated rather
than assumed: splicing the install into `MosaicHostValue` passes the original
assertion and fails the current one.
That step is load-bearing rather than belt-and-braces: SwiftPM compiles every
file under `Sources/App`, so a handler that is copied but never installed still
compiles, links and ships, with no missing symbol and no diagnostic — the first
symptom would be an `Await` going unanswered at runtime and taking the session's
persistence with it.

