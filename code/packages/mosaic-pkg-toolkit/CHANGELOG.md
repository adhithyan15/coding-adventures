# Changelog — mosaic-pkg-toolkit

## [Unreleased] — v0.11 — Select

### Added

**`Select`** — Bootstrap's `<select class="form-select">` distilled
to a toolkit component. Same toggle-button + revealed-options
pattern as DropdownMenu, but `onChange` carries the selected
option's *text* (not its index).

Composition: `Column[select] { HostButton[select-toggle], If(open)
Column[select-options] { For options HostButton[select-option] } }`.

Slots: `value`, `options: list<text>`, `placeholder`, `open`,
`disabled`. Emits: `onToggle`, `onChange(value: text)`.

### v0.11 note

The toggle's label is always `value`. When `value` is empty, the
host should pass the placeholder text in via `value` to display
the "Choose…" hint. Branching on value-truthiness inside the .mll
would collide on the `select-toggle` part name (moslayout compiler
rejects duplicate parts in If/Else branches); pushing the choice
to the host is the cleanest workaround for v0.11. When a native
`HostSelect` kernel primitive ships, Select can re-implement on
top of it without changing its public .mil interface.

### Tests

`tests/package_compiles.rs` grew to 23 (was 22): one more interface
test (`select_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [Unreleased] — v0.10 — Navbar

### Added

**`Navbar`** — Bootstrap's top-of-page brand + nav-link row.
Composition: `Row[navbar] { Text[navbar-brand], For items
HostLink[navbar-link] }`. Slots: `brand`, `items: list<text>`,
`active-index`. Emit: `onSelect(index: number)`.

Like Nav and Breadcrumb (v0.4), each link wraps the UI29-4
`HostLink` primitive — platform-native `role="link"` semantics +
Tab/Enter keyboard activation come from the kernel.

The brand is non-clickable in v0.10; an `onBrandClick` emit can be
added in a follow-up if hosts ask for it.

### Tests

`tests/package_compiles.rs` grew to 22 (was 21): one more interface
test (`navbar_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [Unreleased] — v0.9 — DropdownMenu

### Added

**`DropdownMenu`** — Bootstrap's toggle-button-with-revealed-menu
pattern. Composition: `Column[dropdown] { HostButton[dropdown-toggle],
If(open) Column[dropdown-menu] { For items HostButton[dropdown-item] } }`.
Slots: `label`, `items: list<text>`, `open`. Emits: `onToggle`,
`onSelect(index: number)`. Host owns the `open` state.

v0.9 renders the menu inline beneath the toggle (it takes vertical
space and pushes downstream content down). Bootstrap's absolutely-
positioned overlay needs a mosstyle z-index/position story that
the kernel doesn't yet expose; a future PR can add that.

### Tests

`tests/package_compiles.rs` grew to 21 (was 20): one more interface
test (`dropdown_menu_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [Unreleased] — v0.8 — Tabs

### Added

**`Tabs`** — Bootstrap's horizontal tab bar + single body panel.
Composition: `Column[tabs] { Row[tabs-bar] { For headers
HostButton[tabs-tab/tabs-tab-active] }, Text[tabs-panel] (content: active-body) }`.
Slots: `headers: list<text>`, `active-body`, `active-index`. Emit:
`onSelect(index: number)`.

Host owns the active-index → active-body mapping. Simpler than
Accordion's parallel-bodies workaround since only one body renders
at a time.

The active header now renders through a distinct `tabs-tab-active`
part so every backend can style selection without relying on
HostButton state support.

### Tests

`tests/package_compiles.rs` covers both the interface
(`tabs_interface_matches_spec`) and active-header part/style
regression (`tabs_active_header_part_compiles_and_is_styled`).
All pass.

## [Unreleased] — v0.7 — Accordion

### Added

**`Accordion`** — Bootstrap's vertical expand/collapse panel stack.
Composition: `Column[accordion] { For headers Column[accordion-item]
{ HostButton[accordion-header], Text[accordion-body] } }`.

Slots: `headers: list<text>`, `bodies: list<text>` (parallel array
to headers), `open-index: number` (-1 = all closed). Emit:
`onToggle(index: number)`.

### Known limitation

v0.7 ships with always-visible bodies because UI29's `If(when:)`
only supports truthiness, not the `i == open-index` comparison a
proper Accordion needs. The host simulates close by clearing the
body string for closed panels (empty body + `.msl` zero padding-on-
empty looks closed-enough).

The `.mil` surface (`open-index`, `onToggle`) is forward-compatible
— once kernel expression comparison lands, the `.mll`'s
`Text[accordion-body]` will be wrapped in `If(when: i ==
open-index)`. Hosts that adopt v0.7 today won't need to change
their glue code.

### Tests

`tests/package_compiles.rs` grew to 19 (was 18): one more interface
test (`accordion_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [Unreleased] — v0.6 — InputGroup

### Added

**`InputGroup`** — Bootstrap's "input with addons" pattern.
A single-line text input flanked by optional left and/or right
addon text (`$` before an amount, `.00` after, `@username`).
Composition: `Row[input-group] { If(prefix) Text[input-group-prefix],
HostInput[input-group-field], If(suffix) Text[input-group-suffix] }`.
Both addons are If-guarded on slot truthiness so an empty string
collapses the branch.

Slots: `prefix`, `suffix`, `value`, `placeholder`, `disabled`.
Emits: `onChange(value: text)`, `onCommit`.

Button-addons (e.g. a search-icon button on the right) need UI29-2
children pass-through; for now both addons are plain text.

### Tests

`tests/package_compiles.rs` grew to 18 (was 17): one more interface
test (`input_group_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [Unreleased] — v0.5 — Pagination

### Added

**`Pagination`** — Bootstrap's page-navigation row: `« prev | 1 2 3
| next »`. Composition: `Row[pagination] { HostLink[pagination-prev],
For (each: pages, as: page) { HostLink[pagination-page] (label: page,
onActivate: onPageSelect) }, HostLink[pagination-next] }`. Slots:
`pages: list<text>`, `prev-label`, `next-label`, `active-index`.
Emits: `onPrev`, `onNext`, `onPageSelect(index: number)`.

Like Nav and Breadcrumb (v0.4), every chip wraps the UI29-4
`HostLink` kernel primitive. Bootstrap's real-world DOM uses `<a>`
tags for the same elements — `role="link"` semantics + Tab/Enter
keyboard activation come from the kernel for free. `href: "#"` +
`external: false` mirror the Nav/Breadcrumb conventions.

### Tests

`tests/package_compiles.rs` grew to 17 (was 16): one more interface
test (`pagination_interface_matches_spec`) plus the new component
in the `COMPONENTS` array. All pass.

## [0.4.0] — UI29-4 — HostLink wires Breadcrumb + Nav; Tooltip + NumberInput added

UI29-4 added three new kernel primitives (`HostLink`, `HostTooltip`,
`HostNumberInput` — positions 19/20/21). v0.4 takes advantage of all
three: Breadcrumb + Nav move from `HostButton` to `HostLink`
(internal rewrite — user-facing emits unchanged), and two new
components (`Tooltip`, `NumberInput`) wrap `HostTooltip` and
`HostNumberInput` respectively.

### `Breadcrumb` — `HostLink` rewrite

- **Layout:** each crumb now emits `HostLink[breadcrumb-link]`
  instead of `HostButton[breadcrumb-link]`. The platform-native
  link semantics — `role="link"` + visited-state styling on web,
  `Link` widget on SwiftUI/Flutter, rich-text anchor on Qt,
  `Hyperlink` on XAML — come from the kernel for free now.
- **Internal change:** The wrapped `HostLink`'s click event is
  `onActivate` (not `HostButton`'s `onClick`). The toolkit's
  user-facing `Breadcrumb.onSelect(index)` is unchanged.
- **`href: "#"` default + `external: false`:** the toolkit
  doesn't know the host's routing scheme, so each crumb ships
  with an inert anchor and `external: false` tells the kernel
  to skip the `window.open`/`launchUrl` shape (host routes via
  `onSelect`). A follow-up could add a `hrefs: list<text>`
  slot for hosts that want true per-crumb URLs.

### `Nav` — `HostLink` rewrite (same shape)

- **Layout:** each item now emits `HostLink[nav-link]` instead of
  `HostButton[nav-link]`. Same rationale + same `href: "#"` +
  `external: false` choices as Breadcrumb.
- **Interface:** unchanged — `Nav.onSelect(index)` still fires
  on activation; internal wiring moved from `onClick` to
  `onActivate`.

### `Tooltip` — new component

- **Layout:** `HostTooltip[tooltip-wrapper] { Text[tooltip-label] }`.
  Wraps a visible label `Text` in a `HostTooltip` whose `text`
  shows on hover (desktop / web) or long-press (mobile).
- **Interface:** `message: text` (tooltip body — named `message`
  rather than the kernel-side `text` because `text` is a
  reserved keyword in the .mil grammar), `label: text` (visible
  text). No emits — tooltips are display-only.
- **Scope (v0.4):** plain-text tooltips only, per UI29-4
  spec §3.2's scoping decision. Rich-content tooltips (icons,
  multiline, formatted) wait for a follow-up `Popover`
  component or for the kernel to grow children-pass-through.

### `NumberInput` — new component

- **Layout:** single `HostNumberInput[number-input]` with
  toolkit-default border / padding chrome (matches `Input`).
- **Interface:** `value: number`, `placeholder: text`,
  `disabled: bool`, `onChange(value: number)`. Per spec §3.3,
  `onChange` fires on commit (Enter / blur), not per-keystroke.
- **Why a separate component (not just `Input`):** the kernel's
  `HostNumberInput` lights up the numeric keypad on mobile
  (iOS/Android), ± stepper buttons on Qt SpinBox / XAML
  NumberBox, and decimal-format locale awareness — none of
  which `Input` + manual validation can replicate per-backend.
- **`min`/`max`/`step` omitted from the toolkit interface:**
  these are compile-time numeric literals on the kernel
  primitive rather than runtime slots. Authors who need range
  constraints compose `HostNumberInput` directly.

### Migration guide

User-facing emits are unchanged across all four touched components:

| Component   | Emit (v0.3) | Emit (v0.4) | Change           |
|---|---|---|---|
| Breadcrumb  | `onSelect(index)` | `onSelect(index)` | none (internal rewrite only)  |
| Nav         | `onSelect(index)` | `onSelect(index)` | none (internal rewrite only)  |
| Tooltip     | n/a (new)         | (no emits)        | new component                 |
| NumberInput | n/a (new)         | `onChange(value)` | new component                 |

Hosts using only the documented toolkit-component emits should
see no behavioural difference beyond platform-native a11y /
keyboard upgrades on Breadcrumb + Nav (link role, visited-state
styling, Tab/Enter activation come from the kernel). The only
breaking surface would be hosts that reached into generated
artifact internals' `onClick` handlers — those need to migrate
to `onActivate`.

### Manifest

- `version` bumped 0.3.0 → 0.4.0.
- `exports` now lists 16 components (was 14): added `NumberInput`
  and `Tooltip`.

## [0.3.0] — UI29-2 — Checkbox + Radio rewritten on native primitives

**Breaking change.** `Checkbox` and `Radio` previously composed from
`HostButton` + an `If/Else` to fake checked/unchecked glyph states.
That shape lost the platform-native a11y role, focus ring, tri-state
visual, keyboard semantics, and (for radios) browser/platform-enforced
group mutex. UI29-2 added `HostCheckbox` and `HostRadio` to the kernel
as the 17th and 18th primitives; v0.3 makes both userland components
thin wrappers around those primitives.

### `Checkbox`

- **Layout:** `Row[checkbox]{If/Else{HostButton},HostButton,Text[label]}`
  collapses to a single `HostCheckbox[checkbox]` line.
- **Interface changes:**
  - `onChange` now carries `checked: bool` (was payloadless).
  - New optional slot `indeterminate: bool` for tri-state. Fully
    wired on Qt / XAML / HTML / WebComponent; React backend
    silently ignores it pending the indeterminate follow-up PR.
- **Parts removed:** `checkbox-box-checked`, `checkbox-box-unchecked`,
  `checkbox-label`. The native widget renders its own glyph and the
  label is wired internally; only the `checkbox` root part survives.
- **.msl files trimmed** to just the root-part padding rule.

### `Radio`

- **Layout:** identical `Row{If/Else{HostButton},HostButton,Text}`
  collapse to a single `HostRadio[radio]` line.
- **Interface changes:**
  - Slot `selected` renamed to `checked` for consistency with both
    Checkbox and the kernel-canonical `HostRadio.checked`.
  - `onSelect` now carries `value: text` (was payloadless).
  - New slot `value: text` — the value this radio represents,
    carried as the `onSelect` payload.
  - New slot `group: text` — the radio-group name. Backends with
    native group semantics (HTML `name=`, WinUI `GroupName=`) wire
    this to the platform's mutex; backends without (SwiftUI / Qt v1)
    preserve it as metadata.
- **Parts removed:** `radio-box-selected`, `radio-box-unselected`,
  `radio-label`. Only the `radio` root part survives.

### Migration

Pre-v0.3 hosts using `Checkbox`:

```moslayout
// Before (v0.2):
Checkbox (
  label    : "Remember me" ,
  checked  : slot: remember ,
  disabled : false ,
  onChange : emit: onToggle      // host inverted slot manually
)

// After (v0.3): same usage; host receives the new value.
Checkbox (
  label    : "Remember me" ,
  checked  : slot: remember ,
  disabled : false ,
  onChange : emit: onToggle      // payload: { checked: bool }
)
```

Pre-v0.3 hosts using `Radio`:

```moslayout
// Before (v0.2):
Radio (
  label    : "Vanilla" ,
  selected : slot: is-vanilla ,
  disabled : false ,
  onSelect : emit: onPick
)

// After (v0.3):
Radio (
  label    : "Vanilla" ,
  checked  : slot: is-vanilla ,
  value    : "vanilla" ,
  group    : "flavor" ,
  disabled : false ,
  onSelect : emit: onPick        // payload: { value: text }
)
```

### Tests

Two existing tests (`checkbox_interface_matches_spec`,
`radio_interface_matches_spec`) updated for the new slot rosters and
emit payloads. Full test suite stays at 16 passing.

## [Unreleased] — v0.1 PR-5 — Field (form group)

### Added

**`Field`** — Bootstrap's "form group" pattern. Labelled text input
with optional help text below (or error text in red when validation
fails). Composition: `Column[field]` containing label `Text`, the
`HostInput[field-input]`, and an `If error / Else help` pair of
muted-vs-danger texts. Slots: `label`, `value`, `placeholder`,
`help`, `error`, `disabled`. Emits: `onChange(value: text)`,
`onCommit`.

### Why HostInput inline, not Input via component reference

Field would naturally reference the toolkit's `Input` component
since both wrap HostInput similarly. But UI29 §4.4 routes
component-reference resolution through the manifest's
`[dependencies]` table — and a package can't depend on itself,
so Field can't reference Input from the same package today.

Inlining HostInput in Field.mll is the path of least resistance. A
small follow-up can add self-reference support to the resolver
(auto-register the active package's own `[components].exports`);
until then the duplication is acceptable.

### Tests

`tests/package_compiles.rs` grew to 13 (was 12). All pass.

**11 of 13 Tier-1 components shipped.** Remaining: Card, Container
— both still blocked on the UI29-2 children pass-through spec
(#3947).

## [Unreleased] — v0.1 PR-4 — ListGroup / Modal

### Added — 2 more Tier-1 components

- **`ListGroup`** — vertical list of selectable text rows.
  Bootstrap's `<ul class="list-group">` distilled to `list<text>` +
  onSelect-with-index. Composition:
  `Column[list-group] { For (each: items, as: item, index: i) {
  HostButton[list-group-item] (label: item, onClick: onSelect) } }`.
  First toolkit component that uses the `For` block.

- **`Modal`** — titled modal dialog wrapping the kernel `HostDialog`.
  Composition: `HostDialog[modal-shell] (open, title, onClose,
  modal: true) { Column { Box[modal-body], Box[modal-actions] {
  HostButton[modal-close-btn] } } }`. The XAML backend hoists
  HostDialog to a `<ContentDialog>` root (Fix A1 from the demo
  catalog); other backends produce their native dialog idioms.

### Constraints carried forward

Both components ship a "text-only" surface for v0.1:
- ListGroup's items are `list<text>` — not `list<node>`. Rich rows
  (icons, secondary text) need the children-pass-through kernel
  feature that's the subject of the upcoming UI29 follow-up spec.
- Modal's body is a single `message: text` slot — same reason. The
  full-fidelity dialog with arbitrary content lands when
  children-pass-through does.

A future `RichListGroup` and a children-aware Modal variant can
layer on top once the kernel feature exists; the v0.1 surfaces stay
forward-compatible.

### Tests

`tests/package_compiles.rs` grew to 12 (was 10). 10 components total
in the manifest. All pass.

**10 of 13 Tier-1 components shipped.** Remaining: Card, Container,
Field — all blocked on the children-pass-through spec.

## [Unreleased] — v0.1 PR-3 — Input / Checkbox / Radio (form controls)

### Added — 3 form-control components

- **`Input`** — styled single-line text input. `HostInput[input]`
  wrapper. Slots: `value`, `placeholder`, `disabled`, `size`. Emits:
  `onChange(value: text)`, `onCommit`.
- **`Checkbox`** — labeled checkbox. `Row[checkbox] { If checked
  HostButton[checkbox-box-checked] Else HostButton[checkbox-box-unchecked],
  Text[checkbox-label] }`. State is host-owned. Slots: `label`,
  `checked`, `disabled`. Emit: `onChange`.
- **`Radio`** — labeled radio button. Structurally identical to
  Checkbox but with circular styling (border-radius=9 on the
  18×18 box). Slots: `label`, `selected`, `disabled`. Emit:
  `onSelect`.

Both Checkbox and Radio use the `If/Else over the checked/selected
slot` pattern to swap the glyph between checked/unchecked states.
Each branch is a separate styleable part (e.g. `checkbox-box-checked`
vs `checkbox-box-unchecked`), so the .msl theme can apply different
colors per state.

### Notes on the kernel-mapping trade-off

The kernel doesn't include a `HostCheckbox` / `HostRadio` primitive
(UI29 v1 scope). The toolkit's Checkbox / Radio compose from
HostButton + If/Else + Text. This works but means the underlying
control is a button semantically, not a checkbox/radio — screen
readers may announce it as "button" rather than "checkbox" on some
backends. A future UI29-2 spec could add `HostCheckbox` /
`HostRadio` to close this a11y gap; the toolkit can switch
implementation without changing its public surface.

### Tests

`tests/package_compiles.rs` grew to 10 (was 7):
- Manifest exports list updated alphabetically.
- Per-component surface tests added for Input, Checkbox, Radio.

All pass. Total components in the manifest: 8.

## [Unreleased] — v0.1 PR-2 — Badge / Spinner / Toast

### Added — 3 more Tier-1 components

- **`Badge`** — pill label. `Box[badge] { Text[badge-text] }`. Slots:
  `label`, `variant`. No emits. Used for counts and status tags.
- **`Spinner`** — indeterminate loading indicator.
  `Stack[spinner] { Icon[spinner-glyph] }`. Slots: `size`, `variant`,
  `aria-label`. No emits. Animation is intentionally backend-driven
  (CSS keyframes / SwiftUI `.rotationEffect()` / etc.); v0.1 ships
  the static glyph + color and the host adds rotation in its own
  stylesheet overlay.
- **`Toast`** — bottom-anchored notification.
  `If open { Box[toast] { Column { Row[toast-header], Box[toast-body] } } }`.
  Slots: `title`, `message`, `variant`, `open` (bool). Emit:
  `onClose`. Visibility driven by the `open` slot via an `If` block
  (which auto-triggers `BoolToVisibilityConverter.cs` emission in
  the XAML backend — proving the Fix A5 pipeline works for
  toolkit components too).

Each new component ships `.light.msl` and `.dark.msl` themes.

### Changed

- `mosaic-package.toml` `[components].exports` grew to 5: Alert,
  Badge, Button, Spinner, Toast (alphabetical).
- Smoke test (`tests/package_compiles.rs`) now drives 5 components.
  Total: 7 tests (was 4 in PR-1), all passing.

### Compile-time verification

All three new components compile cleanly through `mosaic-compile
--backend xaml`. Toast's `If open` block produces the expected
`<ContentControl Visibility="...">` wrapping pattern + the
auto-emitted `BoolToVisibilityConverter.cs` side-file.

## [0.1.0] — Unreleased — PR-1 scaffold

### Added

First implementation per `code/specs/mosaic-pkg-toolkit.md`.

- **Package scaffold**: `mosaic-package.toml` (kernel-only,
  v1-targeting, zero dependencies), `Cargo.toml` (smoke-test
  harness, opted out of workspace per the userland-package
  convention), `src/lib.rs` (doc-only), `README.md`, this
  `CHANGELOG.md`.

- **`Button` component** — the first Tier-1 catalog entry.
  - `Button.mil` — slots: `label`, `variant`, `size`, `disabled`. Emit: `onClick`.
  - `Button.mll` — wraps the kernel `HostButton` with `part_name: button`.
  - `Button.light.msl` + `Button.dark.msl` — base part styling.
    Variant sub-parts (`button/primary`, `button/danger`, etc.)
    deferred to the spec §4.1 sub-part syntax decision.

- **`Alert` component** — the second Tier-1 catalog entry.
  - `Alert.mil` — slots: `message`, `variant`, `dismissible`. Emit: `onClose`.
  - `Alert.mll` — `Box[alert]` → `Row` → `Text[message]` + `If
    dismissible HostButton[close-btn]`. Pure kernel composition.
  - `Alert.light.msl` + `Alert.dark.msl` — base part styling.

- **Smoke test** (`tests/package_compiles.rs`) — asserts the
  manifest is consistent (correct name/version, exports list
  matches the per-component compile loop, zero dependencies,
  kernel v1) and that every exported component's
  `.mil`/`.mll`/`.{light,dark}.msl` triple round-trips through
  the three IR compilers. Modeled on `mosaic-pkg-card`'s smoke
  test.

### Known limitations (deferred to follow-up PRs)

- **Variant styling.** The `variant` slot on `Button` and `Alert`
  is accepted by the .mil but doesn't currently change the
  rendered output. The mosstyle sub-part syntax for variants is
  spec §4.1's open question and lands when the first follow-up PR
  picks the design.
- **The full Tier-1 catalog** (13 components from spec §3.1) —
  Badge, Card, Checkbox, Container, Field, Input, ListGroup,
  Modal, Radio, Spinner, Toast — lands across follow-up PRs.
- **Tier 2 / Tier 3 / theming work** per the spec's §7 phasing
  plan.
- **Per-backend visual smoke tests.** The current smoke test
  verifies only IR-level round-trip; backend-specific rendering
  tests (does Button look right in XAML? in React? in SwiftUI?)
  belong in each backend's own integration test suite and will
  land per-component as the catalog grows.
