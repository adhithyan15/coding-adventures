# Changelog — mosaic-pkg-toolkit

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
