# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added - host-owned surface composition

`HostSurface ( content: slot: ... )` now lowers to a styled Qt Quick
`Rectangle` with a filling `Loader` bound to the host's QML `Component`.

### Changed - Qt project shells support host file dialogs

Generated Qt project shells now use `QApplication` and link `Qt6::Widgets` in
addition to `Qt6::Quick`, allowing optional `MosaicHost.h/.cpp` adapters to
open native file dialogs for platform-owned host intents without hand-patching
generated CMake or `main.cpp`.

### Added - optional MosaicHost bridge for Qt project shells

Generated Qt project shells now compile with or without an installed
`MosaicHost.h/.cpp` adapter. When present, `main.cpp` injects the host object
into QML, hydrates initial props through `applyMosaicProps`, and emitted
`mosaicEvent` envelopes round-trip through `mosaicHost.handleEvent(event)`.
`CMakeLists.txt` also picks up an installed host adapter and copies a colocated
Engram-style native host library next to the executable.

### Added - generic Mosaic event signal for QML hosts

Generated QML components with emits now expose `signal mosaicEvent(var event)`
and re-emit each specific signal through it as an object preserving the original
Mosaic emit name and payload keys. Qt hosts can connect once to `mosaicEvent`
for the same event-envelope bridge used by the other native shells.

### Added — UI32-K-qt — `--emit-project` Qt6 + CMake desktop shell

L6 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2 React #4297, L3 HTML #4309, L4 WebComponent #4315, L5 Flutter #4319). `mosaic-compile --backend qt --emit-project` now produces a Qt6 + CMake desktop scaffold:

- `CMakeLists.txt` — pinned Qt6 `6.7` + CMake `3.21` + C++17 per UI32 §3.6.3. `qt_add_executable` + `qt_add_qml_module` embed the component into a `Mosaic.<Component>` QML module.
- `main.cpp` — `QGuiApplication` + `QQmlApplicationEngine` loading `qrc:/qt/qml/Mosaic/<Component>/<Component>.qml` (the path `qt_add_qml_module` exposes).
- `qmldir` — `module Mosaic.<Component>` + `<Component> 1.0 <Component>.qml`.
- `README.md` — `cmake -B build && cmake --build build && ./build/<Component>` recipe + file map.

New public API (matches L2-L5 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_qt_version`, `pinned_cmake_min`, `pinned_cxx_standard`.
- `pub struct ProjectFiles` — `cmake_lists`, `main_cpp`, `qmldir`, `readme`.
- `pub struct PipelineEmitResultWithProject`.
- `pub fn from_pipeline_with_options(...)`. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 Qt row: no per-PR constraint beyond upstream — CMake target names exclude `-` (already excluded by `validate_component_name`). qmldir module names follow PascalCase (component name already PascalCase).

9 new tests cover the spec §3 gates plus a qrc-path test that verifies main.cpp loads the component from the embedded QML module via the correct `qrc:/qt/qml/Mosaic/<Component>/<Component>.qml` resource path. Total tests: 90 (was 81, +9).

### Added — UI31-K-qt — `HostTable` RTL contract

The Qt `HostTable` lowering (which produces a structural
`ColumnLayout` of `RowLayout` rows) now honours the UI31 §3.2 RTL
contract via QML's `LayoutMirroring` attached property:

- `dir: rtl` → `LayoutMirroring.enabled: true` +
  `LayoutMirroring.childrenInherit: true` so the flip propagates
  into the body's `RowLayout` rows and cell order matches the
  column flip.
- `dir: ltr` → `LayoutMirroring.enabled: false` — explicit
  disable, which is the right thing for an author overriding an
  ambient RTL ancestor.
- `dir: auto` → no attached property emitted; the spec-mandated
  "let the host decide" semantic is the QML default of inheriting
  from an ancestor (typically the `ApplicationWindow` root's
  `LayoutMirroring`).
- `dir: slot: layout-direction` →
  `LayoutMirroring.enabled: layoutDirection`, where the slot must
  evaluate to a `bool`. The slot name passes through
  `is_safe_identifier` so it can't smuggle malicious QML through
  the binding expression.
- Unknown keywords drop silently — the allow-list is the security
  gate against attacker-controlled keywords smuggling arbitrary
  QML through the attribute position. The bare `ColumnLayout` still
  renders so the rest of the table is intact.

7 new tests cover the a11y gate (ColumnLayout + RowLayout shape
preserved), the three allow-listed keywords (including the no-
emit `auto` case), the slot-ref binding, the silent-drop with an
injection-style payload (`"true; Component.onCompleted: pwn()"`),
and a regression guard for the no-`dir` case. Total tests: 81 (was
74).

### Added — U29-4-K-qt — `HostLink` + `HostTooltip` + `HostNumberInput` kernel primitive lowerings

Three new UI29-4 kernel primitives lower to QML widgets:

- **`HostLink` → `Text { textFormat: Text.RichText; text:
  "<a href='...'>label</a>"; onLinkActivated: ... }`**. QtQuick
  has no first-class hyperlink widget; rich-text `Text` is the
  idiomatic shape. `external: false` + `onActivate` dispatches the
  emit without opening the URL externally (host's router handles
  it). Default behavior is `Qt.openUrlExternally(link)`; `external:
  false` + `onActivate` switches to dispatch-only; bare
  `onActivate` does BOTH (dispatch + external open).
- **`HostTooltip` → `Item { ToolTip.text: "..."; ToolTip.visible:
  hoverHandler.hovered; HoverHandler { id: hoverHandler }; child(ren) }`**.
  Wraps the child(ren) so the tooltip activates on hover via
  `HoverHandler` (QtQuick 2.12+). `HoverHandler` is used instead of
  `MouseArea` so clicks still reach the wrapped child unimpeded.
- **`HostNumberInput` → `TextField { text; DoubleValidator;
  enabled; onTextEdited }`**. QtQuick.Controls 2.15 TextField keeps
  direct text entry, preserves fractional `number` values, and
  dispatches parsed values only for user edits.

Infrastructure:

- `tree_needs_controls_import` now also fires on `HostTooltip` and
  `HostNumberInput` (both lower to QtQuick.Controls widgets).
  `HostLink` is intentionally NOT in the list — it lowers to plain
  `Text`, not a Controls widget.
- New `find_number_prop` helper alongside the existing
  `find_string_prop` / `find_slot_ref_prop` / `find_emit_ref_prop`
  for `HostNumberInput`'s numeric-literal value/min/max props.

6 new tests cover: HostLink rich-text rendering with
openUrlExternally, the external-false + onActivate dispatch-only
mode, HostTooltip Item wrapper with HoverHandler, bare
HostNumberInput TextField emission, fractional value preservation,
DoubleValidator bounds, and `onTextEdited` parsed-value dispatch wiring.

### Added — U29-2-K-qt — `HostCheckbox` + `HostRadio` kernel primitive lowerings

Both new UI29-2 primitives lower to QtQuick.Controls 2.15 widgets,
inheriting the native a11y role, focus ring, and keyboard semantics
that composing from QtQuick basics would lose.

`HostCheckbox` -> `CheckBox { ... }`:

- `checked: slot|bool` -> `checked: c` / `checked: true|false`
- `disabled: slot|bool` -> `enabled: !d` (polarity flip; same as
  HostButton)
- `label: str|slot` -> `text: "..."` / `text: <slot>`
- `onToggle: emit: onX` -> `onToggled: x(checked)` (Qt's
  `toggled(bool)` signal forwards the new state to the host)
- `indeterminate: slot: i` -> `tristate: true` + a `checkState: i ?
  Qt.PartiallyChecked : (checked ? Qt.Checked : Qt.Unchecked)`
  ternary. Qt's tri-state checkbox is fully wired here (unlike the
  React backend, which defers tri-state to a follow-up).

`HostRadio` -> `RadioButton { ... }`:

- `checked`, `disabled`, `label` -> same shape as HostCheckbox
- `onSelect: emit: onX` -> `onCheckedChanged: if (checked)
  x(<value>)`. Qt's `checkedChanged()` signal fires on every flip;
  the `if (checked)` gate enforces the kernel-canonical "onSelect =
  this radio was chosen" semantics (sibling-radio-caused deselects
  are silently dropped).
- `value: str|slot` -> `<value>` is interpolated into the dispatch
  payload; string literals are escaped, slot refs are camelCased and
  validated.
- `group: str|slot` -> preserved as a `// group: ...` line comment
  ahead of the RadioButton. QtQuick.Controls's `ButtonGroup` would
  give true radio-group behavior, but wiring it requires a
  structural pass that synthesises a `ButtonGroup` at the enclosing
  scope; that pass is reserved for UI29-2.1's `RadioGroup` userland
  component.

Security: same line-comment-injection vector as the SwiftUI
backend's `// group: ...` was caught and closed inline. A `group:
"x\nimport Evil"` author string would terminate the `//` line
comment and inject arbitrary QML on the next line. The fix replaces
`\n` and `\r` with spaces inside the comment text. A regression test
asserts the invariant.

12 new tests cover: bare CheckBox/RadioButton blocks, the
QtQuick.Controls import trigger, controlled `checked` slot,
`disabled` polarity flip, string label, `onToggle` -> `onToggled`,
indeterminate tri-state, `// group:` comment, the newline-injection
regression, `onSelect`'s positive-gated dispatch, and the slot-typed
`value:` payload flow.

### Added — U29-1-K-qt — `HostDialog` kernel primitive lowering

Brings the 16th UI29 kernel primitive (`HostDialog`, UI29-1, spec
`code/specs/UI29-1-host-dialog.md`) into the Qt/QML backend.

- **`HostDialog` → `Popup { ... }`** from `QtQuick.Controls 2.15`.
  `Popup` provides focus trap + background dim out of the box when
  `modal: true`; `modal: false` produces an in-flow popover.
- **Prop mappings:**
  - `open: slot: x` → `visible: x` (bare identifier binding); absent →
    `visible: false`.
  - `modal: true` / `modal: false` (compile-time keyword) → `modal:
    true` / `modal: false`; default is `modal: true`.
  - `dismiss-on-backdrop: false` → `closePolicy: Popup.CloseOnEscape`
    (Esc-only); default (absent or `true`) →
    `Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent`.
  - `onClose: emit: onE` → `onClosed: e()` — Popup's signal is
    past-tense `closed`; the Mosaic emit follows the `on` + present
    convention.
  - `onOpen: emit: onE` → `onOpened: e()` — same pattern.
- **`title:` slot or literal** lowers to a synthesised
  `Text { text: ...; font.bold: true }` as the first child of
  `contentItem`. Plain `Popup` has no built-in title slot, so we
  insert a bold text row before the author's body.
- **Children render inside `contentItem: ColumnLayout { ... }`.** We
  always emit the `ColumnLayout` wrapper (even for an empty dialog)
  so the Popup has a well-defined single content element and a
  consistent styling anchor across calls.
- **Conditional `QtQuick.Controls 2.15` import** now triggers on
  `HostDialog` too, alongside `HostButton` and `HostScroll`.
- **9 new unit tests** in `pipeline.rs` cover: empty dialog skeleton;
  `open` slot → `visible`; `modal: true`; `modal: false`; `onClose` →
  `onClosed`; children rendered inside contentItem; `title:` slot
  emits bold Text as first child (ordering pinned); `dismiss-on-backdrop:
  false` switches to escape-only closePolicy (with a sanity check that
  the default keeps outside-press); and that using `HostDialog` triggers
  the QtQuick.Controls 2.15 import. A 10th test pins `onOpen` →
  `onOpened` separately so regressions on one signal direction don't
  slip through.

### Added — U29-K-qt — `For` / `If` / `Else` meta-primitive lowering

Completes the Qt/QML kernel surface for the UI29 §3 meta-primitives.

- **`For (each: <expr>, as: <NAME>, index: <NAME>?) { ... }`** lowers
  to a `Repeater { model: <coll>; delegate: Item { ... } }`. The
  delegate is always an `Item` carrying `property var <as>: modelData`
  and (when bound) `property int <index>: index`, so descendants reach
  the per-iteration values through QML's normal scope rules.
  `each: slot: foo` → `model: foo` (camelCased bare identifier);
  `each: <expr>` → `model: <expr>` (passed verbatim).
- **`If (when: <expr>) { ... } Else { ... }`** lowers to one or two
  `Loader { active: <cond>; sourceComponent: Component { ... } }`
  blocks. With no `Else`, only the then-branch Loader is emitted;
  with an `Else`, a second Loader carries `active: !<cond>` (with
  parenthesisation for compound expressions so the negation binds the
  whole predicate).
- **`If`+`Else` sibling pairing** is done by a shared
  `emit_qml_children` walker so that the rule "Else must immediately
  follow If" matches the UI29 §3.2 grammar. An `Else` that doesn't
  immediately follow an `If` — or appears at the root — emits a
  `// orphan Else (no preceding If)` self-documenting comment instead
  of erroring.
- **Branch-body wrapping.** A `Component { ... }` accepts exactly one
  top-level element; multi-child branches are wrapped in an inner
  `Item { ... }` so the `Component` stays well-formed. Single-child
  branches emit the child inline to keep output clean.
- **11 new unit tests** in `pipeline.rs` cover: Repeater with
  camelCased model from a slot ref; both `as:` + `index:` delegate
  properties when bound; expression-valued `each:` passed verbatim;
  body referencing the as-bound name; single-Loader If; If+Else with
  inverted `active:`; expression-valued `when:` plus parenthesised
  negation; orphan-Else comment at root; non-immediate-sibling Else
  orphan path; nested Repeaters share the children walker; multi-child
  If body wraps in `Item`.
- Replaces the previous "If/For still error as UnknownPrimitive"
  regression test with a generic `unknown_primitive_still_errors` test
  that pins the failure path for *genuinely* unknown tags.

### Added — U29-K-qt — `HostTable` lowering (structural first cut)

Brings `HostTable` and its four sub-tags into the Qt backend. The
first cut uses layout primitives (`ColumnLayout` of `RowLayout` rows)
rather than QtQuick's data-driven `TableView`+`QAbstractTableModel` —
that integration is a follow-up once `For` and a row-model source are
specified.

- **`HostTable` → `ColumnLayout { spacing: 0; ... }`.** Sub-tags are
  walked in order:
  - `HostTableHead { Row { Text ... } }` → `RowLayout { Text { ...;
    font.bold: true } }`, followed by a 1-pixel `Rectangle` divider.
  - `HostTableBody { Row { Text ... } }` → `RowLayout { Text { ... } }`
    (no bolding, no divider).
  - `HostTableFoot { Row { Text ... } }` → 1-pixel `Rectangle` divider
    followed by a `RowLayout` (visually separates foot from body).
  - `HostTableColGroup` → ignored with a `// HostTableColGroup (no
    QML analog)` comment.
- **Orphan sub-tag handling.** A `HostTableHead`/`Body`/`Foot`/
  `ColGroup` used outside a `HostTable` parent emits a self-documenting
  `// orphan HostTableX (outside HostTable)` comment rather than
  erroring. Keeps the emitter resilient.
- **`part_name` on `HostTable`** is accepted but not yet consumed —
  styling integration for table parts is a follow-up. A regression
  test pins the don't-break behaviour.
- **8 new unit tests** in `pipeline.rs` cover: empty table skeleton;
  bold head cells; plain body rows; foot-preceded-by-divider;
  head→divider→body ordering; ColGroup-as-comment; orphan tolerance;
  `part_name` resilience.
- Removed `HostTable` from the `UnknownPrimitive` deferred-error test
  (kept `If` and `For`, which remain deferred to U29-G1/U29-G2).

### Spec divergence

`HostTable` is specified to lower to QML `TableView { ... }`. This PR
deliberately ships a `ColumnLayout`+`RowLayout` shape instead: real
`TableView` requires a `QAbstractTableModel` (or `Qt.labs.qmlmodels
.TableModel`) and per-column delegates, neither of which exist in this
pipeline until `For` lands. The divergence is documented in
`emit_host_table_qml`'s rustdoc and the module-level lowering table.

## [0.2.0] - 2026-05-19

### Added — U29-K-qt (partial) — UI29 kernel primitives: Stack, HostInput, HostButton, HostScroll

Extends the Qt/QML backend with four of UI29's seven kernel primitives.
The remaining three (`If`, `For`, `HostTable`) wait on the
U29-G1..U29-G3 grammar work and a `TableView` spec, and continue to
return `UnknownPrimitive` today.

- **`Stack` → `Item { ... }` with `anchors.fill: parent` on each child.**
  QML has no dedicated Z-stack primitive; the idiomatic shape is an
  `Item` overlay with anchored children. We deliberately do NOT use
  `StackLayout` from `QtQuick.Layouts` — its semantics are "show one
  child at a time, switch with `currentIndex`", which is a navigation
  primitive, not an overlay.
- **`HostInput` → `TextInput { ... }`.** Props lower as follows:
  - `value: slot: x` → `text: x` (bare identifier binding)
  - `value: "literal"` → `text: "literal"` (escaped)
  - `read-only: slot: x` → `readOnly: x`
  - `read-only: true/false` → `readOnly: true/false`
  - `placeholder: "..."` → comment line only (`TextInput` has no
    placeholder attribute; the QML idiom is a sibling `Text` shown
    when `text === ""`, deferred to a follow-up PR).
  - `onChange: emit: onE` → `onTextChanged: e()`
  - `onCommit: emit: onE` → `onAccepted: e(text)` (Enter)
  - `onCancel: emit: onE` → `Keys.onEscapePressed: { e(); event.accepted = true }`
- **`HostButton` → `Button { ... }`** (from `QtQuick.Controls 2.15`).
  Props lower as follows:
  - `label: slot: x` → `text: x`
  - `label: "literal"` → `text: "literal"`
  - `disabled: slot: x` → `enabled: !x` (polarity flip at lowering time)
  - `disabled: true/false` → `enabled: !true/false`
  - `onTap: emit: onE` → `onClicked: e()`
- **`HostScroll` → `ScrollView { ... children ... }`** (from
  `QtQuick.Controls 2.15`). The wrapper accepts arbitrary children
  directly, which matches the UI29 §4 spec mapping. We use `ScrollView`
  rather than the lower-level `Flickable` because the wrapper is
  simpler and the spec table calls it out as the preferred mapping.
- **Conditional `QtQuick.Controls 2.15` import.** The Controls import
  is added only when the layout tree references a primitive that
  lowers to a Controls element (today: `HostButton`, `HostScroll`).
  Importing Controls unconditionally is harmless at runtime but adds
  noticeable startup cost on resource-constrained platforms, so we
  keep the import set minimal.
- **9 new unit tests** covering each primitive's structural output,
  the `onAccepted` / `Keys.onEscapePressed` mapping for `HostInput`,
  the `enabled: !` polarity flip for `HostButton`, the conditional
  `QtQuick.Controls` import, and the continued `UnknownPrimitive`
  error path for the deferred `If` / `For` / `HostTable` primitives.

### Spec divergence

None. The lowerings track UI29 §4 row "Qt" exactly. The `placeholder`
prop on `HostInput` is the one deferred behaviour and is called out in
both the README and the inline doc on `emit_host_input_qml`.

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
