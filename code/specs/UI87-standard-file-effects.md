# UI87 — `files.save`, and file-effect handlers shared by every host

**Status:** decided (revision 3, 2026-09-25); in progress. The owner chose
(A), generalised: **Mosaic provides shared per-OS host libraries** for platform
capabilities, and no application carries its own copy (§7).

| host | platform library | state |
| --- | --- | --- |
| Compose (desktop) | `MosaicFileEffects.kt` + `MosaicPlatformEffects.kt` | done (#16016; split for Android, UI89 §3.8) |
| web family | `mosaic-file-effects.mjs` answers `files.*` | done (#16032) |
| SwiftUI (macOS) | `MosaicPlatformEffects.swift` | done; iOS/iPadOS fail each request with a message until UI89 step 6 |
| Qt | `MosaicPlatformEffects.{h,cpp}` | done (§7.4a) |
| XAML (WinUI 3) | `MosaicPlatformEffects.cs` | done (§7.6) |
| Flutter | `mosaic_platform_effects.dart` + `mosaic_platform_effects_core.dart` | done (§7.7); Android/iOS fail each request with a message until UI89 |

First consumer: Journal's Export (J6a, #16034).

**Builds on:**
- [UI47 — host capability effects](UI47-host-capability-effects.md), which
  added `Await` effects, `mosaic_app_complete_effect` and the per-package
  `[host_effects]` handler hook;
- [UI59 — `files.open`](UI59-files-open-effect.md), which made a generic
  *pick a file* effect a Mosaic convention and shipped handlers for it on XAML,
  Qt, Compose and Flutter.

> **Correction (revision 2).** The first revision of this note proposed
> `file.save` / `file.open` as if no convention existed, and missed UI59. This
> revision builds on UI59 instead. It adopts UI59's `files.*` naming and
> payload, adds the `files.save` that UI59 deliberately deferred, and restates
> the open question as the one step beyond UI59.

---

## 1. What exists

| what | kinds | where the handler code lives |
| --- | --- | --- |
| UI59 convention | `files.open` (`accept: [MIME…]` → `{name, mimeType, bytes}`) | **per app**, copy-adapted: `photo-picker-app`'s reference handlers on XAML, Qt, Compose and Flutter. UI59 §4.1: "there is no shared-library mechanism for handler *code* across packages yet, only for the *contract*" |
| browser executor | `file.open`, `file.save` (singular; `mimeType` + `extension`, `suggestedName`) | shared: `mosaic-app-wasm/js/mosaic-file-effects.mjs`, used by VisiCalc's web app |
| Engram | `importAnki`, `exportAnki` (its own kinds) | per app, about 1,300 lines across four native backends |
| Journal | needs *save* (export), later *open* (import) | nothing yet |

## 2. The two gaps

1. **No `files.save`.** UI59 deferred it ("a real, separate capability …
   not needed by anything in flight"). Journal export now needs it.
2. **Handler code is copied per app.** It is a convention without shared
   code. Every app that saves or opens a file carries its own four or five
   handler files. The browser is the one host with a shared executor.

There is also a **naming split**: the browser executor answers `file.open` /
`file.save`, while UI59's convention is `files.open`.

## 3. Proposal

### 3.1 `files.save`, UI59's companion

```text
kind:     "files.save"
delivery: Await
payload:  { "suggestedName": "journal-2026-09-24.json",
            "accept": ["application/json"],
            "bytes": "<base64>" }
result:   ok        { "name": "journal-2026-09-24.json" }   -- a name, never a path
          cancelled {}                                      -- a dismissed dialog is not an error
          failed    { "message": "…" }
```

It uses the same `accept` MIME list as `files.open`, mapped per host to the
picker's own filter, as UI59 §3 describes. The limits are the browser
executor's, applied everywhere:
- 16 MiB of bytes at most;
- `suggestedName` is a plain name, checked identically by every host (the
  Compose, SwiftUI, XAML, Qt and Flutter libraries and the browser executor;
  tests pin them to one rule): no path separators or `:`, not `.`/`..`, no
  leading `.` (dot-files), no control, format, line- or paragraph-separator characters,
  no leading or trailing whitespace, no run of two or more whitespace
  characters (characters that render blank — Hangul fillers, BRAILLE
  PATTERN BLANK — count as whitespace), no surrogate, unassigned or
  private-use code point, no trailing dot, at most 255 UTF-16 units, and not
  a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM0`-`COM9`,
  `LPT0`-`LPT9`, `COM¹²³`, `LPT¹²³`, `CONIN$`, `CONOUT$`), compared on the
  part before the first dot with trailing spaces removed and ASCII letters
  folded to upper case (only those: .NET's own upper-casing leaves `ı` alone
  where the others make it `I`, so a shared fold keeps one rule) --
  `con.txt` and `NUL .json` open the console or the null device on Windows,
  never a file, so every host refuses them and a name saves alike everywhere.
  Hosts check by code point (Swift by scalar, never by grapheme cluster: a
  combining letter merged with a `.` would otherwise hide it);
- when the app names no accepted type, the name may not end in an extension
  that runs, installs or mounts when opened (`.command`, `.terminal`,
  `.webloc`, `.exe`, `.scf`, `.iso`, `.desktop`, `.AppImage`, … — one list,
  shared by every host, compared after folding case through upper case).
  The list includes documents that run code when opened: web pages and SVG
  (`.html`, `.htm`, `.xhtml`, `.svg`, …, whose script runs from the local
  file), saved web archives and shortcuts (`.mht`, `.website`), the Office
  formats made to carry macros (`.docm`, `.xlsm`, `.pptm`, …, `.xlsb`,
  `.xla`), Python scripts (`.py`, `.pyw`, `.pyz`, `.pyzw`, `.pyc`), and
  shortcuts that fetch or connect when opened (`.iqy`, `.slk`, `.rdp`). The
  older binary Office formats (`.doc`, `.xls`, `.ppt`) can carry macros too
  but are everyday documents, and Office opens them protected, so they stay
  allowed. An extension with any non-ASCII character counts
  too: a lookalike letter (`.ехе` with Cyrillic `е`) or a combining mark
  after `.exe` makes an extension no list can name but a reader takes for an
  executable one. With a type, the name must already end in one of that
  type's extensions;
- one file operation at a time; a second request gets `failed`, not a queue.

**No download marks of our own.** The platform libraries add no
Mark-of-the-Web (`Zone.Identifier`) on Windows and no `com.apple.quarantine`
on macOS. Those mark files that arrived from somewhere else; a `files.save`
is the app writing its own data where the person chose, as an application's
Save As does. Marking them would make the operating system warn about the
app's own documents, and teach people to click through the warning. Where
the operating system or browser adds a mark itself -- the browser
executor's download fallback and File System Access writes in Chromium, a
file created by a sandboxed macOS app -- it is left in place. The extension
rules above are what keep a save from becoming a launcher.

### 3.2 One name

The convention is UI59's `files.*`. The browser executor would answer
`files.open` / `files.save`, and keep `file.*` as an alias until VisiCalc
migrates. It would also accept UI59's `accept` list alongside its current
`mimeType` + `extension` pair.

### 3.3 Where the handler code lives: the open question

- **(A) Built into the templates.** Each generated host ships a
  `files.open` / `files.save` handler. The package's own `[host_effects]`
  handler is asked first, and the built-in is the fallback, so a package can
  still answer those kinds itself. Apps emit the kinds and write no host code.
  UI47 §5.5.1 objected to *app-specific* kinds in templates (Engram's
  `.apkg` dialog). A generic file dialog parameterised by the app is the
  platform capability, not app policy.
- **(B) Keep UI59's model.** Journal copy-adapts `photo-picker-app`'s
  handlers (and adds `files.save` to each). This works today with no
  template changes. The cost is the copy, repeated per app.

**This note recommends (A)**, because the second and third apps
(Journal, then Engram if it migrates) are already here. (B) is a valid
first step if template changes are unwelcome. It would make Journal the
reference implementation of `files.save`.

## 4. Journal export on top of it

- **Export** emits `files.save` with the journal-core state serialised as JSON
  (the versioned shape the snapshot already validates).
- **Import** (later) emits `files.open` and validates the bytes with the same
  loader `restore` uses, before anything changes (UI47 §8.3: invalid,
  cancelled or failed Open leaves the journal intact).
- The bytes are captured before the effect is emitted (UI47 §8.3). A pending
  export blocks snapshots until it completes (UI47 §8.1).

## 5. Open questions for the owner

1. **(A) or (B)** (§3.3)? This note recommends (A).
2. **Naming:** adopt `files.*` everywhere, with the browser's `file.*` kept as
   an alias (§3.2)?
3. **Export format:** Journal's own JSON only (round-trips with Import), or
   also Markdown (readable, one-way)? Or Day One's JSON?
4. **Order:** Compose and web first (the lanes Journal is driven on in CI),
   then the rest?

## 7. Decided: shared per-OS host libraries (revision 3)

The owner's answer to §5.1 goes further than (A): Mosaic should provide the
operating-system capabilities itself, as libraries every generated app gets,
with a generalised core, rather than per-app handler files. This section
replaces §3.3 and answers §5.

### 7.1 What ships where

One **platform library per backend**, shipped by `mosaic-app-bindings`
beside the runtime-binding template it already ships, and so copied into every
generated project the same way:

| backend | library | per-OS inside it |
| --- | --- | --- |
| Compose | `MosaicFileEffects.kt` (shared) + `MosaicPlatformEffects.kt` (per target) | desktop: `java.awt.FileDialog` (the native macOS/Windows/GTK dialog, not Swing's); Android (UI89): `ActivityResultContracts` |
| SwiftUI | `MosaicPlatformEffects.swift` | macOS: `NSOpenPanel` / `NSSavePanel`; iOS/iPadOS (UI89): `UIDocumentPickerViewController` (until then each request fails with "… is not available on this platform yet", never a silent cancel) |
| Flutter | `mosaic_platform_effects.dart` (+ its plain-Dart `_core.dart`, §7.7) | `file_selector` on desktop; share sheet for save on mobile (until UI89, each mobile request fails with a message) |
| Qt | `MosaicPlatformEffects.{h,cpp}` | `QFileDialog` (native where the platform has one) |
| XAML | `MosaicPlatformEffects.cs` | `FileOpenPicker` / `FileSavePicker` |
| web family | `mosaic-file-effects.mjs` (exists) | `showOpenFilePicker` / `showSaveFilePicker`; without them, `files.save` downloads the bytes and reports `ok { name, download: true }` (a download is not a durable save the person placed), and an `<input type=file>` fallback for `files.open` follows |

The generalised core is the **contract**, not code: each library answers the
same kinds with the same payloads, limits and results (§3.1, UI59), and each
has a conformance test driving it through its host's test harness.

### 7.2 Routing: which handler answers a kind

The generated entry point already installs the package's `[host_effects]`
handler (UI47 §5.5). It now also installs the platform library, and routes
each effect by **kind**:

1. a kind the package **claims** goes to the package handler;
2. a **standard kind** (`files.open`, `files.save`, later more) goes to the
   platform library;
3. anything else completes as `failed { "message": "unsupported effect kind" }`,
   so an unanswered `Await` never wedges snapshot and restore (UI47 §8.1).

A package claims kinds in its manifest:

```toml
[host_effects]
handlers = [
  { backend = "qt", include = "engram_effects.h", install = "installEngramEffects",
    kinds = ["importAnki", "exportAnki"] },
]
```

A handler without `kinds` keeps today's meaning (it receives every
non-standard kind), so existing packages build unchanged. A package that
lists a standard kind overrides the platform library for it; that is the
escape hatch, used deliberately and visibly.

### 7.3 Answers to §5

1. **(A), generalised** to per-OS host libraries (§7.1–7.2).
2. **Naming:** `files.*` everywhere. The browser executor answers `files.*` and
   keeps `file.*` as an alias until VisiCalc migrates. It also accepts UI59's
   `accept` list alongside its `mimeType` + `extension` pair.
3. **Export format:** Journal's own versioned JSON first, because it
   round-trips with Import. Markdown (readable, one-way) is a later option.
4. **Order:** Compose and web first, with Journal export as the first consumer;
   then SwiftUI, Qt, XAML, Flutter; then mobile with UI89.

### 7.4 Migration

- `photo-picker-app` drops its four per-backend `files.open` handlers and uses
  the platform library, becoming the second consumer. *Done* (Compose, Qt,
  XAML and Flutter handlers removed; SwiftUI never had one): the app claims
  no kinds, so once each backend's library existed the router sent
  `files.open` there and the app's handler was never reached. The Flutter
  handler's `file_selector` `[host_assets]` coordinate went with it, since
  every generated Flutter project pins `file_selector` for the library
  (§7.7). The app now carries no host code at all.
- Engram's `importAnki` / `exportAnki` can later become `files.open` /
  `files.save` plus Rust-side parsing, which removes about 1,300 lines of
  per-backend handler code. That is a separate change.

### 7.4a Qt: one routed handler, the signal as the fallback

Compose and SwiftUI hosts each have one `effectHandler` property, so the
platform library wraps the app's handler and every effect has exactly one
owner. The Qt host instead emitted a fan-out signal, `effectRequested`, that
every connected handler receives, and handlers answer inline (a blocking
`QFileDialog`). A library connected beside photo-picker's handler would have
opened a second dialog for one `files.open`, and `deferEffect` accepted a
second owner of an id. So on Qt:

- `MosaicHost` gains one handler slot, `setEffectHandler(EffectHandler)`.
  When it is set, the host calls it **instead of** emitting
  `effectRequested`; when it is not, the signal is emitted as before. Never
  both, so an effect never has two owners.
- `installMosaicPlatformEffects(host, appKinds)` sets that slot to the
  router (§7.2). A kind that goes to the app is delivered the old way: to a
  handler previously set in the slot, or else by emitting `effectRequested`,
  so `connect`ed package handlers (Engram's) keep working unchanged.
  (Photo-picker's Qt handler, which claimed no kinds and so was never
  reached once the library answered `files.open`, was retired, §7.4.) A standard kind the app did not claim is answered by the
  library, and the signal is not emitted for it.
- `deferEffect` refuses an id that is already deferred: an effect has one
  owner.
- Dialogs are `QFileDialog`'s static functions, answered inline like the
  existing Qt handlers; a fake can be injected, so the library's behaviour is
  tested headless against Qt Core only.
- `MosaicPlatformEffects.{h,cpp}` are compiled into every Qt project beside
  `MosaicHost.{h,cpp}`, under the same guard, and `main.cpp` installs the
  library after the package's own handler.

### 7.5 First PRs

1. `files.save` and routing in the Compose platform library, with a
   conformance test; the browser executor answering `files.*`.
2. Journal export (Compose and web) on top of it.

### 7.6 XAML, as built

`MosaicPlatformEffects.cs` answers the Compose library's contract: the same
kinds, limits (50 MiB open, bounded while reading; 16 MiB save, checked on
the encoded length first), MIME table in the same order, plain-name rule,
executable list and failure messages. A Rust test pins the tables and
messages to the Kotlin file. What differs is how WinUI 3 and a static host
shape the rest:

- **The host is static.** `MosaicRuntimeHost` is a static class, so the
  router talks to it through `IMosaicPlatformEffectHost`, implemented by
  `MosaicRuntimeHostEffects` (and by a fake in the test). The
  generated `MainWindow.xaml.cs` installs the library with
  `MosaicPlatformEffects.Install(this, appKinds: …)` right after
  `MosaicRuntimeHost.LoadRequired();` and the package's own `[host_effects]`
  handler, whose `kinds` become the `appKinds` array (`null` without them).
  Before `LoadRequired` the host's `EffectHandler` setter assigns nothing, so
  the order is load-bearing, as it is for the package handler. The stub shell
  (no `LoadRequired`) compiles the file but installs nothing; it has no host.
  A retried start (`Close`, then `LoadRequired`) is a new runtime with no
  handler, and gets a new router along with the package handler.
- **Answers go to the runtime that asked.** The static `CompleteEffect`
  answers whichever runtime is loaded when it is called, and a fresh
  runtime's effect ids restart at 1. A picker still open across a retried
  start would therefore answer the new runtime, where the reused id could be
  an unrelated effect — the conformance runtime confirms both ids are 1, and
  the stale answer settled it. So `MosaicRuntimeHost` exposes an
  `EffectScope`: a handle bound to the runtime loaded when it was taken,
  whose `DeferEffect` refuses once that runtime is no longer the loaded one
  and whose `CompleteEffect` throws `ObjectDisposedException` once it is
  closed. `MosaicRuntimeHostEffects` wraps the scope taken at install, so a
  late answer meets the closed runtime and the router drops it; the new
  runtime's own router (with its own busy flag) is unaffected. The
  effect-completion driver checks this against the real runtime.
- **Threading.** The host calls `EffectHandler` inside its settle, holding a
  reentrant lock, and WinUI pickers are asynchronous COM calls that can pump
  messages. So each standard effect is deferred (`DeferEffect`) and the picker
  is started from the window's `DispatcherQueue`, after the settle returns;
  the answer comes from the picker's continuation on the same UI thread.
  The file itself is read (up to 50 MiB, then base64-encoded) or written and
  flushed on the thread pool (`await Task.Run`), so the window keeps
  painting; the await resumes on the UI thread, where the answer is given.
  Every path after deferral ends in `CompleteEffect`: a picker that throws
  (an elevated process, a file with no local path) is
  `failed { "the file dialog failed" }`, and so is a queue that refuses the
  work (a closing window), which Compose's `invokeLater` cannot do. A host
  closed underneath swallows the answer; nothing escapes. The payload is
  cloned before the router marks itself busy (so nothing between taking
  and handing on the busy flag can throw and strand it), because it belongs
  to the host's parsed update. One file operation at a time, as elsewhere.
- **Pickers.** `FileOpenPicker` / `FileSavePicker`, initialised with the
  window's handle (`InitializeWithWindow`), which an unpackaged WinUI 3 app
  must do or the picker throws. Open filters on the accepted extensions, or
  `*`. Save is given the suggested name without its extension and that
  extension as the first file-type choice (the picker appends the chosen
  one), then the accepted ones; a name with no extension gets the `.` choice,
  since the list may not be empty.
- **Files through System.IO.** The pickers return a path, and reading and
  writing go through `FileStream`, so the bounded read and the save are the
  code the headless test runs. Save writes a `CreateNew` temporary beside the
  target (owner-only on Unix) and flushes it to disk. On Windows, over an
  existing file, it is put in place with `File.Replace` (`ReplaceFileW`,
  `ignoreMetadataErrors`), which keeps the replaced file's ACL, attributes
  and alternate streams — a plain move would give it the folder's inherited
  ACL, widening a file the person had locked down; this goes further than
  the Compose and Qt libraries, which leave the folder's ACL. A new file (or
  a target gone by the time of the replace) falls back to
  `File.Move(overwrite: true)` (`MoveFileEx`), and on Unix the move is
  `rename`, with the replaced file's rwx bits (never setuid, setgid or
  sticky) applied to the temporary through the open handle. Unverified:
  `FileSavePicker` may create an empty placeholder at the chosen path before
  returning (UWP's did); if it does, the save takes the `File.Replace` path
  over that placeholder, whose ACL is the folder's anyway. A chosen path
  that is a symlink or junction is written through to its target by
  `File.Replace` (the move replaced the link itself); the person picked that
  path, so this is the file they chose.
  .NET has no `O_NONBLOCK`, so unlike SwiftUI and Qt a Unix FIFO cannot be
  refused without blocking; the WinUI picker shows only the Windows file
  system, which has none. `Convert.FromBase64String` skips whitespace, so the
  alphabet is checked first, to refuse what Compose and SwiftUI refuse.
- **Headless split.** Everything WinUI (the pickers, the window handle, the
  `DispatcherQueue`) sits inside `#if !MOSAIC_HEADLESS_TEST`, which no
  generated project defines, in one file rather than a split pair, so the
  builder copies one file and a project cannot keep the logic without the
  pickers. `conformance/xaml-platform-effects/` defines the symbol and runs
  the Compose test's cases (and the SwiftUI harness's router cases) with a
  fake picker and a fake host on plain .NET; CI runs it in the Windows XAML
  lane against TaskApp's generated files, so the save's move runs on NTFS.
  The WinUI half is compiled for real only by that lane's TaskApp build.

**Props after a deferred answer.** An answer given after deferral moves the
app with no call from the window, so the host tells it, as Compose's
`propsChangedHandler` and Flutter's `setPropsChangedHandler` do:
`MosaicRuntimeHost.PropsChanged` is raised once per deferred effect
answered, on the thread that answered, after the host's lock is released
(an answer given inside the handler that was offered the effect is returned
by the dispatch that minted it, and raises nothing). The runtime-backed
window sets it right after `LoadRequired` and re-applies the required props
from its `DispatcherQueue` -- to the root showing, in a window that switches
layouts. Closing the runtime drops the handler; a retried start sets it
again. The XAML effect driver checks it is raised once for a deferred answer
and not for a deferral or an answer in the handler.

### 7.7 Flutter, as built

`mosaic_platform_effects_core.dart` answers the Compose library's contract:
the same kinds, limits (50 MiB open, bounded while reading; 16 MiB save,
checked on the encoded length first), MIME table in the same order,
plain-name rule (including the default-ignorable ranges), executable list and
failure messages. A Rust test pins the tables, ranges and messages to the
Kotlin file. What differs is how Dart and Flutter shape the rest:

- **Two files.** Dart has no conditional compilation, and the dialogs need
  the Flutter engine. So the contract, the file I/O and the router are plain
  Dart (dart:io, dart:ffi, package:ffi) in `mosaic_platform_effects_core.dart`,
  and `mosaic_platform_effects.dart` adds `package:file_selector`'s dialogs and
  `installMosaicPlatformEffects(host, appKinds: …)`, re-exporting the core.
  The builder writes both beside `mosaic_host.dart` in every generated
  project; `main.dart` imports the second.
- **Installation.** The generated `main.dart` installs the library right after
  the host is assigned, after the package's own `[host_effects]` handler (the
  router wraps whatever handler is set, so that order is what lets the app's
  kinds reach the app): on the strict shell's local `host`, before
  `setPropsChangedHandler` and the first props read; on the permissive shell
  through the same guarded local (`mosaicEffectHost`) the package handler
  uses. The handler's `kinds` become `const <String>['…']`, re-checked against
  the manifest's dotted-name shape before being written (a quote, backslash
  or `$` would otherwise become Dart code); an empty list is
  `const <String>[]`, and no `kinds` is `null`, the original meaning.
  `MosaicHost` gains an `effectHandler` getter so the router can wrap the
  app's handler. A second install on the same host changes nothing.
- **Answers go to the host that asked.** The router holds the `MosaicHost`
  instance it was installed on (through `MosaicHostEffects`), never a static
  facade. The shell's retried start disposes that host and loads a new one,
  running the install lines again, so the new runtime gets its own router and
  busy flag. A dialog left open across the retry answers the disposed host,
  whose runtime's `completeEffect` throws (`_ensureOpen`), and the router
  drops it; it cannot settle the new runtime's effect that reuses the id. No
  equivalent of XAML's `EffectScope` is needed.
- **Threading.** A Dart isolate is single-threaded: there is no host lock
  and nothing to marshal. For each standard `Await` the payload is deep-copied
  (before the busy flag is taken), the effect deferred, and the dialog started
  from a microtask, after the settle returns. The chosen file is read and
  base64-encoded, or written and flushed, in a short-lived background isolate
  (`Isolate.run`, from top-level functions that capture only a path and the
  bytes), so a 50 MiB file never stalls a frame; the answer is given back on
  the UI isolate, and the host's props-changed handler redraws, as XAML's
  `PropsChanged` does (§7.6). A scheduler that throws and a dialog
  that throws are both `failed { "the file dialog failed" }`; a disposed host
  swallows the answer; nothing escapes.
- **Dialogs.** `file_selector` (published by the Flutter team), pinned
  exactly: `file_selector: 1.0.4` in every generated `pubspec.yaml`. 1.0.4
  rather than 1.1.0 because 1.1.0 requires Flutter 3.35 and generated projects
  declare 3.32 as their floor; 1.0.4 requires 3.29. Generated projects ship no
  lockfile, so its platform packages (`file_selector_linux` and so on) resolve
  within 1.0.4's own ranges at `pub get`. A package's `[host_assets]`
  coordinate for a package the project already declares is left out, because
  YAML refuses a duplicate key: Engram's
  `file_selector: '>=1.0.0 <2.0.0'` admits the pin. Extensions are passed bare,
  as one type group, or no group at all for "any file" (Linux refuses an
  empty group). A choice with no local path is a failure, not a cancel.
- **Asking before replacing, on Linux.** NSSavePanel and the Windows dialog
  ask before a save goes onto an existing name; GTK's chooser can, but
  `file_selector_linux` never turns that on. So on Linux the library asks
  itself (`mosaicConfirmReplacing`): when anything is at the chosen path -- a
  link, even a dangling one, counts -- it asks "Replace "<name>"?" in a
  Material dialog on the app's root navigator (`mosaicAskToReplace`), found
  by walking the widget tree rather than through a key `main.dart` would have
  to hand over. The dialog is a `DialogRoute` pushed on that navigator, not
  `showDialog`: `showDialog` reaches Flutter's desktop windowing code, whose
  macOS FFI structs abort the AOT snapshotter ("Class with illegal cid",
  Flutter 3.44 and 3.47), so carrying it broke every macOS release build of
  a Flutter app (seen in Engram's release lane). Replace saves; Cancel, or
  dismissing the dialog, is a cancel.
  With no navigator to ask through the question throws, and the save fails
  ("the file dialog failed") rather than replacing a file nobody was asked
  about. The save itself is unchanged: it still replaces atomically.
- **Files through libc, as Qt and SwiftUI do it.** dart:io cannot do what
  the save and the open need: its exclusive `createSync` closes the file it
  created, and `openSync` then reopens by path (`O_CREAT | O_TRUNC`, no
  `O_EXCL`, no `O_NOFOLLOW`), following whatever is at that name by then; it
  cannot create with a mode, `fchmod`, or open without blocking. So both go
  to libc through dart:ffi, with the `open(2)` flags taken per OS and
  architecture from the system headers (they differ: `O_NOFOLLOW` is 0x20000
  on Linux x64, 0x8000 on Linux arm64 and 0x100 on Apple platforms). Those
  tables cover the ABIs Flutter builds desktop apps for (Linux x64 and arm64,
  macOS x64 and arm64); on any other POSIX ABI a save fails rather than
  guessing.
  Save on POSIX: `open(<folder>/.mosaic-save-<random>.tmp, O_WRONLY |
  O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600)` -- a new file or nothing,
  so a link or file planted at that name (possible in a folder another user
  can write to) fails the save and is left alone. The bytes are written,
  `fchmod`-ed and `fsync`-ed through that descriptor, which is closed and
  renamed onto the chosen path, replacing a chosen link rather than writing
  through it, as Compose and Qt do; the temporary is `unlink`ed only if
  something failed. The mode follows Qt: a regular file this user owns
  lends its rwx bits (never setuid, setgid or sticky); someone else's file,
  a link, or nothing gives 0600. Ownership comes from `lstat`: Linux calls
  `statx` (glibc 2.28+), whose layout is the same on every architecture;
  Apple platforms call the 64-bit-inode `lstat` (`lstat$INODE64` on x86_64).
  Where there is no `statx` the owner is unknown, and the replaced file's
  bits are kept without group and other write (`& 0755`).
  Open on POSIX: the path is checked first, following a link the person
  chose. dart:io types a device as not found, so a directory, a missing
  file, a FIFO or a device is "not a regular file" before anything opens
  it. The resolved path is then opened once with `O_RDONLY | O_NONBLOCK |
  O_NOFOLLOW | O_CLOEXEC` and typed with `fstat` on that descriptor, so a
  FIFO or device swapped in after the check is refused without blocking,
  and a link swapped in is not followed. Where that descriptor's type
  cannot be had -- no `statx` (glibc older than 2.28), or a sandbox that
  refuses it -- the open fails closed with "couldn't read the selected
  file" rather than read an untyped descriptor. The bytes are read through
  the same descriptor in 64 KiB chunks, bounded while reading.
  On Windows the save is `CreateFileW(CREATE_NEW, no sharing,
  FILE_FLAG_OPEN_REPARSE_POINT)` beside the target, `WriteFile` and
  `FlushFileBuffers` through that handle,
  `CloseHandle`, then `MoveFileExW(MOVEFILE_REPLACE_EXISTING |
  MOVEFILE_WRITE_THROUGH)`, the move the host uses for its own state (the
  temporary is deleted only on failure). The folder's ACL applies, as with
  Compose and Qt; XAML's `File.Replace` keeps the replaced file's. The
  Windows open reads through dart:io after the same path check, since a
  file dialog cannot return a FIFO there. The base64 alphabet is checked
  first, because Dart's decoder also accepts the URL-safe alphabet and `%3D`
  padding.
- **Platforms.** Linux, macOS and Windows have the dialogs. On Android and iOS
  each request fails at once with "`<kind>` is not available on this platform
  yet", nothing deferred, as SwiftUI does on iOS, until UI89 adds a document
  picker and a share sheet. A sandboxed macOS build needs the
  `com.apple.security.files.user-selected.read-write` entitlement, which the
  runner `flutter create` generates does not have; without it the panel fails
  and the request is answered `failed { "the file dialog failed" }`. So the
  builder writes `macos/Runner/DebugProfile.entitlements` and
  `Release.entitlements` into every Flutter project: Flutter's own keys (the
  sandbox; JIT and the VM service's socket in debug) plus that one.
  `flutter create` writes only the files a project lacks, so the runner it
  makes later keeps them. `user-selected` is the narrowest file entitlement:
  the app gets the one file the person chose, nothing else on disk.
- **Tests.** `conformance/flutter-platform-effects/` runs the core on the
  plain Dart VM with a fake host and fake dialogs: the Compose test's cases,
  the SwiftUI and XAML harnesses' router cases (including the host disposed
  while a dialog is open), a FIFO refused, a chosen link replaced, no dialogs
  on mobile, and the adapter over a `MosaicHost` with no runtime. It also
  pins the four flag tables, probes the running ABI's table against the
  kernel with raw `open(2)` calls (`O_NOFOLLOW` refuses a link, `O_EXCL` an
  existing name, `O_NONBLOCK` opens a FIFO at once, and `O_CLOEXEC` sets
  `FD_CLOEXEC`), plants a link, a dangling link and a file at the
  temporary's name (each fails the save and nothing is written through it),
  checks the ownership rule on real files, and hands a FIFO, a device and a
  directory straight to the descriptor read.
  It also asks before replacing exactly when the dialog did not and
  something is at the path (a dangling link included), names the file
  rather than the path, treats "keep" as a cancel, and fails a save whose
  question cannot be asked without touching the file.
  `conformance/flutter-replace-dialog/` drives the dialog itself with the
  widget tester (Replace, Cancel, dismissal, the navigator found without a
  key, a throw with none), in CI's Flutter TaskApp lane.
  `tests/flutter_platform_effects.rs` runs it wherever `dart` is installed;
  the Flutter CI lane checks TaskApp's `main.dart` installs the library and
  runs the harness against TaskApp's generated files. The dialogs file is
  compiled by that lane's `flutter analyze` and `flutter build linux` of every
  generated project.

**Where the libc calls run.** The Flutter lane runs the headless harness
on Linux x64 with every TaskApp build. The CI job `Flutter platform library`
runs the same harness, through `tests/flutter_platform_effects.rs`, on a Mac
(arm64), on Windows x64 and on Linux arm64 whenever a change can affect the
Flutter runtime. So the macOS and Linux arm64 `open(2)` flag values, the
macOS `stat` layout and the Windows save (`CreateFileW`, `FlushFileBuffers`,
`MoveFileExW`) are exercised, not just pinned. On that job a runner without
`dart` fails (`MOSAIC_REQUIRE_DART`) instead of skipping.

## 6. What this does not decide

- Photos in Journal (an image in the UI) and encryption at rest are separate.
  Photos also need a Mosaic display primitive for images (see
  `engram-latex-rendering.md` §0).
- Directory pickers, multiple files and streaming large files are out of
  scope, as in UI59.
