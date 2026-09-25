# UI87 — `files.save`, and file-effect handlers shared by every host

**Status:** decided (revision 3, 2026-09-25); nothing implemented yet. The
owner chose (A), generalised: **Mosaic provides shared per-OS host libraries**
for platform capabilities, and no application carries its own copy (§7).

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
- `suggestedName` is a plain name (no path separators, no NUL, at most 255
  characters);
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
| SwiftUI | `MosaicPlatformEffects.swift` | macOS: `NSOpenPanel` / `NSSavePanel`; iOS/iPadOS (UI89): `UIDocumentPickerViewController` |
| Flutter | `mosaic_platform_effects.dart` | `file_selector` on desktop; share sheet for save on mobile |
| Qt | `MosaicPlatformEffects.{h,cpp}` | `QFileDialog` (native where the platform has one) |
| XAML | `MosaicPlatformEffects.cs` | `FileOpenPicker` / `FileSavePicker` |
| web family | `mosaic-file-effects.mjs` (exists) | `showOpenFilePicker` / `showSaveFilePicker`, falling back to `<input type=file>` and a download link |

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
  the platform library, becoming the second consumer.
- Engram's `importAnki` / `exportAnki` can later become `files.open` /
  `files.save` plus Rust-side parsing, which removes about 1,300 lines of
  per-backend handler code. That is a separate change.

### 7.5 First PRs

1. `files.save` and routing in the Compose platform library, with a
   conformance test; the browser executor answering `files.*`.
2. Journal export (Compose and web) on top of it.

## 6. What this does not decide

- Photos in Journal (an image in the UI) and encryption at rest are separate.
  Photos also need a Mosaic display primitive for images (see
  `engram-latex-rendering.md` §0).
- Directory pickers, multiple files and streaming large files are out of
  scope, as in UI59.
