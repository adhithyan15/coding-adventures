# UI29-1 — HostDialog kernel primitive

**Status:** Specification (amendment)
**Layer:** UI / cross-cutting (kernel primitive vocabulary + every backend emitter)
**Amends:** UI29 (primitive kernel)
**Supersedes (partial):** `mosaic-pkg-dialog` v0.1.0, which composed a fake dialog from Box + Column + Text + HostButton

---

## 1. Why this exists

The v0.1.0 `mosaic-pkg-dialog` (PR #3751) was built without a kernel
`HostDialog` because we hadn't yet validated whether dialogs met the
UI29 §2.2 kernel-inclusion criteria. They do — and the v0.1.0 package
demonstrates exactly why.

A composed-from-`Box` dialog cannot provide:

- **Modal blocking.** Clicks outside the dialog continue to reach the
  background UI. Real modal behavior needs the platform's top-layer /
  popup / sheet primitive.
- **Focus trap.** `Tab` escapes the dialog into the background; screen
  readers can wander out. Native dialogs trap focus to the dialog's
  descendants until close.
- **`Esc`-to-close.** Native dialogs handle this without any author
  wiring; a composed dialog has to bind a global keydown handler.
- **Top-layer rendering.** A `<div>`-based dialog inherits its
  ancestor's `z-index` stack — it sits *below* anything with a higher
  z-index. Native `<dialog>` (DOM), `Popup` (Qt), `.sheet` (SwiftUI),
  `ContentDialog` (XAML) all render in a platform-level top layer that
  is always above page content.
- **Accessibility role.** Screen readers announce a `<dialog>` /
  `Popup` / `ContentDialog` as "dialog" because the host primitive
  carries the `dialog` accessibility role (and `aria-modal` when modal).
  A composed `<div>` carries no such semantics.

Every host platform Mosaic targets ships a dedicated dialog primitive
with these properties built in. Per UI29 §2.2:

1. ✓ Every host has a native equivalent.
2. ✓ No reasonable composition exists — the modal/focus/top-layer/
   accessibility semantics are not expressible in lower primitives.
3. ✓ Semantically irreducible — screen-reader users *require* the
   dialog role to be present *as* a dialog, not as a div.

Therefore `HostDialog` belongs in the kernel.

---

## 2. The primitive

### 2.1 Slots

| Slot | Type | Default | Effect |
|---|---|---|---|
| `open` | `bool` | `false` | Visibility — when `true` the dialog is presented |
| `modal` | `bool` (literal) | `true` | Compile-time keyword. `true` → modal (focus trap + backdrop + top-layer); `false` → non-modal popover. |
| `title` | `text` | `""` | Optional title; the host primitive's title slot when one exists |
| `dismiss-on-backdrop` | `bool` (literal) | `true` | Whether clicking the backdrop closes the dialog. Compile-time keyword. |

The `modal` and `dismiss-on-backdrop` keywords are compile-time
constants (same pattern as `Grid`'s `sticky-header`) so backends can
choose the matching API at lowering time — e.g. React selects
`showModal()` vs `show()`, Qt selects `Popup`'s `modal: true` vs
`false`, SwiftUI selects `.sheet` vs `.popover`.

### 2.2 Emits

| Emit | Payload | When |
|---|---|---|
| `onOpen` | (none) | The dialog has transitioned from closed to open. Fires once per open. |
| `onClose` | (none) | The dialog has transitioned from open to closed. Fires once per close. Triggers: user pressed Esc, clicked backdrop (when `dismiss-on-backdrop: true`), clicked a child element that emitted `onClose`, or `open` slot flipped to `false`. |

### 2.3 Children

`HostDialog` takes arbitrary kernel-primitive children that render as
the dialog's body. The host inserts them inside the platform's dialog
element (`<dialog>`'s children, `Popup.contentItem`'s children, the
trailing closure on `.sheet`, `ContentDialog.Content`, etc.).

### 2.4 Sub-parts

| Sub-part | Targets |
|---|---|
| `dialog` | The outer dialog element (`<dialog>`, `Popup`, `.sheet` content view, `ContentDialog`) |
| `dialog:open` | While `open: true` |
| `dialog/backdrop` | The backdrop (DOM `<dialog>::backdrop`, Qt `Popup.Overlay`, SwiftUI fullScreenCover dim, XAML `ContentDialog`'s background scrim) |

Authors style the backdrop via the sub-part path the same way they
style any other sub-part (UI27 §3.1).

---

## 3. Backend lowerings

### 3.1 React (`mosaic-emit-react`)

```tsx
<dialog
  ref={dialogRef}
  /* effect: if (open) dialogRef.current?.showModal() else dialogRef.current?.close() */
  onClose={() => dispatch({ type: "close" })}
>
  {children}
</dialog>
```

- `modal: true` → `showModal()` (top layer + `::backdrop` element).
- `modal: false` → `show()`.
- `dismiss-on-backdrop` wired via a backdrop click handler that calls
  `dialogRef.current?.close()`.
- `open` slot drives a `useEffect` that calls `showModal()` / `close()`.

### 3.2 SwiftUI (`mosaic-emit-swiftui`)

```swift
content
  .sheet(isPresented: $open, onDismiss: { dispatch(.close) }) {
    // children
  }
```

- `modal: true` → `.sheet(...)` (default modal).
- `modal: false` → `.popover(...)`.
- `dismiss-on-backdrop` on SwiftUI is `interactiveDismissDisabled(!dismissOnBackdrop)`.

The `content` host's existing view body sits to the left of the
modifier; the dialog's children fill the closure.

### 3.3 Qt/QML (`mosaic-emit-qt`)

```qml
import QtQuick.Controls 2.15

Popup {
    modal: true
    visible: open
    closePolicy: dismissOnBackdrop ? Popup.CloseOnEscape | Popup.CloseOnPressOutside : Popup.NoAutoClose
    onClosed: close()    // signal call
    contentItem: ColumnLayout { /* children */ }
}
```

- `modal: true` → `Popup.modal: true` (focus trap + background dim).
- `modal: false` → `Popup.modal: false` (popover-style).
- `Popup`'s `Overlay.modal` slot is the styling hook for the backdrop sub-part.

### 3.4 WebComponent (`mosaic-emit-webcomponent`)

Same shape as React but inside the shadow DOM. The `<dialog>` element
is in the shadow root; `this.dispatch({type:"close"})` replaces React's
`dispatch` prop.

### 3.5 HTML (`mosaic-emit-html`)

```html
<dialog id="..." {{open ? "open" : ""}}>
  {{children}}
</dialog>
<script>
  // tiny inline: when modal=true, dialog.showModal() on connect; otherwise dialog.show()
</script>
```

- Static HTML cannot wire `onClose` to a Mosaic dispatch (no JS
  runtime by design). The emitter drops the emit with a comment.
- The browser still handles Esc and top-layer modality natively for
  free.

### 3.6 XAML / WinUI (`mosaic-emit-xaml`)

```xml
<ContentDialog
  Title="{Binding title}"
  IsOpen="{Binding open}"
  Closed="OnClose">
  <!-- children -->
</ContentDialog>
```

- `modal` is implicit (ContentDialog is always modal); `modal: false`
  on XAML lowers to a `Flyout` instead.
- `Closed` event dispatches `onClose`.

### 3.7 Paint-VM (`mosaic-emit-paint`)

Future. A `PaintHostDialogInstruction` is added to the IR; the
runtime layer renders it as a centered top-layer rect with a backdrop
fill and forwards keyboard events.

---

## 4. Inclusion criteria revisit (UI29 §2.2)

The original UI29 §2.2 criteria, applied:

| Criterion | HostDialog |
|---|---|
| Every host platform has a native equivalent | ✓ React `<dialog>`, SwiftUI `.sheet`, Qt `Popup`, HTML `<dialog>`, WebComponent `<dialog>`, XAML `ContentDialog` |
| No reasonable composition exists | ✓ Modal blocking + focus trap + top-layer + accessibility role are not expressible at the layout level |
| Semantically irreducible | ✓ Screen readers require the dialog role to be present as a dialog |

HostDialog therefore satisfies all three. It is the **16th kernel
primitive**. The kernel as of UI29-1:

```
Box  Row  Column  Stack
Text  Image  Spacer  Divider  Icon
If  Else  For
HostInput  HostButton  HostTable  HostScroll  HostDialog
```

---

## 5. Migration: `mosaic-pkg-dialog` v0.1.0 → v0.2.0

The userland `mosaic-pkg-dialog` (PR #3751) is rewritten on top of
`HostDialog`:

```moslayout
// v0.2.0
component Dialog {
  slot open: bool;
  slot title: text;
  slot message: text;
  slot close-label: text;
  emit onClose();
}

layout Dialog {
  HostDialog [dialog-shell] (
    open: slot: open,
    modal: true,
    title: slot: title,
    onClose: emit: onClose,
  ) {
    Column [dialog-stack] {
      Box [dialog-message] {
        Text (content: slot: message)
      }
      Box [dialog-actions] {
        HostButton (label: slot: close-label, onTap: emit: onClose)
      }
    }
  }
}
```

v0.2.0 gains: a real `open: bool` slot (replacing v0.1.0's
host-controlled visibility), modal behavior, focus trap, Esc-to-close,
top-layer rendering, screen-reader announcement. The mosstyle file
keeps the same parts but the host `dialog-shell` part now styles the
native dialog element directly via the platform's `Popup`/`<dialog>`
styling hook.

The userland code shrinks (no more `dialog-root` wrapper Box — the
`HostDialog` IS the root) and the behavior gets dramatically richer.

---

## 6. Implementation roadmap

| ID | Work | Depends on |
|---|---|---|
| U29-1-G | `moslayout-compiler`: add `"HostDialog"` to `PRIMITIVES` | — |
| U29-1-R | `mosaic-package-resolver`: add `"HostDialog"` to `KERNEL_PRIMITIVES` | — |
| U29-1-K-react | `mosaic-emit-react`: `<dialog>` lowering | U29-1-G |
| U29-1-K-swiftui | `mosaic-emit-swiftui`: `.sheet` lowering | U29-1-G |
| U29-1-K-qt | `mosaic-emit-qt`: `Popup` lowering | U29-1-G |
| U29-1-K-html | `mosaic-emit-html`: `<dialog>` lowering | U29-1-G |
| U29-1-K-webcomp | `mosaic-emit-webcomponent`: `<dialog>` lowering | U29-1-G |
| U29-1-K-xaml | `mosaic-emit-xaml`: `ContentDialog` lowering | U29-1-G |
| U29-1-P | `mosaic-pkg-dialog` v0.2.0: rewrite on top of HostDialog | U29-1-K-react + U29-1-K-swiftui + U29-1-K-qt (at minimum) |

All K-* PRs run in parallel. P fans in.

---

## 7. Open questions (resolved in implementation PRs)

1. **Where does the `open: bool` slot live in the cross-backend
   model?** React uses a ref + `useEffect`. SwiftUI uses a `Binding`.
   Qt binds it directly. HTML uses a presence attribute. The kernel
   primitive's *contract* is the slot semantic; backends decide the
   mechanism. Each K-PR implements its own pattern.

2. **Nested dialogs.** Spec is silent for v1. Most platforms allow
   them (React `<dialog>` stacks via top layer; Qt `Popup` stacks via
   z); UI29-2 can revisit if it surfaces real bugs.

3. **Animation.** Out of scope for the kernel primitive — backends use
   their platform's default open/close animation. A future
   `.transition` or `.animation` modifier can ride atop the existing
   sub-part vocabulary.

4. **Form-submission integration** (React `<dialog>` + `<form
   method="dialog">`). Out of scope; userland Form composition handles
   it.

---

## 8. What this spec does *not* do

- Does not change UI29's kernel-versioning policy: HostDialog is added
  via a numbered amendment (UI29-1), the kernel surface grows from 15
  to 16 primitives, and it remains frozen thereafter unless another
  numbered amendment ships.
- Does not deprecate the v0.1.0 `mosaic-pkg-dialog`. The v0.1.0
  composition still compiles and renders — it just lacks the native
  semantics. v0.2.0 supersedes it; consumers update at their pace.
- Does not opine on visual design. Backends use their platform default
  styling; authors style via the `dialog` sub-part.
