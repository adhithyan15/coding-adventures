# Trestle — icon/SVG assets (v1)

Closes most of the "Icon/SVG assets" line of the design-fidelity gap in
`code/programs/mosaic/task-app/BACKLOG.md`: "brand mark glyph,
segmented-switch icons, a progress ring, a stroked moon icon for the
theme toggle..., the pill status dot, group-count badge, composer '+'
icon box." `code/programs/mosaic/task-app/design/ui-prototype.html`
(the reference mock) has real, working markup for every one of these —
this doc maps each to a Mosaic-native construction.

## What ships in this slice

- **Pill status dot** — a small circle before "On track" / "N overdue"
  in the topbar's status pill, `background: currentColor` (the mock's
  own `.pdot` rule) so it always matches the pill's own semantic
  colour with no separate binding.
- **Group-count badge** — a count pill next to each List-view group
  heading ("IN PROGRESS 2"), from a new appended row cell.
- **Composer "+" icon box** — a dashed-border box with a plus mark
  before the task-name input, replacing the bare input-row start.
- **Theme toggle, moved into the topbar with a real icon** — currently
  a `position: fixed` unicode-glyph button living entirely outside the
  Mosaic-compiled component (`main.tsx` renders it as a sibling of
  `<Emitted>`, styled by hand, invisible to mostyle). Becomes a real
  `HostButton` in `TaskApp.mll`'s topbar tools, with a drawn crescent
  moon / filled sun instead of "☾"/"☀".
- **Progress ring** — the project's overall percent-complete as a
  circular ring in the topbar, next to the view switcher (matching the
  mock's `.ring` placement).
- **Brand mark: a bridge arc** — two upright posts joined by an arc,
  replacing the currently-empty honey square. User-chosen from a
  proposed shortlist (Truss triangle / Three pillars / Bridge arc /
  A-frame chevron) — see the commit history for the ask.

## What does NOT ship — segmented-switch icons

The six view-switcher buttons (List/Board/Sheet/Calendar/Notes/
Timeline) each want a small line icon in the mock (a stack of bars, a
kanban-column trio, a calendar grid, …). Technically these are the
*same* construction technique as everything else here (small Box/Stack
compositions, no new primitive) — the reason they're deferred isn't a
capability gap, it's that six icons need to read as one *matched
family* at a glance, and getting six tiny glyphs visually consistent
in weight/scale benefits from iterating on the actual rendered set
side-by-side rather than shipping six independent first guesses in one
pass. Tracked as the next icon-assets slice.

## How every shape gets drawn — no image files, no new SVG primitive

Investigated first: does the kernel have a raw-SVG-embedding
primitive? There's an `Icon` primitive, but it lowers to a bare
`<span class="icon">` (`mosaic-emit-react`'s `emit_icon_jsx`) — no
path/glyph rendering, just a marker span expecting a `content:` value
the way `Text` does. No primitive accepts arbitrary SVG path data.

Rather than add one (a much bigger, more speculative kernel surface
than anything else touched this session), every shape here is built
from primitives that already exist:

- **`Stack`** (`position: relative`, per `emit_stack_jsx`) lets
  children layer via `position: absolute` — this is what makes a
  crescent moon (two overlapping circles), a donut ring (a filled
  circle behind a smaller "hole" circle), and the brand mark (two
  posts + an arc, independently positioned) possible with zero new
  kernel work.
- **Individual-corner `border-*-radius`** (already proven safe —
  `border-right-width`/`-color`/`-style` are used for the Gantt
  day-grid's dividers) draws the arc: a short, wide box with only its
  top two corners rounded reads as an arch at icon scale.
- **A crescent** is the classic pure-CSS trick: a filled circle, then
  a same-sized-or-slightly-smaller circle in the button's own
  background colour, offset up-and-right — no clip-path, no SVG.
- **A sun** is simpler still: one filled circle, sized larger than the
  pill dot so it reads as distinct at a glance.
- **The plus icon** is two crossed bars (a horizontal Box, a vertical
  Box, both centred in a `Stack`) inside a dashed-border container —
  matching the mock's `.composer .plus` treatment exactly.

## The one real gap — the progress ring needs UI36 widened by one property

Every shape above is 100% static per theme (light/dark are separately
compiled files, so "which crescent" or "which arc colour" needs no
runtime data at all). The ring is the only one that must vary
continuously with real data — the project's percent-complete, 0..100 —
and a `conic-gradient(...)` background is the only realistic way to
draw a smoothly-filling ring without an actual SVG arc primitive.

UI36 (`code/specs/UI36-data-driven-sizing.md`) already exists to solve
exactly this shape of problem — "the values a stylesheet cannot
know" — but its implementation (`dynamic_size_style` in
`mosaic-emit-react`) hard-lists exactly six bindable properties, all
literal sizes (`width`/`height`/`min-width`/`max-width`/`min-height`/
`max-height`). `background` isn't one of them, so `background: (t[N])`
today silently... doesn't silently do anything, actually — UI36's own
design principle is "accepted-but-ignored is the worst outcome", so an
unbound-property attempt would need to be checked, not assumed.

This slice adds `background` as a seventh bindable property, same
three value forms (number/slot/expression) the existing six already
support, same precedence rule (bound value beats both base part style
and hover-state spread). It's not a new mechanism — it's the existing
one, widened by one property that fits UI36's own stated purpose
precisely. `main.tsx` computes the full `conic-gradient(...)` string
(the same "host formats, layout just places it" discipline the Gantt
tooltip text and every engine-projected cell already follows) and
binds it the same way a Gantt bar's width is bound today.

### Follow-up: `ring-gradient` is a web-only mechanism (#12028 item 2)

The `background: slot: ring-gradient` binding above is a **React-only**
capability — UI36's bindable-property widening lives entirely in
`mosaic-emit-react`'s `dynamic_bound_style`; no native backend reads a
slot-bound `background` layout prop on a plain `Box` at all. Native
hosts previously received `ring-gradient: ""` (an always-empty string,
since only the web host's own `main.tsx` computes the CSS gradient)
with **no numeric fallback** to build any rendering of their own —
"a leak in the data contract," per the epic's own framing.

`slot ring-percent-value : number ;` (added to `TaskApp.mil`) closes
that leak: every host now receives the real 0..100 percent as typed
data, computed once in the shared `task-mosaic-app` Rust engine (the
same value the web host previously recomputed redundantly in
TypeScript). `ring-gradient`/`ring-percent` are unchanged and still
drive the web backend's own CSS-trick rendering — appropriate for its
platform, per this whole doc's philosophy.

Native **rendering** of the ring from `ring-percent-value` — a real
circular progress indicator per backend — is deliberately **not** part
of this fix; it needs its own design decision (native determinate
progress-ring primitive vs. a shared drawing primitive, the same
capability #12028 item 3 already calls out) and is tracked separately.

## Wiring summary

- `mosaic-emit-react`: `background` added to the bindable-property
  list. `dynamic_size_style`/`SIZE_PROPS` renamed to
  `dynamic_bound_style`/`BINDABLE_PROPS` (four call sites) — cheap
  enough to do properly rather than leave a now-inaccurate name. New
  tests alongside the existing UI36 property tests.
- `TaskApp.mil`: `slot status-dot` isn't needed (the dot is undecorated
  chrome, not data) — `pdot` is written directly in `.mll`/`.msl`.
  `slot ring-gradient : text ;` (the computed `conic-gradient(...)`
  string) and `slot ring-percent : text ;` (the number, for the
  "NN% complete" caption) added near `status-warn`. Task rows' group
  heading gains an appended count cell (`row[14]`, present only
  alongside the heading cell). **Diverged from the original plan here**:
  rather than `slot theme-icon-kind : text ;` (`"sun"`/`"moon"`), shipped
  as `slot theme-is-dark : text ;` — the existing non-empty-string-as-
  boolean idiom already used by `status-warn`/`allow-timeline`, driving
  an `If`/`Else` in `.mll` between the two `HostButton`s, rather than a
  three-way string the layout would need to branch on twice. `emit
  onToggleTheme ;` replaces the host-owned floating button, as planned.
- `TaskApp.mll`/`.msl`: the shapes described above, each a `Stack` of
  a small number of `Box` children with static or (ring only)
  UI36-bound styling.
- `main.tsx`/`theme.ts`: drops the fixed-position theme button entirely;
  `toggleTheme` is dispatched like any other emit but intercepted in
  `Root` before reaching `controller.apply` — theme is page-level React
  state, not part of the engine-backed controller, which never learns
  about it. `theme.ts` gained `ringGradient(theme, percent)` (the actual
  `conic-gradient(...)` string, needing the resolved theme the shared
  controller can't see — same duplicated-palette reasoning as `GROUND`
  in the same file) and `getProps()` gained `ringPercentValue` (a raw
  number) plus the group-count tally ahead of the row-building loop that
  already walks the same list, mirroring `timeline.ts`'s "pure
  arithmetic, host formats, layout just places it" pattern.
