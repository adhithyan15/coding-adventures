# UI29-1 — `HostDialog` kernel primitive

> **Status.** Draft, accompanies the first cross-backend implementation
> pass (`U29-1-K-react`).
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Scope.** Defines the sixteenth kernel primitive, `HostDialog`, and
> pins down its slot/emit surface so every Mosaic backend lowers it
> identically.

---

## 1. Why this primitive belongs in the kernel

A modal/popover dialog is a textbook UI29 §2.2 kernel candidate:

1. **Every host platform has a native equivalent.** HTML has the
   `<dialog>` element (since Chrome 37, Safari 15.4, Firefox 98 — all
   long shipped); SwiftUI has `.sheet(...)` and `.alert(...)`; Qt/QML
   has `Dialog { ... }`. Each platform pulls the dialog out of the
   normal in-flow layout, layers it on a top z-stack (or in HTML's
   "top layer"), and renders an OS-correct backdrop. None of that is
   feasible to re-build from boxes and z-indices — the browser's `::backdrop`
   pseudo-element alone cannot be polyfilled, focus-trap and scroll-lock
   are accessibility-critical, and the screen reader integration is
   not optional.
2. **No reasonable composition exists.** A `<div>`-based modal
   re-implementation cannot reach the top layer, cannot route
   keyboard events correctly while the rest of the document is
   inert, and cannot be made screen-reader-equivalent to a real
   `<dialog>`. Userland decoration (custom headers, themed close
   buttons, slide animations) composes *into* `HostDialog`, not
   in place of it.

Adding `HostDialog` brings the kernel to 16 primitives. UI29 §2.4
explicitly allows the kernel to grow "slowly — perhaps one or two
primitives per year — and never shrink"; this is the first such
growth since the freeze.

## 2. Slot/emit surface

| moslayout prop          | Kind       | Required | Meaning                                           |
|---|---|---|---|
| `open`                  | slot ref   | yes      | boolean — when `true`, the dialog is visible      |
| `modal`                 | keyword    | no       | `true` → modal (top layer + backdrop), `false` → non-modal popover. Default `true`. |
| `title`                 | slot ref   | no       | text — rendered as a heading first-child          |
| `dismiss-on-backdrop`   | keyword    | no       | `true`/`false`. Default `true`. When `false`, backdrop click / Escape does not close. |
| `onOpen`                | emit ref   | no       | fires when `open` transitions `false → true`      |
| `onClose`               | emit ref   | no       | fires when the dialog closes (close button, Escape, programmatic) |

`HostDialog` accepts **children**; they render inside the dialog after
the optional title heading.

## 3. Per-backend lowering

### 3.1 React (`mosaic-emit-react`)

```tsx
const dialogRef_<n> = useRef<HTMLDialogElement>(null);
useEffect(() => {
  const d = dialogRef_<n>.current;
  if (!d) return;
  if (<open>) {
    if (<modal>) d.showModal(); else d.show();
  } else {
    d.close();
  }
}, [<open>]);

return (
  <dialog
    ref={dialogRef_<n>}
    onClose={() => dispatch({ type: "<close-event>" })}
    onCancel={e => e.preventDefault()}   // only when dismiss-on-backdrop: false
  >
    <h2>{<title>}</h2>                    {/* only when title slot present */}
    {/* children walked normally */}
  </dialog>
);
```

* Each `HostDialog` in a component gets a stable `dialogRef_<n>` name
  (counter is per-function, assigned in source order).
* When *any* `HostDialog` is present in the layout, the emitted file
  imports `useRef` and `useEffect` from React in addition to whatever
  `import React` line the existing namespace-detection logic decides
  on.
* `modal: true | false` is a compile-time choice: the emitter inlines
  literally `d.showModal()` or `d.show()` and does not carry the
  keyword into runtime.
* `dismiss-on-backdrop: false` adds an `onCancel={e => e.preventDefault()}`
  attribute that swallows the dialog's `cancel` event (Escape key,
  backdrop click) before the browser closes the dialog.
* `onOpen` is fired from inside the `useEffect`, in the same branch
  that calls `showModal()` / `show()` — see §3.1.1.
* `onClose` becomes the `onClose` attribute on the `<dialog>`.

#### 3.1.1 `onOpen` placement

When the `open` slot transitions `false → true`, the dialog needs to
both *appear* and *fire `onOpen`* in the same tick. The simplest shape
that satisfies React's effect timing is:

```tsx
useEffect(() => {
  const d = dialogRef_<n>.current;
  if (!d) return;
  if (<open>) {
    if (<modal>) d.showModal(); else d.show();
    dispatch({ type: "<open-event>" });
  } else {
    d.close();
  }
}, [<open>]);
```

The `dispatch` fires every time `open` is observed truthy, which
matches the "fires when the dialog opens" semantics (one dispatch per
open, not one dispatch per render).

### 3.2 SwiftUI / Qt (deferred)

Out of scope for `U29-1-K-react`. The lowering shapes are sketched in
UI29 §2.1 row for `HostDialog` and will be tightened in `U29-1-K-swiftui`
and `U29-1-K-qt`.

## 4. Validation surface

For the React backend:

1. Empty `HostDialog` emits a `<dialog>` + a `useRef` + a `useEffect`.
2. `open: slot: x` puts `x` in the effect dependency array.
3. `modal: true` (or no `modal:` prop) calls `showModal()`.
4. `modal: false` calls `show()`.
5. `onClose: emit: onX` wires the `<dialog>`'s `onClose` to
   `dispatch({ type: "x" })`.
6. Children render normally inside the dialog.
7. `title: slot: t` adds an `<h2>{t}</h2>` first child.
8. `dismiss-on-backdrop: false` adds the `onCancel` preventDefault
   handler.
9. The presence of any `HostDialog` adds `useRef`/`useEffect` to the
   React imports.
