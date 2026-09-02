# Mosaic component program v1 — leaf-to-root, isolation-first

**Tracked by:** [#14011](https://github.com/adhithyan15/coding-adventures/issues/14011)
**Status:** Program specification. Supersedes the app-first working method in
`task-app-platform-completion-v1.md`.
**Depends on:** UI29 (primitive kernel), UI19 (MosaicBook), `mosaic-pkg-toolkit`
**Pauses:** #13517 (TaskApp completion epic)
**Absorbs:** the standalone Checklist app (`code/programs/typescript/checklist-app`)

---

## 1. Why this exists

TaskApp was built app-first: author the whole shell, then discover what the
platform cannot do when a feature needs it. That order produced working
software, but it discovers architecture at the worst possible moment — after
the code depending on it is written.

Two examples from one working session:

- #13692 (a compact-window layout) turned out to need a runtime host
  environment that does not exist in any of nine backends, specified after the
  fact as UI48.
- Writing UI48 then surfaced that Mosaic has no adaptive container primitive to
  lower onto `UISplitViewController` / `NavigationView`, so the "fix" would have
  hand-rolled an imitation of a control every platform already ships.

Neither was visible from the app. Both were visible immediately from the
component.

### The evidence, quantified

`TaskApp.light.msl` declares **166 styled parts**. `TaskApp.mll` uses **47 raw
`HostButton`s**, **9 raw `HostInput`s**, and **48 `If` / 22 `Else`** nodes.
Meanwhile `mosaic-pkg-toolkit` already ships **23 composed components** —
`Button`, `Input`, `Field`, `Badge`, `Alert`, `Modal`, `Tabs`, `Select`, and
more — built purely from kernel primitives and lowering to every backend.
**TaskApp uses none of them.**

The clearest single artifact: the view switcher is one segmented control, and
it occupies **36 parts** — `seg-list-on`, `seg-list-off`, `seg-list-off2`
through `seg-list-off5`, times six views. The cause is a six-way nested
`If`/`Else` chain that re-declares all six buttons in every branch, each needing
a unique part name. One `SegmentedControl` component with a `selected` slot is
one instance. The timeline legend is duplicated four times for the same reason
(`tl-legend-item`, `item2`, `item3`, `item4`).

This is not a styling problem. It is what an application looks like when it is
written without a component layer beneath it.

---

## 2. What already exists

This program is largely reconciliation, not invention. Verified against `main`.

| Piece | State |
| --- | --- |
| `mosaic-pkg-toolkit` | **23 components shipped**, kernel-only, all backends. Has its own spec with a Tier 1/2/3 catalog and phasing plan |
| Other `mosaic-pkg-*` | 18 more packages, mostly Engram's (`card`, `deck-options`, `review-*`, `note-*`) plus `sheet`, `calendar`, `notes`, `project-nav`, `grid`, `dialog` |
| MosaicBook | **Implemented** — `code/programs/go/mosaicbook-server`, discovers three-file UI29 components, compiles on demand, hot-reloads |
| Test-app precedent | `toolkit-multi-demo`, `toolkit-xaml-showcase`, `hello-dialog-xaml` — ad hoc, not one per component |

### Four gaps that block the method

1. **MosaicBook is not in CI.** No workflow references it. "Tested in isolation
   in MosaicBook and then committed" is currently a manual habit, not a gate.
2. **MosaicBook is browser-only.** It compiles HTML, Web Component, and React.
   Qt, SwiftUI, XAML, Compose, and Flutter are not previewable, so isolation
   testing cannot see the backends most likely to break.
3. **Nothing is published.** `mosaic-pkg-toolkit` is `publish = false`. A
   "sharable and core" package that cannot be consumed outside this repository
   is not yet a package.
4. **There is no per-component test-app lane.** Two demo apps exist; neither is
   generated from a component nor released per component.

These are the first work items. A component program without them is a
convention, not a process.

---

## 3. The method

For every component, in order, leaf to root:

1. **Build it in isolation**, composed from kernel primitives (`Host*`) or from
   already-landed components. Never from a lower-level primitive that a landed
   component already wraps.
2. **Story it in MosaicBook** with fixtures covering every state — variants,
   sizes, empty, error, overflow, both themes.
3. **Gate it in CI** through MosaicBook, on every supported backend.
4. **Package it if it is sharable and core**, and publish it.
5. **Ship a test app for it** — its own tiny Mosaic program, released on the
   supported platforms, kept as a durable artifact for future work.
6. **Then, and only then**, compose it into something larger.

**The app is allowed to be an empty screen for as long as this takes.** TaskApp
is the last consumer, not the driver. Landing one correct component beats
advancing the app.

**Missing a component is acceptable.** The inventory below is a starting
hypothesis, not a contract. Components discovered mid-flight get filed and
slotted into the tree at the right level.

---

## 4. Component inventory

Derived from TaskApp's 166 parts and the Checklist app's decision-tree model.
Level = distance from the kernel. Nothing at level N may depend on level N+1.

### L0 — kernel primitives

**Shipped:** `HostButton`, `HostInput`, `HostNumberInput`, `HostCheckbox`,
`HostRadio`, `HostSwitch`, `HostSlider`, `HostLink`, `HostScroll`,
`HostSurface`, `HostTable` (+ head/body/foot/colgroup), `HostDialog`,
`HostTooltip`, `HostDraggable`, `HostDropTarget`, `HostProgressRing`. Layout:
`Row`, `Column`, `Stack`, `Box`, `Text`, `Grid`, `If`/`Else`, `For`.

**Missing, each needing its own kernel spec:**

- **`HostNavigationSplit`** — the adaptive container from UI48 §5.4. Lowers to
  `UISplitViewController`, WinUI `NavigationView`, `NavigationSuiteScaffold`.
  Blocks the app shell and TaskApp #13692.
- **Host environment** — UI48 (#14003). Blocks anything size- or
  input-responsive.

### L1 — atoms

| Component | Built on | Status |
| --- | --- | --- |
| `Button`, `Input`, `NumberInput`, `Checkbox`, `Radio`, `Select`, `Badge`, `Alert`, `Spinner`, `Tooltip`, `Field` | `Host*` | **In toolkit** — needs stories, CI gate, test app |
| `ProgressRing` | `HostProgressRing` | Spec'd as UI40; #13176 open; TaskApp hand-rolls it as 7 parts (`ring-*`) |
| `Chip` | `Box` + `Text` | **Missing.** TaskApp has 5 (`chip-due`, `chip-labels`, `chip-over`, `chip-priority`, `chip-sched`) |
| `StatusPill` | `Row` + `Box` + `Text` | **Missing.** TaskApp has 4 (`pill-ok`, `pill-warn`, `pill-dot-*`) |
| `EmptyState` | `Column` + `Text` | **Missing.** TaskApp has 3 (`empty-state`, `empty-title`, `empty-body`) |
| `Icon` | `Box`/`Path` (UI39) | **Missing.** The deferred segmented-switch icon set needs it |

### L2 — molecules

| Component | Built on | Why |
| --- | --- | --- |
| **`SegmentedControl`** | `ButtonGroup` + `Button` | **The highest-value item in this document.** Replaces 36 parts with one instance |
| `Legend` | `For` + `Chip` | Replaces the 4× duplicated `tl-legend-*` |
| `ValidatedField` | `Field` + `Alert` | TaskApp hand-rolls focus/error/corrected across 10 parts per field |
| `InlineEditForm` | `ValidatedField` + `ButtonGroup` | `edit-*`, 8 parts |
| `Composer` | `ValidatedField` + `Button` | `composer-*`, 5 parts |
| `StatusPanel` | `Alert` + `Text` | `storage-*`, 5 parts |
| `Toolbar` | `Row` + `SegmentedControl` | `topbar`, `title-block`, `subline` |

### L3 — organisms

`TaskRow`, `TaskList`, `TaskDetail`, `BoardCard`, `BoardColumn`, `Board`,
`GanttChart`, plus the four that already exist as packages (`Sheet`,
`Calendar`, `Notes`, `ProjectNav`) and must be re-verified against this method.

**From the Checklist app:** `ChecklistItem`, `DecisionNode`, `ChecklistRunner`.
The standalone TypeScript/Electron app is deprecated and its decision-tree model
becomes these components (§6).

### L4 — applications

`TaskApp` and `Checklist`, each assembled from L3 and nothing lower.

---

## 5. Build order

Strictly leaf to root. Each phase completes — stories, CI gate, package,
published, test app released — before the next begins.

- **Phase 0 — infrastructure.** The four gaps in §2 — MosaicBook in CI
  ([#14012](https://github.com/adhithyan15/coding-adventures/issues/14012)), native previews ([#14013](https://github.com/adhithyan15/coding-adventures/issues/14013)), publishing
  ([#14014](https://github.com/adhithyan15/coding-adventures/issues/14014)), the test-app lane ([#14015](https://github.com/adhithyan15/coding-adventures/issues/14015)). Nothing else
  can be "done" until "done" is enforceable.
- **Phase 1 — L1 atoms.** Retrofit the 23 existing toolkit components to the
  full contract first ([#14017](https://github.com/adhithyan15/coding-adventures/issues/14017)); they are already written, so this
  measures what the contract costs before it is imposed on new work. Then the
  four missing atoms.
- **Phase 2 — L0 gaps.** `HostNavigationSplit` and the host environment. Placed
  after Phase 1 because both need real components to demonstrate against.
- **Phase 3 — L2 molecules.** `SegmentedControl` first ([#14016](https://github.com/adhithyan15/coding-adventures/issues/14016)).
- **Phase 4 — L3 organisms**, including folding in Checklist ([#14018](https://github.com/adhithyan15/coding-adventures/issues/14018)).
- **Phase 5 — L4.** TaskApp is rebuilt on the tree. Only here does the app
  stop being an empty screen.

---

## 6. Deprecating the Checklist app

`code/programs/typescript/checklist-app` is a TypeScript/Vite/Electron app
outside the Mosaic stack entirely. It models checklists as decision trees, which
is a genuinely reusable idea and the reason it is folded in rather than deleted.

- Its model becomes `ChecklistItem` / `DecisionNode` / `ChecklistRunner` at L3.
- The Electron shell is retired; `release-checklist.yml` is retired with it.
- The app is rebuilt at L4 on the same tree TaskApp uses.
- The existing code stays in git history and is not force-deleted; the spec
  `checklist-app.md` is amended to point here rather than rewritten.

---

## 7. Definition of done, per component

A component is done when **all** of these are true. Anything less is in
progress, regardless of how it renders.

- [ ] Composed only from kernel primitives or already-landed components
- [ ] MosaicBook stories covering every variant, size, empty, error, and
      overflow state, in both themes
- [ ] CI gates those stories on every supported backend
- [ ] Degradations recorded explicitly where a backend cannot honor a construct
- [ ] In a package if sharable and core, and that package is published
- [ ] README, CHANGELOG, and >80% test coverage per repo standard
- [ ] Its own test app, released on the supported platforms, retained as a
      durable artifact

---

## 8. What this does not claim

It does not claim the inventory is complete — §3 says missing components are
expected and get filed as found. It does not re-litigate the four packages that
already exist (`Sheet`, `Calendar`, `Notes`, `ProjectNav`); they are grandfathered
into the tree at L3 and re-verified against §7 rather than rewritten. And it
does not delete any working software: TaskApp and Checklist both keep running
from `main` throughout.
