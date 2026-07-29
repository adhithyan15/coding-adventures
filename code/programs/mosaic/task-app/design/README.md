# task-app — UI design reference

This folder holds the **visual design source of truth** for the task-app: a
high-fidelity, self-contained prototype of the full UI, ahead of the Mosaic
component work that will implement it.

## `ui-prototype.html`

Open it in any browser — no build, no dependencies, no server. It realizes the
design specified in [`code/specs/task-app-ui-design.md`](../../../../specs/task-app-ui-design.md)
at full fidelity:

- **Four views** — List (with progressive-disclosure task detail), Board (Kanban
  with pointer *and* keyboard drag), Timeline (Gantt with critical path, slack, and
  dependency arrows), and Calendar.
- **Three projects that really switch**, plus a composer to create more. Each project
  owns its own plan, so picking one in the rail reloads every view from it.
- **Warm & approachable** visual identity — a honey accent reserved for *now*,
  warm-biased neutrals, tabular figures, authored **light and dark** themes (toggle
  top-right, and it respects your OS preference).
- **Live motion** — complete a task and watch it spring into Done (FLIP); drag a
  board card and watch it land; every move is announced to screen readers.

It runs on a small hard-coded sample project that stands in for the engine's
render-ready projections. Nothing in it computes a schedule — exactly as the real
UI won't: the engine stays fat, the UI stays a renderer (see
[`task-app-super-app.md`](../../../../specs/task-app-super-app.md)).

## How to use it

- **To see the design:** open the file.
- **To build a component:** read the spec, then diff your component against this
  file. If they disagree on spacing, color, or motion, resolve it here first — this
  file wins.

The design is also published as a shareable interactive artifact; ask for the link
if you don't have it.
