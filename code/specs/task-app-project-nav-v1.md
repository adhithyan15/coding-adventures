# task-app project-nav — v1 scope

Phase 9 of [task-app-super-app.md](task-app-super-app.md) names
`mosaic-pkg-project-nav` (nested-project tree, view switcher) as one of the
two remaining Phase 9 items (the other being the per-project/task
complexity config — a separate, larger effort needing a product decision
the spec doesn't make; not addressed here). This spec scopes the
extraction.

## What v1 ships

**The nested-project tree + add/add-subproject composer only** —
extracted verbatim, behavior-identical, from TaskApp's own rail block
(`project-rows`/`new-project-name` slots, `onAddProject`/`onAddSubproject`/
`onSelectProject`/`onNewProjectNameChange` emits). Same part names,
same styling (both themes), same layout structure — this is a refactor,
not a redesign. `code/programs/mosaic/task-app/CHANGELOG.md`'s "Added -
multiple projects in the UI" entry already fully documents the feature
this repackages; nothing about its behavior changes here.

## What stays in TaskApp, and why

- **The brand row** (logo mark + app name) — app identity, not project
  navigation. Also the one thing the still-open "rename off Planner"
  backlog item touches; keeping it out of this package means that
  decision, whenever it lands, doesn't ripple through a package boundary.
- **The view-switcher** (the segmented List/Board/Sheet/Calendar/Notes/
  Timeline tab row) — named alongside "nested-project tree" in the same
  roadmap bullet, but deliberately NOT extracted here. It's a single,
  deeply-coupled 36-button block (six view families × six mutually-
  exclusive branches, one branch added per new view this session) that
  has been edited in every recent view-addition PR (Sheet, Calendar,
  Notes). Moving it into a separate package right now — immediately after
  several rapid additions to it — would be a large, high-blast-radius
  refactor for a purely internal reorganization, with no corresponding
  extraction precedent to derisk it (unlike the project rail, which is a
  simpler, more self-contained widget). Left as a follow-up, tracked in
  `BACKLOG.md`, the same "ship narrower" sequencing every other phase in
  this backlog has used.

## Package structure

`code/packages/mosaic/mosaic-pkg-project-nav/`, mirroring the established
file trio (`Cargo.toml`, `mosaic-package.toml`, `src/lib.rs` (doc-only),
`src/ProjectNav.mil`, `src/ProjectNav.mll`, `src/ProjectNav.{light,dark}.msl`,
`tests/package_compiles.rs`). No dependency on another package — built
from kernel primitives only, same as `mosaic-pkg-grid`/`mosaic-pkg-calendar`.

## TaskApp wiring

`TaskApp.mil` keeps `project-rows`/`new-project-name` as pass-through
slots (unchanged names — the package's own slots match exactly, so the
`pkg::` call site is a straight forward with no renaming) and keeps its
four project-nav emits declared (forwarded to/from the package). The
rail's `Column [ rail ]` in `TaskApp.mll` keeps the brand row, then
embeds `pkg::mosaic-pkg-project-nav::ProjectNav ( ... )` in place of the
inline `rail-label`/`rail-projects`/`rail-composer`/`project-sub` block
that block was previously composed of. The now-unused part styles for
that block are removed from `TaskApp.{light,dark}.msl` (they moved into
the package's own `.msl` files, styled identically) — dead styles left
behind would be exactly the kind of thing this repo's literate-code
standard flags.
