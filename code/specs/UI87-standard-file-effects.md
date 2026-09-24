# UI87 — Standard file effects: `file.save` and `file.open` on every host

**Status:** proposed (design note; nothing implemented). Needs a decision
before Journal export (J4 of #14416) is built.

**Builds on:** [UI47 — host capability effects](UI47-host-capability-effects.md),
which added `Await` effects, `mosaic_app_complete_effect` and the per-package
`[host_effects]` handler hook.

---

## 1. The question

Journal needs **Export** (and, later, Import): write the journal to a file the
person chooses, and read one back. The Rust app can already produce and parse
the bytes. What it cannot do is ask the host for a file.

Two apps now need the same capability:

| app | today |
| --- | --- |
| Engram | `importAnki` / `exportAnki` as `Await` effects, answered by **hand-written per-app handlers** on four native backends (`host/{qt,swiftui,compose,flutter}/…Effects…`, about 1,300 lines) plus the browser |
| VisiCalc (web) | `file.save` / `file.open`, answered by the **generic** browser executor `mosaic-app-wasm/js/mosaic-file-effects.mjs` |
| Journal | nothing; export is blocked |

Engram's handlers are almost entirely generic. Each one shows a native
save or open dialog filtered to one extension, writes or reads base64 bytes, and
answers `ok` / `cancelled` / `failed`. Only the kind names (`exportAnki`) and the
filter (`.apkg`) are Engram's. The browser already has the generic version.

So the choice is:

- **(A) Standard kinds, handled by the templates.** Every generated host
  answers `file.save` and `file.open` itself, with the payload contract the
  browser executor already defines. Apps emit those kinds and write no host
  code.
- **(B) Per-app handlers.** Journal copies Engram's four handlers, renames the
  kinds, and changes the filter. That is about 1,300 more lines, and every
  future app that saves a file does it again.

**This note recommends (A).**

## 2. Why UI47's objection does not apply

UI47 §5.5.1 rejected "teach the templates well-known effect kinds", because
"`importAnki` opens a file dialog filtered to `.apkg`" would make every app
carry one app's policy. That objection is about **app-specific** kinds.
`file.save` and `file.open` are not app policy. They are the platform
capability itself, parameterised by the app:

- the app decides the file name, type and bytes;
- the host only runs the dialog and moves bytes.

The browser executor already has this split, and VisiCalc's web app
(`code/programs/typescript/visicalc`) uses it.

## 3. The contract (unchanged from the browser)

Both kinds are `Await` effects. The payload is the one `mosaic-file-effects.mjs`
validates today:

```json
{ "kind": "file.save",
  "payload": { "suggestedName": "journal-2026-09-24.json",
               "mimeType": "application/json", "extension": ".json",
               "bytes": "<base64>" } }

{ "kind": "file.open",
  "payload": { "mimeType": "application/json", "extension": ".json" } }
```

Results are UI47's tagged outcomes, as the browser executor returns them today:
`{"ok": {"name": "…"}}` (save; the chosen file's name, never its path) or
`{"ok": {"name": "…", "bytes": "<base64>"}}` (open), `{"cancelled": {}}` (a
dismissed dialog is not an error), or `{"failed": {"message": "…"}}`.

Every host enforces the same limits the browser does:

- 16 MiB of bytes at most;
- `suggestedName` is a plain name: no path separators, no NUL, at most 255
  characters;
- the MIME type and extension must match those patterns;
- one file operation at a time; a second request gets `failed`, not a queue.

A conformance fixture pins these, so every template answers the same malformed
payloads the same way.

## 4. Where the handler lives

One built-in handler per template, installed by the generated entry point
**alongside** the package's own `[host_effects]` handler. The package's
handler is asked first; the built-in is the fallback:

- a package handler still sees every kind, and one that needs different
  behaviour for `file.save` / `file.open` can answer them itself;
- the built-in answers only those two kinds, and only when the package's
  handler did not.

UI47 §5.5 allows one handler per backend, so this is a chain of two: the
package handler, then the built-in fallback. It is not a list.

| backend | dialog |
| --- | --- |
| Compose (Desktop) | `JFileChooser`, as Engram's handler uses today |
| SwiftUI (macOS) | `NSSavePanel` / `NSOpenPanel` |
| Qt | `QFileDialog` |
| Flutter | `file_selector` |
| XAML | `FileSavePicker` / `FileOpenPicker` |
| Web | the existing `mosaic-file-effects.mjs` |

Engram then migrates to the standard kinds as its own step, and its four
handler files shrink to nothing or are deleted. That step is optional and
separate. Nothing here requires it.

## 5. Journal export on top of it

Once the standard kinds exist, Journal needs no host code:

- **Export** emits `file.save` with the journal-core state serialised as JSON
  (the same versioned shape the snapshot already validates), named
  `journal-YYYY-MM-DD.json`.
- **Import** (later) emits `file.open`, then validates the bytes with the same
  loader `restore` uses, before anything changes (UI47 §8.3: invalid, cancelled
  or failed Open leaves the journal intact).
- Before the effect is emitted, the bytes are captured as one consistent point
  in time (UI47 §8.3). A pending export blocks snapshots until it completes,
  as UI47 §8.1 requires.

## 6. Open questions for the owner

1. **(A) or (B)?** This note recommends (A).
2. **Export format:** Journal's own JSON only (round-trips with Import), or
   also Markdown (readable, one-way)? Or Day One's JSON?
3. **Order:** build the standard handlers on all five native templates first,
   or Compose + web first (the lanes Journal is driven on in CI), then the rest?

## 7. What this does not decide

- Photos in Journal (an image slot) and encryption at rest are separate.
- Directory pickers, multiple files and streaming large files are out of
  scope. The 16 MiB cap is the browser's and applies everywhere.
