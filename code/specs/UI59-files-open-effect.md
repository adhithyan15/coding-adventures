# UI59 — `files.open`: a generic file/gallery-picker effect

## Status

XAML shipped (PR #15218, merged); Qt shipped in this revision. Depends
on `UI47` (host capability effects — `[host_effects]`, `Effect`,
`Delivery`, `mosaic_app_complete_effect`), which is fully implemented
and tested on all five native backends (Qt, SwiftUI, Compose, Flutter,
XAML). This spec adds no protocol surface — it is a *convention* for
one effect kind's `kind` string and payload/result shape, plus real
implementations of it per backend. Compose and Flutter follow in the
same order already used for `[host_effects]` itself; SwiftUI is out of
scope for this environment (no Apple toolchain).

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

**XAML and Qt this slice.** Compose and Flutter follow as separate
PRs, in the same order the underlying `[host_effects]` mechanism
itself landed in. SwiftUI is out of scope for this environment (no
Apple toolchain available).

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

## 6. Acceptance gates (XAML)

1. §5's manifest/generation and compile-check tests are green.
2. `dotnet build` of the emitted `native-complete` project succeeds
   (modulo the documented benign packaging-step error).
3. `cargo clippy -p photo-picker-mosaic-app --all-targets -- -D
   warnings` clean.
4. Security review passed before push — the file-open path handles
   arbitrary user-selected file bytes; size limits and MIME/extension
   validation are worth a specific look, since a base64-inflated
   multi-hundred-MB photo is a real caller-reachable resource cost.

## 7. Qt implementation

### 7.1 Where the handler lives

`photo-picker-app`'s own `host/qt/PhotoPickerEffects.{h,cpp}`, mirroring
Engram's Qt effect handler (`engram-app/host/qt/engram_effects.{h,cpp}`)
in structure — the one other real `[host_effects]` Qt handler in this
repo, and per UI47 §5.5.2 a package's own handler code, not a shared
library.

### 7.2 The handler shape

```cpp
void installPhotoPickerEffects(MosaicHost &host)
{
    QObject::connect(&host, &MosaicHost::effectRequested, &host,
        [&host](const QVariant &effectId, const QString &kind,
                const QVariant &payload, const QString &delivery) {
            if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
            if (kind != QStringLiteral("files.open")) return;
            try {
                handlePickPhoto(host, effectId, payload);
            } catch (...) {
                host.completeEffect(effectId, failedOutcome(QObject::tr("couldn't pick a photo")));
            }
        },
        Qt::DirectConnection);
}
```

`installPhotoPickerEffects` matches `UI47`'s Qt install contract exactly
(`void install(MosaicHost &host)`, connecting to `effectRequested` —
confirmed against the generated `MosaicHost.h` template's actual
`effectRequested(const QVariant &, const QString &, const QVariant &,
const QString &)` signal signature). Unlike XAML, Qt's manifest entry
*does* take an `include` (the header) — confirmed against
`qt_main_with_host_effects`, which requires an anchor
(`MosaicHost mosaicHost;`) to exist or hard-fails the build, then
splices `installPhotoPickerEffects(mosaicHost);` immediately after it
and, if `include` is set, `#include "PhotoPickerEffects.h"` before
`main(`.

**Why no `deferEffect`.** `QFileDialog::getOpenFileName` (the static
function used here, matching Engram's Qt `handleImport`/`handleExport`)
blocks synchronously — it runs a nested Qt event loop internally and
only returns once the user has picked a file or dismissed the dialog.
That means the effect can be answered with `completeEffect` inline,
before the lambda returns, under `Qt::DirectConnection` — no deferral
needed, unlike XAML's necessarily-async `PickSingleFileAsync()`. This
is a real choice, not a forced one (a non-blocking `QFileDialog::open()`
+ `deferEffect`/`answerDeferredEffect` pattern exists in the Qt
bindings and is exercised by a test driver, but has zero precedent in
any shipped `[host_effects]` handler); the blocking pattern was chosen
because it is the one actually proven in this repo (Engram's own Qt
handler), per "check for an existing standard first."

**`Qt::DirectConnection`, not queued.** A queued connection returns
from the emit immediately, so the host's own effect sweep would fail
the effect as unanswered before the dialog even opened — the same
reasoning `installEngramEffects`'s own comment documents.

**Filter syntax.** Qt's filter is one string,
`"Description (*.ext1 *.ext2)"` — a single parenthesized group of
space-separated globs — distinct from XAML's per-extension
`FileTypeFilter.Add(".ext")` list, so the handler carries its own
MIME→extension table and builds the Qt-shaped string from it, dropping
any `accept` MIME type it doesn't recognise (§3) rather than failing.

**Size limit, TOCTOU-hardened from the start.** A picked file is read
into memory and base64-encoded, exactly the resource-exhaustion
concern gate 4 (§6) and gate 4b (§9) both flag. Unlike the XAML
handler's *first* cut (which checked size once via
`GetBasicPropertiesAsync().Size` and was found TOCTOU-vulnerable by
`/security-review` on PR #15218 — the check and the read were separate
operations, so a file growing in between wasn't actually bounded), the
Qt handler reads in bounded 64 KiB chunks from the start and fails the
moment the running total exceeds 50 MiB (matching XAML's cap for
parity), so there is no gap between "checked" and "read" to begin
with.

**Error handling.** Like XAML (after its own security-review fix),
`failed.message` is a short, generic string — never a raw
`QFile::errorString()` or `std::exception::what()` — since that field
is app-visible data a future copy of this handler (this file is meant
to be copied near-verbatim, like Engram's own `*_effects.*` family) could
plausibly log or display remotely. This is a deliberate departure from
`installEngramEffects`'s own precedent (which does surface
`file.errorString()`/`error.what()` for its own, already-merged,
existing handler) — not a claim that code is wrong, just that this
newer handler applies the lesson `/security-review` already taught on
this same effect kind's XAML implementation.

## 8. Test strategy (Qt)

- **Manifest/generation round trip**: given `photo-picker-app`'s real
  manifest declaring the Qt handler, the generated `main.cpp` calls
  `installPhotoPickerEffects(mosaicHost);` after the `MosaicHost
  mosaicHost;` declaration and includes `PhotoPickerEffects.h`; the
  generated `CMakeLists.txt` lists both `PhotoPickerEffects.h` and
  `PhotoPickerEffects.cpp` under `target_sources`.
- **Package compile-check** (`tests/package_compiles.rs`, extended):
  the manifest declares exactly the Qt `[host_effects]` file and
  handler entries this spec describes, and the Qt source files exist
  and declare `installPhotoPickerEffects`/`effectRequested`/
  `"files.open"`.
- **Real build**: emit the app with `--profile native-complete` and
  build the generated `CMakeLists.txt` with Ninja + MSVC (the actual
  toolchain installed in this environment) — a full, unqualified
  success, unlike XAML's one documented benign packaging-step
  exception. Interactively exercising the native `QFileDialog` dialog
  itself is not something this session can automate, same limitation
  as §5's XAML entry.

## 9. Acceptance gates (Qt)

1. §8's manifest/generation and compile-check tests are green.
2. `cmake --build` of the emitted `native-complete` project succeeds
   with zero errors (no documented exception, unlike XAML's gate 2).
3. `cargo clippy -p photo-picker-mosaic-app --all-targets -- -D
   warnings` clean (the Rust app crate is backend-agnostic and
   unchanged by this addition).
4. Security review passed before push, applying the same scrutiny
   §6 gate 4 already required for XAML — this time with the TOCTOU
   lesson from that review applied from the first draft rather than
   fixed after the fact.
