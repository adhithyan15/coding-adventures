# journal-app

Journal's Mosaic app package (J3c-2, [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416)).

It exports `JournalApp`: a timeline of entries beside one editor. It is the
product assembly layer only. It composes reusable components and holds no
behaviour of its own:

| part | component | package |
| --- | --- | --- |
| the timeline | `RecordList` (rows grouped by day) | `mosaic-pkg-toolkit` |
| the first-run state | `EmptyState` | `mosaic-pkg-toolkit` |
| the editor | `DraftEditor` (title, body, Save / Delete / Cancel) | `mosaic-pkg-draft-editor` |

The layout is a `HostNavigationSplit`: the timeline and a **New entry** button
in the pane, and the editor in the detail.

## The behaviour lives in Rust

Every slot value and every event is handled by
[`journal-mosaic-app`](../../../packages/rust/journal-mosaic-app), the
`MosaicApp` over `journal-core`. See its spec,
[`journal-mosaic-app.md`](../../../specs/journal-mosaic-app.md). That spec's
tables are the contract `src/JournalApp.mil` declares, and the tests hold the
two together.

```text
JournalApp.mil ──(slots/emits)── journal-mosaic-app ──(commands)── journal-core
      │
JournalApp.mll ── RecordList · EmptyState · DraftEditor
```

## Files

- `mosaic-package.toml` is the manifest (a `[app]` package, 1100×760).
- `src/JournalApp.mil` declares the slots and emits.
- `src/JournalApp.mll` is the layout.
- `src/JournalApp.{light,dark}.msl` style the app's own parts. They use only
  properties every native backend lowers; the components style themselves.

## Tests

`cargo test` runs the smoke harness in `Cargo.toml`:

- `package_compiles` checks four things:
  - the sources compile in both themes;
  - the manifest parses and the three components resolve;
  - the Rust app's props are exactly the `.mil` slots;
  - the package builds on all eight backends.
- `adapter_event_contract` checks that every declared emit is routed by the
  Rust app, and that an undeclared one is rejected.
- `native_complete_gate` fails on any native degradation, and on any style
  drop beyond the pinned set. It checks the pins both ways.

## Not yet

This package has no host shells, native packaging, CI lanes or release yet
(J5). The web and native hosts load `journal-mosaic-app` as `libmosaic_app`
the same way Engram's and Trestle's do.
