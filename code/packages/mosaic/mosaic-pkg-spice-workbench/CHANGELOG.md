# Changelog — mosaic-pkg-spice-workbench

## [Unreleased]

### Added — this changelog (SW-5)

- The package had none, the only `code/packages/mosaic` package without
  one. The history below is reconstructed from the merged PRs that touched
  `src/` and `tests/`.

### Changed — the README's state of data-bound `Path`

- The README called data-bound `Path` geometry "a native XAML follow-up".
  XAML binds it since #14682. Qt, Flutter and Compose bind it since #16926
  (SW-2), so the workbench's trace surface now compiles on seven of eight
  backends. SwiftUI waits on #13206.

## [0.1.0] - 2026-09-20

Built up over 24 PRs from 2026-09-08 to 2026-09-20, without a release. In order:

- **Foundation** (#14594): deck editor, Inspect and Run controls,
  runnable-analysis selection and source-span diagnostics. Result tables
  (#14652) and waveform traces with axes (#14667) followed. MosaicBook gates
  its stories in CI (#14688).
- **Schematic editing**:
  - session synchronization (#14888), grid routing (#14899), and the
    passive and source palette (#14908);
  - a property inspector (#14929), metadata editing (#15021), net labels
    (#15052) and terminal routing (#15097);
  - an edit lifecycle (#15011), with Undo and Redo (#15026).
- **Analysis plans**: configurable cards (#14935), multi-analysis plans
  (#14985), then refinements (#15035).
- **Schematic document I/O** (#15002): versioned
  `spice-mosaic/schematic` JSON through protocol-2 awaited file effects.
- **Output probes**: saved (#15513), scoped to analysis cards (#15536),
  differential outputs (#15581), and nonlinear currents (#15750).
- **Device models**: transistor polarity (#15759), the primary nonlinear
  model fields (#15771), and the corpus-backed model fields (#15782).
