# mosaic-pkg-project-nav

> Nested-project tree + add-project composer, extracted from TaskApp's rail.

Phase 9 of the task-app roadmap (`mosaic-pkg-project-nav`, per
[task-app-super-app.md](../../../specs/task-app-super-app.md) §4's reuse
map) — the nested-project tree and add/add-subproject composer that used
to live inline in `TaskApp.mil`/`.mll`, now a standalone reusable
component. See
[task-app-project-nav-v1.md](../../../specs/task-app-project-nav-v1.md)
for the full extraction rationale and scope, including what deliberately
stayed behind in TaskApp (the brand row, the view-switcher).

## What this package exports

One component, per `mosaic-package.toml`'s `[components].exports`:

| Component    | Role                                     | File trio |
|---|---|---|
| `ProjectNav` | nested-project list + add-project composer | `ProjectNav.mil` / `ProjectNav.mll` / `ProjectNav.{light,dark}.msl` |

## How it fits in the stack

```
          ┌──────────────────────────────────────────┐
          │  Host application (task-app's rail)       │
          └─────────────────────┬──────────────────────┘
                                │ component reference
                                ▼
          ┌──────────────────────────────────────────┐
          │  mosaic-pkg-project-nav (this package)    │
          │  ProjectNav → Column/Row/Text/HostButton/ │
          │               HostInput                   │
          └──────────────────────────────────────────┘
```

## Fat engine, dumb UI

ProjectNav does no tree-walking, sorting, or id-minting of its own. The
host builds `project-rows` depth-first from the workspace's project
forest — see `code/programs/mosaic/task-app/host/web/src/main.tsx`'s
`projects()` function for the reference host consumer (same shape it
already built before this extraction; the extraction didn't change how
the host derives the data, only where the rendering lives).

## A refactor, not a redesign

Every part name, style value, and layout structure in this package is
copied verbatim from `TaskApp.mil`/`.mll`/`.{light,dark}.msl`'s own rail
block, which shipped and was verified live across several prior PRs (see
`CHANGELOG.md`'s "Added - multiple projects in the UI" entry in
`code/programs/mosaic/task-app/`). Nothing about the rail's appearance or
behavior is different after this extraction — verified live: create a
project, create a nested sub-project (indent glyph renders), switch
selection between projects (the "on" raised-card styling follows), in
both themes.

## Usage

```moslayout
// In a host component's .mll:
pkg::mosaic-pkg-project-nav::ProjectNav (
  nav-title:        "Projects" ,
  project-rows:      slot: project-rows ,
  new-project-name:  slot: new-project-name ,
  onSelectProject:         emit: onSelectProject ,
  onNewProjectNameChange:  emit: onNewProjectNameChange ,
  onAddProject:            emit: onAddProject ,
  onAddSubproject:         emit: onAddSubproject
)
```

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-project-nav
cargo test
```

Mirrors `mosaic-pkg-notes`'s own smoke test: manifest parses and declares
the expected export; `ProjectNav.mil` compiles via `mosmodel-compiler`;
`ProjectNav.mll` compiles against that interface via `moslayout-compiler`;
both themes' `.msl` compile against the resulting part map.

## License

MIT OR Apache-2.0.
