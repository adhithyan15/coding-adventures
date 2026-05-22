# UI29-1 — `HostDialog` kernel primitive

**Status:** Specification (draft)
**Layer:** UI / cross-cutting (moslayout kernel addition + every backend emitter)
**Depends on:** UI29 (primitive kernel), UI24 (emit→dispatch)
**Extends:** UI29 §2 — adds a sixteenth kernel primitive.

---

## 1. Why this exists

UI29 froze a 15-primitive kernel: enough to express layout, leaf
content, two interactive host widgets (`HostInput`, `HostButton`), and
a semantic data table. What it deliberately omitted — and listed in
§2.3 as "not in the kernel" — was the dialog / modal pattern, calling
out that it composes from `Stack` + portal mechanism + a "host service
TBD".

Three demos in flight (visicalc save-as, the OpenClaw character-sheet
mock, and the adjudication-evidence preview pane) need a modal dialog.
Each of them attempted to compose one from `Stack` + `Box` and
discovered the same three holes:

1. **Native semantics.** Browsers, SwiftUI, Qt, and Compose all
   provide a *native* dialog primitive (`<dialog>`, `.sheet`,
   `Dialog { ... }`, `Dialog`). Screen readers, keyboard focus traps,
   and z-order stacking integrate with the native primitive in ways
   that a `Stack`+`Box` composition cannot replicate. This is the same
   accessibility argument UI29 §2.2 used for `HostTable`.
2. **Open/close state.** A `Stack`+`Box` composition has to render its
   own conditional (`If when: open`) and rely on the host to manage
   the bool. The native primitives all accept the open/close state as
   a binding and own the show/hide animation themselves.
3. **Backdrop and dismissal.** Click-outside-to-close, escape-key,
   focus restoration on dismiss — every native primitive provides
   these as flags. Re-implementing them in moslayout would mean
   shipping focus-management primitives no userland package needs.

`HostDialog` is therefore a kernel addition under UI29-1, the first
expansion of the UI29 kernel.

---

## 2. Inclusion criteria check (UI29 §2.2)

1. **Every host platform has a native equivalent.** Yes: DOM
   `<dialog>`, SwiftUI `.sheet` / `.popover`, Qt/QML `Dialog`,
   Compose `Dialog`.
2. **No reasonable composition exists.** A `Stack`+`Box` composition
   cannot replicate focus trap, backdrop blocking, native z-order, or
   accessibility role semantics.
3. **Semantically irreducible.** Screen readers announce `<dialog>`
   with role="dialog" and treat its content as modal context. A fake
   dialog made of divs does not get this announcement; this is a
   first-class accessibility regression.

All three criteria are met.

---

## 3. Grammar surface

`HostDialog` is a kernel primitive. Its tag, like every other UI29
primitive, parses through the existing moslayout primitive-tag rule —
no grammar additions.

```moslayout
HostDialog (
  open: slot: dialog-open,
  modal: true,
  title: "Save changes?",
  dismiss-on-backdrop: false,
  onClose: emit: onDialogClose,
) {
  Column {
    Text (content: "Are you sure you want to save?")
    Row {
      HostButton (label: "Cancel", onTap: emit: onCancel)
      HostButton (label: "Save",   onTap: emit: onSave)
    }
  }
}
```

### 3.1 Props

| Prop                    | Type         | Default       | Meaning                                       |
|-------------------------|--------------|---------------|-----------------------------------------------|
| `open`                  | slot: bool   | required      | Drives the open/close state.                  |
| `modal`                 | keyword      | `true`        | `true` → modal sheet; `false` → popover.      |
| `title`                 | string \| slot | none        | Optional dialog title.                        |
| `dismiss-on-backdrop`   | keyword      | `true`        | When `false`, backdrop click does not close.  |
| `onClose`               | emit ref     | none          | Fired when the host dismisses the dialog.     |

### 3.2 SwiftUI lowering

SwiftUI exposes dialog-style presentation as a **view modifier**
(`.sheet(isPresented:onDismiss:content:)` or
`.popover(isPresented:content:)`) attached to a parent view, not as a
standalone view. Because the UI29 kernel emitter walks the layout tree
as standalone view nodes, the simplest implementation is to emit an
invisible *anchor* (`Color.clear.frame(width: 0, height: 0)`) that
carries the modifier. The dialog children become the modifier's
content closure.

Shape:

```swift
Color.clear.frame(width: 0, height: 0)
    .sheet(isPresented: $open, onDismiss: { dispatch(.dialogClose) }) {
        VStack {
            // children
        }
        .navigationTitle("Save changes?")          // if title bound
        .interactiveDismissDisabled(true)          // if dismiss-on-backdrop: false
    }
```

| Moslayout prop                  | SwiftUI                                                |
|---------------------------------|--------------------------------------------------------|
| `open: slot: x`                 | `isPresented: .constant(x)` *(see binding note)*       |
| `modal: true` (default)         | `.sheet(...)`                                          |
| `modal: false`                  | `.popover(...)`                                        |
| `title: "..."` / `title: slot:` | `.navigationTitle("...")` inside content closure       |
| `dismiss-on-backdrop: false`    | `.interactiveDismissDisabled(true)` inside closure     |
| `onClose: emit: onX`            | `onDismiss: { dispatch(.x) }` *(sheet only)*           |

#### 3.2.1 Binding choice — `.constant(x)`

SwiftUI bindings need mutable state, but Mosaic components receive
slots as immutable `let`s. We follow the same `.constant(value)`
pattern `HostInput` uses (UI29-K-swiftui §HostInput binding choice):
the host owns close-via-dispatch through the UI24 Flux loop. A
`@State` proxy that lifts the slot into mutable local state is a
documented future enhancement.

#### 3.2.2 `.popover` and `onClose`

SwiftUI's `.popover(isPresented:content:)` does **not** accept an
`onDismiss:` closure. When `modal: false`, the emitter still wires the
content closure but omits the `onDismiss` handler — the host should
observe its own `open` slot change and dispatch the close event itself.

### 3.3 Other backend lowerings

- **DOM / React**: `<dialog open={open}>` with the children inside;
  `modal: true` calls `.showModal()`, `modal: false` calls `.show()`.
- **Qt / QML**: `Dialog { visible: open; modal: modal; ... }`.
- **WebComponent**: shadow-root `<dialog>` mirroring the DOM lowering.
- **HTML (static)**: emit a `<dialog>` element; runtime opens it.

These are out of scope for this spec — each backend gets its own
implementation PR (U29-1-K-react, U29-1-K-qt, etc.).

---

## 4. Tests (SwiftUI backend)

The U29-1-K-swiftui PR ships at least eight tests:

1. Empty `HostDialog` emits a `Color.clear` anchor + `.sheet`.
2. `open: slot: x` references `x` as the binding.
3. `modal: true` uses `.sheet(...)`.
4. `modal: false` uses `.popover(...)`.
5. Children render inside the content closure's `VStack`.
6. `onClose` wires the `onDismiss` callback.
7. `title: slot: t` emits `.navigationTitle(t)`.
8. `dismiss-on-backdrop: false` emits `.interactiveDismissDisabled(true)`.

---

## 5. Migration

This is a kernel addition. No existing demo uses `HostDialog`; no
migration is required. Future demos that need a dialog stop composing
`Stack`+`Box` and use `HostDialog` directly.

---

## 6. Open questions (resolved in implementation PRs)

1. **Multiple stacked dialogs.** A `HostDialog` inside another
   `HostDialog`'s content closure should "just work" with `.sheet`, but
   SwiftUI's stacking rules for nested sheets are platform-specific.
   Pin in the test suite after the first dialog-stacking demo ships.
2. **`title` slot binding for non-NavigationStack hosts.**
   `.navigationTitle` only renders inside a `NavigationStack` parent;
   for free-standing sheets the title needs to be rendered as a
   `Text(...).font(.headline)` at the top of the content. Implementer
   may emit both (the navigation modifier is a no-op outside a stack).
