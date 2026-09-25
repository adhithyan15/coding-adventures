# UI87 — `files.save`, and file-effect handlers shared by every host

**Status:** proposed (design note; nothing implemented). Needs a decision
before Journal export (J4 of #14416) is built.

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

## 6. What this does not decide

- Photos in Journal (an image in the UI) and encryption at rest are separate.
  Photos also need a Mosaic display primitive for images (see
  `engram-latex-rendering.md` §0).
- Directory pickers, multiple files and streaming large files are out of
  scope, as in UI59.
