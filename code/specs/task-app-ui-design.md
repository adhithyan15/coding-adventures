# task-app — UI design system & view specification

**Status:** design spec (Phase 5–9 UI). Precedes the Mosaic component work.
**Companion prototype:** [`code/programs/mosaic/task-app/design/ui-prototype.html`](../programs/mosaic/task-app/design/ui-prototype.html)
— a single self-contained HTML file that realizes everything below at full
fidelity (List, Board, Timeline, Calendar; light + dark; live animations). Open
it in a browser to *see* the design; read this document to *build* it.

**Reads on from:**
- [`task-app-super-app.md`](task-app-super-app.md) — the product north star and the
  fat-engine / dumb-UI split this design is bound by.
- [`task-app-view-layer.md`](task-app-view-layer.md) — the engine already returns
  **render-ready** projections (`table`, `calendar`, `view_selection`, `gantt`);
  every screen here is a thin renderer over one of those.
- [`UI35-host-drag-drop.md`](UI35-host-drag-drop.md) — the drag kernel the Board
  view is built on.

---

## 1. Why this document exists

The engine has been the crown jewel; the UI has been a single unstyled column of
text. That was correct sequencing — prove the schedule math first — but it means
the app does not yet *look* like something a person would choose to open. This
spec fixes that with a complete visual system and four first-class views, drawn so
that **the engine stays fat and the UI stays a renderer**: nothing here computes a
schedule, a sort, a rollup, or a format. Every date string, every "overdue", every
critical-path flag, every group and order arrives already decided by `task-core`.

The design has one governing idea, taken straight from the super-app spec:
**progressive disclosure**. A brand-new list is a to-do list a child could use. The
CPM scheduler, resource leveling, dependency graph, and baselines are all *there*,
but folded away until asked for. Complexity is opt-in, never in your face.

---

## 2. Visual language

The chosen identity is **warm & approachable** — but deliberately *not* the
cream-paper-plus-serif-plus-terracotta treatment that reads as generic. Warmth
here is grounded in the subject's own world: **time, sequence, and working days**.
The hero of the whole app is the timeline; the accent is reserved for *now*.

### 2.1 Color — tokens, both themes first-class

Neutrals are warm-biased (a hair toward the accent), never a dead mid-grey. The
accent is a single confident **honey**, spent only on *time / now / active /
primary*. Semantic colors (good / critical) are a **separate** axis from the accent
— they never double as brand color.

| Token | Light | Dark | Used for |
|---|---|---|---|
| `--bg` | `#f0ebe3` | `#1a1714` | app ground (one step deeper than cards) |
| `--surface` | `#fffdfa` | `#252019` | cards, bars, sheets |
| `--surface-2` | `#f7f2ea` | `#2d271f` | inset panels, expanded detail |
| `--ink` | `#2b2723` | `#f1ebe1` | primary text (warm charcoal, not black) |
| `--ink-soft` | `#6a625a` | `#b3a99c` | secondary text |
| `--ink-faint` | `#9b9289` | `#867c70` | captions, disabled |
| `--line` | `#e6ded3` | `#352e25` | hairlines |
| `--honey` | `#e0942a` | `#eaa63f` | **the accent** — today, active, primary button, progress |
| `--honey-tint` | `#faedd6` | `#3a2c17` | honey chip fills |
| `--sage` | `#4f8e6a` | `#6fb489` | semantic: on-track / done |
| `--red` | `#cf4b34` | `#e26a52` | semantic: critical path / overdue |
| `--blue` | `#4a7ab0` | `#78a3d4` | dependency links |

Both themes are authored, not inverted: the palette is defined on `:root`,
re-declared under `@media (prefers-color-scheme: dark)`, and again under
`:root[data-theme="light"|"dark"]` so the in-app toggle wins over the OS in both
directions. Contrast and the accent's legibility are checked on both grounds.

### 2.2 Type

A deliberately-weighted **system humanist** stack (`-apple-system, "Segoe UI",
system-ui, …`). No serif display face (the cliché), no Inter / Space Grotesk (the
"safe" default). Hierarchy and color carry the warmth; the type stays quiet and
legible. `font-variant-numeric: tabular-nums` is on globally, because this app is
full of aligned digits — dates, durations, the Gantt grid, the progress percent.

Scale: 22/680 project title · 16/650 section title · 14/500 body · 12–12.5 chips &
captions · 10.5/650 uppercase labels with `.06–.08em` tracking.

> **Production note.** The prototype uses the system stack to avoid a CDN webfont
> (blocked by the artifact CSP) and any silent fallback. A shipped host may inline
> one warm humanist face as a `@font-face` data-URI — but it must be a *chosen*
> face, not a reflex.

### 2.3 Spacing, radius, elevation

Layout is done with flex/grid `gap`, never per-element margins. Radii: 13px cards,
9px controls, 20px pills. Two soft, warm-tinted shadows (`--shadow`, `--shadow-lg`)
— shadows carry a brown tint, not neutral black, so they sit in the warm world.

### 2.4 Motion — soft, springy, meaningful

Motion is the part the user asked for by name ("animations to indicate how tasks
have moved"). The rule: **animate change of place, celebrate completion, never
decorate.**

- **Spring** (`460ms cubic-bezier(.34,1.4,.5,1)`) for anything that *moves* —
  a completed task relocating to Done, a card landing in a new column.
- **Ease** (`240ms`) for reveals — expanding a task's detail, crossfading views.
- **FLIP** for reflow: measure every row's rect *before* the data change, apply the
  change, then animate each row from its old rect to its new one. This is what makes
  "the task slid down into Done" legible instead of a jarring jump.
- **Completion** is a two-beat: the check *draws* (stroke-dashoffset) and the title
  strikes through first, then a beat later the row springs into the Done group.
- Every animation is inside `@media (prefers-reduced-motion: reduce)`, which
  collapses all durations to ~0. Nothing here is load-bearing for meaning.

---

## 3. Information architecture

```
┌───────────┬──────────────────────────────────────────────┐
│  RAIL     │  TOPBAR: title · summary · progress ring ·    │
│  (quiet)  │          [ List | Board | Timeline | Calendar]│
│  projects │──────────────────────────────────────────────│
│  views    │  CONTENT: exactly one view, crossfaded        │
│  archive  │                                               │
└───────────┴──────────────────────────────────────────────┘
```

- **Rail** — projects (each a colored dot + live task count), saved cross-project
  views ("My week", "Everything due"), archive/settings. Quiet by default; this is
  where nested projects (super-app spec §"first-class citizens") will live as a
  disclosure tree. Hidden below 780px.
- **Topbar** — the project's *summary before its detail*: name, "9 tasks · 2 done",
  projected finish date, an **on-track / at-risk pill** (semantic color), and a
  progress ring. The projected finish and the pill both come from the engine's
  schedule, never computed here. Then the **view switcher** and the theme toggle.
- **Content** — one view at a time, chosen by the switcher, crossfaded on change.

The summary line is the top of the progressive-disclosure funnel: glanceable state
first, one click to a view, one more click to a task's full detail.

---

## 4. The four views

All four render the *same* task set from different engine projections. A task is a
strict superset (super-app spec): a checklist item is a task with a done-flag; a
board card is a task grouped by status; a Gantt bar is a fully-scheduled task; a
calendar entry is a task on its dated day. One model, four lenses.

### 4.1 List (default) — progressive disclosure

The hero of *everyday* use and the safest default. Backed by the engine's `table`
projection (`task-app-view-layer.md`): columns, groups, and every cell's display
string arrive pre-formatted.

- **Composer** at top — name + optional due + Add. The lowest-friction path;
  matches the shipped app's affordance, dressed properly.
- **Groups** — "In progress / Up next / Done", each with a count. Grouping, order,
  and membership are the engine's `view_selection`, not a client sort.
- **Row** — a completion toggle (the animated check), the name, and a compact
  **chip cluster**: `critical` (semantic red, only while incomplete), `due today`
  (honey), `Nd slack` (sage), the scheduled window, and label dots. Chips encode
  state in *form and color*, so what needs attention reads at a glance without
  reading words.
- **Progressive disclosure** — clicking a row expands an inset panel (animated via
  `grid-template-rows: 0fr → 1fr`) showing Schedule (start → finish, working days),
  Depends-on (predecessor chips), Slack (with the plain-language consequence),
  Priority, Labels, and Notes. This is where MS-Project-grade detail lives —
  present, but folded away until asked for.

### 4.2 Board (Kanban) — the drag showcase

Columns by workflow status (Up next / In progress / In review / Done); cards move
between them. This is the visible payoff of the **UI35 drag kernel**
(`UI35-host-drag-drop.md`) and it honors that spec's three non-negotiables:

- **Touch works** — the drag is pointer-driven, not HTML5 DnD, and hit-tests with
  `elementFromPoint` (so implicit pointer capture doesn't pin every drop back to the
  source column — the exact bug the HTML backend review caught).
- **Keyboard equivalence** — focus a card, `Space` to grab, `←/→` to move across
  columns, `Space` to drop, `Esc` to cancel — dispatching the *same* move as a
  pointer drop.
- **Announcements** — a visually-hidden `aria-live` region narrates grab / over /
  moved / cancelled.

Moves animate with FLIP + a spring "land"; a drop is a **proposal** — in the real
app the engine validates and performs the status change, the UI never mutates
directly.

### 4.3 Timeline (Gantt) — the thesis

The view the user asked for, and the design's centre of gravity: *your plan laid out
on time*. Backed by the engine's `gantt` / `schedule` output.

- A **date grid** with day columns, weekday labels, weekend shading, and a **honey
  "today" line**.
- **Bars** positioned by start/finish, with a darker fill showing % complete.
- **Critical path** bars and their dependency arrows in semantic **red** — the
  chain with zero slack; the legend says so, and the caption explains the stakes.
- **Free slack** drawn as a translucent dashed tail past a non-critical bar (the
  design makes "this can slip N days without moving anything" *visible*).
- **Milestones** as diamonds; **dependency arrows** as FS connectors (curved, red
  when both endpoints are critical).
- Hovering a bar shows a dated tooltip. Dragging a bar to reschedule is the next
  increment (out of scope for the first prototype; the kernel already exists).

The whole point: the critical path, slack, and dependencies are the engine's
answers, drawn — not re-derived in the browser.

### 4.4 Calendar

A month grid for deadline-driven work, backed by the engine's `calendar`
projection over an inclusive day range. Weekends shaded, today ringed in honey,
each task a pill on its scheduled day (critical in red, milestones inked, done
struck through). Month navigation is the obvious next increment.

---

## 5. Accessibility & correctness (non-negotiable)

- Keyboard operates everything, including the full drag equivalence above.
- Visible focus ring (honey) on every interactive element.
- Live-region announcements for drag and for completion.
- `prefers-reduced-motion` honored everywhere.
- All user-supplied text (task names) is escaped before it reaches the DOM — the
  prototype routes every name through an `esc()` helper so a name like
  `<img onerror=…>` renders as text, mirroring the escaping discipline the Mosaic
  emitters enforce.
- Theme-aware, contrast-checked in both light and dark.

---

## 6. How this maps to the build (fat engine, dumb UI)

Nothing in §4 is business logic. Each view is a pure function of an engine
projection the view layer already emits:

| View | Engine projection (already spec'd / built) |
|---|---|
| List | `table` + `view_selection` — columns, groups, order, formatted cells |
| Board | `view_selection` grouped by workflow status; moves → `move`/`set_status` ops |
| Timeline | `gantt` / `schedule` — early/late dates, slack, `critical`, milestones |
| Calendar | `calendar` — dated events over `[start, end]` |

The Mosaic components are thin renderers emitting raw intent; the host marshals
that intent to the engine over the WASM / C-ABI boundary (`task-app-architecture`).
The prototype's `tasks` array is a stand-in for exactly these projections — every
value it reads (`crit`, `slack`, formatted dates) is something `task-core` computes.

### 6.1 Mosaic primitives needed

Most of this composes from kernel primitives that already exist (Stack, HostButton,
HostCheckbox, HostTable\*, HostInput, HostDialog, HostTooltip) plus the **UI35 drag
family** (`HostDraggable` / `HostDropTarget`, now lowered on React + HTML). The
genuinely new UI work is compositional, not new primitives:

- a **disclosure row** pattern (List) — Stack + HostButton + an animated detail region;
- a **timeline/Gantt** component — the one real net-new renderer, likely a dedicated
  `mosaic-pkg-gantt` given the SVG dependency arrows and date-grid math;
- a **board column** pattern wiring HostDraggable/HostDropTarget to status ops;
- a **calendar grid** component.

These become the Phase 6–7 component packages in the super-app roadmap.

---

## 7. What to build next (phasing)

This spec is the design contract for the roadmap's remaining UI phases:

1. **Design tokens + app shell** — the palette, type, motion tokens, rail, topbar,
   view switcher, theme toggle, as reusable Mosaic style + a host adapter.
2. **List view with progressive disclosure** (Phase 5 "sheet") — the disclosure row
   over the `table` projection. Highest everyday value; ship first.
3. **Board view** (Phase 6) — columns + the UI35 drag family + move animations.
4. **Timeline / Gantt** (Phase 7) — the `mosaic-pkg-gantt` renderer over `gantt`.
5. **Calendar** — over the `calendar` projection.
6. **Notes & app shell polish** (Phases 8–9).

Each is a thin renderer; the engine work they depend on is already merged.

---

## 8. The prototype as living reference

`design/ui-prototype.html` is the source of truth for *look and motion*. It is
intentionally self-contained (no build, no deps) so anyone can open it, and it
carries real interactions — completing a task and watching it animate into Done,
dragging a card by pointer or keyboard, switching to the timeline and reading the
critical path. When a Mosaic component lands, it should be diffed against this file:
if they disagree on spacing, color, or motion, one of them is wrong and the
disagreement gets resolved here first.
