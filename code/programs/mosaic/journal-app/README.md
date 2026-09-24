# journal-app

Journal's Mosaic app package (J3c-2, [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416)).

It exports `JournalApp`: a timeline of entries beside one editor. It is the
product assembly layer only. It composes reusable components and holds no
behaviour of its own:

| part | component | package |
| --- | --- | --- |
| the timeline | `RecordList` (rows grouped by day) | `mosaic-pkg-toolkit` |
| the first-run state, and "No entries match", "No starred entries" | `EmptyState` (mounted three times) | `mosaic-pkg-toolkit` |
| search (J4a) | a `HostInput` and a *Clear* button above the timeline | kernel primitives |
| stars (J4b) | *Star* above the editor, *Starred only* under the search field | kernel primitives |
| on this day (J4c) | a second `RecordList`, above the timeline, of earlier years' entries on today's date | `mosaic-pkg-toolkit` |
| the editor | `DraftEditor` (title, body, Save / Delete / Cancel) | `mosaic-pkg-draft-editor` |

The layout is a `HostNavigationSplit`: a **New entry** button, the search field
and the timeline in the pane, and the editor in the detail. A search turns the
timeline into ranked results (the engine's `search`: every word must match).

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

## Where it runs

**Qt (Linux)** is the first native host (J5a). In CI, its Qt lane:

- builds the generated project against `journal-mosaic-app`, which the host
  loads as `libmosaic_app`;
- installs it, and launches `JournalApp` offscreen twice, so the second launch
  restores the state the first saved.

The same lane also runs this package's tests.

**SwiftUI and Compose** (J5b) build in CI: the SwiftUI lane runs `swift build`
and the Compose lane runs `gradle compileKotlin`. Both use the same strict
binding, with zero degradations.

**The browser** (J5c): `host/web` is a Vite and React page. It mounts the
generated `JournalApp` over the `journal-mosaic-app` runtime built for wasm32,
and it keeps the journal in `localStorage`. See [`host/web/README.md`](host/web/README.md).
`scripts/build-web.sh` regenerates its inputs, and
`.github/workflows/journal-mosaic-web.yml` tests it against the real wasm.

Not yet:

- launching on SwiftUI or Compose, and the Flutter and XAML lanes;
- packaging and release (J5).
