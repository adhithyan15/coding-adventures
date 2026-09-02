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

### What the first careful look already found

Studying MosaicBook and the toolkit before writing any code — rather than
starting from the app — surfaced a chain that had been invisible:

1. **Stories are structurally impossible** for three-file components, the only
   form this repo uses ([#14031](https://github.com/adhithyan15/coding-adventures/issues/14031)). 63 components, 0 story files, and
   a synthesized empty `Default` for every one.
2. **So no component has ever been rendered across its own variants.**
3. **So six components ship `variant`/`size` slots that do nothing**
   ([#14036](https://github.com/adhithyan15/coding-adventures/issues/14036)) — `Button`, `Alert`, `Badge`, `Toast`, `Spinner`,
   `Input`. `Button.light.msl` says so in its own header: "the variant slot is
   accepted by the .mil and stays unused at the styling layer — every variant
   renders with the base style." All eight Button variants are identical.
4. **Because the mechanism to vary a part by a slot value was never designed**
   ([#14037](https://github.com/adhithyan15/coding-adventures/issues/14037)). `mosaic-pkg-toolkit.md` §4.1 proposes `part alert/danger`
   sub-parts and §10 leaves the syntax an open question; the components shipped
   their API surface ahead of it.

A story per variant renders eight identical buttons. It is unmissable in a
MosaicBook grid and invisible in prose — which is the entire argument for this
program, demonstrated on the first component examined under it.

It also reorders Track A: #14037 (design) → #14036 (variants work) → #14031
(stories possible) → #14012 (gate them). Gating first would have certified 63
components on empty fixtures, and passed a story of eight identical buttons.

### Four gaps that block the method

1. **MosaicBook cannot story the components this repo actually has, and is not
   in CI.** Two problems, and the first is the deeper one.

   *Stories are structurally unavailable.* `loadStoriesFile` is called from
   exactly one place — inside MosaicBook's single-file `.mosaic` walk
   (`stories.go:367`). `threeFileComponent` never calls it; it hardcodes
   `Stories: [{Name: "Default", Fixtures: {}}]` at `stories.go:169`. Since
   three-file UI29 is, in MosaicBook's own README, "what every component in this
   repo uses", **every** component previews as one Default story with every slot
   empty. Measured: **63 `.mll` components in `code/packages/`, and 0
   `.stories.json` files.** No variant, size, error, overflow, or populated
   state has ever been previewable for any component — and the tool reports
   success while doing it. ([#14031](https://github.com/adhithyan15/coding-adventures/issues/14031))

   *And no workflow runs it.* "Tested in isolation in MosaicBook and then
   committed" is a manual habit, not a gate — but gating it while stories cannot
   exist would certify 63 components on empty fixtures, so #14031 lands
   first.
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

## 4.5 Where each backend can be verified

Relevant because it decides what a developer can prove locally versus what must
wait for CI. Verified against a working macOS checkout.

| Backend | macOS | Linux | Windows |
| --- | --- | --- | --- |
| `react`, `html`, `webcomponent` | yes | yes | yes |
| `qt` | yes | yes | yes |
| `flutter` | yes | yes | yes |
| `compose` | yes (JVM) | yes | yes |
| `paint` | yes (cairo) | yes | yes |
| `swiftui` | **yes — only here** | no | no |
| iOS | **yes — only here** | no | no |
| `xaml` / WinUI | **no** | no | **yes — only here** |

**A Mac covers eight of the nine backends**, and is strictly better than a Linux
box for this work because SwiftUI and the iOS simulator exist nowhere else.

The one gap is WinUI. Installing .NET on macOS does not close it: WinUI 3 needs
the Windows App SDK and a `net9.0-windows10.0.*` target framework, neither of
which resolves off Windows. XAML is therefore CI-only, and every component's
definition of done must treat the Windows leg as something proven by the
release lane rather than by the author.

---

## 5. Build order

Two tracks run in parallel, because they block on different things: Track A is
release and distribution plumbing for components that already exist, and Track B
is a small real app that proves the components are actually usable. Within each
track, order is strictly leaf to root.

### Track A — make what exists visible

Every component already built gets released with a test app that can be
downloaded and run, and a showcase page that links to it. Nothing here requires
a new component; it is entirely the "done" contract applied to existing work.

- **A1 — the Phase 0 gaps, in this order:** stories for three-file components
  ([#14031](https://github.com/adhithyan15/coding-adventures/issues/14031)) — because a CI gate over impossible stories certifies
  nothing — then MosaicBook in CI ([#14012](https://github.com/adhithyan15/coding-adventures/issues/14012)), native previews
  ([#14013](https://github.com/adhithyan15/coding-adventures/issues/14013)), publishing ([#14014](https://github.com/adhithyan15/coding-adventures/issues/14014)), and the
  per-component demo-app lane ([#14015](https://github.com/adhithyan15/coding-adventures/issues/14015)).
- **A2 — a Mosaic section on GitHub Pages** ([#14026](https://github.com/adhithyan15/coding-adventures/issues/14026)). The repository already deploys 18
  sub-sites this way (`destination_dir: arithmetic`, `engram`,
  `language-ladder`, …), so this is a new `destination_dir: mosaic` and a
  landing page, not new infrastructure. Each component links to its release
  artifacts and its live web build.
- **A3 — retrofit the existing 23 toolkit components** to the full §7 contract
  ([#14017](https://github.com/adhithyan15/coding-adventures/issues/14017)), shipping each one's own generated demo app — released
  on every supported platform — and its documentation-site entry as it lands.

### Track B — Checklist as reference app one ([#14027](https://github.com/adhithyan15/coding-adventures/issues/14027))

The Checklist app is the first release target, not TaskApp. It is a genuinely
smaller consumer, and the gap is not marginal:

| | Checklist needs | TaskApp needs |
| --- | --- | --- |
| Existing toolkit components | `Checkbox`, `Button`, `ListGroup`, `Accordion`, `Field`, `Input`, `Select`, `Modal`, `Badge`, `Alert` — all shipped | the same, plus much more |
| Missing atoms | `EmptyState`, a progress indicator | `Chip`, `StatusPill`, `EmptyState`, `Icon`, `ProgressRing` |
| Missing molecules | none — `Accordion` already covers branch disclosure | `SegmentedControl`, `Legend`, `ValidatedField`, `InlineEditForm`, `Composer`, `StatusPanel`, `Toolbar` |
| New L3 | `ChecklistItem`, `DecisionNode`, `ChecklistRunner`, `TemplateEditor` | `TaskRow`, `TaskList`, `TaskDetail`, `Board`+`BoardColumn`+`BoardCard`, `GanttChart` |
| Missing kernel primitives | none | `HostNavigationSplit`, host environment (#14003) |

Checklist needs **two missing atoms and no new kernel primitives**. TaskApp
needs five atoms, seven molecules, and two kernel primitives that do not exist.
That is the whole argument for the order.

- **B1 —** the two missing atoms Checklist needs.
- **B2 —** the four Checklist L3 components, each in isolation.
- **B3 —** the Checklist app itself, released, replacing the Electron version
  ([#14018](https://github.com/adhithyan15/coding-adventures/issues/14018)).

### Then TaskApp

- **C1 —** remaining atoms and molecules, `SegmentedControl` first
  ([#14016](https://github.com/adhithyan15/coding-adventures/issues/14016)).
- **C2 —** the kernel gaps: `HostNavigationSplit`, host environment (#14003).
- **C3 —** TaskApp's L3 organisms, then the app.

Each phase completes — stories, CI gate, package, published, test app released —
before the next begins.


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
- [ ] **Its own demo app** — one per component, not one per package — generated
      from the component and its stories, and **released on every platform
      Mosaic supports**, retained as a durable artifact ([#14015](https://github.com/adhithyan15/coding-adventures/issues/14015))
- [ ] Listed on the Mosaic documentation site, with its live web build running
      inline and its per-platform downloads linked ([#14026](https://github.com/adhithyan15/coding-adventures/issues/14026))

**Isolation is the point, and it is not negotiable.** A component proven only
inside a larger app has not been proven; that is the failure mode this whole
program exists to correct. One shared showcase app covering many components
would reintroduce it in miniature — a regression in one component could hide
behind another's working demo. The documentation site is an *index* over
independently proven components, never a substitute for proving them.

### Why per-component demos force generation

The matrix is real: ~6 release artifacts per component (web, Linux ×3, macOS,
Windows) across 23 components today is ~138 artifacts, and the catalog grows.
That is affordable only if demo apps are **generated** from the component and
its MosaicBook stories — which already enumerate every variant, size, state, and
theme, i.e. exactly a demo app's content — and if the release lane rebuilds only
*affected* components. The repository's Go build tool already does the second
part with `--diff-base origin/main` and `affected_nodes()`; the lane reuses it
rather than reinventing it. Per-component SemVer follows from the same
requirement.

Hand-writing demos does not scale past a handful and drifts from the component
the moment it changes. The existing hand-written demos
(`toolkit-multi-demo`, `toolkit-xaml-showcase`, `hello-dialog-xaml`) are
precedent that this is wanted, not a model to copy.

---

## 8. Backlog — recorded, not scheduled

Ideas that belong to this program but are deliberately not in any track yet.

- **Self-host MosaicBook** ([#14028](https://github.com/adhithyan15/coding-adventures/issues/14028)). MosaicBook is the tool that
  renders Mosaic components; it should eventually be a Mosaic app itself, which
  is the most honest dogfooding test available. Today it is a Go server serving
  one hand-written 634-line `static/index.html`, and that shell is a natural
  Mosaic app — `ListGroup`, `Nav`, `Tabs`, `Field`, `Alert`, `Spinner`,
  `HostSurface` — most of which the toolkit already ships.

  It is unscheduled because of a real bootstrapping problem: if MosaicBook's UI
  is a Mosaic app, developing it requires a working MosaicBook. The likely answer
  is to split the layers — the Go server keeps discovery, compilation, watching,
  and serving, and only the shell becomes Mosaic, with the current static HTML
  retained as a fallback so the tool can never become unusable. That should be
  settled in a spec rather than discovered mid-implementation, and it should not
  start until the components it needs are released (Track A) and Checklist has
  proven the tree on a non-circular app first.

---

## 9. What this does not claim

It does not claim the inventory is complete — §3 says missing components are
expected and get filed as found. It does not re-litigate the four packages that
already exist (`Sheet`, `Calendar`, `Notes`, `ProjectNav`); they are grandfathered
into the tree at L3 and re-verified against §7 rather than rewritten. And it
does not delete any working software: TaskApp and Checklist both keep running
from `main` throughout.
