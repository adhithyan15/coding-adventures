# UI30 — Multi-Layout Pipelines

> **Status.** Draft, gates the implementation cycle (`ML1`/`ML2`/`ML3`)
> and the VisiCalc Phase 2 cross-backend demos.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Scope.** Defines how a single Mosaic component can ship multiple
> *layout variants* (desktop, touch, tablet, …) while keeping its
> interface (`.mil`) and style (`.msl`) sources mostly unchanged. Pins
> down the filename convention, the compiler CLI surface, the fallback
> chain, and the per-backend output naming.

---

## 1. Why multi-layout belongs in the pipeline

A real cross-platform UI cannot ship one layout for every form factor.
A spreadsheet, calendar, or chat app needs:

- **Desktop** — wide rows, dense columns, mouse-sized tap targets
  (~24×24 px), keyboard-driven editing, hover affordances.
- **Touch / phone** — vertically-stacked cards instead of rows, large
  tap targets (~44×44 px per Apple HIG), no hover, swipe gestures
  replace right-click.
- **Tablet** — somewhere between the two; often a 2-column split-view
  where desktop has a single 5-column row.
- **TV / wall display** — focus-driven 10-foot UI, no fine pointer
  input, very large type.

A single `.mll` cannot satisfy all of these. Userland conditional
rendering (`If sizeClass == compact { Column } Else { Row }`) is
possible in theory but rapidly bloats components and loses the
"declarative layout per form factor" intent.

**The kernel already separates concerns the right way** — `.mil`
(what data flows in/out), `.mll` (how it's arranged), `.msl` (how
it's painted). Multi-layout simply *generalises* the existing
`.dark.msl` / `.light.msl` axis (already shipped) onto the `.mll`
file too. The `.mil` stays singular because the interface contract
should NOT change between form factors — a `Grid` is still a `Grid`
whether the host renders it as a desktop table or a touch list.

The convention is *already half-built*: VisiCalc's existing files are
`FormulaBar.desktop.mll` and `Grid.desktop.mll`. UI30 makes the
infix meaningful end-to-end.

---

## 2. Filename convention

A Mosaic package's `src/` directory now recognises the following
patterns:

```
<Component>.mil                      ← interface (singular)
<Component>.mll                      ← layout, default variant
<Component>.<variant>.mll            ← layout, named variant
<Component>.<theme>.msl              ← style, named theme
<Component>.<variant>.<theme>.msl    ← style, variant×theme cross-product (optional)
```

Where:

- `<Component>` is the PascalCase component name (matches the `.mil`
  component declaration).
- `<variant>` is a kebab-case form-factor identifier. The **reserved
  built-ins** are:
  - `desktop` — pointer-driven, wide viewport (≥1024 px).
  - `touch` — finger-driven, narrow viewport (≤768 px).
  - `tablet` — finger-driven, mid viewport (768–1024 px).
  - `tv` — focus-driven, 10-foot UI, viewport ≥1920 px.
  - `print` — paged output, no interactivity.
- Authors may define additional variants (e.g. `watch`, `auto`,
  `compact`) — the variant string is opaque to the compiler.
- `<theme>` continues to follow the existing convention (`light`,
  `dark`, or any author-defined string).

### 2.1 Backwards-compatibility rule

A bare `<Component>.mll` (no infix) is the **default variant**. It
fires when:

- No `--variant` flag is passed to the compiler, AND
- No multi-variant resolution mechanism (artifact-builder
  enumeration) is active.

Existing packages that ship one `.mll` per component continue to
build identically — UI30 is purely additive.

### 2.2 Why the variant infix is on the layout, not the interface

The interface (`.mil`) MUST stay singular across variants. Slots and
emits define the *contract* between the component and the host; if a
touch layout removed a slot the desktop layout requires, the host
code would diverge per form factor and the whole "one component,
many layouts" guarantee collapses.

Variants MAY style the same slot differently (e.g. touch wraps the
formula bar's `cell-address` slot in a larger font), but they cannot
hide a slot or change its type. Compile-time enforcement: every
variant `.mll` is validated against the singular `.mil` exactly as
the default variant is today.

---

## 3. Compiler CLI surface

`mosaic-compile` grows one new flag and gains directory-input support
on `--layout`:

```
mosaic-compile \
  --backend react \
  --interface  <pkg>/src/Grid.mil \
  --layout     <pkg>/src/                # ← directory, not file
  --variant    touch                     # ← new flag
  --style      <pkg>/src/Grid.dark.msl
  -o           Grid.touch.tsx
```

### 3.1 Resolution rules

When `--layout` is a **file path**, behaviour is unchanged from
pre-UI30 (the file is the layout source verbatim; `--variant` is
ignored with a deprecation warning). This preserves every existing
build script.

When `--layout` is a **directory path**, the compiler:

1. Reads the `.mil` to determine the component name (call it `C`).
2. Looks for `<dir>/<C>.<variant>.mll`. If found, that's the layout
   source.
3. If not found, looks for `<dir>/<C>.mll` (the default variant). If
   found, that's the layout source.
4. If neither found, errors out with a clear message:

   ```
   error: no layout file for component 'Grid' with variant 'touch' in /path/to/src/
     looked for: Grid.touch.mll, Grid.mll
   ```

### 3.2 The `--variant` flag default

If `--variant` is omitted, the compiler treats it as if `--variant
default` were passed — which after step 2 falls through to step 3 and
matches the bare `<C>.mll`. The string `default` is reserved and
cannot be used as a variant infix on disk (it would imply
`Grid.default.mll`, which is redundant with `Grid.mll`).

### 3.3 Output filename convention

The default output filename (when `-o` is omitted) follows the input
variant:

| Variant | React | HTML | WebComp | Qt | SwiftUI | XAML | Flutter |
|---|---|---|---|---|---|---|---|
| default | `Grid.tsx` | `Grid.html` | `Grid.js` | `Grid.qml` | `Grid.swift` | `Grid.xaml` | `Grid.dart` |
| desktop | `Grid.desktop.tsx` | `Grid.desktop.html` | `Grid.desktop.js` | `Grid.desktop.qml` | `Grid.desktop.swift` | `Grid.desktop.xaml` | `Grid.desktop.dart` |
| touch   | `Grid.touch.tsx`   | `Grid.touch.html`   | `Grid.touch.js`   | `Grid.touch.qml`   | `Grid.touch.swift`   | `Grid.touch.xaml`   | `Grid.touch.dart`   |

The variant infix lands in the filename so a directory like

```
src/components/
  Grid.tsx          # default
  Grid.desktop.tsx
  Grid.touch.tsx
  Grid.tablet.tsx
```

can coexist without collision. Hosts pick whichever variant they
want to import at runtime (or use a runtime resolver, see §6).

---

## 4. Package manifest declaration

`mosaic-package.toml` grows an optional `[variants]` section:

```toml
[variants]
# Variants the package ships. When absent, only the default variant
# is built.
all = ["desktop", "touch"]

# Optional: per-component variant overrides if some components don't
# ship every variant (e.g. a Sidebar that only has a desktop
# layout). Authors omit the entry for components that follow the
# package-wide `all` list.
overrides = { Sidebar = ["desktop"] }

# Optional: variant fallback chain. If a component has no .touch.mll
# but the host requests touch, the artifact builder picks the
# nearest variant up the chain.
fallback = { touch = "desktop", tablet = "desktop" }
```

The `fallback` table is the multi-layout equivalent of CSS's
`@media` cascade — a touch host gracefully degrades to desktop
when no touch layout was authored. Without fallback, the artifact
builder errors out on missing variants.

---

## 5. Artifact-builder enumeration

`mosaic-package-artifact-builder` (the multi-backend bulk
compiler) extends its current "one component → one artifact per
backend" loop to:

```
for component in package.exports:
    for variant in package.variants.for(component):
        for backend in target_backends:
            mosaic-compile \
                --backend <backend> \
                --interface <pkg>/src/<component>.mil \
                --layout    <pkg>/src/ \
                --variant   <variant> \
                --style     <pkg>/src/<component>.<theme>.msl \
                -o <out>/<component>.<variant>.<ext>
```

The output tree groups by component first, then variant — the host
import path is `import { Grid } from "./components/Grid.touch"`.

### 5.1 No-multiplication clause

Packages that don't declare `[variants]` continue to build exactly
one artifact per (component, backend) pair, with no variant infix
in the filename. The whole UI30 machinery activates only on
opt-in. **Existing toolkit + dialog packages are unaffected.**

---

## 6. Host runtime resolution

UI30 does *not* prescribe how the host picks a variant at runtime —
that's outside the compiler's scope. Two common patterns:

1. **Build-time selection** — the host's bundler picks one variant
   per build (e.g. one bundle for web/desktop, another for the
   mobile web app). The unused variants aren't shipped.
2. **Runtime media-query switch** — the host imports all variants
   and picks one based on `window.matchMedia('(max-width: 768px)')`
   or the platform's equivalent (UIKit's `traitCollection.horizontalSizeClass`,
   QML's `Screen.devicePixelRatio`, etc.).

A future toolkit component (`<LayoutSwitch>`) could wrap pattern 2
generically, but it's not required for UI30.

---

## 7. Implementation plan

The cycle splits into three small PRs:

### ML1 — `mosaic-compile --variant` (compiler CLI)

- Add `--variant <name>` flag to the `mosaic-compile` CLI spec.
- Teach the layout resolver to handle directory inputs per §3.1.
- Update CLI tests to cover both file-path and directory-path
  `--layout` arguments + the variant resolution.
- Update README/CLI help.

### ML2 — artifact-builder variant enumeration

- Add `[variants]` parsing to `mosaic-package-artifact-builder`'s
  manifest reader.
- Loop over (component × variant × backend) and emit
  `<Component>.<variant>.<ext>` per §3.3.
- Honour the fallback chain per §4.

### ML3 — touch variant proof-of-concept for VisiCalc

- Write `code/programs/mosaic/visicalc/FormulaBar.touch.mll` and
  `Grid.touch.mll` that meaningfully diverge from the desktop
  variants (large tap targets, no sticky header, etc.).
- Verify both variants compile through `mosaic-compile --variant
  touch` and `--variant desktop`.
- Output two FormulaBar `.tsx` files; show the diff in the PR
  description.

Future cycles can add:

- **ML4 — runtime LayoutSwitch toolkit component.**
- **ML5 — per-backend responsive bundling** (one `.tsx` exports
  every variant under a guard, host imports the bundle and picks).

---

## 8. Open questions

1. **Should `.mil` be allowed to declare variant-specific defaults?**
   E.g. a touch FormulaBar might default `placeholder` to a shorter
   string. The spec currently says NO — defaults live in `.mil`
   which is singular — but a future amendment could add a
   `default <variant>` override syntax if a real need emerges.
2. **Variant inheritance.** Should `Grid.tablet.mll` be able to
   `extends Grid.desktop` and only override the diverging parts?
   The kernel currently has no inheritance machinery and the spec
   leaves this out. Authors duplicate the layout in v1.
3. **Cross-product explosion.** A package with 4 variants × 7
   backends × 2 themes = 56 artifacts per component. For a 20-
   component package that's 1,120 files. The artifact builder
   needs incremental compilation (only rebuild changed inputs)
   well before this becomes a real-world problem; track as a
   follow-up but don't gate UI30 on it.
4. **Variant naming policy.** Should the spec lock down the
   built-in variant list (`desktop` / `touch` / `tablet` / `tv` /
   `print`) and reject unknowns, or treat the string as
   author-opaque? Current spec: opaque — authors can ship a
   `vr` variant if they like. The reserved built-ins are
   documentation, not enforcement.

---

## 9. Out of scope

- The runtime resolver (host code that picks `desktop` vs `touch`).
- Variant inheritance / overrides.
- A new kernel primitive — UI30 is pure pipeline plumbing; no
  primitive count change.
- Responsive *primitives* (e.g. `MediaQuery`-style conditionals
  inside a single `.mll`). Those would be a separate spec.

---

**Reviewer checklist:**

- [ ] Does the file-path-vs-directory `--layout` resolution rule
      cover every existing build script unchanged?
- [ ] Is the `[variants]` manifest section opt-in (no behaviour
      change for existing packages)?
- [ ] Does the output filename convention avoid collisions when
      multiple variants are emitted to the same directory?
- [ ] Is the spec clear that `.mil` MUST stay singular across
      variants?
- [ ] Are the reserved variant names (`desktop`/`touch`/`tablet`/
      `tv`/`print`) documented as recommendations, not
      enforcements?
