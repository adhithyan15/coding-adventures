# mosaic-pkg-toolkit — Bootstrap-shaped Mosaic component library

**Status:** Specification (draft)
**Layer:** UI / userland component package
**Depends on:** UI29 (primitive kernel), UI29-1 (HostDialog), UI24
(emit→dispatch). Implements against the kernel only — no backend
dependencies.
**Sibling packages:** `mosaic-pkg-grid` (data grid), `mosaic-pkg-card`
(minimal card), `mosaic-pkg-dialog` (modal helper). The toolkit
re-exports a richer Card on top of those primitives where it makes
sense.

---

## 1. Vision

Twitter Bootstrap proved a thesis: ship a small, opinionated set of
ready-made UI components and the average web app's UI cost drops by an
order of magnitude. Authors stop writing buttons; they pick a button.

Mosaic's UI29 kernel is the right *foundation* for that thesis — 16
primitives that lower to native widgets on every backend — but the
kernel deliberately stops at primitives. A `<HostButton>` says
"button," not "primary danger button with a leading icon and a
loading spinner."

`mosaic-pkg-toolkit` is the **Mosaic answer to Bootstrap**: a
single userland package composed *purely from UI29 kernel primitives*,
publishing a catalog of ready-made components that any host —
React/SwiftUI/Qt/HTML/WebComp/XAML — can drop in. One package, one
import, every backend.

The key architectural property: **the toolkit has no backend code**.
Every component is `.mil` + `.mll` + `.msl` triples that compile
through every existing Mosaic emitter unchanged. Adding a new backend
to Mosaic automatically adds the toolkit to it. No per-backend forks.

### What's "Bootstrap-shaped" mean

Bootstrap's value isn't its visual style — that gets re-themed
constantly. The value is the catalog + the API consistency:

- A predictable set of components every web dev knows (Alert, Badge,
  Button, Card, Modal, Nav, Pagination, Spinner, …).
- A predictable variant system (`-primary`, `-secondary`, `-success`,
  `-danger`, `-warning`, `-info`, `-light`, `-dark`).
- Predictable spacing/sizing utilities.
- A grid system for layout (Container/Row/Col — though this
  collides with kernel `Row`; see §6.1).
- Form controls with consistent labelling/validation patterns.
- Default styling that gets out of the way.

The toolkit copies the *catalog* and the *API consistency*; the
default styling is a Mosaic-native look that re-themes via
`.msl` overrides (§7).

---

## 2. Naming + package layout

### 2.1 Package name

Open question — see §10. Working name: **`mosaic-pkg-toolkit`**.

Alternatives considered:
- `mosaic-pkg-bootstrap` — trademark + brand confusion.
- `mosaic-pkg-ui` — too generic.
- `mosaic-pkg-mortar` — cute (mortar holds mosaic tiles), but
  unsearchable.
- `mosaic-pkg-kit` — generic but short.
- `mosaic-pkg-widgets` — accurate, dated feel.

Going with `mosaic-pkg-toolkit` until a better name surfaces.

### 2.2 Layout

Standard userland package shape per UI29 §4.1:

```
code/packages/mosaic/mosaic-pkg-toolkit/
├── mosaic-package.toml             # manifest — lists every export
├── Cargo.toml                      # smoke-test harness only
├── README.md
├── CHANGELOG.md
├── src/
│   ├── lib.rs                      # empty (doc only)
│   ├── Alert.mil / .mll / .light.msl / .dark.msl
│   ├── Badge.mil / .mll / .light.msl / .dark.msl
│   ├── Button.mil / .mll / .light.msl / .dark.msl
│   ├── Card.mil / .mll / .light.msl / .dark.msl
│   ├── Checkbox.mil / .mll / .light.msl / .dark.msl
│   ├── Container.mil / .mll / .light.msl / .dark.msl
│   ├── Field.mil / .mll / .light.msl / .dark.msl
│   ├── Input.mil / .mll / .light.msl / .dark.msl
│   ├── ListGroup.mil / .mll / .light.msl / .dark.msl
│   ├── Modal.mil / .mll / .light.msl / .dark.msl
│   ├── Nav.mil / .mll / .light.msl / .dark.msl
│   ├── Pagination.mil / .mll / .light.msl / .dark.msl
│   ├── Radio.mil / .mll / .light.msl / .dark.msl
│   ├── Select.mil / .mll / .light.msl / .dark.msl
│   ├── Spinner.mil / .mll / .light.msl / .dark.msl
│   ├── Toast.mil / .mll / .light.msl / .dark.msl
│   └── ...
└── tests/
    └── package_compiles.rs          # asserts every triple round-trips
```

One package, many components. Authors `import` one thing.

### 2.3 Why one package, not many small ones

Bootstrap is one library, not seventeen. The cost of one big package
is paid once at install time; the cost of seventeen tiny packages is
paid every time you need a new component.

When (not if) the toolkit grows large enough that tree-shaking
matters, a follow-up spec can split it — but that's a real cost only
the JS backends pay, and only when the bundle is shipped to a browser.
For SwiftUI/Qt/XAML, "tree-shaking" is just "don't reference the
component" — the compiler drops it.

---

## 3. Component inventory — v0.1 catalog

Three tiers, by implementation complexity. **v0.1 ships tier 1 only.**
Tier 2 + 3 are follow-up specs.

### 3.1 Tier 1 — kernel-only, no novel mechanisms

These compose from existing kernel primitives only (`Box`, `Row`,
`Column`, `Stack`, `Text`, `Icon`, `HostButton`, `HostInput`,
`HostDialog`, `If`, `For`). Each is a single `.mil`/`.mll`/`.msl`
triple.

| Component   | What it is                       | Kernel composition |
|-------------|----------------------------------|--------------------|
| `Alert`     | colored info banner              | `Box[alert] { Row { Icon, Text } }` |
| `Badge`     | small pill label                 | `Box[badge] { Text }` |
| `Button`    | styled button + variants         | `HostButton (variant: prop)` |
| `Card`      | content container, header/body/footer | `Box[card] { Column { Box[header], Box[body], Box[footer] } }` |
| `Checkbox`  | binary input + label             | `Row { HostButton[checkbox] (toggle), Text[label] }` (treated as toggle) |
| `Container` | width-constrained layout box     | `Box[container] { children }` |
| `Field`     | label + control + help text + error | `Column { Text[label], <slot>, Text[help], If error Text[error] }` |
| `Input`     | text input + label/help wrapper  | uses `Field` + `HostInput` |
| `ListGroup` | vertical list of selectable rows | `Column { For (each: items, as: item) { Box[item] { Text } } }` |
| `Modal`     | titled modal dialog              | `HostDialog (title: slot:t, modal: true) { Column { children } }` |
| `Radio`     | radio button + label             | similar to Checkbox |
| `Spinner`   | indeterminate loading indicator  | `Stack { Icon[spinner-glyph] }` — animation is host's job |
| `Toast`     | dismissible bottom-anchored note | `Stack { Box[toast] { Row { Icon, Text, HostButton[close] } } }` |

13 components for v0.1.

### 3.2 Tier 2 — adds composition complexity

These compose multiple Tier-1 components. Shipped after v0.1 proves
the model.

| Component      | Composition |
|----------------|-------------|
| `Nav`          | `Row { For (links) { HostButton[nav-link] } }` |
| `Navbar`       | `Box[navbar] { Row { Container { ... } } }` |
| `Pagination`   | `Row { HostButton[prev], For (pages) { HostButton[page] }, HostButton[next] }` |
| `Breadcrumb`   | `Row { For (crumbs) { HostButton[crumb], Text[separator] } }` |
| `InputGroup`   | `Row { Text[addon], HostInput, Text[addon] }` |
| `ButtonGroup`  | `Row { For (buttons) { HostButton } }` |
| `Tabs`         | `Column { Row[tab-bar] { For (tabs) { HostButton[tab] } }, Box[tab-panel] { ... } }` |
| `DropdownMenu` | `Stack { HostButton[trigger], If open { Box[menu] { For (items) { HostButton } } } }` |
| `Accordion`    | `Column { For (sections) { Box[section] { HostButton[header], If open { Box[body] } } } }` |
| `Select`       | uses `DropdownMenu` |

### 3.3 Tier 3 — needs new infrastructure

Each item flags a kernel feature not yet present.

| Component   | Blocker |
|-------------|---------|
| `Tooltip`   | needs hover state + portal positioning (UI29 doesn't have either) |
| `Popover`   | same as Tooltip |
| `Modal` with focus trap | needs first-class focus mgmt across backends |
| `Offcanvas` | needs slide-in animation primitive |
| `Carousel`  | needs animated transitions |
| `ProgressBar` (determinate) | tier 1 if just a styled Box; tier 3 if smooth animation |

Tier 3 unblocks as the kernel grows.

---

## 4. The variant system

Bootstrap's variants (`-primary`, `-secondary`, `-success`,
`-danger`, `-warning`, `-info`, `-light`, `-dark`) are the central
API consistency Mosaic has to copy.

### 4.1 Variant as a keyword slot

Every toolkit component that has variants declares:

```moslayout
component Alert {
  slot variant : text ;  // one of: primary, secondary, success, danger, warning, info, light, dark
  slot message : text ;
}
```

The `.mll` is variant-agnostic — it just attaches `part_name: "alert"`
to the outer Box. The `.msl` defines the variant-specific colors:

```mosstyle
part alert {
  padding : 12 ;
  border-radius : 4 ;
}
part alert/primary {  // sub-part for the variant
  background : "#cfe2ff" ;
  color : "#084298" ;
}
part alert/danger {
  background : "#f8d7da" ;
  color : "#842029" ;
}
// ... etc.
```

The XAML / React / SwiftUI / Qt emitters lower the sub-part
references to whatever styling mechanism they support (CSS class,
SwiftUI modifier, QML state).

**Open question** (§10): the `part/variant` sub-part syntax. Existing
mosstyle has `part X { state Y { ... } }` for hover/focus states.
Variants are conceptually similar — a sub-key into the same part.
Either reuse `state` semantically, add a new `variant` block, or
treat variants as straight conditional styling via `If` blocks in the
.mll. Pick when implementing the first variant'd component.

### 4.2 Sizes

Bootstrap has `-sm`, `-md`, `-lg`. Same mechanism — a `size` slot.

### 4.3 Why slot-driven, not class-driven

Bootstrap's `class="btn btn-primary btn-lg"` works because HTML is
the host language. Mosaic targets multiple host languages; an
HTML-class-based variant system would force every backend to
implement CSS-class semantics. Slot-driven is the kernel-native form
and lets every backend lower it idiomatically.

---

## 5. Theming

### 5.1 Light / dark / custom

Each component ships at least `<Component>.light.msl` and
`<Component>.dark.msl`. The host project picks one at compile time
(currently the file extension is the selection mechanism).

Hosts that want to brand-override use the UI29 §8 open-question 6
proposal: ship their own `.msl` ResourceDictionary (XAML) /
modifier-table (SwiftUI/Qt) / class-overrides (React) that loads
*after* the package's defaults. Host's setters win on collision.

### 5.2 Default visual style

The toolkit's default theme is a **Mosaic-native, modern-flat**
aesthetic — neutral colors, generous spacing, rounded corners,
no shadows by default. Deliberately distinct from Bootstrap's
factory aesthetic so designers immediately recognise "Mosaic" not
"Bootstrap." The semantics are Bootstrap; the look isn't.

Mockups belong in a follow-up spec (`mosaic-pkg-toolkit-design.md`).

### 5.3 Tokens

A small `<Component>.tokens.msl` per component declares named
color/spacing tokens; the variant `.msl` files reference them. This
makes the entire theme re-theme-able from a single tokens file the
host can override.

---

## 6. Cross-cutting design questions

### 6.1 The `Row` / `Container` / `Col` naming collision

Bootstrap's grid system is `Container > Row > Col`. Kernel `Row` is
already a flex-row primitive. Mosaic's `Container` toolkit component
will resolve to something like:

```moslayout
component Container {
  slot children : node ;
}
layout Container {
  Box [container] {
    children
  }
}
```

…and the toolkit doesn't redefine `Row` — it just *uses* the kernel
`Row` directly. Authors write:

```moslayout
Container {
  Row {
    Col [span-6] { ... }
    Col [span-6] { ... }
  }
}
```

`Col` becomes a toolkit component because it has the span/offset
props the kernel doesn't.

### 6.2 Responsive breakpoints

**The big one.** Bootstrap's grid is fundamentally responsive:
`col-md-6 col-lg-4` means "6 cols on medium screens, 4 on large."
Mosaic's kernel has no notion of viewport / breakpoint.

Three options:

| Option | Description | Verdict |
|---|---|---|
| **A** | Defer responsive — Col takes a single numeric `span` prop; host picks the value at render time | Recommended for v0.1 |
| **B** | Add breakpoints to mosstyle | A bigger spec (mosstyle §X) |
| **C** | Slot-driven breakpoint props (`span-md: number`) where host watches viewport and updates the slot | Workable interim; awkward for the host |

**v0.1 ships Option A.** Open follow-up: extend mosstyle with
breakpoint-aware `@media`-like syntax (Option B). Until then, hosts
calculate the responsive `span` themselves.

### 6.3 Icon system

Bootstrap ships Bootstrap Icons as a separate library. The kernel
has `Icon` but it just renders a glyph from whatever font the
backend's default icon stack uses. The toolkit doesn't ship its own
icon set in v0.1; each component's `Icon` slot takes a glyph name
the host's icon stack understands.

A future `mosaic-pkg-toolkit-icons` could ship a curated 200-icon
set as SVG bundles that each backend lowers natively.

### 6.4 Validation states

Bootstrap inputs have `is-valid` / `is-invalid` classes. The toolkit
`Field` component declares a `state` slot (`valid` / `invalid` /
`neutral`) that maps to part-state styling.

### 6.5 Accessibility

Every toolkit component is built on kernel `Host*` primitives where
possible (Modal → HostDialog, Input → HostInput, Button →
HostButton). The kernel's a11y story is what each backend's native
widget provides; the toolkit doesn't add anything beyond ARIA
labels exposed as slots on each component (`aria-label: text`).

---

## 7. Implementation phasing

### 7.1 v0.1 — Tier-1 catalog (this spec)

13 components from §3.1. Single package. Light + dark themes. Smoke
test that every triple compiles through every backend's `from_pipeline`
without error (modelled on `mosaic-pkg-grid/tests/package_compiles.rs`).

### 7.2 v0.2 — Tier 2

10 more components from §3.2. Adds the component-references-component
pattern (e.g. `Pagination` references `Button`). This will exercise
PR-5's `ComponentRegistry` resolver on every backend.

### 7.3 v0.3 — Bootstrap-aesthetic theme

A `mosaic-pkg-toolkit/themes/bootstrap.msl` overlay that ships the
classic Bootstrap visual style. Hosts that want "looks like
Bootstrap" load this overlay; everyone else gets the default
Mosaic-native theme.

### 7.4 v0.4 — Tier 3

Tooltips, popovers, focus trap. Each will likely require a kernel
follow-up first (UI29-2 spec for hover/positioning primitives, etc.).

### 7.5 v0.5 — Responsive grid

Either Option B (mosstyle breakpoints) or a richer slot-driven
breakpoint model. Decided after the kernel-level work for B is scoped.

---

## 8. Per-backend rendering matrix

Spot-check that v0.1 components round-trip through each existing
emitter. Cells are illustrative — they're what the emitter produces,
not what the toolkit author writes.

| Component | React (JSX)                          | SwiftUI                | Qt/QML            | XAML                | Web Component       | HTML                |
|-----------|--------------------------------------|------------------------|-------------------|---------------------|---------------------|---------------------|
| `Alert`   | `<div class="alert"><div class="row"><i/><span/></div></div>` | `Group { HStack { Image, Text } }` | `Item { RowLayout { Image, Text } }` | `<Border><StackPanel Orientation="Horizontal"><FontIcon/><TextBlock/></StackPanel></Border>` | `<div>...</div>` shadow DOM | `<div>...</div>` static |
| `Button`  | `<button ...>` | `Button { ... }` | `Button { ... }` | `<Button .../>` | `<button>` | `<button>` |
| `Modal`   | `<dialog>` | `.sheet` | `Popup` | `<ContentDialog>` | `<dialog>` | `<dialog>` |
| `Card`    | nested `<div>`s | `VStack` of `Group` | `ColumnLayout` of `Item` | nested `<Border>`s | nested `<div>`s shadow | nested `<div>`s |

All backends already implement every primitive used — the toolkit
adds no new primitive dependency.

---

## 9. Public consumer experience

A host project consuming the toolkit looks like:

```toml
# mosaic-package.toml (the host's package)
[dependencies]
mosaic-pkg-toolkit = "0.1"
```

And in their `.mll`:

```moslayout
component LoginScreen {
  slot username : text ;
  slot password : text ;
  slot is-submitting : bool ;
  slot error : text ;

  emit onLogin (u: text, p: text) ;
}

layout LoginScreen {
  Container {
    Card {
      Text [title] (content: "Sign in")

      Field (label: "Username") {
        Input (value: slot: username)
      }
      Field (label: "Password", help: "8 characters minimum") {
        Input (value: slot: password, type: "password")
      }

      If (when: slot: error) {
        Alert (variant: "danger", message: slot: error)
      }

      Button (
        variant: "primary",
        label: "Sign in",
        disabled: slot: is-submitting,
        onClick: emit: onLogin
      )

      If (when: slot: is-submitting) {
        Spinner ()
      }
    }
  }
}
```

That's the entire LoginScreen. No CSS files, no `.tsx` per
component, no SwiftUI views to hand-write. The toolkit composes from
the kernel and lowers through every backend.

---

## 10. Open questions

1. **Package name** — `mosaic-pkg-toolkit`, `-kit`, `-ui`, or
   something else? Picked at v0.1 commit time.
2. **Variant syntax in mosstyle** — `part/variant` sub-part, a new
   `variant` block, or `If`-driven? Picked when the first
   variant'd component lands.
3. **Responsive grid** — slot-driven (Option A), mosstyle
   breakpoints (Option B), or both? v0.1 ships A; B is a kernel-spec
   follow-up.
4. **Default theme aesthetic** — does the package ship the
   Bootstrap-classic look or a fresh Mosaic look as the default? The
   spec recommends fresh-Mosaic default + bootstrap-classic as an
   overlay. Lock that decision at design-mockup time.
5. **Icon system** — curated set in `mosaic-pkg-toolkit-icons`, or
   defer entirely? Recommend defer for v0.1.
6. **Component-references-component** — at what point does a
   toolkit component reference another? `Pagination → Button` is the
   first natural one. Defer to Tier 2 / v0.2.
7. **Test strategy** — `package_compiles.rs` (frontend round-trip)
   for v0.1; visual regression tests (per-backend rendering
   screenshots) for v0.3+ when there's something visual to compare.

---

## 11. Out of scope for v0.1

- **Hover / focus / positioning** primitives (Tooltip, Popover).
- **Animation** primitives (Carousel, Offcanvas slide-in,
  Accordion slide-down).
- **Responsive breakpoints** beyond slot-driven.
- **Form validation framework** — `Field` ships with a slot for
  error text; how the host computes "valid" is its problem.
- **Internationalisation** — strings are passed as slots; no
  i18n framework in the toolkit.
- **Icon library** — see §10.5.
- **A bespoke design language** — the default look is utilitarian.
  Brand-specific overlays live in host projects, not the toolkit.

---

## 12. Why this fits Mosaic's strategic story

Three reasons:

1. **It proves UI29's thesis.** A 13-component library written
   purely against the kernel, lowering to every backend with no
   per-backend code, is exactly the validation UI29 §1 promised.
   The current `mosaic-pkg-grid` and `mosaic-pkg-dialog` packages
   each prove one component; the toolkit proves the *catalog*.

2. **It removes the "ok but what do you build with this?" wall.**
   Today a Mosaic adopter sees the kernel primitives and has to
   compose every component themselves. With the toolkit, the next
   adopter writes their LoginScreen in 25 lines of `.mll` (§9) and
   that compiles to React + SwiftUI + Qt + XAML + HTML +
   WebComponent simultaneously.

3. **It clarifies the kernel's frozen contract.** The toolkit can
   only do what the kernel supports. The Tier-3 list (§3.3) is
   exactly the set of kernel features still needed for a
   "complete" Bootstrap-level catalog. That's a roadmap, not a
   wishlist.
