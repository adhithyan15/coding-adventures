# UI29-6 — `HostNavigationSplit`

Issue: [#15481](https://github.com/adhithyan15/coding-adventures/issues/15481)

**Status:** Specification (draft). Implementation follows in the slices in §7.

**Parent:** [UI29](UI29-primitive-kernel.md) §2.4, which requires a numbered
amendment per primitive. **Requested by:** [UI48](UI48-host-environment.md) §5.4,
which names this primitive and defers it deliberately: *"it is a kernel
primitive under UI29's rules and needs its own spec and its own per-backend
degradation story."*

(UI29-5 is reserved by UI29-4 §4 for the `HostSelect` / `HostMenu` batch, which
is gated on record slot types. This takes the next free number instead of
jumping that queue.)

## 1. The question this settles

Every product in #14415 draws a navigation pane beside a detail area. The
kernel has no container for it, so each one builds the same shape out of `Row`
and `Column` — and each answers the same question differently:

| where | pane width | separator | narrow screens |
| --- | --- | --- | --- |
| `TaskApp.mll` rail + main | `width: 236; flex-shrink: 0` | `border-right` on the pane | none — this is #13692 |
| `mosaic-pkg-notes` list + editor | `width: 220; flex-shrink: 0` | `border-right` on the pane | none |
| `mosaic-pkg-note-type-editor` | none (intrinsic) | none | none |
| `mosaic-pkg-spice-workbench` | `width: "30%"` / `"70%"` | none | none |

Four layouts, four mechanisms, one shape. None collapses on a narrow window,
and UI30 variant selection is build-time, so none *can* at runtime.

## 2. Measured: nothing native is emitted today

A search of every emitter in `code/packages/rust/` for `NavigationView`,
`NavigationSplitView`, `SplitView`, `NavigationRail`, `ListDetailPaneScaffold`,
`NavigationDrawer` and `NavigationSuiteScaffold` finds **no matches**. Each
backend lowers `Row`/`Column` to its flex or stack container and nothing else.

Pane semantics fare no better. `a11y-role` reaches only `heading` and `none` on
the five native backends; every other role, including a landmark, is dropped
without a report. Only React and WebComponent pass a role through.

## 3. Why this belongs in the kernel

UI29 §2.3 lists **Drawer** as userland, and the boxes above prove the *visual*
shape composes. This is the UI29-2 situation exactly — "a checkbox is a
`HostButton` underneath" — and the answer is the same: what composition cannot
reproduce is not the boxes.

1. **Every host platform has a native equivalent** (§4 has the table). Apple,
   Windows and Android ship adaptive navigation containers; Qt ships
   `SplitView`; the web has landmark structure. What each *lacks* is recorded
   per backend in §4.3 rather than hidden.
2. **No reasonable composition exists** for the adaptive collapse. UI48 §5.6:
   *"A `UISplitViewController` or a WinUI `NavigationView` adapts internally, in
   the platform's own layout pass. Routing its behavior through an event —
   resize, dispatch, adapter state, new props, re-render — would be slower,
   would jank against the platform's own animation, and would replace a real
   native control with a hand-rolled imitation."* A layout cannot express that
   at all today: UI30 picks a variant at build time.
3. **It is semantically irreducible.** A navigation pane is a landmark. A
   screen reader user moves by landmark, and "the pane" versus "the detail" is
   the structure they navigate by. Two `Column`s carry no such structure, and
   the collapsed state adds a second irreducible piece: the platform's own
   back-navigation between pane and detail.

Adding it brings the kernel to 35 primitives. UI29 §2.4 permits exactly this
growth, through exactly this kind of numbered amendment.

## 4. The surface

```
HostNavigationSplit [ app-shell ] (
  pane-title : slot: nav-title ,
  pane-width : 236 ,
  collapse : auto
) {
  Column [ rail ] { ...the navigation pane... }
  Column [ main ] { ...the detail area... }
}
```

### 4.1 Slots

| prop | kind | required | meaning |
| --- | --- | --- | --- |
| `pane-title` | text (literal, slot or expression) | yes | Names the pane for assistive technology, and fills the platform's pane header where one exists. Required because an unnamed landmark is the defect this primitive exists to fix. |
| `pane-width` | number | no | The pane's **preferred** width, in the same units the style layer uses. A hint: a platform that has its own expanded-pane metric may keep it. Absent means the platform's default. |
| `collapse` | keyword `auto` \| `never` | no | `auto` (the default) lets the platform collapse the pane as the window narrows. `never` pins it side-by-side, for a layout whose pane is the content (VisiCalc's workbench). |

**Children:** exactly two, in order — the pane, then the detail. A different
count is a compile error in `moslayout-compiler`, not a silent drop. A missing
`pane-title` is a compile error in the same place, for the same reason: five
emitters each inventing an answer to an absent name is five different unnamed
panes, and by lowering time the author is gone. The kernel
has no named child slots, and adding them for one primitive would be a grammar
change; order is what `HostTable`'s rows already rely on.

**Sub-parts:** `navigation-split:pane` and `navigation-split:detail`, so a style
can reach each region without the layout naming a part per side.

**No emits.** The control's adaptation is not an application event (UI48 §5.6);
the app does not need telling that the platform collapsed a pane, and asking
for that back would be the round trip §5.6 rules out. If a product later needs
the collapsed state, it arrives through UI48's environment, not through here.

### 4.2 Lowering

| backend | lowers to |
| --- | --- |
| SwiftUI | `NavigationSplitView { pane } detail: { detail }`, `.navigationTitle(pane-title)` on the pane |
| XAML | `NavigationView` with `PaneDisplayMode="Auto"`, `IsSettingsVisible="False"`, the pane as `PaneCustomContent`, the detail as its content |
| Compose | `NavigationSuiteScaffold` (or `ListDetailPaneScaffold` where the pane is a list), pane as the navigation suite, detail as content |
| Qt | `SplitView` with the pane as the first item, `SplitView.preferredWidth` from `pane-width`, `Accessible.role: Accessible.Pane` on the pane |
| Flutter | `Row` of pane and detail at regular width, the pane as a `Drawer` at compact, wrapped in `Semantics` |
| React / HTML / WebComponent | a flex container: pane as `<nav aria-label="…">`, detail as `<section>`; `pane-width` as the pane's flex basis |

### 4.3 What each backend does not have, recorded

- **Qt** has no adaptive collapse: `SplitView` is static. `collapse: auto`
  therefore reports `interaction.navigation-split-collapse-static` in the
  non-gating `behaviorDegradations` inventory.
- **Flutter** has no core adaptive split container; the lowering above is a
  composition and reports the same code in `behaviorDegradations`.
- **The web backends** can carry the landmark structure but not the collapse,
  which needs a viewport signal (UI48). They are not native-complete targets,
  so this is stated here rather than reported.
- **Every backend, before its lowering lands**, reports
  `primitive.navigation-split-unimplemented` — the `HostSwitch` pattern,
  narrowed one backend at a time as `HostProgressRing` was. **XAML** came off
  that list in slice `K-xaml`; SwiftUI, Compose, Qt and Flutter remain.
- **XAML** carries no gap of its own. `NavigationView`'s `Auto` mode is the
  adaptive ladder this primitive exists to reach, `PaneTitle` is both the
  drawn header and the UIA name, and `OpenPaneLength` is a preferred width in
  exactly the sense §4.1 means it — WinUI keeps its own compact and flyout
  widths when the pane narrows or folds. `collapse: never` pins the pane with
  `PaneDisplayMode="Left"` plus `IsPaneToggleButtonVisible="False"`, so the
  toggle cannot undo by hand what the author pinned.

The two report axes are deliberately different. A
`primitive.navigation-split-unimplemented` entry is a gating capability gap:
the backend has not lowered the primitive at all. An
`interaction.navigation-split-collapse-static` entry is an honest record of a
platform limitation after a real lowering exists. It is written to
`behaviorDegradations`, stays outside `degradations` and `nativeComplete`, and
therefore needs no consumer allowlist. `collapse: never` requests the static
shape and produces no behaviour entry.

## 5. Consumers, and the order they must land in

The eleven `native_complete_gate.rs` files allow **no** capability allowlist:
*"the assertion is zero and an allowlist would only be somewhere for a
regression to hide."* So a package rewritten onto this primitive turns its gate
red until **every** native backend lowers it. Qt and Flutter's permanent
collapse limitation remains visible in `behaviorDegradations` after their
capability entries clear; it is not smuggled through an allowlist and does not
make the consumers permanently impossible to build. The order is forced:

1. the spec (this file) and the issue;
2. registration plus the unconditional degradation, together;
3. one lowering PR per backend, in parallel;
4. only then the consumers: TaskApp's rail (closing #13692 without a variant),
   `mosaic-pkg-notes`, `mosaic-pkg-note-type-editor`, and
   `mosaic-pkg-spice-workbench` with `collapse: never`.

## 6. Acceptance criteria

- Registered in `moslayout-compiler::PRIMITIVES` and
  `mosaic-package-resolver::KERNEL_PRIMITIVES`, with the child-count rule
  enforced by the layout compiler and a test for the error.
- `primitive.navigation-split-unimplemented` lands in the same PR as the
  registration, and each backend's lowering PR removes that backend from the
  guard in the same change that adds the lowering — so the report cannot drift
  from what is emitted (UI84 §3).
- Every native lowering is checked against the real toolchain, as UI86's were:
  a fixture package per backend, generated native-complete with zero
  capability degradations and built by that platform's compiler in CI. Qt and
  Flutter additionally pin their expected `behaviorDegradations` entry for
  `collapse: auto` and its absence for `collapse: never`.
- Collapse behaviour is asserted by resizing a running window, not by reading
  the markup back — but **not in the emitter fixtures**, which cannot do it.
  A fixture component has no engine behind it, so its executable fail-fasts at
  startup; the merged UI86 fixture behaves the same way, and
  `xaml-selected-button-smoke.ps1` says in its own help that its runtime half
  is for a locally patched build. The runtime proof therefore lands in the `P`
  slice, against TaskApp, which is a real app with a real engine and already
  has a UIA smoke in CI. Tracked as
  [#15486](https://github.com/adhithyan15/coding-adventures/issues/15486),
  including the `collapse: never` contrast case that is what actually
  distinguishes `Auto` from `Left`.
- The four consumers keep their current appearance: the same pane width, the
  same separator, and no new gating degradation in any package gate. Expected
  permanent platform limitations remain machine-readable in
  `behaviorDegradations`.

## 7. Implementation slices

| slice | what |
| --- | --- |
| `U29-6-0` | this spec |
| `U29-6-G` | registration + child-count validation + the unconditional degradation |
| `U29-6-K-xaml` | **done** — `NavigationView`, `PaneDisplayMode="Auto"` |
| `U29-6-K-swiftui` … `-compose` … `-qt` … `-flutter` | one lowering each, in parallel |
| `U29-6-K-web` | React, HTML, WebComponent landmarks |
| `U29-6-P` | the four consumers, after every `K` has landed |

## 8. What this spec does not do

- **No resize handle.** The repo has no splitter anywhere, and a draggable
  divider is a separate primitive with its own persistence question.
- **No runtime environment.** Which size class the window is in belongs to
  UI48 (#14003). This primitive consumes the platform's own adaptation instead
  of asking for it.
- **No routing.** What the detail shows stays the application's business, as it
  is in all four consumers today.
- **No collapsed-state event** (§4.1).
