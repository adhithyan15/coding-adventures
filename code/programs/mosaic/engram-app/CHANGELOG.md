# Changelog

## Unreleased

### Added — Engram can be rendered on Compose (#14828, #14799)

Engram is gated on SwiftUI and Qt in CI and has no Compose lane, so nothing
produced a picture of it on that backend. `conformance/compose/EngramScreenshots.kt`
and `scripts/render-compose.sh` now do, in one command.

The first render it produced is not flattering, and both defects were then
measured against the semantics tree rather than read off the image:

- **UI60 (#14828), confirmed.** All eleven deck-stat and collection chips draw
  the count on top of its label — `[0]` and `[Total]` both at
  `Rect.fromLTRB(49.0, 247.0, ...)`, identical origins. Compose gives `Box` the
  overlay behaviour UI29 assigns to `Stack`.
- **#14837, new.** The composition root is `1280 x 728` inside a `1280 x 900`
  window: 172px of the app is unpainted. Same class of defect as Trestle's
  in #14798.

Every semantics gate stayed green through both, because a node drawn on top of
another node is still present, still named, and still "displayed". The harness
makes the same narrow claim Trestle's does — that rendering *succeeds* and
produces a surface of the size asked for — and is explicitly not a pixel-diff
gate.

The script asserts `nativeComplete` before rendering, so a picture of a
degraded fallback cannot be mistaken for a picture of the product.


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

### Fixed — the Qt file-dialog extension check accepted a trailing newline

`fileFilter` decides whether a payload-supplied string "looks like an
extension" before it becomes a glob in the file dialog's filter. It used
`^\.?[A-Za-z0-9_-]{1,16}$` — and `QRegularExpression` is PCRE2, whose `$`
matches before a trailing newline unless `DollarEndOnlyOption` is set. So
`"apkg\n"` passed, and the check did not mean what its own comment said it
meant.

Measured against Qt 6 rather than assumed, because the answer is engine-specific
and does not transfer. On the build measured here PCRE2 conceded LF alone —
but that is a property of the build, not of the language: PCRE2's newline
convention is chosen at compile time, and a copy built with `ANYCRLF` or `ANY`
concedes `\r`, `\r\n`, NEL, LS and PS through `$` as well. So the hole was *at
least* LF-wide and possibly wider depending on whose PCRE2 Qt was linked
against. ICU answers the same question differently again (it concedes all of
them unconditionally), and Rust a third way — there `$` is end-of-haystack and
refuses every one, which is what the `[host_effects]` injection analysis turned
on.

Now `\z`, which is absolute: end of subject, no terminator concession on any
build or convention. Specifically `\z` and **not** `\Z` — PCRE2's `\Z` makes the
same concession `$` does, so it would have renamed the hole rather than closed
it.

`^` → `\A` in the same pattern fixes nothing today and is not pretending to:
`QRegularExpression` sets no `MultilineOption` by default, so `^` was already
start-of-subject. It is there so that turning that option on later cannot
reopen the other end.

Not exploitable: this application builds the payload, and nothing puts a newline
in an extension. It is worth fixing anyway because the failure it allowed is
**silent**. A glob of `*apkg\n` is refused nowhere downstream; it simply matches
no file, so the dialog opens showing nothing and reports no reason. A defensive
check whose comment overstates it is worse than no check, because the next
person reads the comment.

Verified by compiling the real `fileFilter` against Qt 6 and driving it: a
trailing LF now falls to the fallback filter, a good extension still survives
alongside a rejected one, and the previously-accepted cases are unchanged.

Scope, so the next reader does not over-read this: at the time of this fix Qt was
the only host with a shape check at all — Compose, Flutter and Electron normalise
instead, trimming and dot-prefixing whatever arrives — so there was no twin hole
to close alongside it.

*Superseded in part by the SwiftUI migration above*, which landed in the same
release: `EngramEffects.swift` now carries a shape check too, and it was written
with `\A`/`\z` from the start precisely because this fix had established that
`^`/`$` does not mean what it appears to. ICU concedes more terminators than
PCRE2 does, so the SwiftUI hole would have been the wider of the two.

The divergence that remains: Qt and SwiftUI now *reject* `".apkg\n"` where
Compose, Flutter and Electron *accept* it by trimming. Identical behaviour with
today's producers, which send static literals, but a real source of
host-specific filter differences if that ever stops being true.

### Changed — Qt reaches the engine through the standard Mosaic runtime (#13728)

Engram's Qt project no longer overrides the generated `MosaicHost`. Props,
events, snapshot and restore go through `engram-mosaic-app` and the standard
binding; the only Qt-specific file left is `host/qt/engram_effects.cpp`, which
answers the Anki import and export effects with `QFileDialog`.

The 654-line `host/qt/MosaicHost.h/.cpp` is retired. It bound to `engram-capi`
— a bespoke ABI of 44 `eg_*` symbols — and reimplemented the entire application
boundary: session lifecycle, prop camel-casing, snapshot persistence, JSON
marshalling. None of that was Engram-specific; all of it is what the standard
runtime already does.

**What this unlocks, and what it cost to get here.** Qt could not reach
`native-complete` while the override existed: that profile switches the
generated `main.cpp` to the strict standard-binding shape — `registerTypes`,
`requireRuntime`, `configureRequiredProps`, `attach` — and the override exposed
`props`/`handleEvent`/`ensureLoaded` instead, so the combination did not
compile at all. The CI lane said so in a comment and deliberately gated the
weaker configuration.

It now emits with **`nativeComplete: true`, zero degradations, and an empty
`replacedGeneratedFiles`**, and the emitted project builds. That is an epic
definition-of-done bullet, and UI47 §5.5.5's stated test of whether this
migration worked.

The path ran through the whole of UI47: `Effect` needed a completion path
(steps 1–3), all five hosts needed to answer one (step 4), Engram needed to
emit its Anki intents as `Await` effects (step 5), and a package needed a way
to supply the handler that connects them (§5.5). Only then could the override
come off.

**Qt only.** SwiftUI, Compose, Flutter, XAML and the two web backends still
ship their own `MosaicHost` through `[host_assets]`, unchanged — each still
reimplements the boundary against `engram-capi`, and `engram-capi` itself stays
for them and for the Anki entry points the standard app ABI does not model.

#### The CI lane's pinned assertion, updated

`replacedGeneratedFiles` was pinned to `["MosaicHost.cpp", "MosaicHost.h"]`
specifically so this migration could not be forgotten. It is now pinned to `[]`
— pinned rather than dropped, because a package that starts overriding the
generated host again has undone this and should fail here rather than pass
quietly. The lane also asserts zero degradations and greps the generated
`main.cpp` for the install call, which is the half no assertion over a source
file can reach.

#### Sixteen substring assertions retired with the file

Engram's own suite asserted that the Qt override contained `eg_engram_app_props`,
`QLibrary`, `mosaicPropName`, `hydrateSession` and a dozen more. None of that
ships for Qt now, so those assertions were testing text nothing builds — the
"substring assertions over emitted text" problem the epic named, in its purest
form.

What replaces them is narrower because the file is: the handler answers two
effects and does nothing else. The assertions cover all three outcomes, because
a handler that only ever answers `ok` leaves a cancelled dialog looking like a
hang, and they pin `Qt::DirectConnection` — a queued connection would return
before the dialog answers, the sweep would fail the effect as unanswered, and
the eventual answer would be rejected as already completed.

#### From the security review of the handler

**An exception could strand the effect and kill persistence for the process.**
The handler runs inside the host's `settleEffects`, and the `emit` there has no
handler around it: anything thrown unwinds past it, `failOutstanding` never
runs, the id stays in `awaiting_`, and the runtime then refuses every snapshot
and restore for the life of the process. That is the silent permanent failure
the whole sweep mechanism exists to prevent, reached by throwing rather than by
forgetting — and an exception leaving a directly-connected slot is undefined
behaviour in Qt 6 besides. Both handlers are now wrapped, and the reason is
answered as the effect's failure, which discharges it and keeps persistence
alive.

**The import read was unbounded.** `readAll` with no size check, then base64,
then UTF-16, then the JSON envelope, then the runtime's own copy — roughly
seven times the file's size in flight, with the application's cap sitting at the
far end of all of it and so unable to prevent any of it. A multi-gigabyte file
would OOM, and `readAll` on a fifo or character device never reaches EOF at all.

Now sized up before it is opened, against the 256 MiB the package layer will
accept on native targets, so a file that cannot import is refused before it is
read rather than after. The regular-file test is separate work rather than a
restatement, because `QFileInfo::size()` reports 0 for a fifo.

The generic WebAssembly handler for this same protocol already bounded its read
at 16 MiB. Qt was the outlier.

**Two inputs are sanitised that the retired binding used to sanitise.**
`suggestedName` is forced to a bare filename — `QDir::filePath` joins a relative
path happily, so a suggestion carrying separators would open the save dialog in
a different directory with only the basename visible — and extensions must look
like extensions before reaching the dialog filter, where `)` and `;;` are
structural and `*?[` are globs. Neither is reachable today: nothing sends
`suggestedName`, and both extension lists are literals in the engine. The old
binding scrubbed its deck-derived name anyway, so accepting either unchecked
would be losing a check rather than never having had one.

The CI lane's degradation assertion now checks the key is an array before its
length, since `null | length` is `0` in jq and a report that lost the key would
otherwise pass.


- **Fixed: XAML layout variants generated duplicate partial classes.** The
  touch layout now emits its own `EngramAppTouch` class and event union, so the
  WinUI project can compile both layouts without duplicate members (#14234).

- The deck list on the home screen shows each deck's due and new counts, so the
  screen answers "what should I study?" without selecting each deck in turn.
- `onSelectDeck` carries a row index; see the `mosaic-pkg-deck-stats` changelog
  for why.

- **Fixed: the web bundle only worked when served from a domain root.** The
  React host's `WASM_URL` was `"/engram_engine.wasm"` — root-absolute — while
  its sibling `engram-host.mjs` had always used the relative form. The `.ts`
  copy is the one the Vite bundle ships. Combined with Vite's default
  `base: "/"`, the published v0.3.0 bundle returned 200 for `index.html` and
  404 for both its script and its engine when served from any subdirectory,
  rendering a blank page that looks like a working deploy.

  Verified by serving the corrected bundle from `/deep/nested/engram/`: entry
  point, hashed chunk, and wasm all 200, with the engine's magic bytes intact.
  The `base` fix is in the Mosaic React emitter, since Engram is the first app
  to build through `--emit-project` and so the first to depend on the
  generated Vite config at all.

- Added `scripts/build-web.sh`, a cross-platform build for the web host:
  compiles the engine to wasm, emits the app as a complete React/Vite project,
  and installs the wasm runtime into it. `--build` also produces `dist/`;
  `--theme` selects a stylesheet.

  The runtime-install step is what makes the emitted project buildable at all.
  `src/engram-host.ts` imports `./engram-mosaic-host-wasm` and emission copies
  only the `.d.ts`, so without it the build fails with
  `Could not resolve "./engram-mosaic-host-wasm"`. That step previously existed
  only in `build-all.ps1` — PowerShell, and therefore unavailable on the Linux
  CI runner, which is why no lane could produce a web bundle.

  With `--build` the script also `cmp`s the wasm in `dist/` against the one it
  compiled. Vite copies `public/` into `dist/`, and a missing engine there is a
  runtime failure behind a successful build — the same shape of bug the install
  step exists to prevent, so it is checked rather than assumed.

- **Fixed: the generated Qt project never compiled.** `main.cpp` calls
  `MosaicHost::registerTypes()` and `mosaicHost.attach(root)` on whatever
  `MosaicHost` the project ships. Engram installs its own over the generated one
  through `[host_assets]`, and it declared neither:

  ```
  main.cpp:27: error: 'registerTypes' is not a member of 'MosaicHost'
  main.cpp:47: error: 'class MosaicHost' has no member named 'attach'
  ```

  Added as no-ops, matching Mosaic's generated host, which declares both and
  leaves both empty — they are extension points, not behaviour. This host reaches
  the engine over `engram-capi` and needs neither QML type registration nor a
  root-object handle.

  Nothing caught this because nothing ever compiled the output: `build-all.ps1`
  emits without building, and `tests/package_compiles.rs` asserts on emitted text.
  The new Qt CI gate builds it, which is how it surfaced.

- Preserved Engram Anki import/export host-side error details in Qt, SwiftUI,
  and Compose `hostResult` statuses so generated shells show actionable read,
  import, export, and write failures instead of generic status text.
- Added `EngramApp.touch.mll`, a touch / mobile layout variant of the app shell
  (UI30). The desktop shell's horizontal header — app title beside a Row of six
  nav buttons — overflows on a phone-width viewport, so the touch variant stacks
  the header and nav vertically (Row → Column) while keeping the interface
  (`EngramApp.mil`) and every component mount / slot binding byte-for-byte
  identical. `mosaic-package-artifact-builder`'s `discover_variants` auto-emits
  it, so every backend now produces both `EngramApp` (desktop) and
  `EngramApp.touch` artifacts. Verified across all nine backends
  (react/html/webcomponent/swiftui/qt/flutter/compose/xaml); the emitted React
  differs from desktop only in the two `flexDirection: row → column` containers.
- Added a light-theme stylesheet (`EngramApp.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).
- Wired the generated HTML, WebComponent, and React Engram web hosts to handle
  Anki import/export `hostIntent` payloads with browser file input/download
  helpers and the `engram-wasm` APKG byte API, surfacing `hostResult` errors
  when the current browser WASM build delegates package parsing to native hosts.
- Wired the generated Flutter Engram shell to handle Anki import/export
  `hostIntent` payloads with `file_selector` dialogs and the shared
  `engram-capi` APKG import/export functions.
- Wired the generated Compose Desktop Engram shell to handle Anki import/export
  `hostIntent` payloads with desktop file choosers and the shared `engram-capi`
  APKG import/export functions.
- Wired the generated Electron Engram shell to handle Anki import/export
  `hostIntent` payloads with native Electron dialogs, snapshot persistence,
  and a native `engram-host-cli` sidecar for APKG import/export.
- Wired the generated SwiftUI macOS Engram shell to handle Anki import/export
  `hostIntent` payloads with AppKit file panels and the shared `engram-capi`
  APKG import/export functions, with an explicit unsupported result on
  non-macOS SwiftUI targets until Mosaic has an async document-picker bridge.
- Wired the generated Qt Engram shell to handle Anki import/export
  `hostIntent` payloads with Qt file dialogs and the shared `engram-capi` APKG
  import/export functions.
- Wired the generated XAML Engram shell to handle Anki import/export
  `hostIntent` payloads with WinUI file pickers and the shared `engram-capi`
  APKG import/export functions, so package files round-trip through the native
  Rust core instead of stopping at status text.
- Persisted Engram Mosaic SwiftUI, Qt, XAML, Flutter, and Compose native host
  snapshots through `engram-capi`, matching the web/Electron snapshot behavior
  with an `ENGRAM_SNAPSHOT_PATH` override and a shared home-directory fallback.
- Persisted Engram Mosaic web, React, WebComponent, and Electron host snapshots
  across launches, seeding from the built-in language-learning demo only when no
  saved state is available.
- Wired the generated Flutter Engram shell to the optional Mosaic host contract
  with a Dart FFI `engram-capi` adapter, shared slot hydration, and generated
  event-envelope routing through the Rust core.
- Added Anki `initialFactor` import/export and a shared initial-ease deck option
  control across the Rust core, WASM facade, and generated Mosaic shells.
- Aligned state-aware study queues with imported Anki new-card `due` positions
  so Mosaic/web/native hosts share the same new-card order.
- Cleared stale imported Anki card-row scheduling and flag fields after shared
  reducer mutations, keeping browser filters and APKG export current.
- Moved HTML/React host adapter activation into Mosaic package host asset
  emission, leaving `build-all.ps1` responsible for generated runtime binaries
  and loaders instead of editing generated shell entry files.
- Added shared browser support for Anki `preset:` deck option searches and
  `prop:pos` / `prop:position` new-card queue position filters.
- Aligned `note:` / `noteType:` and `card:` / `template:` browser filters with
  Anki-style exact-or-wildcard name matching.
- Aligned `tag:*` browser searches with Anki's universal tag-filter behavior.
- Added Anki-style no-combining `tag:nc:` browser searches.
- Aligned `deck:` browser searches for imported filtered cards with preserved
  original deck metadata.
- Added imported Anki card-flag support for `flag:` and `is:flagged` browser
  searches.
- Aligned `is:marked` / `marked:true` searches with Anki's `marked` note tag.
- Added imported Anki card-id timestamp support for `added:` browser searches.
- Added imported Anki card-row metric support for `prop:ivl`, `prop:reps`,
  `prop:lapses`, and `prop:ease` browser filters.
- Added shared browser support for Anki custom card data searches with
  `has-cd:`, `prop:cdn:`, and `prop:cds:` filters, including Anki's nested
  `cd` payload.
- Added imported Anki queue-aware browser filters for `is:buried-manually` and
  `is:buried-sibling`.
- Aligned browser `is:learn` / `is:review` semantics so relearning cards match
  Anki-style lapsed-card search intersections.
- Added imported Anki revlog-aware `resched:` and `prop:resched` manual
  reschedule browser filters, while excluding those rows from imported
  `rated:` searches.
- Normalized Anki-style recent-day browser searches so top-level `:0` windows
  behave as one-day searches for added, edited, introduced, rated, and
  rescheduled cards.
- Added Anki-style answer-button suffix support for `prop:rated`, such as
  `prop:rated<-7:again`.
- Aligned `introduced:` with Anki revlog semantics by ignoring imported manual
  reschedule rows when detecting a card's first real review.
- Treated unknown `key:value` browser searches as Anki-style custom field
  searches, enabling queries such as `Extra:` and `Sentence:re:...`.
- Aligned unqualified browser text searches with Anki note-field scope, while
  keeping standalone Engram cards searchable by front/back text.
- Added Anki-style `did:` deck ID and `mid:` notetype ID browser filters,
  including preserved original IDs from imported packages.
- Added Anki-style `dupe:notetype,text` duplicate first-field browser searches,
  including imported sort-field metadata and HTML/media filename normalization.
- Added imported Anki FSRS stability, difficulty, and retrievability browser
  filters via `prop:s`, `prop:d`, and `prop:r`.
- Aligned imported Anki state and due browser filters with preserved type,
  queue, due, original-due, and collection day metadata.
- Expanded the shared Engram browser search core with Anki-style `w:`, `nc:`,
  `sc:`, and `re:` text modifiers, field-scoped regex searches, tag regexes,
  and single-character `_` wildcards.
- Added the reusable `mosaic-pkg-review-history` dependency and mounted its
  `ReviewHistoryPanel` component so generated host shells expose shared
  review totals, accuracy, per-rating counts, and first/last review fields.
- Added the reusable `mosaic-pkg-deck-options` dependency and mounted its
  `DeckOptionsPanel` component so generated host shells expose shared
  Anki-style deck scheduler option controls.
- Expanded deck options with native checkbox bindings for Anki-style sibling
  burying defaults.
- Expanded the generated deck option contract with learning/relearning step
  list slots and events.
- Routed generated deck option change events through the shared Engram facade
  so native/web hosts can persist settings without platform-specific reducers.
- Updated generated React and Electron renderer shells to use the Mosaic host
  adapter contract (`window.mosaicHost.getProps` / `handleEvent`) with sample
  fallback props, so Engram events can be routed to shared Rust-backed hosts.
- Updated generated Electron preload/main shells to expose those host adapter
  calls over context-isolated IPC channels instead of a placeholder host object.
- Added the Engram WASM Mosaic host bridge so generated React/Electron shells
  can consume shared Rust facade props/events with generated camelCase prop
  names.
- Updated `scripts/build-all.ps1` to build the Engram WASM host and install the
  React/Electron host adapter assets into generated app shells automatically.
- Added an Engram XAML `MosaicHost` bridge that calls `engram-capi` for shared
  Rust facade props/events, and installs it into generated WinUI project shells.
- Added an Engram SwiftUI `MosaicHost` bridge plus `CEngram` module staging so
  generated SwiftPM shells can hydrate from and dispatch into `engram-capi`.
- Added an Engram Qt `MosaicHost` bridge that runtime-loads `engram-capi` and
  installs into generated Qt/CMake shells so QML properties/events share the
  Rust facade.
- Added the reusable `mosaic-pkg-collection-actions` dependency and mounted its
  `CollectionActions` component so generated host shells expose shared
  collection counts, Anki import/export intents, and note/note-type workflow
  events.
- Routed generated Mosaic browser events through the shared Engram event facade,
  including card-ID-targeted mark and suspend actions for browser rows.
- Added browser result and selected-card metadata slots to the Engram app
  contract, backed by `EngramSession::engram_browser_props`, so generated host
  shells can route browser actions by card ID.
- Added the reusable `mosaic-pkg-card-browser` dependency and mounted its
  `CardBrowser` component so the Mosaic app exposes Anki-style browser/search
  slots and events through the shared host contract.
- Added the reusable `mosaic-pkg-deck-stats` dependency and mounted its
  `DeckStatsPanel` component in the app shell.
- Added the reusable `mosaic-pkg-session-progress` dependency and mounted its
  `SessionProgress` component in the app shell.
- Added the reusable `mosaic-pkg-review-actions` dependency and mounted its
  `ReviewActions` component so Mosaic/native review screens expose undo, bury,
  suspend, and mark events through the shared Engram event bridge.
- Added a multi-backend artifact-builder smoke test proving `EngramApp` emits
  through HTML, React, SwiftUI, Qt, XAML, Flutter, and Compose while consuming
  `mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
  `mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
  `mosaic-pkg-review-actions`, `mosaic-pkg-review-card`, and
  `mosaic-pkg-session-progress`.
- Added `scripts/build-all.ps1` to emit HTML, WebComponent, React, Electron,
  SwiftUI, Qt, XAML, Flutter, and Compose artifacts from the same Engram
  Mosaic app package into `target/mosaic-engram-app/`.
- Asserted that the generated React, SwiftPM, and Flutter shells mount
  `EngramApp` with sample slot values and dispatch callbacks instead of
  non-compiling empty initializers.
- Added a pinned Gradle Compose Desktop project shell for generated Compose
  artifacts and asserted it mounts `EngramApp` with sample slot values and
  Mosaic event-envelope logging.
- Asserted that nested package styles from `DeckStatsPanel`, `SessionProgress`,
  `ReviewActions`, `ReviewCard`, and `RatingControls` reach the generated
  Engram HTML artifact.

## 0.1.0

- Added the initial Engram Mosaic app package.
- Added an `EngramApp` root component that consumes `ReviewCard` from
  `mosaic-pkg-review-card`.
- Added smoke tests for manifest boundaries, source compilation, and component
  dependency resolution.
