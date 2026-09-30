# UI87 — `files.save`, and file-effect handlers shared by every host

**Status:** decided (revision 3, 2026-09-25); in progress. The owner chose
(A), generalised: **Mosaic provides shared per-OS host libraries** for platform
capabilities, and no application carries its own copy (§7).

| host | platform library | state |
| --- | --- | --- |
| Compose (desktop) | `MosaicPlatformEffects.kt` | done (#16016) |
| web family | `mosaic-file-effects.mjs` answers `files.*` | done (#16032) |
| SwiftUI (macOS) | `MosaicPlatformEffects.swift` | done; iOS/iPadOS fail each request with a message until UI89 step 6 |
| Qt | `MosaicPlatformEffects.{h,cpp}` | done (§7.4a) |
| XAML (WinUI 3) | `MosaicPlatformEffects.cs` | done (§7.6) |
| Flutter | — | not started |

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
  Compose and SwiftUI libraries and the browser executor; tests pin the three
  to one rule): no path separators or `:`, not `.`/`..`, no leading `.`
  (dot-files), no control, format, line- or paragraph-separator characters,
  no leading or trailing whitespace, no run of two or more whitespace
  characters (characters that render blank — Hangul fillers, BRAILLE
  PATTERN BLANK — count as whitespace), no surrogate, unassigned or
  private-use code point, no trailing dot, at most 255 UTF-16 units. Hosts
  check by code point (Swift by scalar, never by grapheme cluster: a
  combining letter merged with a `.` would otherwise hide it);
- when the app names no accepted type, the name may not end in an extension
  that runs, installs or mounts when opened (`.command`, `.terminal`,
  `.webloc`, `.exe`, `.scf`, `.iso`, `.desktop`, `.AppImage`, … — one list,
  shared by every host, compared after folding case through upper case). With a type, the name must
  already end in one of that type's extensions;
- one file operation at a time; a second request gets `failed`, not a queue.

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
| Compose | `MosaicPlatformEffects.kt` | desktop: `java.awt.FileDialog` (the native macOS/Windows/GTK dialog, not Swing's); Android (UI89): `ActivityResultContracts` |
| SwiftUI | `MosaicPlatformEffects.swift` | macOS: `NSOpenPanel` / `NSSavePanel`; iOS/iPadOS (UI89): `UIDocumentPickerViewController` (until then each request fails with "… is not available on this platform yet", never a silent cancel) |
| Flutter | `mosaic_platform_effects.dart` | `file_selector` on desktop; share sheet for save on mobile |
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
  the platform library, becoming the second consumer. *Compose and Qt done*
  (their handlers removed; SwiftUI never had one). XAML's library exists
  (§7.6); photo-picker's XAML handler claims no kinds, so it is no longer
  reached, and removing it is the follow-up. Flutter follows its library.
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
  router talks to it through `IMosaicPlatformEffectHost`, implemented by a
  forwarding `MosaicRuntimeHostEffects` (and by a fake in the test). The
  generated `MainWindow.xaml.cs` installs the library with
  `MosaicPlatformEffects.Install(this, appKinds: …)` right after
  `MosaicRuntimeHost.LoadRequired();` and the package's own `[host_effects]`
  handler, whose `kinds` become the `appKinds` array (`null` without them).
  Before `LoadRequired` the host's `EffectHandler` setter assigns nothing, so
  the order is load-bearing, as it is for the package handler. The stub shell
  (no `LoadRequired`) compiles the file but installs nothing; it has no host.
  A retried start (`Close`, then `LoadRequired`) is a new runtime with no
  handler, and gets a new router along with the package handler.
- **Threading.** The host calls `EffectHandler` inside its settle, holding a
  reentrant lock, and WinUI pickers are asynchronous COM calls that can pump
  messages. So each standard effect is deferred (`DeferEffect`) and the picker
  is started from the window's `DispatcherQueue`, after the settle returns;
  the answer comes from the picker's continuation on the same UI thread.
  Every path after deferral ends in `CompleteEffect`: a picker that throws
  (an elevated process, a file with no local path) is
  `failed { "the file dialog failed" }`, and so is a queue that refuses the
  work (a closing window), which Compose's `invokeLater` cannot do. A host
  closed underneath swallows the answer; nothing escapes. The payload is
  cloned before deferral, because it belongs to the host's parsed update.
  One file operation at a time, as elsewhere.
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
  target (owner-only on Unix), flushes it to disk and moves it over the
  target with `File.Move(overwrite: true)` — `MoveFileEx` on Windows,
  `rename` on Unix. On Windows the file's ACL is inherited from the folder,
  as the Compose and Qt libraries leave it; on Unix the replaced file's rwx
  bits (never setuid, setgid or sticky) are applied through the open handle.
  `File.Replace` was not used: it fails when the target does not yet exist.
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

**Known gap, the host's rather than the library's:** the XAML host has no
props-changed notification (Compose's `propsChangedHandler`, SwiftUI's
equivalent), so an answer given after deferral reaches the runtime at once
but the window only at the next dispatch (or environment report, which
re-applies a newer revision). The photo-picker handler this replaces had the
same limit. Closing it is a host change.

## 6. What this does not decide

- Photos in Journal (an image in the UI) and encryption at rest are separate.
  Photos also need a Mosaic display primitive for images (see
  `engram-latex-rendering.md` §0).
- Directory pickers, multiple files and streaming large files are out of
  scope, as in UI59.
