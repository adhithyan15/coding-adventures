# UI16 — Mosaic Compiler Pipeline

## Overview

This spec defines the **mosaic compiler pipeline**: the four Rust crates that
take `.mil` / `.mll` / `.msl` source files and produce target-platform artefacts.
It also specifies the **backend emitter interface** that every rendering target
must implement, including the React TSX backend (v1, done) and the two native
paint backends — **Skia** and **Cairo** — that will be implemented next.

The pipeline transforms three source files into one output file. The target
determines whether that output is JSX, Swift, a `PaintScene`, or something else.
The three compilers are identical for all targets. Only the final emitter differs.

```text
                         ┌──────────────┐
   Grid.mil ─────────────▶ mosmodel-    │
   (interface)           │ compiler     ├─ descriptor JSON
                         └──────┬───────┘
                                │ validates slot/emit names
                         ┌──────▼───────┐
   Grid.mll ─────────────▶ moslayout-  │
   (layout)              │ compiler     ├─ part-map JSON
                         └──────┬───────┘
                                │ validates part names
                         ┌──────▼───────┐
   Grid.msl ─────────────▶ mosstyle-   │
   (style)               │ compiler     ├─ resolved style map
                         └──────┬───────┘
                                │
                         ┌──────▼───────────────────────────────┐
                         │         mosaic-driver                 │
                         │  orchestrates + selects backend       │
                         └──┬──────────┬───────────┬────────────┘
                            │          │           │
                     ┌──────▼──┐  ┌────▼────┐ ┌───▼──────────┐
                     │ React   │  │  Skia   │ │    Cairo     │
                     │ emitter │  │ emitter │ │   emitter    │
                     │(mosaic- │  │(mosaic- │ │ (mosaic-     │
                     │emit-    │  │emit-    │ │  emit-       │
                     │react)   │  │skia)    │ │  cairo)      │
                     └────┬────┘  └────┬────┘ └─────┬────────┘
                          │            │             │
                     .tsx file    PaintScene    PaintScene
                     (checked in) (→paint-vm-  (→paint-vm-
                                   skia)        cairo)
```

The three compilers run in dependency order. Only the backend emitter changes
per target. The `.mil` / `.mll` / `.msl` source files are written once and
render on every backend without modification.

---

## Position in the Stack

```
UI13 mosmodel   → mosmodel-compiler  (done)
UI14 moslayout  → moslayout-compiler (done, this spec governs its pipeline role)
UI15 mosstyle   → mosstyle-compiler  (done, this spec governs its pipeline role)
UI16            → mosaic-driver      (done v1)
                  mosaic-emit-react  (done v1)
                  mosaic-emit-skia   (PLANNED — this spec)
                  mosaic-emit-cairo  (PLANNED — this spec)

P2D09           → paint-vm-skia     (spec exists, not yet implemented)
                  paint-vm-cairo     (spec exists, not yet implemented)
```

Skia and Cairo backends connect Mosaic to the paint-vm layer (P2D00, P2D01,
P2D09). A Mosaic component compiled to a `PaintScene` can then be rendered by
any PaintVM backend — Skia, Cairo, Metal, Direct2D, or any future backend — with
no changes to the `.mil`/`.mll`/`.msl` source files.

---

## §1 Pipeline Stages

### Stage 1 — mosmodel-compiler

Parses a `.mil` file and produces:

- **`descriptor_json`** — JSON object listing all slots (name, type, required,
  default) and emits (name, params). This is the contract consumed by stages 2 and 3.
- **`rust_binding`** — generated Rust struct matching the component's slot types.

Stage 1 has no dependencies on other Mosaic compilers. It runs first.

### Stage 2 — moslayout-compiler

Parses a `.mll` file. Optionally receives `descriptor_json` from stage 1.

- When `descriptor_json` is provided, validates every `slot:` and `emit:`
  reference against the declared interface. Unknown references are hard errors.
- Produces **`part_map_json`** — JSON object listing named parts (name, primitive).
  This is consumed by stage 3.

#### Grammar-tools workflow

The token grammar and parser grammar for `.mll` files are maintained as
human-readable text files:

```
code/grammars/moslayout.tokens    ← edit this
code/grammars/moslayout.grammar   ← edit this
```

The embedded Rust data structures in
`packages/rust/moslayout-compiler/src/_grammar.rs` are **always generated** from
these text files. Never edit `_grammar.rs` by hand. After any grammar change:

```sh
# From the repo root
grammar-tools compile-tokens  code/grammars/moslayout.tokens  \
  -o code/packages/rust/moslayout-compiler/src/_grammar.rs.tokens.part

grammar-tools compile-grammar code/grammars/moslayout.grammar \
  -o code/packages/rust/moslayout-compiler/src/_grammar.rs.grammar.part

# Or in one shot using the BUILD file:
cargo run -p grammar-tools-cli -- compile-tokens  code/grammars/moslayout.tokens
cargo run -p grammar-tools-cli -- compile-grammar code/grammars/moslayout.grammar
```

The same rule applies to `mosstyle-compiler`:

```
code/grammars/mosstyle.tokens    ← edit this
code/grammars/mosstyle.grammar   ← edit this
→ regenerate packages/rust/mosstyle-compiler/src/_grammar.rs
```

And to `mosmodel-compiler`:

```
code/grammars/mosmodel.tokens    ← edit this
code/grammars/mosmodel.grammar   ← edit this
→ regenerate packages/rust/mosmodel-compiler/src/_grammar.rs
```

The `_grammar.rs` header comment always names the regeneration command. If a
`_grammar.rs` file is newer than the `.tokens` / `.grammar` files it was
generated from, something went wrong — commit both together.

### Stage 3 — mosstyle-compiler

Parses a `.msl` file. Optionally receives `part_map_json` from stage 2.

- When `part_map_json` is provided, validates every `part` block name against
  the declared part map. Unknown part names are hard errors.
- Produces **`style_map_json`** — serde_json serialisation of `StyleDef`.
  All design tokens are resolved to concrete values at compile time.
- Produces **`css`** — scoped CSS string (for DOM / React backends).

Design tokens are resolved in this priority order:

1. Token override file passed at compile time (v2, not yet implemented).
2. Default dark-mode palette baked into the compiler (v1, implemented).

No token reference reaches the runtime unresolved. If a `$token-name` is not in
any palette, it is a hard compile error.

### Stage 4 — mosaic-driver

The `mosaic-driver` binary (`mosaic <ComponentName>`) orchestrates stages 1–3
and dispatches to a backend emitter.

```
mosaic Grid                    # three-stage compile from CWD
mosaic --interface  Grid.mil   # run only stage 1
mosaic --layout     Grid.mll   # run only stage 2
mosaic --style      Grid.msl   # run only stage 3
mosaic --target     skia Grid  # compile and emit a PaintScene (v2)
mosaic --target     cairo Grid # compile and emit a PaintScene (v2)
mosaic --target     react Grid # compile and emit .tsx (v1 default)
```

The JSON handoffs between stages are explicit strings. The driver deserialises
them back to structured `serde_json::Value` for the final summary output.

**Security constraints** (enforced in v1):
- The component name argument is validated against `[a-zA-Z0-9_-]+` before
  being used to construct file paths (prevents path traversal).
- Serialisation errors use `match` + `process::exit(1)`, not `unwrap()`.

---

## §2 Backend Emitter Interface

Every backend emitter must implement the `MosaicEmitter` trait (to be defined
in `mosaic-vm`):

```rust
pub trait MosaicEmitter {
    type Output;

    fn emit(
        &self,
        component: &MosmodelComponent,
        layout:    &LayoutDef,
        style:     &StyleDef,
        tokens:    &TokenPalette,
    ) -> Result<Self::Output, EmitError>;
}
```

The three inputs are the typed IRs from each compiler stage. `TokenPalette`
carries the resolved token values. `Output` differs per backend:

| Backend              | `Output` type            | Description                        |
|----------------------|--------------------------|------------------------------------|
| `mosaic-emit-react`  | `String`                 | `.tsx` source text                 |
| `mosaic-emit-webcomponent` | `String`           | `.js` class source text            |
| `mosaic-emit-skia`   | `PaintScene`             | Feed into `paint-vm-skia`          |
| `mosaic-emit-cairo`  | `PaintScene`             | Feed into `paint-vm-cairo`         |
| `mosaic-emit-metal`  | `PaintScene`             | Feed into `paint-metal`            |
| `mosaic-emit-swift`  | `String`                 | SwiftUI `.swift` source text       |

The v1 React emitter does not yet implement this trait formally — it uses
`MosaicRenderer` from `mosaic-vm`. Backends added in v2 and beyond will be
written against `MosaicEmitter` so that the driver can select the backend at
runtime without knowing its concrete type.

---

## §3 React Backend (v1 — implemented)

**Crate**: `mosaic-emit-react`

Implements `MosaicRenderer`. Driven by `MosaicVM`. Produces a single `.tsx`
string containing a complete TypeScript React functional component.

**Primitive → HTML element mapping:**

| Primitive | HTML element                                        | Self-closing? |
|-----------|-----------------------------------------------------|---------------|
| `Box`     | `<div>`                                             | no            |
| `Column`  | `<div style={{ display:'flex', flexDirection:'column' }}>` | no |
| `Row`     | `<div style={{ display:'flex', flexDirection:'row' }}>` | no |
| `Text`    | `<span>` (or `<h2>` for `a11y-role: heading`)      | no            |
| `Image`   | `<img src=… alt=… />`                               | yes           |
| `Spacer`  | `<div style={{ flex: 1 }}>`                         | no            |
| `Grid`    | Full `<table>` with `.map()` for headers and rows   | yes (inlined) |
| `Divider` | `<hr />`                                            | yes           |
| `Icon`    | `<span className="icon">`                           | no            |
| `Stack`   | `<div style={{ position:'relative' }}>`             | no            |

**Grid (v1 static model):**

The `Grid` primitive in the v1 React backend accepts two slot bindings:

```mll
Grid [ cell-grid ] (
  headers: slot: column-headers ,   // Array<string>
  rows:    slot: viewport-rows      // Array<string>
)
```

Generated JSX:

```tsx
<table>
  <thead><tr>
    {columnHeaders.map((h, _i) => (<th key={_i}>{h}</th>))}
  </tr></thead>
  <tbody>
    {viewportRows.map((row, _i) => (<tr key={_i}><td>{row}</td></tr>))}
  </tbody>
</table>
```

The v1 model uses `list<text>` for rows: each row is a single string displayed
in a single `<td>`. Multi-column rows (`list<list<text>>`) require a grammar
extension in `mosmodel-compiler` and are deferred to v2.

**CSS class injection:**

When `mosstyle-compiler` has run, each primitive node that has a matching
`part` style block receives a `className` set to `.mos-{Component}-{part}`.
The generated CSS is emitted as a separate `<Component>.css` file (v2; in v1
it is returned as a string in the driver's JSON summary).

---

## §4 Skia Backend (planned)

**Crate**: `mosaic-emit-skia`

Depends on: `mosaic-vm`, `paint-instructions`

Does NOT depend on: `paint-vm-skia` (the backend is selected by the application
at runtime, not at emitter compile time).

The Skia emitter converts the Mosaic layout IR into a `PaintScene`. It walks
the `LayoutDef` tree, computes flexbox geometry using `layout-flexbox` (UI03),
then emits `PaintInstruction` calls for each primitive:

| Mosaic primitive | PaintInstruction output                                                        |
|------------------|--------------------------------------------------------------------------------|
| `Box` / `Column` / `Row` | `PaintRect` (fill) + optional `PaintRect` (border) + `PushClip`   |
| `Text`           | `PaintGlyphRun` (font resolved from token palette)                             |
| `Image`          | `PaintBitmap`                                                                  |
| `Spacer`         | (nothing — flex layout accounts for the space)                                 |
| `Grid`           | `PaintRect` per cell + `PaintGlyphRun` per cell text value                    |
| `Divider`        | `PaintLine` (thin horizontal rect)                                             |

The resolved `StyleDef` provides fill colors, border radii, font sizes, and
all other visual properties. All values have been token-resolved by stage 3 —
the Skia emitter never sees a `$token-name`.

**Geometry:**

Skia uses a top-left origin, Y-down coordinate system. The flexbox layout
engine (UI03) produces `LayoutRect` structs in that space. The Skia emitter
converts directly: `(x, y, width, height)` → `SkRect::from_xywh(x, y, w, h)`.

**Output type**: `PaintScene` (from `paint-instructions`).

The application feeds the `PaintScene` into `paint-vm-skia` (P2D09) which
calls the Skia C++ API through an FFI bridge. The Mosaic component never knows
or cares that Skia is the rendering engine.

**Rounded corners and shadows (v2):**

- `border-radius` → `SkRoundRect`
- `box-shadow` → `SkImageFilter` (Gaussian blur + offset)

These are not implemented in v1 but the `PaintInstruction` vocabulary already
supports them (via `PaintShadow` and rounded-rect fill variants in P2D00).

---

## §5 Cairo Backend (planned)

**Crate**: `mosaic-emit-cairo`

Identical interface to `mosaic-emit-skia`. The same `PaintScene` output type.
The same layout pipeline. The same style resolution. The only difference is the
downstream PaintVM backend: the application feeds the `PaintScene` into
`paint-vm-cairo` instead of `paint-vm-skia`.

There is nothing Cairo-specific in the emitter itself. The `mosaic-emit-cairo`
crate may therefore be a thin alias:

```rust
// mosaic-emit-cairo/src/lib.rs
//
// Cairo and Skia both consume a PaintScene from the layout engine.
// The emitter is identical. The difference is the paint-vm backend
// chosen by the application at runtime.
//
// This crate re-exports mosaic-emit-skia under a Cairo-named surface
// so that Cargo dependency graphs remain readable and future Cairo-
// specific overrides (e.g. Pango text shaping vs. HarfBuzz) can be
// introduced without breaking call sites.

pub use mosaic_emit_skia as mosaic_emit_cairo;
```

If Cairo-specific divergences emerge (different text shaping, different
coordinate handling, Pango glyph runs instead of HarfBuzz), the crate can be
fleshed out at that time. Until then, the thin re-export keeps both names
available while eliminating duplication.

---

## §6 Paint-VM Integration Path

The Skia and Cairo emitters connect Mosaic to the existing paint-vm stack:

```text
                    mosaic-emit-skia
                          │
                          ▼
                    PaintScene (paint-instructions P2D00)
                          │
                    paint-vm (dispatch table P2D01)
                          │
              ┌───────────┴───────────┐
              ▼                       ▼
       paint-vm-skia            paint-vm-cairo
       (P2D09 planned)          (P2D09 planned)
              │                       │
    Skia SkCanvas calls       Cairo cairo_t calls
              │                       │
        PixelContainer           PixelContainer
     (PNG, on-screen, etc.)    (PNG, on-screen, etc.)
```

**Prerequisites before mosaic-emit-skia can be implemented:**

1. `paint-vm-skia` must expose a `run(scene: &PaintScene) -> PixelContainer`
   entry point (P2D09).
2. A flexbox layout implementation must accept `LayoutDef` and return `LayoutRect`
   for each node (UI03 / UI08 — partially done).

**Prerequisites before mosaic-emit-cairo can be implemented:**

1. `paint-vm-cairo` must be implemented (P2D06 design, P2D09 convergence).
2. Same flexbox dependency as Skia.

The mosaic-driver `--target skia` and `--target cairo` flags are reserved but
return a "not yet implemented" error until these prerequisites are met.

---

## §7 Grammar-Tools Codegen Workflow

Every grammar change follows this cycle:

```
1. Edit the text grammar files
   code/grammars/<lang>.tokens
   code/grammars/<lang>.grammar

2. Validate the edit (catches syntax errors before generation)
   grammar-tools validate <lang>.tokens <lang>.grammar

3. Regenerate the embedded Rust
   grammar-tools compile-tokens  code/grammars/<lang>.tokens
   grammar-tools compile-grammar code/grammars/<lang>.grammar
   # These print Rust source to stdout; redirect or use -o flag

4. Place the output in the compiler crate
   packages/rust/<lang>-compiler/src/_grammar.rs
   (The file header names the source files and regeneration commands)

5. Run the compiler's tests
   cargo test -p <lang>-compiler

6. Commit the text file AND the generated file together
   git add code/grammars/<lang>.tokens \
           code/grammars/<lang>.grammar \
           packages/rust/<lang>-compiler/src/_grammar.rs
```

**The `_grammar.rs` file is always generated — never hand-authored.** If a
diff includes changes to `_grammar.rs` without a corresponding change to the
`.tokens` / `.grammar` source file, the review should be rejected. CI will
eventually enforce this via a stale-check step (v2).

The commit message for a grammar change must name what changed and why:

```
feat(moslayout): extend prop rule to support shorthand slot binding

grammar: prop = NAME COLON prop_value | KEYWORD COLON NAME
Regenerated: moslayout-compiler/src/_grammar.rs via grammar-tools 0.1.0
```

---

## §8 Component File Layout

Every Mosaic component lives in its own directory under `code/components/`:

```
code/components/
  Button/
    Button.mil           ← interface (required)
    Button.mll           ← layout (required)
    Button.msl           ← style (required)
    Button.desktop.mll   ← desktop layout override (optional)
    Button.mobile.mll    ← mobile layout override (optional)
  Grid/
    Grid.mil
    Grid.mll
    Grid.msl
  FormulaBar/
    FormulaBar.mil
    FormulaBar.mll
    FormulaBar.msl
```

Platform-variant layout files follow the naming convention from UI14 §1.
The base `.mll` file is the fallback used when no platform-specific variant
matches the target.

The style file (`.msl`) is universal — no platform variants in v1. Platform-
specific style overrides are v2, driven by the Lattice token layer.

---

## §9 First End-to-End Demo: Grid

The `Grid` component in `code/components/Grid/` demonstrates the full pipeline:

**Grid.mil** declares:
- `slot column-headers : list<text>` — column label strings
- `slot viewport-rows : list<text>` — data row strings
- `emit onRowClick ( row : number )` — future interactive use

**Grid.mll** arranges:
- `Column [ root ]` — outer flex-column container
- `Grid [ cell-grid ]` — leaf table primitive, bound to both slots

**Grid.msl** provides:
- `part root` — dark surface background, `border-radius: 6px`, `overflow: hidden`
- `part cell-grid` — `width: 100%`, `border-collapse: collapse`, token-resolved text color

**React output** (v1):

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from "react";

interface GridProps {
  columnHeaders: string[];
  viewportRows: string[];
}

export function Grid({ columnHeaders, viewportRows }: GridProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }} className="mos-Grid-root">
      <table className="mos-Grid-cell-grid">
        <thead><tr>
          {columnHeaders.map((h, _i) => (<th key={_i}>{h}</th>))}
        </tr></thead>
        <tbody>
          {viewportRows.map((row, _i) => (<tr key={_i}><td>{row}</td></tr>))}
        </tbody>
      </table>
    </div>
  );
}
```

---

## §10 V1 Scope and Known Limitations

**In scope for v1 (current PR):**

- mosmodel-compiler, moslayout-compiler, mosstyle-compiler, mosaic-driver
- mosaic-emit-react (Grid, Box, Row, Column, Text, Image, Spacer, Divider, Icon, Stack)
- Grid component source files (.mil / .mll / .msl)
- Default dark-mode token palette (UI15 §1 tokens)
- 45 unit tests across all four crates

**Explicitly deferred to v2:**

| Feature | Reason |
|---------|--------|
| `list<list<text>>` for Grid rows | mosmodel grammar extension needed |
| `rgba()` / `hsl()` in `.msl` values | `function_call` grammar rule needed |
| Platform-variant layout files (`.desktop.mll`) | Build-tool integration needed |
| Lattice token override files | Token file compiler not wired up |
| `--target skia / cairo` in mosaic-driver | Depends on paint-vm-skia / paint-vm-cairo |
| mosaic-emit-skia, mosaic-emit-cairo crates | Depends on paint-vm-skia / paint-vm-cairo |
| CSS file output (separate `.css` file) | Currently returned as JSON string |
| `MosaicEmitter` trait in mosaic-vm | React backend still uses `MosaicRenderer` |
| Stale-check CI for `_grammar.rs` | Grammar-tools CI integration |

**Grammar-tools codegen debt:**

The `_grammar.rs` files for `moslayout-compiler` and `mosstyle-compiler`
were written by hand in v1 (before this spec existed). The immediate next
step after this spec is merged is to regenerate them from the text grammar
files using `grammar-tools`, verify they match, and add the stale-check to
the BUILD file. This closes the gap between spec and implementation.

---

## §11 Related Specs

| Spec | Title | Relationship |
|------|-------|--------------|
| UI13 | mosmodel | Stage 1 compiler — component interface |
| UI14 | moslayout | Stage 2 compiler — structural layout |
| UI15 | mosstyle | Stage 3 compiler — visual appearance |
| UI00 | Mosaic overview | Original vision; UI16 is the pipeline that realises it |
| P2D00 | paint-instructions | PaintScene and PaintInstruction types |
| P2D01 | paint-vm | Dispatch table VM that runs PaintScenes |
| P2D06 | paint-vm native backends | Cairo design; Direct2D and GDI implementation |
| P2D09 | paint-vm backend convergence | Skia, Cairo, Vulkan, WGPU — convergence roadmap |
| UI02  | layout-ir | LayoutRect and layout tree used by Skia/Cairo emitters |
| UI03  | layout-flexbox | Flexbox geometry engine driving Skia/Cairo emitters |
