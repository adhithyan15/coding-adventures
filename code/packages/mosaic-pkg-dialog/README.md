# mosaic-pkg-dialog

> A simple cross-backend Dialog component — title, message, close button —
> built on UI29 kernel primitives.

This is the **cross-backend smoke test** for the Mosaic kernel.  A
single userland component that compiles cleanly through *every* Mosaic
emitter on `main` today: React, SwiftUI, Qt, WebComponent, HTML, and
XAML.  If a kernel change breaks any of those backends, Dialog is the
fastest way to find out.

## Why this package exists

`mosaic-pkg-grid` exercises the rich end of the kernel — `For`, `If`,
`HostTable` and its sub-tags, userland component composition, expression
lowering — and is therefore gated on a long tail of in-flight PRs
landing across all six backends.  Dialog goes the other way: it touches
*only* the four primitives that are already present in every backend
today, so it can act as a tripwire for kernel regressions without
waiting on any of those PRs.

## What this package exports

One component, listed in `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Dialog` | Title + message + close button overlay | `Dialog.mil` / `Dialog.mll` / `Dialog.dark.msl` |

## The interface

```mil
component Dialog {
  slot title       : text ;
  slot message     : text ;
  slot close-label : text ;

  emit onClose ;
}
```

Three text slots and one zero-payload event.  Hosts set the three
strings and listen for `onClose`; everything else is layout and style.

## The layout

```mll
layout Dialog {
  Box [ dialog-root ] {
    Column [ dialog-stack ] {
      Box [ dialog-title ]   { Text ( content: slot: title   ) }
      Box [ dialog-message ] { Text ( content: slot: message ) }
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

Four parts (`dialog-root`, `dialog-title`, `dialog-message`,
`dialog-actions`) for hosts to theme.  Four primitives total:

| Primitive  | Why this one |
|---|---|
| `Box`        | Stylable container — owns the `part` annotations |
| `Column`     | Vertical stack across title → message → actions |
| `Text`       | Static read-only label binding (title, message) |
| `HostButton` | Native clickable with platform-appropriate semantics |

**No `If`. No `For`. No `HostTable`.** Those primitives exist in some
backends but not yet in all — see "primitive availability" below.

## How it fits in the stack

```
              ┌────────────────────────────────────────────┐
              │  Host application (.mll uses `Dialog`)     │
              └─────────────────────┬──────────────────────┘
                                    │ component reference
                                    ▼
              ┌────────────────────────────────────────────┐
              │  mosaic-pkg-dialog (this package)          │
              │  Dialog → Box + Column + Text + HostButton │
              └─────────────────────┬──────────────────────┘
                                    │ kernel primitives only
                                    ▼
              ┌────────────────────────────────────────────┐
              │  UI29 kernel (15 primitives)               │
              └─────────────────────┬──────────────────────┘
                                    │ per-backend lowering
                                    ▼
        React / SwiftUI / Qt / WebComponent / HTML / XAML
```

## Primitive availability (the "why these four" answer)

A v0.1.0 Dialog could in principle use richer primitives — e.g. `If`
to gate its own visibility — but those primitives are not yet uniformly
implemented across all six emitters.  The current map (as of the
commit landing this package):

| Primitive   | React | SwiftUI | Qt | WebComp | HTML | XAML |
|---|---|---|---|---|---|---|
| `Box`         | yes | yes | yes | yes | yes | yes |
| `Row`/`Column`/`Stack` | yes | yes | yes | yes | yes | yes |
| `Text`        | yes | yes | yes | yes | yes | yes |
| `Image`       | yes | yes | yes | yes | yes | yes |
| `Spacer`/`Divider`/`Icon` | yes | yes | yes | yes | yes | yes |
| `HostInput`   | yes | yes | yes | yes | yes | yes |
| `HostButton`  | yes | yes | yes | yes | yes | yes |
| `HostScroll`  | yes | yes | yes | yes | yes | yes |
| `HostTable`   | yes | yes | yes | yes | yes | yes |
| `If` / `Else` | landing | landing | landing | yes | yes | yes |
| `For`         | landing | landing | landing | yes | yes | yes |

Dialog uses only rows of the table that are all-yes.

## v0.2.0 plans

Once `If` is wired across React/SwiftUI/Qt, v0.2.0 will grow:

* **`slot visible : bool`** — an internal visibility gate.
* **`If ( when: slot: visible )`** wrapping the layout root.
* **`emit onOpen`** (optional) — companion event to `onClose`.

Until then, the host is the source of truth for whether Dialog is
mounted: a parent component conditionally renders Dialog by including
or excluding it from its layout tree.  This works in every backend
today, because every backend's host language already has its own
conditional-rendering story.

## Smoke test

```bash
cd code/packages/mosaic-pkg-dialog
cargo test
```

The smoke test asserts:

1. `mosaic-package.toml` parses and declares exactly `exports = ["Dialog"]`.
2. `Dialog.mil` compiles via `mosmodel-compiler` (3 slots, 1 emit).
3. `Dialog.mll` compiles via `moslayout-compiler` against the `.mil`,
   and the resulting tree has the exact Box → Column → Box×3 shape
   with `HostButton` inside `dialog-actions`.
4. `Dialog.dark.msl` compiles via `mosstyle-compiler` against the
   layout's part map and declares all four parts.
5. The per-backend artifact builder produces a non-empty Dialog
   artifact for every backend it currently wires (React, SwiftUI, Qt).
   WebComponent and HTML are SKIPPED with a documented `UnsupportedBackend`
   assertion — when those wiring PRs land, the relevant `Backend` enum
   variants flip into `SUPPORTED_BACKENDS` in
   `tests/package_compiles.rs` and the test starts covering them.

## Position in UI29's roadmap

* **U29-P1 (this package)**: ship the smallest userland package proving
  end-to-end kernel + emitter + artifact-builder integration.
* **U29-P2**: a richer companion (Tabs, Tooltip, Toast) — gated on
  cross-backend `If` parity.
* **U29-D-dialog**: a demo app that imports this package and renders a
  one-click "Are you sure?" overlay on every backend.

## License

MIT OR Apache-2.0.
