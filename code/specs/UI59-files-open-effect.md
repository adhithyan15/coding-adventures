# UI59 — `files.open`: a generic file/gallery-picker effect

## Status

New — the first generic (not app-specific) Mosaic effect kind. Depends
on `UI47` (host capability effects — `[host_effects]`, `Effect`,
`Delivery`, `mosaic_app_complete_effect`), which is fully implemented
and tested on all five native backends (Qt, SwiftUI, Compose, Flutter,
XAML). This spec adds no protocol surface — it is a *convention* for
one effect kind's `kind` string and payload/result shape, plus the
first real implementation of it (XAML). Qt, Compose, and Flutter
follow in the same order already used for `[host_effects]` itself.

**Layer:** UI / standard Mosaic app convention (not core protocol)
**Depends on:** `UI47-host-capability-effects.md`
**Unblocks:** `VIS00-vision-roadmap.md`'s L4 phase (Mosaic camera +
gallery effects) and, through it, phase 5 (a QR-scanner reference app
wiring this together with `qr-locate`, merged).

---

## 1. Why this exists

`VIS00-vision-roadmap.md`'s L4 phase needs a way for a Mosaic app to
ask the host "let the user pick a photo," and get file bytes back.
`UI47` built the channel this rides on (`Effect`/`Delivery::Await` +
`mosaic_app_complete_effect`) but defined no effect *kinds* — every
existing kind (`importAnki`, `exportAnki`, `openCard`, ...) is
Engram's own, minted in Engram's own app-logic crate, not a Mosaic
convention any other app could reuse. This spec fixes that gap for the
one capability almost every app eventually needs: pick a file.

## 2. Scope: one effect kind, XAML first

**One effect kind** (`files.open`), not a separate "gallery" vs
"files" kind. On every backend this targets, the same native picker
API serves both: WinUI 3's `FileOpenPicker` just takes a different
`SuggestedStartLocation` and file-type filter depending on what the
caller wants; the same holds for Qt's `QFileDialog`, SwiftUI's
`.fileImporter`, Compose's `ActivityResultContracts.GetContent`, and
Flutter's `file_selector`. A caller that wants "pictures only" sets
`accept` to image MIME types; the effect kind itself knows nothing
about images.

**XAML only in this slice.** Qt, Compose, and Flutter follow as
separate PRs, in the same order the underlying `[host_effects]`
mechanism itself landed in. SwiftUI is out of scope for this
environment (no Apple toolchain available).

**Out of scope, deliberately:**
- Live camera capture — a continuous stream doesn't fit the one-shot
  `Await` shape at all (`VIS00-vision-roadmap.md`'s own L4 section
  already makes this call: "a live-camera-preview effect is still
  genuinely new surface area"). Not a variant of this effect; a
  separate future spec if it's ever built.
- Multi-file selection — `FileOpenPicker.PickMultipleFilesAsync()` and
  equivalents exist on every target platform, but no consumer needs it
  yet (`qr-locate`'s eventual caller wants exactly one photo). Adding
  it later is a new, additive `multiple: true` request field, not a
  breaking change to what's specified here.
- Writing a file back out (`files.save`) — a real, separate capability
  with its own picker API and its own semantics (§8.3 of `UI47`
  already anticipates it exists eventually); not needed by anything in
  flight.

## 3. The contract

```
kind:     "files.open"
delivery: Await

Request payload:
  { "accept": ["image/jpeg", "image/png"] }   -- optional; omitted or
                                                  empty means "any file"

Result:
  ok:        { "name": "photo.jpg", "mimeType": "image/jpeg", "bytes": "<base64>" }
  cancelled: {}   -- the picker was dismissed; not an error
  failed:    { "message": "..." }
```

`accept` is a list of MIME type strings. A host maps them to whatever
its native picker's filter mechanism actually wants (WinUI 3's
`FileTypeFilter` takes extensions, not MIME types — the XAML handler
maintains its own small `image/jpeg` → `.jpg`/`.jpeg` table rather
than the app needing to know platform-specific filter syntax). An
unrecognised MIME type is dropped from the filter rather than failing
the request — a caller asking for a type this particular host doesn't
know how to filter for should still get a working (if less-filtered)
picker, not an error.

`bytes` is the whole file, base64-encoded — matching `UI47`'s existing
convention for `importAnki`'s package payload (`coding_adventures_base64`
is already a dependency for exactly this reason in `engram-mosaic-app`;
the same crate is reused here rather than a new encoder).

`name` is the picked file's own filename (for display — "Selected
photo.jpg"), not a path; no request in this contract ever receives or
needs a filesystem path from the app side, matching the sandboxed
nature of every target platform's real picker API.

## 4. XAML implementation

### 4.1 Where the handler lives

Per-app, like every other `[host_effects]` handler (`UI47` §5.5.2) —
there is no shared-library mechanism for handler *code* across
packages yet, only for the *contract* this spec documents. The
reference implementation lives in `photo-picker-app`'s own
`host/xaml/PhotoPickerEffects.cs`, structured so another app can copy
it near-verbatim (the same way Engram's own `engram_effects.*` family
across four backends is copy-adapted from Qt's, not shared code).

### 4.2 The handler shape

```csharp
// Namespace deliberately not `PhotoPickerApp` -- the generated
// component class is `Mosaic.Generated.PhotoPickerApp` (this package's
// own `component PhotoPickerApp`), and the generated `Install();` call
// site lives inside `namespace Mosaic.Generated`, where an unqualified
// `PhotoPickerApp` resolves to that class, not a same-named handler
// namespace. A real build hit exactly this (CS0117) before the rename.
namespace PhotoPickerHost;

public static class PhotoPickerEffects
{
    public static void Install()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!string.Equals(delivery, "await", StringComparison.OrdinalIgnoreCase)) return;
            if (kind != "files.open") return;
            if (!MosaicRuntimeHost.DeferEffect(id)) return;
            _ = PickAndCompleteAsync(id, payload);
        };
    }

    private static async Task PickAndCompleteAsync(ulong id, JsonElement payload) { ... }
}
```

`Install()` matches `UI47`'s XAML install contract exactly (`static
void Install()`, no arguments, sets `MosaicRuntimeHost.EffectHandler`
— confirmed against the generated `MosaicRuntimeHost.cs` template's
actual `Action<ulong, string, JsonElement, string>?` signature, not
assumed from the spec prose alone).

**Why `DeferEffect` before anything async.** `EffectHandler` is a
synchronous `Action` — it cannot itself `await` a picker result and
still return promptly, and `FileOpenPicker.PickSingleFileAsync()` is
necessarily async (it waits on user interaction, which can take an
unbounded amount of time). `MosaicRuntimeHost.DeferEffect(id)` takes
ownership of completing the effect later; the handler then fires an
async continuation (`_ = PickAndCompleteAsync(...)`, deliberately not
awaited from inside the synchronous handler) that calls
`MosaicRuntimeHost.CompleteEffect(id, outcome)` whenever the picker
task actually resolves — matching the pattern `UI47` §5.5.4 already
establishes for "an effect completion can produce more effects" (the
general shape of "answer now vs. answer later" the runtime is built
around).

**Owner window.** WinUI 3's `FileOpenPicker` needs an owner `HWND`
(`InitializeWithWindow.Initialize`) — confirmed against the legacy,
pre-`UI47` `engram-app` XAML host's own Anki-import picker code, the
one existing real usage of `FileOpenPicker` in this repo. `Install()`
takes no window argument (the `[host_effects]` contract doesn't
provide one) — and, checked directly against the generator
(`mosaic-emit-xaml`'s `emit_app_xaml_cs`/`emit_runtime_required_main_window_cs`),
the generated `App`/`MainWindow` don't expose a static window
reference either: `App._window` is a private instance field, and
`Install()` is spliced into `MainWindow`'s own constructor *before*
`InitializeComponent()` runs, so there is no live window to capture
even if one were exposed. Rather than extend the generator to add a
static accessor (real scope creep for a "first effect handler"
package), the handler calls the Win32 `GetForegroundWindow()` at the
moment the picker actually opens — well after startup, in direct
response to a user's click on this single-window app's own UI, at
which point that app's window *is*, by definition, the foreground
window. A cleaner long-term fix is a generator change exposing a
static window accessor; noted as follow-up, not required here.

**Error handling.** Any exception during the picker call or the file
read completes the effect as `failed`, mirroring Qt's
`installEngramEffects`'s own try/catch-into-`failedOutcome` pattern
(`UI47`'s worked Qt example) — the one already-proven error-handling
shape for a `[host_effects]` handler in this repo. The message is a
short, generic string rather than the raw exception's own — a real
`dotnet build` and the mandatory `/security-review` before push
(§6 gate 4) together caught that `.NET`'s own exception messages for
this case routinely embed the full local filesystem path, and
`failed.message` is app-visible data a future copy of this handler
could plausibly log or display remotely. `FileOpenPicker` being
dismissed (Escape, or the Cancel button) returns `null` from
`PickSingleFileAsync()`, not an exception — that path completes as
`cancelled`, not `failed`, matching §3's explicit "the picker was
dismissed; not an error."

**Size limit.** Before reading a picked file, the handler checks
`StorageFile.GetBasicPropertiesAsync().Size` against a 50 MiB cap and
completes the effect as `failed` if it's exceeded, rather than reading
an arbitrarily large file fully into memory and base64-encoding it
(the concern §6 gate 4 flags). This is a host-side cap, not part of
the wire contract in §3 — a future revision could add a request-side
`maxBytes` field if a caller ever needs a different limit.

## 5. Test strategy

- **Manifest/generation round trip** (Rust side, mirrors `mosaic-
  package-artifact-builder`'s existing XAML `[host_effects]` test
  coverage): given `photo-picker-app`'s real manifest declaring the
  XAML handler, the generated `MainWindow.xaml.cs` calls
  `PhotoPickerHost.PhotoPickerEffects.Install();` with no arguments,
  after `MosaicRuntimeHost.LoadRequired();`.
- **Package compile-check** (mirrors `task-app`/`engram-app`'s own
  `tests/package_compiles.rs`): the `.mil`/`.mll`/`.msl` sources
  compile, the manifest parses, and the manifest declares exactly the
  XAML `[host_effects]` handler this spec describes.
- **Real build**: emit the app with `--profile native-complete` and
  `dotnet build` the result. The known-benign `MSB4062` packaging-step
  failure (documented in `code/programs/csharp/hello-dialog-xaml/
  ISSUES.md` §C1 and `lessons.d/`) is expected and not a build
  failure for this purpose — the actual `.dll`/`.exe` compiling is
  the bar. Interactively exercising the native `FileOpenPicker` dialog
  itself is not something this session can automate (no way to drive
  a real Windows picker dialog from here); that gap is stated
  explicitly rather than silently skipped.

## 6. Acceptance gates

1. §5's manifest/generation and compile-check tests are green.
2. `dotnet build` of the emitted `native-complete` project succeeds
   (modulo the documented benign packaging-step error).
3. `cargo clippy -p photo-picker-mosaic-app --all-targets -- -D
   warnings` clean.
4. Security review passed before push — the file-open path handles
   arbitrary user-selected file bytes; size limits and MIME/extension
   validation are worth a specific look, since a base64-inflated
   multi-hundred-MB photo is a real caller-reachable resource cost.
