# Changelog — mosaic-pkg-toolkit

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
