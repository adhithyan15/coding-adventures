# Changelog — mosaic-pkg-toolkit

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
