# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-19

### Added — WB5 (UI28 §8) — Qt/QML backend skeleton

Initial cut of the Qt backend for the Mosaic three-language pipeline.
This crate emits **QML source** (Qt's declarative UI language), not
C++ — see `README.md` for the rationale.

- `pipeline::from_pipeline(&MosmodelComponent, &LayoutDef, &StyleDef)
  -> Result<PipelineEmitResult, PipelineEmitError>` — the single public
  entry point. Mirrors the React backend's signature so `mosaic-compile`
  can dispatch uniformly across backends.
- `PipelineEmitResult { output, component_name }` — the QML source and
  the component's PascalCase name.
- `PipelineEmitError` with four variants: `ComponentNameMismatch`,
  `UnsafeSlotName`, `UnsafeEmitName`, `UnknownPrimitive`. Each carries
  the offending name in its `Display` form so CLI consumers can print
  the message verbatim.
- File header: auto-generated banner + `import QtQuick 2.15` + `import
  QtQuick.Layouts 1.15`, both version-pinned for reproducibility.
- Root element is always `Item { }`. QML requires exactly one top-level
  element; the wrapper carries the component's public interface
  (`property` and `signal` declarations).
- Slot → `property` lowering. Each `SlotDecl` becomes one `property
  <qmlType> <camelName>: <default>` line on the root `Item`. Type map:
  `text`→`string`, `number`→`real`, `bool`→`bool`, `image`→`url`,
  `color`→`color`, `node`→`Component`, `list<T>`→`var`,
  `Component(X)`→`Component`. Slot names convert kebab→camel.
- Emit → `signal` lowering. Each `EmitDecl` becomes one `signal
  <name>(<params>)` line on the root `Item`. The `on` prefix is
  stripped per UI24 §5; parameterless emits emit `signal foo()`,
  parameterised emits get typed QML parameters (`real`/`string`/
  `bool`/`color`/`var`).
- Primitive lowering:
  - `Box` → `Item { ... }`
  - `Row` → `RowLayout { ... }`
  - `Column` → `ColumnLayout { ... }` — *with a documented TODO for
    the UI28 §2.2 layout-vs-data-Column conflict.*
  - `Text` → `Text { text: "..." }` or `Text { text: slotName }` for
    slot-ref content (bare-identifier binding, not quoted)
  - `Spacer` → `Item { Layout.fillWidth: true; Layout.fillHeight: true }`
  - `Image` → `Image { source: "..." }` or `Image { source: slotName }`
  - `Divider` → `Rectangle { height: 1; color: "#888"; Layout.fillWidth: true }`
- 17 unit tests covering: empty layout skeleton, slot property
  generation for every `SlotType`, signal generation for void emits,
  signal generation with typed parameters, kebab→camel conversion for
  slots and emits and emit params, Row → RowLayout, Column →
  ColumnLayout, Text slot-ref content (bare identifier), Text
  string-literal content (escaped), Image with string and slot-ref
  source, Spacer with both fillWidth + fillHeight, Divider as 1px
  Rectangle, nested container tree, component-name mismatch error,
  unknown primitive error, imports precede the root Item, and crate
  version.

### Known limitations of the first-cut Qt/QML path (deferred follow-ups)

These items are accepted in the IR but not yet emitted:

- **No `Cell` / data-`Column` / `Grid v3` lowering** (UI28 §2). The
  spec's Qt mapping at §4.5 uses C++ classes
  (`QStyledItemDelegate` / `QAbstractTableModel` / `QTableView`).
  This QML backend will lower to `QtQuick.Controls` `TableView`
  instead, keeping the entire backend in one surface. Tracked as a
  follow-up PR.
- **No `connects` wiring.** Today a `signal` is declared but never
  emitted from inside the layout tree. A follow-up will attach a
  `MouseArea` (or equivalent) to each layout element whose props
  contain an `EmitRef` value, with `onClicked` firing the matching
  signal.
- **No style inlining.** The mosstyle `StyleDef` is accepted in the
  signature so downstream callers can build against the stable
  shape, but its properties are not yet inlined into element
  attributes.
