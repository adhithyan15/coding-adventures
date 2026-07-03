# mosaic-pkg-dialog

> A cross-backend Dialog component — title, message, close button —
> built as a thin wrapper around the `HostDialog` kernel primitive
> added by UI29-1.

This is the **canonical userland example** of how a v0.2-era Mosaic
package should look: it does as little as possible itself and hands the
heavy semantic lifting (modal blocking, focus trap, Esc-to-close,
top-layer rendering, screen-reader `dialog` role) off to the platform's
native dialog primitive via the kernel.

## Why v0.2.0 is a rewrite, not a patch

v0.1.0 of this package faked a dialog out of `Box + Column + Text +
HostButton`.  Per UI29-1 §1, that composition could never deliver:

- **Modal blocking** — clicks outside the fake dialog continued to
  reach the background UI.
- **Focus trap** — `Tab` escaped into the page; screen readers
  wandered out.
- **`Esc`-to-close** — no native handler; required a global keydown
  binding.
- **Top-layer rendering** — z-index inheritance meant the dialog sat
  *below* anything with a higher z-index.
- **`dialog` accessibility role** — a `<div>` carries no such role; a
  native `<dialog>` / `Popup` / `.sheet` / `ContentDialog` does.

UI29-1 added `HostDialog` to the kernel so userland could stop faking
it.  v0.2.0 of this package is that rewrite.

## What this package exports

One component, listed in `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Dialog` | Title + message + close button | `Dialog.mil` / `Dialog.mll` / `Dialog.dark.msl` |

## The interface

```mil
component Dialog {
  slot open        : bool ;
  slot title       : text ;
  slot message     : text ;
  slot close-label : text ;

  emit onClose ;
}
```

Four slots and one zero-payload event.  The host:

1. Sets `title`, `message`, `close-label` as static strings (or
   bindings).
2. Flips `open` to `true` when the dialog should appear and back to
   `false` when it should disappear.
3. Listens for `onClose` and flips `open` back to `false` in response.

`onClose` fires on every dismiss path: Esc keypress, backdrop click,
close-button click, or the host setting `open` back to `false`.

## The layout

```mll
layout Dialog {
  HostDialog [ dialog-shell ] (
    open    : slot: open ,
    modal   : true ,
    title   : slot: title ,
    onClose : emit: onClose
  ) {
    Column [ dialog-stack ] {
      Box [ dialog-message ] {
        Text ( content: slot: message )
      }
      Box [ dialog-actions ] {
        HostButton (
          label: slot: close-label ,
          onTap: emit: onClose
        )
      }
    }
  }
}
```

Three parts (`dialog-shell`, `dialog-message`, `dialog-actions`) for
hosts to theme.  Three primitives total beyond the kernel root:

| Primitive  | Why this one |
|---|---|
| `HostDialog` | Native dialog element — `<dialog>` / `.sheet` / `Popup` / `ContentDialog` |
| `Column`     | Vertical stack across message → actions |
| `Box`        | Stylable container — owns the `part` annotations |
| `Text`       | Static read-only label binding (message) |
| `HostButton` | Native clickable with platform-appropriate semantics |

`modal: true` is a compile-time keyword (UI29-1 §2.1) — backends pick
the modal flavor of their dialog primitive at lowering time
(`showModal()` on React, `.sheet` on SwiftUI, `Popup.modal: true` on
Qt).

## How it fits in the stack

```
              ┌────────────────────────────────────────────┐
              │  Host application (.mll uses `Dialog`)     │
              └─────────────────────┬──────────────────────┘
                                    │ component reference
                                    ▼
              ┌────────────────────────────────────────────┐
              │  mosaic-pkg-dialog (this package)          │
              │  Dialog → HostDialog + Column + Box +      │
              │           Text + HostButton                │
              └─────────────────────┬──────────────────────┘
                                    │ kernel primitives only
                                    ▼
              ┌────────────────────────────────────────────┐
              │  UI29 kernel (16 primitives w/ UI29-1)     │
              └─────────────────────┬──────────────────────┘
                                    │ per-backend lowering
                                    ▼
        React / SwiftUI / Qt / WebComponent / HTML / XAML
```

## Per-backend lowering of HostDialog

| Backend | Native element | `open` mechanism | `modal: true` flavor |
|---|---|---|---|
| React | `<dialog ref={...}>` | `useEffect` calls `showModal()` / `close()` | `showModal()` (top layer + `::backdrop`) |
| SwiftUI | `.sheet(isPresented:)` | `Binding<Bool>` | `.sheet` (default modal) |
| Qt | `Popup { visible: open }` | Direct binding | `modal: true` |
| WebComponent | `<dialog>` in shadow root | Same as React | `showModal()` |
| HTML | `<dialog open>` attribute | Presence attribute | inline `dialog.showModal()` |
| XAML | `<ContentDialog IsOpen=...>` | Two-way binding | `ContentDialog` is always modal |

See `code/specs/UI29-1-host-dialog.md` §3 for the full per-backend
contract.

## Parts vocabulary

```mosstyle
style Dialog {
  part dialog-shell   { /* native dialog element */ }
  part dialog-message { /* body text row */ }
  part dialog-actions { /* close-button row */ }
}
```

The `dialog-shell` part is styled by the platform's own dialog-element
styling hook (DOM `dialog::part(dialog-shell)`, Qt `Overlay.modal` /
`background:`, SwiftUI sheet content modifier, XAML
`ContentDialogStyle`).  v0.1.0's `dialog-title` part is gone —
HostDialog renders the title natively and authors theme it through
the platform's own title styling.

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-dialog
cargo test
```

The smoke test asserts:

1. `mosaic-package.toml` parses and declares exactly
   `exports = ["Dialog"]` at version `0.2.0`.
2. `Dialog.mil` compiles via `mosmodel-compiler` (4 slots — including
   the new `open: bool` — and 1 emit).
3. `Dialog.mll` compiles via `moslayout-compiler` and has
   `HostDialog` as the layout root, with `dialog-shell` as the part
   name, a `Column [dialog-stack]` child, and the expected
   `dialog-message` / `dialog-actions` descendants.
4. `Dialog.dark.msl` compiles via `mosstyle-compiler` and declares
   exactly three parts (`dialog-shell`, `dialog-message`,
   `dialog-actions`).
5. The per-backend artifact builder produces a non-empty Dialog
   artifact for React, SwiftUI, and Qt.  A backend whose HostDialog
   lowering has not yet landed on main returns a `PipelineError`; the
   test records that as **deferred** (a documented expected-failure
   state) and does *not* mark the run red.  WebComponent / HTML / XAML
   either return `UnsupportedBackend` or aren't in the builder's enum
   yet and are skipped with the same rationale as v0.1.0.

## Position in UI29's roadmap

* **UI29-1-G** — `moslayout-compiler`: add `"HostDialog"` to
  `PRIMITIVES`.
* **UI29-1-R** — `mosaic-package-resolver`: add `"HostDialog"` to
  `KERNEL_PRIMITIVES`.
* **UI29-1-K-*** — each backend's HostDialog lowering (React,
  SwiftUI, Qt, WebComponent, HTML, XAML).
* **UI29-1-P** — *this PR.*  Rewrite `mosaic-pkg-dialog` on top of
  HostDialog.

The K-* PRs run in parallel; this P PR fans in.

## License

MIT OR Apache-2.0.
