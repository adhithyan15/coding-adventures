//! # Three-file pipeline entry point for the Qt/QML backend.
//!
//! This module is the Qt backend's consumer of the three-language pipeline
//! output produced by [`mosmodel-compiler`], [`moslayout-compiler`], and
//! [`mosstyle-compiler`]. The public entry point is [`from_pipeline`].
//!
//! ## QML in one paragraph (for readers new to Qt)
//!
//! QML is Qt's declarative UI language. A `.qml` file declares exactly one
//! top-level element; that element can host `property` declarations (the
//! analogue of React props or SwiftUI `@Binding`), `signal` declarations
//! (the analogue of React event callbacks), and child elements (the visible
//! tree). When the host application loads the QML file via
//! `QQmlApplicationEngine::load(...)`, the engine instantiates the tree,
//! exposes the properties for binding, and connects the signals to host
//! callbacks. The shape closely mirrors React: properties flow in, signals
//! flow out.
//!
//! ## Per-IR mapping
//!
//! | IR construct                  | QML construct                         |
//! |---|---|
//! | `MosmodelComponent.slots`     | `property <qmlType> name: <default>`  |
//! | `MosmodelComponent.emits`     | `signal name(<params>)`               |
//! | `LayoutNode { tag, … }`       | One QML element (see primitive table) |
//! | `LayoutPropValue::SlotRef(s)` | Bare identifier binding to property   |
//! | `StyleDef`                    | Not yet inlined (deferred — see lib)  |
//!
//! ## Primitive lowering table (first cut, UI28 §4.5 deferred work omitted)
//!
//! | moslayout tag | QML element                                                                  |
//! |---|---|
//! | `Box`         | `Item { ... }`                                                              |
//! | `Row`         | `RowLayout { ... }`                                                         |
//! | `Column`      | `ColumnLayout { ... }` — UI28 §2.2 introduces a *data* `Column`; this is the **layout** Column. Tracked. |
//! | `Text`        | `Text { text: "..." }` or `Text { text: slotName }` for slot-ref content    |
//! | `Spacer`      | `Item { Layout.fillWidth: true; Layout.fillHeight: true }`                  |
//! | `Image`       | `Image { source: "..." }`                                                   |
//! | `Divider`     | `Rectangle { height: 1; color: "#888"; Layout.fillWidth: true }`            |
//! | `Stack`       | `Item { ... }` with `anchors.fill: parent` on each child — Z-axis overlay   |
//! | `HostInput`   | `TextInput { ... }` or `TextField { ...; placeholderText: ... }`            |
//! | `HostButton`  | `Button { text: ...; enabled: ...; onClicked: ... }` (Controls 2.15)        |
//! | `HostScroll`  | `ScrollView { ... children ... }`                                           |
//! | `HostDialog`  | `Popup { modal: ...; visible: ...; closePolicy: ...; contentItem: ColumnLayout { ... } }` (Controls 2.15) |
//! | `HostTable`   | `ColumnLayout { ... }` of `RowLayout` rows (first-cut shape — see `emit_host_table_qml` for the deferred true-`TableView` lowering) |
//! | `For`         | `Repeater { model: <coll>; delegate: Item { property var <as>: modelData; <children> } }` — see `emit_for_qml` |
//! | `If`/`Else`   | `Loader { active: <cond>; sourceComponent: Component { ... } }` pairs — see `emit_if_qml` |
//!
//! ## What this emitter does NOT yet do
//!
//! See the module-level doc on the crate root (`src/lib.rs`). The short list:
//! Cell/data-Column/Grid v3 primitives (UI28 §2); `connects` wiring from
//! `EmitRef` props to `signal` emissions inside the tree; and mosstyle
//! inlining into element attributes. `HostTable` lowers to a structural
//! `ColumnLayout`+`RowLayout` shape today (true `TableView` +
//! `QAbstractTableModel` integration is a follow-up).
//!
//! ## Why the root is always `Item`
//!
//! QML requires exactly one top-level element per file. We always wrap the
//! component in an `Item` even if the moslayout tree is itself a single
//! container. Reasons:
//!
//! 1. **Slots / signals belong on the component wrapper**, not on the inner
//!    layout. If we lifted them onto, say, a top-level `RowLayout` instead,
//!    the host would be binding to a *layout primitive*'s namespace — fragile
//!    and confusing. The wrapper `Item` cleanly carries the component's
//!    public interface; the inner element is purely structural.
//! 2. **`Item` is the lightest QML element**: zero painting, no layout
//!    behaviour. It costs nothing at runtime and disappears visually.

use std::fmt::Write as _;

use std::collections::HashMap;

use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
};
use mosstyle_compiler::{StyleDef, StyleProp};

// =====================================================================
// Public API
// =====================================================================

/// The result of compiling a three-file pipeline triple to QML source.
///
/// Field names mirror the React backend's `PipelineEmitResult` so callers
/// (such as the `mosaic-compile` CLI) can treat all pipeline backends
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResult {
    /// The complete QML source.
    pub output: String,
    /// The component's PascalCase name (matches the source `.mil` / `.mll` files).
    pub component_name: String,
}

/// Errors the Qt pipeline emitter can return.
///
/// Variants are intentionally string-bearing rather than rich values: the
/// downstream consumer (CLI) is expected to print the `Display` form
/// verbatim, so embedding the offending name in the message keeps the
/// error message self-explanatory without additional plumbing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEmitError {
    /// The mosmodel component name and the moslayout component name disagree.
    /// Authoring error — almost always a manifest typo.
    ComponentNameMismatch { mosmodel: String, moslayout: String },
    /// A slot name fails the safe-identifier check after camelCase conversion.
    /// The mosmodel grammar already enforces a safe shape; this is a defense-
    /// in-depth check at lowering time.
    UnsafeSlotName(String),
    /// An emit name fails the same safe-identifier check.
    UnsafeEmitName(String),
    /// A moslayout primitive tag is not recognised by the Qt backend.
    /// See the primitive table at the top of this module for the supported
    /// set.
    UnknownPrimitive(String),
}

impl std::fmt::Display for PipelineEmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineEmitError::ComponentNameMismatch {
                mosmodel,
                moslayout,
            } => write!(
                f,
                "component name mismatch: mosmodel says '{mosmodel}', moslayout says '{moslayout}'"
            ),
            PipelineEmitError::UnsafeSlotName(n) => {
                write!(f, "unsafe slot name '{n}' (post camelCase conversion)")
            }
            PipelineEmitError::UnsafeEmitName(n) => {
                write!(f, "unsafe emit name '{n}' (post conversion)")
            }
            PipelineEmitError::UnknownPrimitive(t) => write!(
                f,
                "moslayout primitive '{t}' is not yet supported by the Qt/QML emitter"
            ),
        }
    }
}

impl std::error::Error for PipelineEmitError {}

// =====================================================================
// UI32-K-qt — `--emit-project` Qt6 + CMake app shell
//
// Mirrors L2 (React #4297), L3 (HTML #4309), L4 (WebComponent #4315),
// L5 (Flutter #4319): EmitOptions / ProjectFiles /
// from_pipeline_with_options.
//
// When `--emit-project` is on, emits a complete CMake-driven Qt6
// project alongside the component .qml. Author runs `cmake -B build
// && cmake --build build && ./build/<Component>` to see the
// component on a desktop window.
// =====================================================================

/// Options controlling the Qt/QML emitter's behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Also emit `CMakeLists.txt`, `main.cpp`, `qmldir`, `README.md`
    /// alongside the component `.qml`. Default `false`.
    pub emit_project: bool,

    /// Pinned Qt6 version constraint to write into `CMakeLists.txt`'s
    /// `find_package(Qt6 REQUIRED ...)`. UI32 spec §3.6.3 requires
    /// exact pinning. Default `"6.7"` — a known-good 6.7 LTS.
    /// CMake accepts `6.7` as a major.minor constraint that resolves
    /// to any installed 6.7.x.
    pub pinned_qt_version: String,

    /// Pinned CMake minimum-required version. Default `"3.21"` — the
    /// floor Qt6 + AUTOUIC support requires.
    pub pinned_cmake_min: String,

    /// CMake C++ standard. Default `"17"` — Qt6's documented minimum.
    pub pinned_cxx_standard: String,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            emit_project: false,
            pinned_qt_version: "6.7".to_string(),
            pinned_cmake_min: "3.21".to_string(),
            pinned_cxx_standard: "17".to_string(),
        }
    }
}

/// Project-shaped artifacts emitted when `EmitOptions::emit_project`
/// is on. Four files — enough for `cmake -B build && cmake --build
/// build && ./build/<Component>` to launch a Qt6 desktop window
/// hosting the component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFiles {
    /// `CMakeLists.txt` — pinned Qt6 + CMake versions, `find_package(Qt6
    /// REQUIRED COMPONENTS Quick QmlImportScanner)`, executable
    /// target that links Qt6::Quick and embeds the QML via
    /// `qt_add_qml_module`. The component `.qml` lands inside the
    /// QML module.
    pub cmake_lists: String,
    /// `main.cpp` — `QGuiApplication` + `QQuickView`
    /// loading the component QML from the embedded QML module. Falls
    /// through gracefully if the view fails to instantiate it
    /// (returns -1).
    pub main_cpp: String,
    /// `qmldir` — Qt's QML module descriptor. Lists the component
    /// as the module's export. The module name is `Mosaic.<Component>`
    /// (derived inline; the single-component CLI path doesn't have
    /// a mosaic-package.toml so we can't use the artifact-builder's
    /// `qmldir_module_name` helper directly).
    pub qmldir: String,
    /// `README.md` — prereqs (Qt6 6.7+, CMake 3.21+, a C++17
    /// compiler), build + run commands, file map.
    pub readme: String,
}

/// Extended pipeline result — carries the optional `ProjectFiles`
/// when `emit_project` is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResultWithProject {
    pub output: String,
    pub component_name: String,
    pub project: Option<ProjectFiles>,
}

/// Compile a three-file Mosaic pipeline triple to QML with explicit
/// emit options.
pub fn from_pipeline_with_options(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
    options: &EmitOptions,
) -> Result<PipelineEmitResultWithProject, PipelineEmitError> {
    let component = from_pipeline(interface, layout, style)?;

    let project = if options.emit_project {
        Some(build_qt_project_files(&component.component_name, options))
    } else {
        None
    };

    Ok(PipelineEmitResultWithProject {
        output: component.output,
        component_name: component.component_name,
        project,
    })
}

/// Build the four Qt-shell side files for a single component.
///
/// UI32 spec §3.6.2 Qt row: no per-PR identifier shape constraint
/// beyond upstream — CMake target names exclude `-` (validated by
/// `validate_component_name` upstream as ASCII identifier shape).
/// The qmldir module name follows the QML PascalCase convention,
/// which the upstream-validated component name already satisfies.
fn build_qt_project_files(name: &str, options: &EmitOptions) -> ProjectFiles {
    // Single-component QML module name: `Mosaic.<Component>`.
    // (The artifact-builder's `qmldir_module_name` derives from
    // package name; the single-component CLI path has no package
    // context, so we inline the convention here.)
    let module_name = format!("Mosaic.{name}");
    ProjectFiles {
        cmake_lists: build_cmake_lists(name, &module_name, options),
        main_cpp: build_main_cpp(name, &module_name),
        qmldir: build_qmldir(name, &module_name),
        readme: build_qt_readme(name, &module_name),
    }
}

const BANNER_CMAKE: &str = "# AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n# Fork the file (remove this banner) to customise.\n";
const BANNER_CPP: &str = "// AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n// Fork the file (remove this banner) to customise.\n";
const BANNER_QMLDIR: &str = "# AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n# Fork the file (remove this banner) to customise.\n";
const BANNER_MD: &str = "<!-- AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit. -->\n<!-- Fork the file (remove this banner) to customise. -->\n";

fn build_cmake_lists(name: &str, module_name: &str, options: &EmitOptions) -> String {
    format!(
        "{BANNER_CMAKE}cmake_minimum_required(VERSION {})\nproject({} VERSION 0.1.0 LANGUAGES CXX)\n\nset(CMAKE_CXX_STANDARD {})\nset(CMAKE_CXX_STANDARD_REQUIRED ON)\nset(CMAKE_AUTOMOC ON)\n\nfind_package(Qt6 {} REQUIRED COMPONENTS Quick QmlImportScanner Widgets)\nqt_standard_project_setup(REQUIRES {})\n\nqt_add_executable({} main.cpp)\n\nif(EXISTS \"${{CMAKE_CURRENT_SOURCE_DIR}}/MosaicHost.cpp\")\n  target_sources({} PRIVATE MosaicHost.cpp MosaicHost.h)\nendif()\n\nqt_add_qml_module({}\n  URI {}\n  VERSION 1.0\n  QML_FILES {}.qml\n)\n\ntarget_link_libraries({} PRIVATE Qt6::Quick Qt6::Widgets)\n\nforeach(_mosaic_native_library IN ITEMS engram_capi.dll libengram_capi.dylib libengram_capi.so venture_browser_qt.dll libventure_browser_qt.dylib libventure_browser_qt.so)\n  if(EXISTS \"${{CMAKE_CURRENT_SOURCE_DIR}}/${{_mosaic_native_library}}\")\n    add_custom_command(TARGET {} POST_BUILD\n      COMMAND ${{CMAKE_COMMAND}} -E copy_if_different\n        \"${{CMAKE_CURRENT_SOURCE_DIR}}/${{_mosaic_native_library}}\"\n        \"$<TARGET_FILE_DIR:{}>/${{_mosaic_native_library}}\"\n    )\n  endif()\nendforeach()\n",
        options.pinned_cmake_min,
        name,
        options.pinned_cxx_standard,
        options.pinned_qt_version,
        options.pinned_qt_version,
        name,
        name,
        name,
        module_name,
        name,
        name,
        name,
        name,
    )
}

fn build_main_cpp(name: &str, module_name: &str) -> String {
    let module_name_slash = module_name.replace('.', "/");
    let mut out = String::new();
    out.push_str(BANNER_CPP);
    out.push_str("#include <QApplication>\n");
    out.push_str("#include <QMetaObject>\n");
    out.push_str("#include <QObject>\n");
    out.push_str("#include <QQuickItem>\n");
    out.push_str("#include <QQuickView>\n");
    out.push_str("#include <QUrl>\n");
    out.push_str("#include <QVariant>\n\n");
    out.push_str("#if __has_include(\"MosaicHost.h\")\n");
    out.push_str("#include \"MosaicHost.h\"\n");
    out.push_str("#define MOSAIC_HAS_HOST 1\n");
    out.push_str("#else\n");
    out.push_str("#define MOSAIC_HAS_HOST 0\n");
    out.push_str("#endif\n\n");
    out.push_str("int main(int argc, char *argv[])\n");
    out.push_str("{\n");
    out.push_str("  QApplication app(argc, argv);\n");
    out.push_str("#if MOSAIC_HAS_HOST\n");
    out.push_str("  MosaicHost::registerTypes();\n");
    out.push_str("#endif\n");
    out.push_str("  QQuickView view;\n");
    out.push_str("  view.setResizeMode(QQuickView::SizeRootObjectToView);\n");
    writeln!(out, "  view.setTitle(QStringLiteral(\"{name}\"));").unwrap();
    out.push_str("  view.resize(1100, 800);\n\n");
    out.push_str("  // Load the Item component into a visible Qt Quick view from the\n");
    out.push_str("  // embedded module. The qrc:// path is set up by qt_add_qml_module in\n");
    writeln!(
        out,
        "  // CMakeLists.txt; the module URI is {module_name}, so the"
    )
    .unwrap();
    writeln!(
        out,
        "  // component resolves at qrc:/qt/qml/{module_name_slash}/{name}.qml."
    )
    .unwrap();
    writeln!(
        out,
        "  const QUrl url(QStringLiteral(\"qrc:/qt/qml/{module_name_slash}/{name}.qml\"));"
    )
    .unwrap();
    out.push_str("  view.setSource(url);\n");
    out.push_str("  if (view.status() != QQuickView::Ready || view.rootObject() == nullptr) {\n");
    out.push_str("    return -1;\n");
    out.push_str("  }\n\n");
    out.push_str("  QObject *root = view.rootObject();\n");
    out.push_str("#if MOSAIC_HAS_HOST\n");
    out.push_str("  MosaicHost mosaicHost;\n");
    out.push_str("  mosaicHost.attach(root);\n");
    out.push_str(
        "  root->setProperty(\"mosaicHost\", QVariant::fromValue(static_cast<QObject *>(&mosaicHost)));\n",
    );
    out.push_str("  QMetaObject::invokeMethod(root, \"applyMosaicResponse\",\n");
    out.push_str(
        "                            Q_ARG(QVariant, QVariant::fromValue(mosaicHost.props())));\n",
    );
    out.push_str("#endif\n\n");
    out.push_str("  view.show();\n");
    out.push_str("  return app.exec();\n");
    out.push_str("}\n");
    out
}

fn build_qmldir(name: &str, module_name: &str) -> String {
    format!("{BANNER_QMLDIR}module {module_name}\n{name} 1.0 {name}.qml\n")
}

fn build_qt_readme(name: &str, module_name: &str) -> String {
    format!(
        "{BANNER_MD}# {name} — Qt6 + CMake desktop shell\n\nAuto-generated by `mosaic-compile --backend qt --emit-project`.\n\n## Prerequisites\n\n- Qt6 ≥ 6.7 (install via the official installer or your package manager).\n- CMake ≥ 3.21.\n- A C++17 compiler (GCC ≥ 9, Clang ≥ 10, MSVC 2019+).\n\n## Build + run\n\n```sh\ncmake -B build\ncmake --build build\n./build/{name}            # macOS / Linux\nbuild\\\\{name}.exe          # Windows\n```\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `{name}.qml` | The Mosaic-compiled QML component. |\n| `CMakeLists.txt` | CMake build definition. Pinned Qt6 + CMake versions per UI32 spec §3.6.3. |\n| `main.cpp` | `QGuiApplication` + `QQmlApplicationEngine` loading the component as root QML. |\n| `qmldir` | QML module descriptor. Module URI: `{module_name}`. |\n| `README.md` | This file. |\n\n## Editing\n\nEvery file except `{name}.qml` carries an AUTO-GENERATED banner. Re-running `mosaic-compile --emit-project` will overwrite them. To customise the shell, remove the banner from a file and rename or relocate it; the next `--emit-project` run will recreate the original at its original name without touching your forked copy.\n"
    )
    .replace(
        "`QGuiApplication` + `QQmlApplicationEngine` loading the component as root QML.",
        "`QGuiApplication` + `QQuickView` loading the component into a visible desktop window.",
    )
}

// =====================================================================
// mosstyle inlining (UI28 §4.5 — the deferred work, now landed)
//
// The Qt emitter previously DROPPED the `StyleDef`: every `Box [part]`
// lowered to a bare, zero-size `Item { }`, so a VisiCalc-style grid
// collapsed to a black smudge (no width, no borders, no background, no
// alignment). The infrastructure below inlines the part styles so a
// styled `Box [cell]` becomes a fixed-size, bordered, background-filled
// `Rectangle` with its text aligned and coloured per the `.msl`.
//
// The shape mirrors the SwiftUI backend's `build_part_style_map` +
// per-property lowering (`mosaic_emit_swiftui::pipeline`), adapted to
// QML property names:
//
//   | mosstyle property      | QML property                              |
//   |------------------------|-------------------------------------------|
//   | background: #RRGGBB    | color: "#RRGGBB"  (on the Rectangle)      |
//   | border-width: Npx      | border.width: N                           |
//   | border-color: #RRGGBB  | border.color: "#RRGGBB"                   |
//   | height: Npx            | implicitHeight: N + Layout.preferredHeight|
//   | width: Npx             | implicitWidth: N                          |
//   | padding: Npx           | anchors.margins: N (on the inner Text)    |
//   | text-align: right      | horizontalAlignment: Text.AlignRight      |
//   | color: #RRGGBB         | color: "#RRGGBB"  (on the inner Text)     |
//   | font-family: monospace | font.family: "monospace"                  |
//   | font-size: Npx         | font.pixelSize: N                         |
//
// State blocks (`state selected { ... }` / `state editing { ... }`)
// drive the Rectangle's conditional `color:` and the inner Text's
// conditional `color:` via the `state-when-selected` /
// `state-when-editing` predicate Exprs already present on the
// `Box [cell]` node (e.g. `( r == selectedRow && c == selectedCol )`).
// =====================================================================

/// Map from a `part` name (or composite `{part}:{state}` key) to its
/// style props. Mirrors the SwiftUI/React backends' `build_part_style_map`
/// shape so downstream tooling can share the data structure.
type PartStyleMap = HashMap<String, Vec<StyleProp>>;

/// Build a `part_name → props` map from a [`StyleDef`].
///
/// Two key shapes populate the map:
///   * Bare part name (`cell`) → that part's `base` props.
///   * Composite `{part}:{state}` (`cell:selected`) → that state
///     block's overriding props.
///
/// Empty `base` / `state` blocks are skipped so callers can rely on
/// `map.get(key).is_some()` as "the author wrote SOMETHING here".
fn build_part_style_map(style: &StyleDef) -> PartStyleMap {
    let mut out = PartStyleMap::with_capacity(style.parts.len());
    for part in &style.parts {
        if !part.base.is_empty() {
            out.insert(part.name.clone(), part.base.clone());
        }
        for state in &part.states {
            if !state.props.is_empty() {
                let key = format!("{}:{}", part.name, state.state);
                out.insert(key, state.props.clone());
            }
        }
    }
    out
}

/// Shared, read-only context threaded through the whole layout walker.
///
/// Bundling these into one struct keeps the per-emitter signatures
/// stable (a single `&EmitCtx` argument) instead of growing a fresh
/// parameter for every cross-cutting concern.
#[derive(Clone)]
struct EmitCtx<'a> {
    /// The component's declared emits — used to pick signal arities.
    emits: &'a [EmitDecl],
    /// The inlined part-style map (empty when the `.msl` declares no parts).
    part_styles: &'a PartStyleMap,
    /// camelCased identifier of the host's column-widths slot (e.g.
    /// `columnWidths`), discovered from a `HostTableColGroup`. `None`
    /// disables per-cell width threading.
    col_widths_slot: Option<String>,
    /// Index identifier of the nearest enclosing `For` (e.g. `c`, `ch`).
    /// Combined with `col_widths_slot` to produce
    /// `Layout.preferredWidth: columnWidths[<idx>]` on table cells.
    enclosing_index: Option<String>,
    /// Item identifier of the nearest enclosing `For` (e.g. `item`,
    /// `option`). Row buttons use this for single text-like signal
    /// payloads.
    enclosing_item: Option<String>,
    /// Text styling (colour / alignment / font / padding) inherited from
    /// the nearest enclosing styled cell `Box`. Applied to descendant
    /// `Text` / `TextInput` so the cell's value renders aligned and
    /// coloured even though it lives several levels deep (inside an
    /// `If`/`Else` `Loader`). `None` outside a styled cell.
    text_style: Option<CellTextStyle>,
    /// Table-level inherited defaults (from the enclosing `HostTable`'s
    /// `sheet` part). CSS cascades `background` / `color` / `font-*` from
    /// the table down to each cell; a `Box [cell]` whose own part omits
    /// these falls back to the sheet's value so an unselected cell gets
    /// the sheet background and the cell text gets the sheet colour /
    /// font rather than QML's bare defaults (white box, black text).
    inherited: InheritedStyle,
    /// True when the next-emitted children are the DIRECT body of a styled
    /// cell `Rectangle`. Such children (an `If`/`Else` pair lowering to
    /// `Loader`s, or a plain `Text`) must `anchors.fill: parent` so they
    /// occupy the whole fixed-size cell rather than collapsing to their
    /// own content size (which also avoids a `Text.fill` anchor loop
    /// against a content-sized `Loader`). Cleared one level down.
    cell_fill_children: bool,
}

/// Table-level style defaults that cascade down to cells (the `sheet`
/// part's `background` / `color` / `font-family` / `font-size`).
#[derive(Clone, Default)]
struct InheritedStyle {
    background: Option<String>,
    color: Option<String>,
    font_family_mono: bool,
    font_pixel_size: Option<String>,
}

impl<'a> EmitCtx<'a> {
    /// A child context that descends into a `For` carrying its item/index bindings.
    fn with_for(&self, item: String, index: Option<String>) -> Self {
        EmitCtx {
            enclosing_item: Some(item),
            enclosing_index: index.or_else(|| self.enclosing_index.clone()),
            ..self.clone()
        }
    }

    /// A child context carrying a cell's resolved text style (and
    /// clearing the enclosing index so a nested table doesn't inherit a
    /// stale width binding).
    fn with_text_style(&self, ts: Option<CellTextStyle>) -> Self {
        EmitCtx {
            text_style: ts,
            ..self.clone()
        }
    }
}

/// Resolved text styling pulled from a cell part's props (+ its
/// `selected` state). Applied to the inner `Text` / `TextInput` of a
/// styled cell so the value aligns and colours correctly.
#[derive(Clone, Default)]
struct CellTextStyle {
    /// QML expression for the text `color:` — a literal `"#RRGGBB"` or a
    /// conditional `(<selected-pred>) ? "#fff" : "#ccc"`.
    color: Option<String>,
    /// `Text.AlignLeft` / `Text.AlignHCenter` / `Text.AlignRight`.
    horizontal_alignment: Option<&'static str>,
    /// True when `font-family: monospace`.
    font_family_mono: bool,
    /// `font.pixelSize` value (digits only).
    font_pixel_size: Option<String>,
    /// Inner content inset, from `padding: Npx`.
    padding: Option<String>,
}

impl CellTextStyle {
    fn is_empty(&self) -> bool {
        self.color.is_none()
            && self.horizontal_alignment.is_none()
            && !self.font_family_mono
            && self.font_pixel_size.is_none()
            && self.padding.is_none()
    }
}

/// Strip a trailing `px` and validate the remainder is a clean numeric
/// length (`digits`, optional `.`, optional leading `-`). Returns `None`
/// for `100%`, `auto`, `calc(...)`, etc. — values with no clean QML
/// numeric analog. This is also the security gate: only well-formed
/// numbers ever reach a QML attribute position.
fn qml_px_or_none(v: &str) -> Option<String> {
    let stripped = v.trim().strip_suffix("px").unwrap_or(v.trim());
    if !stripped.is_empty()
        && stripped
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        Some(stripped.to_string())
    } else {
        None
    }
}

/// Validate a CSS-ish colour value as a `#RGB` / `#RRGGBB` hex literal
/// and return it normalised to `#RRGGBB`. Returns `None` for anything
/// else (named colours, `rgba(...)`, etc.) — those have no guaranteed-
/// safe inline QML form here, so we skip them rather than risk emitting
/// an attacker-controlled token into a QML string. Hex-only is the
/// security gate: the output is always `#` + 6 hex digits.
fn qml_hex_color_or_none(v: &str) -> Option<String> {
    let body = v.trim().strip_prefix('#')?;
    let expanded: String = if body.len() == 3 && body.chars().all(|c| c.is_ascii_hexdigit()) {
        body.chars().flat_map(|c| [c, c]).collect()
    } else if body.len() == 6 && body.chars().all(|c| c.is_ascii_hexdigit()) {
        body.to_string()
    } else {
        return None;
    };
    Some(format!("#{}", expanded.to_ascii_lowercase()))
}

/// Map a CSS `text-align` keyword to its QML `Text.Align*` enum.
fn qml_text_align(v: &str) -> Option<&'static str> {
    match v.trim() {
        "left" | "start" => Some("Text.AlignLeft"),
        "center" => Some("Text.AlignHCenter"),
        "right" | "end" => Some("Text.AlignRight"),
        _ => None,
    }
}

/// Find the first prop named `name` in a base/state prop list.
fn style_prop<'p>(props: &'p [StyleProp], name: &str) -> Option<&'p str> {
    props
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.value.as_str())
}

/// Collect the `state-when-<X>: ( expr )` predicate for one state on a
/// `Box` node, returning the QML condition expression. Mirrors the
/// SwiftUI backend's `collect_state_layers`, scoped to a single state.
///
/// The predicate text (e.g. `( r == selectedRow && c == selectedCol )`)
/// comes from the developer-authored `.mll` / package source and is
/// interpolated verbatim into a parenthesised QML conditional position,
/// exactly as the React/SwiftUI backends do. The moslayout parser wraps
/// `Expr` values in balanced `( ... )`, keeping the expression contained.
fn state_when_predicate(node: &LayoutNode, state: &str) -> Option<String> {
    let prop_name = format!("state-when-{state}");
    let prop = node.props.iter().find(|p| p.name == prop_name)?;
    match &prop.value {
        LayoutPropValue::Expr(t) => Some(t.clone()),
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            is_safe_identifier(&camel).then_some(camel)
        }
        LayoutPropValue::Keyword(k) => is_safe_identifier(k).then(|| k.clone()),
        _ => None,
    }
}

/// The lowered QML for a styled cell `Box`: the Rectangle's own property
/// lines plus the [`CellTextStyle`] to push down to the inner text.
struct StyledBox {
    /// Property lines to place inside the `Rectangle { ... }` (geometry,
    /// border, conditional background colour). Each already indented by
    /// the caller-supplied pad.
    rect_lines: Vec<String>,
    /// Text styling for the cell's value, threaded to descendant `Text` /
    /// `TextInput`.
    text_style: CellTextStyle,
}

/// Lower a styled cell `Box`'s part (base props + selected/editing state
/// blocks) into [`StyledBox`].
///
/// `ctx` supplies the part-style map and the optional column-width
/// threading (`columnWidths[<enclosing-index>]`). The state predicates
/// are read from `node`'s `state-when-selected` / `state-when-editing`
/// props.
fn lower_styled_box(node: &LayoutNode, part: &str, ctx: &EmitCtx) -> StyledBox {
    let base: &[StyleProp] = ctx
        .part_styles
        .get(part)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let selected: &[StyleProp] = ctx
        .part_styles
        .get(&format!("{part}:selected"))
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let editing: &[StyleProp] = ctx
        .part_styles
        .get(&format!("{part}:editing"))
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let sel_pred = state_when_predicate(node, "selected");
    let edit_pred = state_when_predicate(node, "editing");

    let mut rect_lines: Vec<String> = Vec::new();

    // --- Geometry -----------------------------------------------------
    // height → implicitHeight (+ Layout.preferredHeight for layout
    // children). width → implicitWidth. Column-width threading, when
    // available, overrides any part `width` via Layout.preferredWidth.
    if let Some(h) = style_prop(base, "height").and_then(qml_px_or_none) {
        rect_lines.push(format!("implicitHeight: {h}"));
        rect_lines.push(format!("Layout.preferredHeight: {h}"));
    }
    let threaded_width = match (&ctx.col_widths_slot, &ctx.enclosing_index) {
        (Some(slot), Some(idx)) => Some(format!("{slot}[{idx}]")),
        _ => None,
    };
    if let Some(w) = &threaded_width {
        rect_lines.push(format!("Layout.preferredWidth: {w}"));
        rect_lines.push(format!("implicitWidth: {w}"));
    } else if let Some(w) = style_prop(base, "width").and_then(qml_px_or_none) {
        rect_lines.push(format!("implicitWidth: {w}"));
        rect_lines.push(format!("Layout.preferredWidth: {w}"));
    }
    if let Some(max_width) = style_prop(base, "max-width").and_then(qml_px_or_none) {
        rect_lines.push(format!("Layout.maximumWidth: {max_width}"));
    }
    if let Some(max_height) = style_prop(base, "max-height").and_then(qml_px_or_none) {
        rect_lines.push(format!("Layout.maximumHeight: {max_height}"));
    }
    if let Some(min_width) = style_prop(base, "min-width").and_then(qml_px_or_none) {
        rect_lines.push(format!("Layout.minimumWidth: {min_width}"));
    }
    if let Some(min_height) = style_prop(base, "min-height").and_then(qml_px_or_none) {
        rect_lines.push(format!("Layout.minimumHeight: {min_height}"));
    }

    // --- Border -------------------------------------------------------
    if let Some(bw) = style_prop(base, "border-width").and_then(qml_px_or_none) {
        rect_lines.push(format!("border.width: {bw}"));
    }
    if let Some(bc) = style_prop(base, "border-color").and_then(qml_hex_color_or_none) {
        rect_lines.push(format!("border.color: \"{bc}\""));
    }
    if let Some(radius) = style_prop(base, "border-radius").and_then(qml_px_or_none) {
        rect_lines.push(format!("radius: {radius}"));
    }

    // --- Background (conditional on state) ----------------------------
    // Build a nested ternary: editing wins over selected wins over base.
    // (Matches the React/SwiftUI precedence: later/state layers override
    // the base.) Only states whose `.msl` block sets `background` AND
    // whose `state-when-*` predicate is present participate.
    let base_bg = style_prop(base, "background")
        .or_else(|| style_prop(base, "background-color"))
        .and_then(qml_hex_color_or_none)
        // Cascade: a cell with no own background reads against the
        // table's `sheet` surface.
        .or_else(|| ctx.inherited.background.clone());
    let sel_bg = style_prop(selected, "background")
        .or_else(|| style_prop(selected, "background-color"))
        .and_then(qml_hex_color_or_none);
    let edit_bg = style_prop(editing, "background")
        .or_else(|| style_prop(editing, "background-color"))
        .and_then(qml_hex_color_or_none);
    if let Some(expr) = build_conditional_color(
        base_bg.as_deref(),
        sel_bg.as_deref().zip(sel_pred.as_deref()),
        edit_bg.as_deref().zip(edit_pred.as_deref()),
    ) {
        rect_lines.push(format!("color: {expr}"));
    }

    // --- Inner text styling -------------------------------------------
    // Font cascades from the table's `sheet` part when the cell omits it.
    let mut ts = CellTextStyle {
        horizontal_alignment: style_prop(base, "text-align").and_then(qml_text_align),
        font_family_mono: style_prop(base, "font-family")
            .map(|v| v.trim() == "monospace")
            .unwrap_or(ctx.inherited.font_family_mono),
        font_pixel_size: style_prop(base, "font-size")
            .and_then(qml_px_or_none)
            .or_else(|| ctx.inherited.font_pixel_size.clone()),
        padding: style_prop(base, "padding").and_then(qml_px_or_none),
        color: None,
    };
    // Base text colour cascades from `sheet` when the cell omits it.
    let base_fg = style_prop(base, "color")
        .and_then(qml_hex_color_or_none)
        .or_else(|| ctx.inherited.color.clone());
    let sel_fg = style_prop(selected, "color").and_then(qml_hex_color_or_none);
    let edit_fg = style_prop(editing, "color").and_then(qml_hex_color_or_none);
    ts.color = build_conditional_color(
        base_fg.as_deref(),
        sel_fg.as_deref().zip(sel_pred.as_deref()),
        edit_fg.as_deref().zip(edit_pred.as_deref()),
    );

    StyledBox {
        rect_lines,
        text_style: ts,
    }
}

/// Build a QML colour expression from a base colour and optional
/// per-state overrides. Produces, in precedence order (editing > selected
/// > base):
///
///   `( <edit-pred> ) ? "#edit" : ( <sel-pred> ) ? "#sel" : "#base"`
///
/// Returns `None` when no colour information exists at all (so the caller
/// omits the property and the element keeps its QML default).
fn build_conditional_color(
    base: Option<&str>,
    selected: Option<(&str, &str)>, // (color, predicate)
    editing: Option<(&str, &str)>,  // (color, predicate)
) -> Option<String> {
    // Fallback when a state matches but there's no base colour: QML's
    // `Rectangle.color` default is white and `Text.color` default is
    // black, but for a conditional we need a concrete else-branch. Use
    // "transparent" for the background-less case so an unstyled cell
    // stays see-through; the inner-text path always has a base colour in
    // practice (the `sheet`/`cell` parts set one).
    let base_literal = base.map(|c| format!("\"{c}\""));

    match (selected, editing) {
        (None, None) => base_literal,
        _ => {
            let else_branch = base_literal.unwrap_or_else(|| "\"transparent\"".to_string());
            let mut expr = else_branch;
            // selected layer (lower precedence — applied first so editing
            // can wrap it).
            if let Some((c, pred)) = selected {
                expr = format!("( {pred} ) ? \"{c}\" : {expr}");
            }
            if let Some((c, pred)) = editing {
                expr = format!("( {pred} ) ? \"{c}\" : {expr}");
            }
            Some(expr)
        }
    }
}

fn part_style_props<'a>(node: &LayoutNode, ctx: &'a EmitCtx<'_>) -> Option<&'a [StyleProp]> {
    let part = node.part_name.as_deref()?;
    ctx.part_styles.get(part).map(|props| props.as_slice())
}

fn qml_layout_size_lines(props: &[StyleProp]) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(width) = style_prop(props, "width").and_then(qml_px_or_none) {
        lines.push(format!("implicitWidth: {width}"));
        lines.push(format!("Layout.preferredWidth: {width}"));
    }
    if let Some(height) = style_prop(props, "height").and_then(qml_px_or_none) {
        lines.push(format!("implicitHeight: {height}"));
        lines.push(format!("Layout.preferredHeight: {height}"));
    }
    if let Some(max_width) = style_prop(props, "max-width").and_then(qml_px_or_none) {
        lines.push(format!("Layout.maximumWidth: {max_width}"));
    }
    if let Some(max_height) = style_prop(props, "max-height").and_then(qml_px_or_none) {
        lines.push(format!("Layout.maximumHeight: {max_height}"));
    }
    if let Some(min_width) = style_prop(props, "min-width").and_then(qml_px_or_none) {
        lines.push(format!("Layout.minimumWidth: {min_width}"));
    }
    if let Some(min_height) = style_prop(props, "min-height").and_then(qml_px_or_none) {
        lines.push(format!("Layout.minimumHeight: {min_height}"));
    }
    lines
}

fn qml_layout_container_lines(props: &[StyleProp]) -> Vec<String> {
    let mut lines = qml_layout_size_lines(props);
    if let Some(gap) = style_prop(props, "gap").and_then(qml_px_or_none) {
        lines.push(format!("spacing: {gap}"));
    }
    lines
}

fn qml_rectangle_paint_lines(props: &[StyleProp]) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(background) = style_prop(props, "background")
        .or_else(|| style_prop(props, "background-color"))
        .and_then(qml_hex_color_or_none)
    {
        lines.push(format!("color: \"{background}\""));
    }
    if let Some(radius) = style_prop(props, "border-radius").and_then(qml_px_or_none) {
        lines.push(format!("radius: {radius}"));
    }
    if let Some(border_color) = style_prop(props, "border-color").and_then(qml_hex_color_or_none) {
        lines.push(format!("border.color: \"{border_color}\""));
    }
    if let Some(border_width) = style_prop(props, "border-width").and_then(qml_px_or_none) {
        lines.push(format!("border.width: {border_width}"));
    }
    lines
}

fn qml_padding(props: &[StyleProp]) -> Option<String> {
    style_prop(props, "padding")
        .or_else(|| style_prop(props, "padding-top"))
        .or_else(|| style_prop(props, "padding-bottom"))
        .and_then(qml_px_or_none)
}

fn needs_container_wrapper(props: &[StyleProp]) -> bool {
    qml_padding(props).is_some()
        || style_prop(props, "background")
            .or_else(|| style_prop(props, "background-color"))
            .and_then(qml_hex_color_or_none)
            .is_some()
        || style_prop(props, "border-radius")
            .and_then(qml_px_or_none)
            .is_some()
        || style_prop(props, "border-color")
            .and_then(qml_hex_color_or_none)
            .is_some()
        || style_prop(props, "border-width")
            .and_then(qml_px_or_none)
            .is_some()
}

fn qml_font_family(v: &str) -> Option<String> {
    let family = v.trim().trim_matches('"');
    if family.is_empty() {
        None
    } else {
        Some(escape_qml_string(family))
    }
}

fn qml_font_weight_is_bold(v: &str) -> Option<bool> {
    let weight = v.trim().trim_matches('"');
    match weight {
        "bold" | "bolder" => Some(true),
        "normal" | "lighter" => Some(false),
        _ => weight.parse::<u16>().ok().map(|n| n >= 600),
    }
}

fn qml_text_part_style_lines(props: &[StyleProp]) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(color) = style_prop(props, "color").and_then(qml_hex_color_or_none) {
        lines.push(format!("color: \"{color}\""));
    }
    if let Some(size) = style_prop(props, "font-size").and_then(qml_px_or_none) {
        lines.push(format!("font.pixelSize: {size}"));
    }
    if let Some(family) = style_prop(props, "font-family").and_then(qml_font_family) {
        lines.push(format!("font.family: \"{family}\""));
    }
    if let Some(is_bold) = style_prop(props, "font-weight").and_then(qml_font_weight_is_bold) {
        lines.push(format!("font.bold: {is_bold}"));
    }
    if let Some(align) = style_prop(props, "text-align").and_then(qml_text_align) {
        lines.push(format!("horizontalAlignment: {align}"));
    }
    lines
}

fn emit_styled_layout_container_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
    element_name: &'static str,
) -> Result<Option<String>, PipelineEmitError> {
    if node.tag != "Row" && node.tag != "Column" {
        return Ok(None);
    }
    let Some(props) = part_style_props(node, ctx) else {
        return Ok(None);
    };

    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let layout_lines = qml_layout_container_lines(props);
    if !needs_container_wrapper(props) {
        if layout_lines.is_empty() {
            return Ok(None);
        }

        let mut out = String::new();
        writeln!(out, "{pad}{element_name} {{").unwrap();
        for line in &layout_lines {
            writeln!(out, "{inner_pad}{line}").unwrap();
        }
        let is_stack = node.tag == "Stack";
        out.push_str(&emit_qml_children(
            &node.children,
            depth + 1,
            is_stack,
            ctx,
        )?);
        writeln!(out, "{pad}}}").unwrap();
        return Ok(Some(out));
    }

    let inset = qml_padding(props).unwrap_or_else(|| "0".to_string());
    let paint_lines = qml_rectangle_paint_lines(props);
    let mut out = String::new();
    writeln!(out, "{pad}Rectangle {{").unwrap();
    for line in qml_layout_size_lines(props) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }
    writeln!(
        out,
        "{inner_pad}implicitWidth: childrenRect.x + childrenRect.width + {inset}"
    )
    .unwrap();
    writeln!(
        out,
        "{inner_pad}implicitHeight: childrenRect.y + childrenRect.height + {inset}"
    )
    .unwrap();
    if paint_lines.iter().all(|line| !line.starts_with("color:")) {
        writeln!(out, "{inner_pad}color: \"transparent\"").unwrap();
    }
    for line in &paint_lines {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }
    writeln!(out, "{inner_pad}{element_name} {{").unwrap();
    writeln!(out, "{inner_pad}    x: {inset}").unwrap();
    writeln!(out, "{inner_pad}    y: {inset}").unwrap();
    for line in qml_layout_container_lines(props)
        .into_iter()
        .filter(|line| line.starts_with("spacing:"))
    {
        writeln!(out, "{inner_pad}    {line}").unwrap();
    }
    out.push_str(&emit_qml_children(
        &node.children,
        depth + 2,
        /* is_stack = */ false,
        ctx,
    )?);
    writeln!(out, "{inner_pad}}}").unwrap();
    writeln!(out, "{pad}}}").unwrap();
    Ok(Some(out))
}

/// Discover the camelCased column-widths slot from a `HostTable`'s
/// `HostTableColGroup > For (each: slot: <name>) { Col ... }` shape.
///
/// Mirrors the SwiftUI backend's `extract_table_context`. Returns `None`
/// when the structural match fails — cells then render without explicit
/// width threading (auto-sized), which is the pre-styling behaviour.
fn discover_col_widths_slot(host_table: &LayoutNode) -> Option<String> {
    for child in &host_table.children {
        if child.tag != "HostTableColGroup" {
            continue;
        }
        for cg_child in &child.children {
            if cg_child.tag != "For" {
                continue;
            }
            if !cg_child.children.iter().any(|n| n.tag == "Col") {
                continue;
            }
            if let Some(slot) = find_slot_ref_prop(cg_child, "each") {
                let camel = to_camel_case_first_lower(slot);
                if is_safe_identifier(&camel) {
                    return Some(camel);
                }
            }
        }
    }
    None
}

/// Compile a three-file Mosaic pipeline triple to a QML source file.
///
/// The `style` argument is inlined: styled `Box [part]` containers lower
/// to `Rectangle`s carrying the part's geometry, border, background, and
/// inner-text alignment/colour/font (see the mosstyle-inlining section
/// above). Unstyled boxes keep the bare `Item` shape.
pub fn from_pipeline(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
) -> Result<PipelineEmitResult, PipelineEmitError> {
    // 1. The three IRs must agree on the component name. The style IR's
    // `component_name` is allowed to differ when the style targets a
    // specific layout variant (UI23 §4); we therefore only validate
    // interface vs. layout here.
    if interface.component != layout.component_name {
        return Err(PipelineEmitError::ComponentNameMismatch {
            mosmodel: interface.component.clone(),
            moslayout: layout.component_name.clone(),
        });
    }

    let name = &interface.component;
    let mut out = String::new();

    // 2. File header — banner + imports. Both QtQuick 2.15 and
    // QtQuick.Layouts 1.15 ship with Qt 6 by default and are version-
    // pinned so the generated file is reproducible.
    //
    // `QtQuick.Controls 2.15` is added conditionally: only when the
    // layout tree references a primitive that lowers to a Controls
    // element (today: `HostButton` → `Button`, `HostScroll` →
    // `ScrollView`). Importing Controls unconditionally is harmless at
    // runtime but adds a noticeable startup cost on resource-constrained
    // platforms, so we keep the import set minimal.
    writeln!(out, "// Auto-generated by mosaic-emit-qt. Do not edit.").unwrap();
    writeln!(out, "import QtQuick 2.15").unwrap();
    writeln!(out, "import QtQuick.Layouts 1.15").unwrap();
    if tree_needs_controls_import(&layout.root) {
        writeln!(out, "import QtQuick.Controls 2.15").unwrap();
    }
    writeln!(out).unwrap();

    // 3. Open the root `Item`. See the module-level doc for why the root
    // wrapper is always `Item`.
    writeln!(out, "Item {{").unwrap();
    writeln!(out, "    id: mosaicRoot").unwrap();
    writeln!(out, "    // Component: {name}").unwrap();
    writeln!(out, "    property var mosaicHost: null").unwrap();
    writeln!(out, "    property var lastHostIntent: null").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    function applyMosaicProps(props) {{").unwrap();
    writeln!(
        out,
        "        if (props === null || props === undefined) {{ return; }}"
    )
    .unwrap();
    writeln!(out, "        for (var key in props) {{").unwrap();
    writeln!(out, "            mosaicRoot[key] = props[key];").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    function applyMosaicResponse(response) {{").unwrap();
    writeln!(
        out,
        "        if (response === null || response === undefined) {{ return; }}"
    )
    .unwrap();
    writeln!(out, "        if (response.hostIntent !== undefined) {{").unwrap();
    writeln!(out, "            lastHostIntent = response.hostIntent;").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "        if (response.props !== undefined) {{").unwrap();
    writeln!(out, "            applyMosaicProps(response.props);").unwrap();
    writeln!(out, "            return;").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "        applyMosaicProps(response);").unwrap();
    writeln!(out, "    }}").unwrap();

    // 4. Properties — one `property` declaration per slot.
    if !interface.slots.is_empty() {
        writeln!(out).unwrap();
    }
    for slot in &interface.slots {
        out.push_str(&emit_slot_property(slot)?);
    }

    // 5. Signals — one `signal` declaration per emit.
    if !interface.emits.is_empty() {
        writeln!(out).unwrap();
    }
    for e in &interface.emits {
        out.push_str(&emit_signal_declaration(e)?);
    }
    if !interface.emits.is_empty() {
        writeln!(out, "    signal mosaicEvent(var event)").unwrap();
        writeln!(
            out,
            "    onMosaicEvent: function(event) {{ applyMosaicResponse(mosaicHost ? mosaicHost.handleEvent(event) : null) }}"
        )
        .unwrap();
        for e in &interface.emits {
            out.push_str(&emit_mosaic_event_handler(e)?);
        }
    }

    // 6. The layout tree. Build the part-style map up front and seed the
    // walker's context so styled `Box [part]` containers can inline their
    // mosstyle properties (geometry, border, background, text styling).
    let part_styles = build_part_style_map(style);
    let ctx = EmitCtx {
        emits: &interface.emits,
        part_styles: &part_styles,
        col_widths_slot: None,
        enclosing_index: None,
        enclosing_item: None,
        text_style: None,
        inherited: InheritedStyle::default(),
        cell_fill_children: false,
    };
    writeln!(out).unwrap();
    out.push_str(&emit_qml_tree(&layout.root, 1, &ctx)?);

    // 7. Close the root `Item`.
    writeln!(out, "}}").unwrap();

    Ok(PipelineEmitResult {
        output: out,
        component_name: name.clone(),
    })
}

// =====================================================================
// Property + signal section emitters
// =====================================================================

/// Emit one QML `property` declaration for a mosmodel slot.
///
/// Slot names are kebab-case in source; we convert them to camelCase for QML
/// (the convention is established in `mosaic-emit-react`, and matches QML's
/// own JavaScript-property style — `Layout.fillWidth`, `font.pixelSize`,
/// etc.).
fn emit_slot_property(slot: &SlotDecl) -> Result<String, PipelineEmitError> {
    let camel = to_camel_case_first_lower(&slot.name);
    validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
    let (ty, default) = slot_type_to_qml(&slot.r#type);
    let mut out = String::new();
    writeln!(out, "    property {ty} {camel}: {default}").unwrap();
    Ok(out)
}

/// Emit one QML `signal` declaration for a mosmodel emit.
///
/// QML signals follow the same `on`-prefix stripping rule as React (see
/// UI24 §5 and the React backend's `pipeline.rs`). `onAvatarClick` lowers
/// to `signal avatarClick()`; QML host code connects via the standard
/// `onAvatarClick: { ... }` handler attribute (the leading `on` is
/// re-added automatically by Qt's name-mangling convention).
///
/// Parameterless emits produce `signal foo()`. Emits with a payload
/// produce one typed parameter per `EmitParam` — kebab-case → camelCase
/// — with the type chosen from the small `EmitPayloadType` enum.
fn emit_signal_declaration(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    let lowered = strip_on_prefix(&emit.name);
    let signal_name = to_camel_case_first_lower(&lowered);
    validate_safe_identifier(&signal_name).map_err(PipelineEmitError::UnsafeEmitName)?;

    let params = if emit.params.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = emit
            .params
            .iter()
            .map(|p| {
                let field = to_camel_case_first_lower(&p.name);
                let ty = emit_payload_to_qml(&p.r#type);
                format!("{ty} {field}")
            })
            .collect();
        parts.join(", ")
    };

    let mut out = String::new();
    writeln!(out, "    signal {signal_name}({params})").unwrap();
    Ok(out)
}

fn emit_mosaic_event_handler(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    let lowered = strip_on_prefix(&emit.name);
    let signal_name = to_camel_case_first_lower(&lowered);
    validate_safe_identifier(&signal_name).map_err(PipelineEmitError::UnsafeEmitName)?;
    let handler_name = qml_signal_handler_name(&signal_name);

    let mut fields = vec![format!("\"event\": \"{}\"", escape_qml_string(&emit.name))];
    let mut params = Vec::with_capacity(emit.params.len());
    for param in &emit.params {
        let field = to_camel_case_first_lower(&param.name);
        validate_safe_identifier(&field).map_err(PipelineEmitError::UnsafeEmitName)?;
        params.push(field.clone());
        fields.push(format!("\"{field}\": {field}"));
    }

    let mut out = String::new();
    writeln!(
        out,
        "    {handler_name}: function({}) {{ mosaicEvent({{ {} }}) }}",
        params.join(", "),
        fields.join(", ")
    )
    .unwrap();
    Ok(out)
}

fn qml_signal_handler_name(signal_name: &str) -> String {
    let mut chars = signal_name.chars();
    let Some(first) = chars.next() else {
        return "on".to_string();
    };
    let mut out = String::from("on");
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

// =====================================================================
// Layout tree walker
// =====================================================================

/// Walk a `LayoutNode` and produce the corresponding indented QML source.
///
/// Indentation is in units of 4 spaces per level (Qt Creator's default).
/// The `depth` argument is the *block depth from the root Item* — so the
/// outermost layout element starts at depth 1 (one level inside the root
/// wrapper).
fn emit_qml_tree(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    // The UI29 *host* primitives (`HostInput`, `HostButton`) need
    // attribute lowering that depends on the moslayout props — slot refs
    // for `value`, emit refs for `onCommit`, etc. — none of which the
    // structural walker below can express. Those primitives get their
    // own emitter functions, mirroring the React backend's
    // `emit_input_jsx` carve-out for `Input`.
    match node.tag.as_str() {
        "HostInput" => return emit_host_input_qml(node, depth, ctx),
        "HostButton" => return emit_host_button_qml(node, depth, ctx),
        "HostDialog" => return emit_host_dialog_qml(node, depth, ctx),
        "HostSurface" => return emit_host_surface_qml(node, depth, ctx),

        // UI29-2 kernel — `HostCheckbox` and `HostRadio` lower to
        // QtQuick.Controls 2 `CheckBox` and `RadioButton`. Both
        // primitives carry full native a11y role, focus ring, and
        // keyboard semantics (Space toggles, arrow keys navigate radio
        // groups when wrapped in ButtonGroup) that composing from
        // QtQuick basics couldn't replicate.
        "HostCheckbox" => return emit_host_checkbox_qml(node, depth, ctx),
        "HostRadio" => return emit_host_radio_qml(node, depth, ctx),

        // UI29-4 kernel — `HostLink` lowers to a rich-text `Text`
        // with `onLinkActivated` (Qt has no first-class hyperlink
        // widget; `Text { textFormat: Text.RichText; text: "<a
        // href='...'>label</a>" }` is the idiomatic shape).
        // `HostTooltip` lowers to the `ToolTip.text` attached
        // property on any child. `HostNumberInput` lowers to
        // QtQuick.Controls 2 `TextField` plus `DoubleValidator`.
        "HostLink" => return emit_host_link_qml(node, depth, ctx),
        "HostTooltip" => return emit_host_tooltip_qml(node, depth, ctx),
        "HostNumberInput" => return emit_host_number_input_qml(node, depth, ctx),

        "HostTable" => return emit_host_table_qml(node, depth, ctx),
        // UI29 §3.1 — `For` meta-primitive: lower to a `Repeater` with an
        // `Item` delegate that re-exports `modelData` / `index` under the
        // author's chosen names.
        "For" => return emit_for_qml(node, depth, ctx),
        // UI29 §3.2 — `If` meta-primitive. When `If` appears as a *root*
        // node (no preceding sibling to pair with `Else`), it is emitted
        // as a single-Loader conditional with no else branch. The
        // `If`+`Else` sibling pairing happens in `emit_qml_children`
        // when walking a parent's children list.
        "If" => return emit_if_qml(node, None, depth, ctx),
        // UI29 §3.2 — a top-level `Else` (no preceding `If`) has no
        // semantic home. Emit a self-documenting QML comment rather
        // than erroring. Defensive: the grammar should prevent this,
        // but the emitter must not crash on malformed trees.
        "Else" => {
            let pad = "    ".repeat(depth);
            return Ok(format!("{pad}// orphan Else (no preceding If)\n"));
        }
        // Orphan sub-tags: `HostTableHead`/`HostTableBody`/`HostTableFoot`/
        // `HostTableColGroup` outside a `HostTable` parent have no semantic
        // home in QML. Emit a self-documenting comment rather than erroring
        // — keeps the emitter resilient to malformed input.
        "HostTableHead" | "HostTableBody" | "HostTableFoot" | "HostTableColGroup" => {
            let pad = "    ".repeat(depth);
            return Ok(format!("{pad}// orphan {} (outside HostTable)\n", node.tag));
        }
        _ => {}
    }

    let pad = "    ".repeat(depth);

    // ------------------------------------------------------------------
    // Styled cell `Box` path (mosstyle inlining).
    //
    // A `Box` carrying a `part_name` that the `.msl` styles lowers to a
    // `Rectangle` (not a bare `Item`) so the cell has a real size, a
    // border, a fill, and aligned text. The resolved [`CellTextStyle`] is
    // threaded into the children context so the cell's value — which
    // lives several levels deep inside an `If`/`Else` `Loader` — picks up
    // the alignment / colour / font when it renders.
    if node.tag == "Box" {
        if let Some(part) = &node.part_name {
            let has_base = ctx.part_styles.contains_key(part);
            let has_state = ctx.part_styles.contains_key(&format!("{part}:selected"))
                || ctx.part_styles.contains_key(&format!("{part}:editing"));
            // A width thread (column-widths slot + enclosing index) forces
            // the Rectangle even when the part itself carries no props, so
            // the fixed column width still lands.
            let has_width_thread = ctx.col_widths_slot.is_some() && ctx.enclosing_index.is_some();
            if has_base || has_state || has_width_thread {
                let styled = lower_styled_box(node, part, ctx);
                let mut out = String::new();
                writeln!(out, "{pad}Rectangle {{").unwrap();
                for line in &styled.rect_lines {
                    writeln!(out, "{pad}    {line}").unwrap();
                }
                // Descend with the cell's text style in scope; the
                // enclosing index is cleared so a nested table doesn't
                // inherit this cell's width binding. `cell_fill_children`
                // makes the cell's direct body (If/Else Loaders or a plain
                // Text) fill the fixed-size Rectangle.
                let child_ctx = EmitCtx {
                    enclosing_index: None,
                    cell_fill_children: true,
                    ..ctx.with_text_style(if styled.text_style.is_empty() {
                        None
                    } else {
                        Some(styled.text_style)
                    })
                };
                out.push_str(&emit_qml_children(
                    &node.children,
                    depth + 1,
                    /* is_stack = */ false,
                    &child_ctx,
                )?);
                writeln!(out, "{pad}}}").unwrap();
                return Ok(out);
            }
        }
    }

    // Decompose the primitive into its QML element name and any built-in
    // properties (the small chunks of QML that always go on this element
    // regardless of the moslayout props — e.g. Spacer's `Layout.fillWidth`).
    let QmlElement {
        element_name,
        builtin_lines,
        is_text,
        is_image,
    } = primitive_to_qml(&node.tag)?;
    if let Some(styled_container) =
        emit_styled_layout_container_qml(node, depth, ctx, element_name)?
    {
        return Ok(styled_container);
    }

    // `Stack` (UI29 §3): a Z-axis container. QML has no direct ZStack
    // primitive; the idiomatic shape is an `Item` with each child
    // setting `anchors.fill: parent` so it occupies the full overlay
    // area. We carry an `is_stack` flag into the per-child loop so the
    // children get the anchor line *inside their own block*. (We could
    // alternatively set it from the parent side via `data:`, but the
    // child-side anchor is the canonical QML pattern.)
    let is_stack = node.tag == "Stack";

    let mut out = String::new();

    // Self-closing leaf with no children and no special handling — emit a
    // one-line `Foo { }`. Currently no primitive triggers this; every leaf
    // has either built-in lines, a `text`/`source` attribute, or children.
    // The branch exists for safety / future use.

    writeln!(out, "{pad}{element_name} {{").unwrap();

    // Built-in property lines (e.g. Spacer's `Layout.fillWidth: true`).
    for line in &builtin_lines {
        writeln!(out, "{pad}    {line}").unwrap();
    }

    // Text primitive: emit a `text: ...` line. Two sources:
    //  - `content: "literal"`  → `text: "literal"`
    //  - `content: slot: name` → `text: name`  (QML binds the bare
    //    identifier to the matching `property` declaration on the root
    //    `Item`, which is in scope at every depth inside the file).
    if is_text {
        // A `Text` rendered inside a styled cell fills the cell rectangle
        // and adopts the cell's alignment / colour / font / padding (see
        // [`CellTextStyle`]). Outside a styled cell, `ctx.text_style` is
        // `None` and the Text emits exactly as before.
        if let Some(ts) = &ctx.text_style {
            for line in cell_text_style_lines(ts) {
                writeln!(out, "{pad}    {line}").unwrap();
            }
        }
        if let Some(props) = part_style_props(node, ctx) {
            for line in qml_text_part_style_lines(props) {
                writeln!(out, "{pad}    {line}").unwrap();
            }
        }
        if let Some(line) = build_text_attribute(node) {
            writeln!(out, "{pad}    {line}").unwrap();
        }
    }

    // Image primitive: emit a `source: ...` line.
    if is_image {
        if let Some(line) = build_image_source_attribute(node) {
            writeln!(out, "{pad}    {line}").unwrap();
        }
    }

    // Children — recurse, indented one level deeper.
    //
    // If this node is a `Stack`, each child must occupy the full parent
    // overlay area. We can't reach inside the child's generated block
    // from here without re-parsing, so we insert the anchor line as a
    // sibling using QML's `anchors.fill` *binding from the parent* —
    // but that doesn't exist; the property lives on the child. The
    // workable approach is to recurse normally and then post-process
    // each child block, inserting `anchors.fill: parent` immediately
    // after the child's opening `{`. This is mechanical string editing
    // on already-trusted output (we generated it), so it's safe.
    //
    // Children are walked through `emit_qml_children` rather than a
    // bare `for` so that `If`+`Else` sibling pairs are recognised
    // (UI29 §3.2).
    out.push_str(&emit_qml_children(
        &node.children,
        depth + 1,
        is_stack,
        ctx,
    )?);

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Build the QML property lines for a cell's inner `Text` (or
/// `TextInput`) from a [`CellTextStyle`]. The Text fills the cell
/// rectangle (`anchors.fill: parent`) and vertically centres so the
/// value sits in the middle of the row; horizontal alignment, colour,
/// font, and padding come from the part's `.msl` props.
fn cell_text_style_lines(ts: &CellTextStyle) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("anchors.fill: parent".to_string());
    if let Some(p) = &ts.padding {
        // Inset the content on all sides (padding), then let the
        // horizontal alignment push the text to the requested edge.
        lines.push(format!("anchors.margins: {p}"));
    }
    lines.push("verticalAlignment: Text.AlignVCenter".to_string());
    if let Some(a) = ts.horizontal_alignment {
        lines.push(format!("horizontalAlignment: {a}"));
    }
    if let Some(c) = &ts.color {
        lines.push(format!("color: {c}"));
    }
    if ts.font_family_mono {
        lines.push("font.family: \"monospace\"".to_string());
    }
    if let Some(sz) = &ts.font_pixel_size {
        lines.push(format!("font.pixelSize: {sz}"));
    }
    lines
}

/// Walk an ordered list of sibling layout nodes, emitting each one's
/// QML. Performs two extra things on top of a naive map-over-children:
///
/// 1. **`If`+`Else` pairing (UI29 §3.2).** When an `If` is followed
///    *immediately* by an `Else` sibling, the two are consumed
///    together and emitted as a paired `Loader { active: cond } /
///    Loader { active: !cond }` block. A bare `If` (next sibling is
///    not `Else`, or it's the last child) lowers to a single Loader.
///    A bare `Else` (no preceding `If` in the consumed-pair sense)
///    falls through to `emit_qml_tree`, which emits a `// orphan
///    Else …` comment.
/// 2. **`Stack` z-overlay injection.** When `is_stack` is `true`, each
///    emitted child block is post-processed to insert
///    `anchors.fill: parent` after the opening brace.
fn emit_qml_children(
    children: &[LayoutNode],
    depth: usize,
    is_stack: bool,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        // UI29 §3.2 — `If`+`Else` sibling pairing. We *only* pair when
        // the immediately-following sibling is `Else`; if the author
        // wrote `If { } Text { } Else { }`, the `Else` will be reached
        // standalone and emit a comment via the orphan-Else path in
        // `emit_qml_tree`.
        let child_qml = if child.tag == "If" {
            let else_sibling = children.get(i + 1).filter(|n| n.tag == "Else");
            if else_sibling.is_some() {
                i += 1; // consume the Else along with the If
            }
            emit_if_qml(child, else_sibling, depth, ctx)?
        } else {
            emit_qml_tree(child, depth, ctx)?
        };

        if is_stack {
            out.push_str(&inject_anchors_fill_parent(&child_qml, depth));
        } else {
            out.push_str(&child_qml);
        }
        i += 1;
    }
    Ok(out)
}

/// The element decomposition for one moslayout primitive.
struct QmlElement {
    /// The QML element name to open and close (`Item`, `RowLayout`, …).
    element_name: &'static str,
    /// Always-present property lines (e.g. `Layout.fillWidth: true`).
    builtin_lines: Vec<&'static str>,
    /// True iff this element supports a `text: ...` attribute filled from
    /// the `content` moslayout prop.
    is_text: bool,
    /// True iff this element supports a `source: ...` attribute filled
    /// from the `source` moslayout prop.
    is_image: bool,
}

/// Map a moslayout primitive tag to its QML element decomposition.
///
/// See the primitive lowering table at the top of this module for the
/// full mapping. Unknown primitives are rejected — letting them through
/// as a default `Item { }` would silently lose layout semantics.
fn primitive_to_qml(tag: &str) -> Result<QmlElement, PipelineEmitError> {
    Ok(match tag {
        "Box" => QmlElement {
            element_name: "Item",
            builtin_lines: vec![],
            is_text: false,
            is_image: false,
        },
        "Row" => QmlElement {
            element_name: "RowLayout",
            builtin_lines: vec![],
            is_text: false,
            is_image: false,
        },
        "Column" => QmlElement {
            // NOTE (UI28 §2.2): UI28 introduces a *data* `Column` primitive
            // (column metadata for the new Grid). Today the only meaning of
            // the `Column` tag inside moslayout is the layout container,
            // so we lower it to `ColumnLayout` unconditionally. When UI28
            // lands, this branch will need to distinguish the two — most
            // likely by context: a `Column` inside a `Grid` is the data
            // primitive; everywhere else it is the layout container.
            element_name: "ColumnLayout",
            builtin_lines: vec![],
            is_text: false,
            is_image: false,
        },
        "Text" => QmlElement {
            element_name: "Text",
            builtin_lines: vec![],
            is_text: true,
            is_image: false,
        },
        "Spacer" => QmlElement {
            element_name: "Item",
            // Inside a RowLayout/ColumnLayout, a `Spacer` takes whatever
            // axis is the layout's growth axis. Setting both fillWidth and
            // fillHeight is the conservative choice — the layout uses only
            // the relevant one and ignores the other.
            builtin_lines: vec!["Layout.fillWidth: true", "Layout.fillHeight: true"],
            is_text: false,
            is_image: false,
        },
        "Image" => QmlElement {
            element_name: "Image",
            builtin_lines: vec![],
            is_text: false,
            is_image: true,
        },
        "Divider" => QmlElement {
            element_name: "Rectangle",
            builtin_lines: vec![
                "height: 1",
                "color: \"#888\"",
                "Layout.fillWidth: true",
            ],
            is_text: false,
            is_image: false,
        },
        // UI29 §3 `Stack`: a Z-axis container. QML's idiomatic shape is
        // `Item { ... }` with each child setting `anchors.fill: parent`.
        // The anchoring is applied per-child by the tree walker (see
        // `emit_qml_tree`); the element itself is just an `Item`.
        //
        // Note: QtQuick has a `StackLayout` element from
        // `QtQuick.Layouts`, but its semantics are "show one child at a
        // time, switch with `currentIndex`" — that is *not* a Z-stack
        // overlay. The UI29 spec maps `Stack` to a true overlay (see
        // §4 row "Stack — z-axis / absolute container") so we use
        // `Item` + `anchors.fill: parent` on children.
        "Stack" => QmlElement {
            element_name: "Item",
            builtin_lines: vec![],
            is_text: false,
            is_image: false,
        },
        // UI29 §3 `HostScroll`: a scrollable viewport. QML's
        // `ScrollView` element from `QtQuick.Controls` accepts arbitrary
        // children directly and wraps them in a scrollable area, which
        // is exactly the semantics UI29 §4 specifies. We use it instead
        // of the lower-level `Flickable` because the wrapper is simpler
        // and the spec table calls it out as the preferred mapping.
        //
        // The `QtQuick.Controls 2.15` import is added conditionally at
        // the top of the file (see `tree_needs_controls_import`).
        "HostScroll" => QmlElement {
            element_name: "ScrollView",
            builtin_lines: vec![],
            is_text: false,
            is_image: false,
        },
        // `HostInput`, `HostButton`, `HostDialog`, `HostCheckbox`, and
        // `HostRadio` are handled by
        // their own emitters earlier in `emit_qml_tree`; reaching this
        // branch would be an internal logic error.
        "HostInput" | "HostButton" | "HostDialog" | "HostCheckbox" | "HostRadio"
        | "HostLink" | "HostTooltip" | "HostNumberInput" => unreachable!(
            "HostInput/HostButton/HostDialog/HostCheckbox/HostRadio/HostLink/HostTooltip/HostNumberInput are handled by dedicated emitters; should not reach primitive_to_qml"
        ),
        other => return Err(PipelineEmitError::UnknownPrimitive(other.to_string())),
    })
}

/// Build the `text: ...` attribute for a Text primitive, if a `content`
/// prop is present.
///
/// - `content: "literal"`         → `Some("text: \"literal\"")`.
/// - `content: slot: my-name`     → `Some("text: myName")`.
/// - No `content` prop            → `None`.
fn build_text_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "content")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => format!("text: \"{}\"", escape_qml_string(s)),
        LayoutPropValue::SlotRef(s) => {
            // QML property bindings reference the property by its bare
            // (camelCase) name. `is_safe_identifier` guards against
            // unexpected token shapes.
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("text: {camel}")
            } else {
                "text: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => format!("text: \"{k}\""),
        LayoutPropValue::Number(n) => format!("text: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "text: \"\"".to_string(),
        // U29-G3 expression + UI29 §3.4 scope (PR #4398) — the Expr text
        // is the literal QML expression to evaluate in the surrounding
        // Repeater delegate's scope (where For-loop bindings live as
        // delegate `property var` declarations). mosaic-pkg-grid v0.2.0's
        // `Text ( content: ( v ) )` becomes `text: v` (where `v` is the
        // inner Repeater's `property var v: modelData`); the host gets the
        // live cell text per body cell instead of the empty placeholder
        // this branch used to emit before §3.4 made scope rules explicit.
        LayoutPropValue::Expr(text) => format!("text: {text}"),
    })
}

/// Build the `source: ...` attribute for an Image primitive, if a `source`
/// prop is present.
fn build_image_source_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "source")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => format!("source: \"{}\"", escape_qml_string(s)),
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("source: {camel}")
            } else {
                "source: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => format!("source: \"{k}\""),
        LayoutPropValue::Number(n) => format!("source: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "source: \"\"".to_string(),
        // Same §3.4 unlock as `build_text_attribute` — the Expr text is
        // verbatim QML expression text.
        LayoutPropValue::Expr(text) => format!("source: {text}"),
    })
}

// =====================================================================
// UI29 host primitive emitters
// =====================================================================

/// Inject `anchors.fill: parent` into the first child block of a Stack.
///
/// The walker hands us a child's already-formatted QML block — for
/// example:
///
/// ```text
///         Text {
///             text: "hi"
///         }
/// ```
///
/// For a Z-stack child, we need the child to fill the parent overlay,
/// so we insert an `anchors.fill: parent` line directly after the
/// opening `{`. The injection is mechanical text editing on output we
/// produced ourselves — safe by construction. (We always emit the
/// opening brace on its own line, followed by a newline.)
fn inject_anchors_fill_parent(child_qml: &str, depth: usize) -> String {
    let inner_pad = "    ".repeat(depth + 1);
    // Find the first `{\n` — that closes the opening line of the child
    // block — and insert our line right after it.
    if let Some(brace_idx) = child_qml.find("{\n") {
        let insert_at = brace_idx + 2; // after `{\n`
        let mut out = String::with_capacity(child_qml.len() + 64);
        out.push_str(&child_qml[..insert_at]);
        out.push_str(&inner_pad);
        out.push_str("anchors.fill: parent\n");
        out.push_str(&child_qml[insert_at..]);
        out
    } else {
        // The walker always emits `{\n`; this branch exists for
        // defence in depth.
        child_qml.to_string()
    }
}

/// Pick the QML argument list for invoking the named signal.
///
/// QML signals are arity-strict: passing the wrong number of args
/// is a runtime error at the call site, not a silent ignore.  When
/// the .mil declares `emit onFormulaChange ( value : text )`, the
/// QML signal is declared `signal formulaChange(string value)` and
/// must be invoked as `formulaChange(text)`.  When the .mil declares
/// `emit onCommit ;` (no payload), the QML signal is `signal commit()`
/// and must be invoked as `commit()`.
///
/// Returns `""` for zero-arity signals, `"text"` for any signal with
/// one or more declared params (the conventional payload for
/// `HostInput`-style sources — the contents of the text field).
///
/// If the signal name isn't found in `emits` (defensive — should not
/// happen for well-formed input), falls back to `""` so QML at least
/// won't error on extra args.
fn pick_signal_arg(emit_name: &str, emits: &[EmitDecl]) -> &'static str {
    pick_signal_arg_with(emit_name, emits, "text")
}

/// Generic variant: pick the QML invocation argument list, supplying
/// a custom payload token to use when the signal declares ≥1 params.
///
/// Used by emitters where the natural payload is something other
/// than `text` — e.g. `checked` for `Checkbox.toggled(bool)`, or
/// `value` for a custom numeric control.  Returns `""` for the
/// parameterless case so a zero-arity signal is invoked with no
/// args (the bug that prompted this helper in the first place).
fn pick_signal_arg_with(
    emit_name: &str,
    emits: &[EmitDecl],
    payload_token: &'static str,
) -> &'static str {
    let arity = emits
        .iter()
        .find(|e| e.name == emit_name)
        .map(|e| e.params.len())
        .unwrap_or(0);
    if arity == 0 {
        ""
    } else {
        payload_token
    }
}

/// Lower a `HostInput` node to a QML `TextInput { ... }` block.
///
/// ## Property handling
///
/// | moslayout prop          | QML output                                                  |
/// |---|---|
/// | `value: slot: x`        | `text: x` (bare identifier — QML binds to the property)     |
/// | `value: "literal"`      | `text: "literal"` (escaped)                                 |
/// | `read-only: slot: x`    | `readOnly: x`                                               |
/// | `read-only: true/false` | `readOnly: true/false`                                      |
/// | `placeholder: "..."`    | `TextField { placeholderText: "..." }`                     |
/// | `placeholder: slot: x`  | `TextField { placeholderText: x }`                         |
/// | `onChange: emit: onE`   | `onTextChanged: e(<arg>)`  — `arg` is `text` when the signal      |
/// |                         | declares one parameter, empty when it declares zero (see          |
/// |                         | `pick_signal_arg`).                                               |
/// | `onCommit: emit: onE`   | `onAccepted: e(<arg>)` — fires on Enter, same arg rule            |
/// | `onCancel: emit: onE`   | `Keys.onEscapePressed: { e(<arg>); event.accepted = true }`       |
///
/// ## Signal arity matters
///
/// QML enforces signal arity strictly — invoking a parameterless
/// `signal commit()` with `commit(text)` errors with
/// "too many arguments", and a `signal formulaChange(string value)`
/// invoked as `formulaChange()` errors with "Insufficient arguments".
/// The emitter therefore checks the interface's `EmitDecl` for the
/// referenced signal and inserts `text` only when the signal has
/// exactly one declared parameter (the conventional shape — a single
/// payload value: the new text).  Zero-param signals are invoked with
/// no args; multi-param signals are also invoked with `text` for the
/// first slot (callers can wire the rest), which is the v0.1.0
/// compromise — full multi-arg lowering is a follow-up.
///
/// The Enter / Escape mapping mirrors UI25 §10 (the React backend
/// merges both into a single `onKeyDown` handler; QML has dedicated
/// signal handlers for both, so we use them directly).
fn emit_host_input_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();
    let placeholder_line = build_placeholder_text_attribute(node);
    let control_tag = if placeholder_line.is_some() {
        "TextField"
    } else {
        "TextInput"
    };
    writeln!(out, "{pad}{control_tag} {{").unwrap();

    if let Some(part) = node.part_name.as_deref() {
        writeln!(
            out,
            "{inner_pad}objectName: \"{}\"",
            escape_qml_string(part)
        )
        .unwrap();
    }

    // When the input is a styled cell's editor (it sits inside a styled
    // `Box [cell]` whose text style is in scope), fill the cell and adopt
    // its alignment / colour / font so the in-place editor matches the
    // surrounding cells. `TextInput` honours the same `Text.Align*`
    // enums and `font.*` / `color` properties as `Text`.
    if let Some(ts) = &ctx.text_style {
        for line in cell_text_style_lines(ts) {
            writeln!(out, "{inner_pad}{line}").unwrap();
        }
    }

    // text: <slot or literal>
    if let Some(line) = build_value_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // readOnly: <slot or literal>
    if let Some(line) = build_read_only_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // placeholderText is available when HostInput lowers to TextField.
    if let Some(line) = placeholder_line {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // onTextChanged: e(<arg>)
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arg = pick_signal_arg(emit_name, ctx.emits);
        writeln!(out, "{inner_pad}onTextChanged: {camel}({arg})").unwrap();
    }

    // onAccepted: e(<arg>)
    if let Some(emit_name) = find_emit_ref_prop(node, "onCommit") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arg = pick_signal_arg(emit_name, ctx.emits);
        writeln!(out, "{inner_pad}onAccepted: {camel}({arg})").unwrap();
    }

    // Keys.onEscapePressed: { e(<arg>); event.accepted = true }
    if let Some(emit_name) = find_emit_ref_prop(node, "onCancel") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arg = pick_signal_arg(emit_name, ctx.emits);
        writeln!(
            out,
            "{inner_pad}Keys.onEscapePressed: {{ {camel}({arg}); event.accepted = true }}"
        )
        .unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Lower a styled Mosaic button part into QML `Button` property lines.
///
/// This mirrors the conservative native-button subset used by the other
/// backends: padding, foreground colour, background, border, and radius.
/// Values pass through the same hex/pixel validators used by styled cells.
fn host_button_style_qml_lines(node: &LayoutNode, ctx: &EmitCtx) -> Vec<String> {
    let Some(part) = node.part_name.as_deref() else {
        return Vec::new();
    };
    let Some(base) = ctx.part_styles.get(part) else {
        return Vec::new();
    };

    let mut lines = Vec::new();
    if let Some(padding) = style_prop(base, "padding").and_then(qml_px_or_none) {
        lines.push(format!("leftPadding: {padding}"));
        lines.push(format!("rightPadding: {padding}"));
        lines.push(format!("topPadding: {padding}"));
        lines.push(format!("bottomPadding: {padding}"));
    }
    if let Some(foreground) = style_prop(base, "color").and_then(qml_hex_color_or_none) {
        lines.push(format!("palette.buttonText: \"{foreground}\""));
    }
    if let Some(is_bold) = style_prop(base, "font-weight").and_then(qml_font_weight_is_bold) {
        lines.push(format!("font.bold: {is_bold}"));
    }

    let background = style_prop(base, "background")
        .or_else(|| style_prop(base, "background-color"))
        .and_then(qml_hex_color_or_none);
    let border_color = style_prop(base, "border-color").and_then(qml_hex_color_or_none);
    let border_width = style_prop(base, "border-width").and_then(qml_px_or_none);
    let radius = style_prop(base, "border-radius").and_then(qml_px_or_none);
    if background.is_some() || border_color.is_some() || border_width.is_some() || radius.is_some()
    {
        lines.push("background: Rectangle {".to_string());
        if let Some(background) = background {
            lines.push(format!("    color: \"{background}\""));
        } else {
            lines.push("    color: \"transparent\"".to_string());
        }
        if let Some(radius) = radius {
            lines.push(format!("    radius: {radius}"));
        }
        if let Some(border_color) = border_color {
            lines.push(format!("    border.color: \"{border_color}\""));
        }
        if let Some(border_width) = border_width {
            lines.push(format!("    border.width: {border_width}"));
        }
        lines.push("}".to_string());
    }

    lines
}

/// Lower a `HostButton` node to a QML `Button { ... }` block.
///
/// `Button` lives in `QtQuick.Controls 2.15`, which we import only when
/// the layout tree references a primitive that needs it (see
/// `tree_needs_controls_import`).
///
/// ## Property handling
///
/// | moslayout prop          | QML output                                |
/// |---|---|
/// | `label: slot: x`        | `text: x` (bare identifier)               |
/// | `label: "literal"`      | `text: "literal"` (escaped)               |
/// | `disabled: slot: x`     | `enabled: !x`                             |
/// | `disabled: true/false`  | `enabled: !true` / `enabled: !false`      |
/// | `onTap: emit: onE`      | `onClicked: e()`                          |
fn emit_host_button_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();
    writeln!(out, "{pad}Button {{").unwrap();

    if let Some(part) = node.part_name.as_deref() {
        writeln!(
            out,
            "{inner_pad}objectName: \"{}\"",
            escape_qml_string(part)
        )
        .unwrap();
    }

    // text: <slot or literal> — sourced from the `label` prop.
    if let Some(line) = build_label_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // enabled: !<slot or literal> — inverted from `disabled`.
    if let Some(line) = build_disabled_to_enabled_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    for line in host_button_style_qml_lines(node, ctx) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // onClicked: e(<arg>) — buttons fire QML's `clicked()` signal
    // which carries no payload.  If the author declared `emit onTap
    // ;` (parameterless) we emit `e()`; if they declared a payload
    // (rare but allowed) we still emit `e()` because the button has
    // nothing to pass — surface the gap as a comment so the
    // mismatch is visible to the author.  `pick_signal_arg` returns
    // `""` for arity-0 signals which is what we want; for arity-≥1
    // it returns `"text"`, but a button has no `text` in scope, so
    // we explicitly take the empty-arg path here.
    if let Some(emit_name) =
        find_emit_ref_prop(node, "onClick").or_else(|| find_emit_ref_prop(node, "onTap"))
    {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        if let Some(args) = host_button_signal_args(emit_name, ctx) {
            writeln!(out, "{inner_pad}onClicked: {camel}({args})").unwrap();
        } else {
            let arity = ctx
                .emits
                .iter()
                .find(|e| e.name == *emit_name)
                .map(|e| e.params.len())
                .unwrap_or(0);
            writeln!(
                out,
                "{inner_pad}// NOTE: signal '{emit_name}' declares {arity} param(s) but Qt's Button.clicked() has no supported row payload; invoking parameterless"
            )
            .unwrap();
            writeln!(out, "{inner_pad}onClicked: {camel}()").unwrap();
        }
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

fn host_button_signal_args(emit_name: &str, ctx: &EmitCtx) -> Option<String> {
    let Some(emit) = ctx.emits.iter().find(|e| e.name == emit_name) else {
        return Some(String::new());
    };
    if emit.params.is_empty() {
        return Some(String::new());
    }
    if emit.params.len() != 1 {
        return None;
    }
    match &emit.params[0].r#type {
        EmitPayloadType::Text | EmitPayloadType::Color | EmitPayloadType::Component(_) => {
            ctx.enclosing_item.clone()
        }
        EmitPayloadType::Number => ctx.enclosing_index.clone(),
        EmitPayloadType::Bool => None,
    }
}

/// Lower a `HostDialog` node (UI29-1, the 16th kernel primitive) to a
/// QML `Popup { ... }` block.
///
/// `Popup` lives in `QtQuick.Controls 2.15`. With `modal: true` it
/// installs a focus trap and a backdrop dim automatically; with
/// `modal: false` it behaves as an in-flow popover.
///
/// ## Property handling
///
/// | moslayout prop                         | QML output                                                              |
/// |---|---|
/// | `open: slot: x`                        | `visible: x` (bare identifier binding)                                  |
/// | `open: true/false`                     | `visible: true/false`                                                   |
/// | `modal: true` (keyword)                | `modal: true`                                                           |
/// | `modal: false` (keyword)               | `modal: false`                                                          |
/// | `title: slot: x`                       | a `Text { text: x; font.bold: true }` as the first contentItem child   |
/// | `title: "literal"`                     | a `Text { text: "literal"; font.bold: true }` as the first child       |
/// | `dismiss-on-backdrop: false` (kw)      | `closePolicy: Popup.CloseOnEscape`                                      |
/// | `dismiss-on-backdrop: true` (kw)       | `closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent`    |
/// | (no `dismiss-on-backdrop`)             | same as `true` — the default                                            |
/// | `onClose: emit: onE`                   | `onClosed: e()` — Popup's signal name is past-tense `closed`            |
/// | `onOpen: emit: onE`                    | `onOpened: e()` — past-tense `opened` to match Qt convention            |
///
/// ## Why `contentItem: ColumnLayout`
///
/// `Popup` accepts a single `contentItem` element that hosts the
/// dialog's body. We always wrap children in a `ColumnLayout` so
/// authors can drop in any number of children without thinking about
/// `Popup`'s single-element constraint. The children walk through
/// `emit_qml_children` so nested `If`/`Else`/`For` inside the dialog
/// body work without special-casing.
///
/// ## Title: no native `Popup.title`
///
/// Plain `Popup` has no built-in title slot (QtQuick.Controls's
/// `Dialog` subclass does, but it brings in extra footer/header
/// machinery we don't want for a primitive). When a `title:` prop is
/// bound, we synthesise a bold `Text` element as the first child of
/// `contentItem`, before the author's children.
fn emit_host_dialog_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let content_pad = "    ".repeat(depth + 2);
    let mut out = String::new();
    writeln!(out, "{pad}Popup {{").unwrap();

    // modal: <bool>. Defaults to `true` when the prop is absent, per
    // UI29-1 §2.1. The grammar passes the boolean literal through as a
    // `Keyword("true"|"false")` value.
    let modal_keyword = find_keyword_prop(node, "modal").unwrap_or("true");
    let modal_value = if matches!(modal_keyword, "true" | "false") {
        modal_keyword
    } else {
        "true"
    };
    writeln!(out, "{inner_pad}modal: {modal_value}").unwrap();

    // visible: <slot or literal>. Sourced from `open`. When absent the
    // dialog stays hidden until something binds visibility — emit
    // `visible: false` so the QML is well-formed and predictable.
    if let Some(line) = build_open_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    } else {
        writeln!(out, "{inner_pad}visible: false").unwrap();
    }

    // closePolicy: chosen by `dismiss-on-backdrop`. The default
    // (absent or explicit `true`) accepts both Esc and outside-press;
    // `dismiss-on-backdrop: false` keeps only Esc so the dialog must
    // be closed by the program (e.g. a child button calling the
    // `onClose` emit).
    let dismiss = find_keyword_prop(node, "dismiss-on-backdrop").unwrap_or("true");
    let close_policy = if dismiss == "false" {
        "Popup.CloseOnEscape"
    } else {
        "Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent"
    };
    writeln!(out, "{inner_pad}closePolicy: {close_policy}").unwrap();

    // onClosed / onOpened — wire to the Mosaic emit signals. Popup's
    // own signals are past-tense (`closed`/`opened`); the host-side
    // Mosaic emits are present-tense (`onClose`/`onOpen`) per
    // UI29-1 §2.2.
    if let Some(emit_name) = find_emit_ref_prop(node, "onClose") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(out, "{inner_pad}onClosed: {camel}()").unwrap();
    }
    if let Some(emit_name) = find_emit_ref_prop(node, "onOpen") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(out, "{inner_pad}onOpened: {camel}()").unwrap();
    }

    // contentItem: ColumnLayout { ... children ... }. We always emit
    // the ColumnLayout — even when there are zero children and no
    // title — so the Popup has a well-defined body to anchor styling
    // and so the structural shape stays consistent across calls.
    writeln!(out, "{inner_pad}contentItem: ColumnLayout {{").unwrap();

    // Optional title row: bold Text as the first child of contentItem.
    if let Some(title_line) = build_dialog_title_text_line(node) {
        writeln!(out, "{content_pad}Text {{").unwrap();
        writeln!(out, "{content_pad}    {title_line}").unwrap();
        writeln!(out, "{content_pad}    font.bold: true").unwrap();
        writeln!(out, "{content_pad}}}").unwrap();
    }

    // Author's children — walked through the shared children walker so
    // nested meta-primitives (If/Else/For) get the same treatment they
    // get anywhere else in the tree. `is_stack: false` — a dialog
    // body is a normal column, not a Z-stack.
    out.push_str(&emit_qml_children(&node.children, depth + 2, false, ctx)?);

    writeln!(out, "{inner_pad}}}").unwrap();
    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

// =====================================================================
// UI29-2 — HostCheckbox / HostRadio emitters
// =====================================================================

/// Lower a `HostCheckbox` node (UI29-2, 17th kernel primitive) to a
/// QtQuick.Controls 2.15 `CheckBox { ... }` block.
///
/// ## Property handling
///
/// | moslayout prop          | QML output                                    |
/// |---|---|
/// | `checked: slot: c`      | `checked: c` (bare identifier binding)        |
/// | `checked: true/false`   | `checked: true/false`                         |
/// | `disabled: slot: d`     | `enabled: !d` (polarity flip)                 |
/// | `disabled: true/false`  | `enabled: !true/!false`                       |
/// | `indeterminate: slot:i` | `tristate: true; checkState: i ? Qt.PartiallyChecked : (checked ? Qt.Checked : Qt.Unchecked)` |
/// | `label: "..."` / `slot:`| `text: "..."` / `text: <slot>`                |
/// | `onToggle: emit: onX`   | `onToggled: x(checked)` — Qt's `toggled(bool)` signal |
///
/// ## `onToggled` carries the new state
///
/// QtQuick `AbstractButton` (CheckBox's parent) exposes a
/// `toggled(bool checked)` signal. We forward the `checked` parameter
/// into the Mosaic emit call so the host sees the new state, matching
/// the kernel-canonical `onToggle(checked: bool)` payload.
fn emit_host_checkbox_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();
    writeln!(out, "{pad}CheckBox {{").unwrap();

    // text: <label> — same builder as HostButton's label attr.
    if let Some(line) = build_label_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // checked: <slot or literal> — sourced from the `checked` prop.
    if let Some(line) = build_checked_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // enabled: !<disabled> — same polarity flip HostButton uses.
    if let Some(line) = build_disabled_to_enabled_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // tristate + checkState — only when the author wires `indeterminate`.
    // Qt's tri-state checkbox uses `Qt.Checked` / `Qt.Unchecked` /
    // `Qt.PartiallyChecked` for the three states. We rebuild the
    // checkState ternary so the indeterminate slot wins when truthy,
    // and the `checked` slot resolves the binary case otherwise.
    if let Some(slot) = find_slot_ref_prop(node, "indeterminate") {
        let camel = to_camel_case_first_lower(slot);
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{inner_pad}tristate: true").unwrap();
        // The `checked` binding above is sufficient when indeterminate
        // is false; when it's true we override via checkState. This
        // matches Qt's documented behaviour for tri-state checkboxes.
        writeln!(
            out,
            "{inner_pad}checkState: {camel} ? Qt.PartiallyChecked : (checked ? Qt.Checked : Qt.Unchecked)"
        )
        .unwrap();
    }

    // onToggled: x(<arg>) — Qt fires `toggled(bool checked)` whenever
    // the user flips the checkbox.  Respect the signal's declared
    // arity: a parameterless `emit onToggle ;` gets invoked as
    // `x()`; a `emit onToggle ( value : bool )` (or any arity ≥ 1)
    // gets `x(checked)` since `checked` is the natural payload Qt
    // makes available in the signal-handler scope.
    if let Some(emit_name) = find_emit_ref_prop(node, "onToggle") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arg = pick_signal_arg_with(emit_name, ctx.emits, "checked");
        writeln!(out, "{inner_pad}onToggled: {camel}({arg})").unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Lower a `HostRadio` node (UI29-2, 18th kernel primitive) to a
/// QtQuick.Controls 2.15 `RadioButton { ... }` block.
///
/// ## Property handling
///
/// | moslayout prop          | QML output                                          |
/// |---|---|
/// | `checked: slot: c`      | `checked: c`                                        |
/// | `group: "..."`          | `ButtonGroup.group: "..."` *(deferred — see notes)* |
/// | `value: "..."`          | preserved as `// value: ...` annotation comment     |
/// | `disabled: slot: d`     | `enabled: !d`                                       |
/// | `label: ... / slot:`    | `text: "..."` / `text: <slot>`                      |
/// | `onSelect: emit: onX`   | `onCheckedChanged: if (checked) x(<value>)`         |
///
/// ## `onSelect` fires only on positive transition
///
/// Per UI29-2 §2.2, `onSelect` represents "this radio was chosen", not
/// "this radio was toggled". Qt's `RadioButton` exposes
/// `checkedChanged()` (no payload) whenever its checked state flips;
/// we gate the dispatch on `if (checked)` so a sibling-radio-caused
/// deselect doesn't dispatch.
///
/// ## Group coordination — v1 limitation
///
/// QtQuick.Controls provides `ButtonGroup` for true radio-group
/// behavior, but wiring it requires synthesising a `ButtonGroup { id: ... }`
/// element at the enclosing scope and attaching every radio's
/// `ButtonGroup.group: …` reference to it. That structural pass is
/// reserved for UI29-2.1's `RadioGroup` userland component; v1
/// preserves the `group:` prop as a `// group: ...` comment, identical
/// to the SwiftUI backend's choice, so the metadata stays visible.
fn emit_host_radio_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();

    // `// group: ...` comment — preserves the group metadata for a
    // future structural pass. SAME line-comment injection vector as
    // the SwiftUI backend, so use the same newline-stripping fix.
    fn escape_for_line_comment(s: &str) -> String {
        let mut escaped = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '\n' | '\r' => escaped.push(' '),
                other => escaped.push(other),
            }
        }
        escaped
    }
    if let Some(g) = find_string_prop(node, "group") {
        writeln!(out, "{pad}// group: {}", escape_for_line_comment(g)).unwrap();
    } else if let Some(slot) = find_slot_ref_prop(node, "group") {
        let camel = to_camel_case_first_lower(slot);
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{pad}// group: slot {camel}").unwrap();
    }

    // `// value: ...` annotation — Qt's RadioButton has no native
    // `value` slot (ButtonGroup tracks the checkedButton instance, not
    // an arbitrary value). The author's value: prop carries through as
    // a payload to the dispatch call below; we also surface it as a
    // comment so the metadata is visible in the generated source.
    let value_for_dispatch: String = if let Some(v) = find_string_prop(node, "value") {
        writeln!(out, "{pad}// value: {}", escape_for_line_comment(v)).unwrap();
        format!("\"{}\"", escape_qml_string(v))
    } else if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{pad}// value: slot {camel}").unwrap();
        camel
    } else {
        "\"\"".to_string()
    };

    writeln!(out, "{pad}RadioButton {{").unwrap();

    // text: <label>.
    if let Some(line) = build_label_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // checked: <slot or literal>.
    if let Some(line) = build_checked_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // enabled: !<disabled>.
    if let Some(line) = build_disabled_to_enabled_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // onCheckedChanged: positive-transition-gated dispatch.
    // Respect signal arity: a parameterless `emit onSelect ;` gets
    // invoked as `x()`; a ≥1-arity signal gets the value payload
    // computed above (`value_for_dispatch`).
    if let Some(emit_name) = find_emit_ref_prop(node, "onSelect") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arity = ctx
            .emits
            .iter()
            .find(|e| e.name == *emit_name)
            .map(|e| e.params.len())
            .unwrap_or(0);
        let call_args = if arity == 0 {
            String::new()
        } else {
            value_for_dispatch.clone()
        };
        writeln!(
            out,
            "{inner_pad}onCheckedChanged: if (checked) {camel}({call_args})"
        )
        .unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

// =====================================================================
// UI29-4 — HostLink / HostTooltip / HostNumberInput emitters
// =====================================================================

/// Lower a `HostLink` node (UI29-4, 19th kernel primitive) to a QML
/// rich-text `Text` element with `onLinkActivated`.
///
/// ## Why `Text` with rich-text, not a control
///
/// QtQuick has no first-class hyperlink widget. The idiomatic
/// pattern is `Text { textFormat: Text.RichText; text: "<a href='...'>
/// label</a>"; onLinkActivated: Qt.openUrlExternally(link) }`. We
/// emit exactly that shape, with the `<a>` tag built from the
/// author's `href:` and `label:` props.
///
/// ## Property handling
///
/// | moslayout prop      | QML output                                                 |
/// |---|---|
/// | `href: "..."`       | embedded in the rich-text as `<a href="...">`             |
/// | `label: "..."`      | the visible link text inside `<a>...</a>`                 |
/// | `target: new-tab`   | `onLinkActivated: Qt.openUrlExternally(link)` (always external — Qt has no in-window tab concept; new-tab and same map to the same external-browser call) |
/// | `external: false`   | `onLinkActivated: x()` — dispatches the emit instead of opening, letting the host route in-app |
/// | `onActivate: emit`  | dispatched on link activation                             |
fn emit_host_link_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let mut out = String::new();

    let href = find_string_prop(node, "href").unwrap_or("#");
    // Two-layer escaping. The author's strings are embedded inside
    // an HTML payload that is itself embedded inside a QML double-
    // quoted string literal. Each layer needs its own escaping:
    //
    //   layer 1: HTML — `&` `<` `>` and (for href/label both, since
    //                     label may contain `"` that closes inner
    //                     attrs in future tag additions) `"` -> entities
    //   layer 2: QML  — backslash and double-quote -> `\\` and `\"`
    //
    // Skipping either layer is exploitable. A bare `\` in the
    // author's string would otherwise eat the closing QML quote.
    // A bare `"` in label would otherwise break out of an inner
    // HTML attribute. Both vectors caught by security review.
    fn html_entity_encode(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    let href_html = html_entity_encode(href);
    let rich_text = host_link_rich_text_expr(node, href, &href_html)?;

    writeln!(out, "{pad}Text {{").unwrap();
    writeln!(out, "{inner}textFormat: Text.RichText").unwrap();
    writeln!(out, "{inner}text: {rich_text}").unwrap();

    // onLinkActivated wiring:
    //   - external: false + onActivate -> dispatch emit, don't open
    //   - onActivate alone               -> dispatch AND open externally
    //   - bare (no onActivate)            -> open externally
    let external_false = matches!(find_keyword_prop(node, "external"), Some("false"));
    let on_activate = find_emit_ref_prop(node, "onActivate");

    let handler_body: String = match (external_false, on_activate) {
        (true, Some(emit)) => {
            // Pure in-app routing — dispatch only.  Respect signal
            // arity: arity-0 → `e()`; arity ≥1 → `e(link)` (Qt's
            // onLinkActivated handler scope exposes the activated
            // URL as `link`).
            let camel = to_camel_case_first_lower(&strip_on_prefix(emit));
            validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
            let arg = host_link_signal_args(emit, ctx).unwrap_or_default();
            format!("{camel}({arg})")
        }
        (false, Some(emit)) => {
            let camel = to_camel_case_first_lower(&strip_on_prefix(emit));
            validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
            let arg = host_link_signal_args(emit, ctx).unwrap_or_default();
            // Dispatch AND open externally.
            format!("{{ {camel}({arg}); Qt.openUrlExternally(link); }}")
        }
        (_, None) => "Qt.openUrlExternally(link)".to_string(),
    };
    writeln!(out, "{inner}onLinkActivated: {handler_body}").unwrap();

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

fn host_link_rich_text_expr(
    node: &LayoutNode,
    fallback_href: &str,
    href_html: &str,
) -> Result<String, PipelineEmitError> {
    fn html_entity_encode(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    if let Some(label) = find_string_prop(node, "label") {
        let label_html = html_entity_encode(label);
        let rich_text_raw = format!(r#"<a href="{href_html}">{label_html}</a>"#);
        return Ok(format!("\"{}\"", escape_qml_string(&rich_text_raw)));
    }

    let label_expr = if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        Some(camel)
    } else if let Some(keyword) = find_keyword_prop(node, "label") {
        let camel = to_camel_case_first_lower(keyword);
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        Some(camel)
    } else {
        None
    };

    if let Some(label_expr) = label_expr {
        let prefix = escape_qml_string(&format!(r#"<a href="{href_html}">"#));
        let suffix = escape_qml_string("</a>");
        let escaped_label = qml_html_escape_expr(&label_expr);
        Ok(format!("\"{prefix}\" + {escaped_label} + \"{suffix}\""))
    } else {
        let label_html = html_entity_encode(fallback_href);
        let rich_text_raw = format!(r#"<a href="{href_html}">{label_html}</a>"#);
        Ok(format!("\"{}\"", escape_qml_string(&rich_text_raw)))
    }
}

fn qml_html_escape_expr(expr: &str) -> String {
    format!(
        "String({expr}).replace(/&/g, \"&amp;\").replace(/</g, \"&lt;\").replace(/>/g, \"&gt;\").replace(/\"/g, \"&quot;\")"
    )
}

fn host_link_signal_args(emit_name: &str, ctx: &EmitCtx) -> Option<String> {
    let Some(emit) = ctx.emits.iter().find(|e| e.name == emit_name) else {
        return Some("link".to_string());
    };
    if emit.params.is_empty() {
        return Some(String::new());
    }
    if emit.params.len() != 1 {
        return None;
    }

    let field = to_camel_case_first_lower(&emit.params[0].name);
    if field == "href" {
        return match &emit.params[0].r#type {
            EmitPayloadType::Text => Some("link".to_string()),
            _ => None,
        };
    }

    match &emit.params[0].r#type {
        EmitPayloadType::Text | EmitPayloadType::Color | EmitPayloadType::Component(_) => ctx
            .enclosing_item
            .clone()
            .or_else(|| Some("link".to_string())),
        EmitPayloadType::Number => ctx.enclosing_index.clone(),
        EmitPayloadType::Bool => None,
    }
}

/// Lower a `HostTooltip` node (UI29-4, 20th kernel primitive) to a
/// QML wrapping `Item` with `ToolTip.text` + `ToolTip.visible` on
/// hover. The single child is the wrapped element.
///
/// ## Generated shape
///
/// ```qml
/// Item {
///     ToolTip.text: "..."
///     ToolTip.visible: hoverHandler.hovered
///     HoverHandler { id: hoverHandler }
///     /* child(ren) here */
/// }
/// ```
///
/// `HoverHandler` (QtQuick 2.12+) gives the hover state without
/// needing a `MouseArea` (which would intercept clicks on the child).
fn emit_host_tooltip_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let mut out = String::new();

    let text = find_string_prop(node, "text").unwrap_or("");
    let escaped = escape_qml_string(text);

    writeln!(out, "{pad}Item {{").unwrap();
    writeln!(out, "{inner}ToolTip.text: \"{escaped}\"").unwrap();
    writeln!(out, "{inner}ToolTip.visible: hoverHandler.hovered").unwrap();
    writeln!(out, "{inner}HoverHandler {{ id: hoverHandler }}").unwrap();

    // Walk children (the wrapped element) through the standard children
    // walker so nested kernel primitives lower normally.
    out.push_str(&emit_qml_children(&node.children, depth + 1, false, ctx)?);

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Lower a `HostNumberInput` node (UI29-4, 21st kernel primitive)
/// to a QtQuick.Controls 2.15 `TextField` with a `DoubleValidator`.
/// This preserves decimal `number` payloads for scheduler options such
/// as ease factors and interval multipliers.
///
/// ## Property handling
///
/// | moslayout prop  | QML output                                |
/// |---|---|
/// | `value: slot|n` | `text: String(<bare-identifier>|n)`          |
/// | `placeholder`   | `placeholderText: "..."`                    |
/// | `min: <n>`      | `validator.bottom: <n>`                     |
/// | `max: <n>`      | `validator.top: <n>`                        |
/// | `disabled: ...` | `enabled: !d` (polarity flip)               |
/// | `onChange: emit`| `onTextEdited` parses and dispatches `nextValue` |
fn emit_host_number_input_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let validator_inner = "    ".repeat(depth + 2);
    let mut out = String::new();
    writeln!(out, "{pad}TextField {{").unwrap();

    // value:
    if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        if is_safe_identifier(&camel) {
            writeln!(out, "{inner}text: String({camel})").unwrap();
        }
    } else if let Some(n) = find_number_prop(node, "value") {
        writeln!(out, "{inner}text: String({n})").unwrap();
    }

    if let Some(placeholder) = find_string_prop(node, "placeholder") {
        let escaped = escape_qml_string(placeholder);
        writeln!(out, "{inner}placeholderText: \"{escaped}\"").unwrap();
    }
    writeln!(out, "{inner}inputMethodHints: Qt.ImhFormattedNumbersOnly").unwrap();
    writeln!(out, "{inner}validator: DoubleValidator {{").unwrap();
    writeln!(out, "{validator_inner}decimals: 12").unwrap();
    if let Some(n) = find_number_prop(node, "min") {
        writeln!(out, "{validator_inner}bottom: {n}").unwrap();
    }
    if let Some(n) = find_number_prop(node, "max") {
        writeln!(out, "{validator_inner}top: {n}").unwrap();
    }
    writeln!(out, "{inner}}}").unwrap();

    // disabled -> enabled: !d (polarity flip; same shape as HostButton).
    if let Some(line) = build_disabled_to_enabled_attribute(node) {
        writeln!(out, "{inner}{line}").unwrap();
    }

    // onChange -> onTextEdited. Respect signal arity: arity-0 →
    // `e()`; arity ≥1 → `e(nextValue)`.
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        let arg = pick_signal_arg_with(emit_name, ctx.emits, "nextValue");
        writeln!(out, "{inner}onTextEdited: {{").unwrap();
        writeln!(out, "{validator_inner}const nextValue = Number(text)").unwrap();
        writeln!(
            out,
            "{validator_inner}if (text.length > 0 && !isNaN(nextValue)) {{ {camel}({arg}) }}"
        )
        .unwrap();
        writeln!(out, "{inner}}}").unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Build the `checked: <slot|literal>` attribute used by both
/// `HostCheckbox` and `HostRadio`. Mirrors `build_open_attribute` for
/// `HostDialog`'s `visible:` prop — same shape, just a different
/// destination attribute name.
fn build_checked_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "checked")?;
    Some(match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("checked: {camel}")
            } else {
                "checked: false".to_string()
            }
        }
        LayoutPropValue::Keyword(k) if k == "true" || k == "false" => {
            format!("checked: {k}")
        }
        _ => "checked: false".to_string(),
    })
}

/// Build the `visible: ...` attribute for a `HostDialog` from its
/// `open` prop. Same shape as `build_read_only_attribute` — accepts a
/// slot ref or the `true`/`false` keyword literals. Returns `None`
/// when the prop is absent so the caller can pick a sensible default.
fn build_open_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "open")?;
    Some(match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("visible: {camel}")
            } else {
                "visible: false".to_string()
            }
        }
        LayoutPropValue::Keyword(k) if k == "true" || k == "false" => {
            format!("visible: {k}")
        }
        // A string/number/emit-ref isn't a meaningful boolean source.
        // Default to hidden — the safe shape.
        _ => "visible: false".to_string(),
    })
}

/// Build the `text: ...` line that goes inside the synthesised title
/// `Text` element for a `HostDialog`. Mirrors `build_text_attribute`
/// but reads from `title` instead of `content`. Returns `None` when
/// the dialog has no `title:` prop.
fn build_dialog_title_text_line(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "title")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => format!("text: \"{}\"", escape_qml_string(s)),
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("text: {camel}")
            } else {
                "text: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => format!("text: \"{k}\""),
        LayoutPropValue::Number(n) => format!("text: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "text: \"\"".to_string(),
        // U29-G3 expression — same treatment as other build_*_attribute fns.
        LayoutPropValue::Expr(_) => "text: \"\"".to_string(),
    })
}

/// True iff any node in the layout tree lowers to a `QtQuick.Controls`
/// element. Today: `HostInput` with `placeholder` → `TextField`,
/// `HostButton` → `Button`, `HostScroll` → `ScrollView`,
/// `HostDialog` → `Popup`, `HostCheckbox` → `CheckBox`,
/// `HostRadio` → `RadioButton`, `HostTooltip` → `ToolTip` attached
/// property, `HostNumberInput` → `TextField`. `HostLink` is intentionally
/// NOT here because it lowers to a plain `Text` element with rich-
/// text + onLinkActivated, not a QtQuick.Controls widget.
fn tree_needs_controls_import(node: &LayoutNode) -> bool {
    if node.tag == "HostInput" && build_placeholder_text_attribute(node).is_some() {
        return true;
    }
    matches!(
        node.tag.as_str(),
        "HostButton"
            | "HostScroll"
            | "HostDialog"
            | "HostCheckbox"
            | "HostRadio"
            | "HostTooltip"
            | "HostNumberInput"
    ) || node.children.iter().any(tree_needs_controls_import)
}

/// Lower a `HostTable` node to a QML `ColumnLayout` of `RowLayout` rows.
///
/// ## Lowering strategy (first cut)
///
/// QtQuick has a real, data-driven `TableView` element backed by a
/// `QAbstractTableModel` (or the lighter `Qt.labs.qmlmodels.TableModel`).
/// That is the *correct* long-term lowering for `HostTable`, but it
/// requires:
///
/// 1. A model object exposed from the host side (or a QML
///    `TableModel { TableModelColumn { ... } ... }` declaration).
/// 2. A `DelegateChooser` or per-column delegate to render each cell.
/// 3. Pluming `For`/slot data through that model.
///
/// None of those exist yet in this backend — `For` itself is still
/// deferred to U29-G2. So for this first cut we lower `HostTable` to a
/// structural shape made of layout primitives only:
///
/// ```qml
/// ColumnLayout {
///   spacing: 0
///   // HostTableHead — each Row becomes a RowLayout of bold Texts
///   RowLayout { Text { text: "A"; font.bold: true } }
///   Rectangle { Layout.fillWidth: true; height: 1; color: "#888" }
///   // HostTableBody — Rows become plain RowLayouts
///   RowLayout { Text { text: "1" } Text { text: "2" } }
///   // HostTableFoot — same shape as body, preceded by a divider
/// }
/// ```
///
/// The TODO/follow-up: emit a real `TableView` + `TableModel` once the
/// pipeline can synthesise the model from a `For`-bound slot.
///
/// ## Sub-tag handling
///
/// | sub-tag             | output                                                       |
/// |---|---|
/// | `HostTableHead`     | RowLayout(s); descendant `Text` nodes get `font.bold: true`  |
/// | `HostTableBody`     | RowLayout(s) per `Row` child                                 |
/// | `HostTableFoot`     | RowLayout(s) preceded by a divider Rectangle                 |
/// | `HostTableColGroup` | Ignored — no QML analog. Emitted as a `// ColGroup …` comment |
/// | (any other child)   | Walked normally (so a stray `Text` inside `HostTable` works) |
///
/// `part_name` on the `HostTable` itself is *currently* not consumed —
/// styling integration for table parts is a follow-up. Tests assert
/// that its presence does not break emission.
fn emit_host_table_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();

    // Discover the host's column-widths slot from a nested
    // `HostTableColGroup` (UI31 §3.2 shape). When present, every styled
    // cell inside this table threads `columnWidths[<index>]` into its
    // `Layout.preferredWidth` so columns are fixed-width rather than
    // auto-sizing to content. `None` preserves the prior auto-sized
    // behaviour. The discovered slot lives on a child context so the
    // whole table sub-tree sees it.
    // Cascade the table's own part (`sheet`) styling down to cells: a
    // cell whose own `.msl` part omits background / color / font falls
    // back to these so it reads against the sheet's surface, matching
    // CSS inheritance on the other backends.
    let inherited = match &node.part_name {
        Some(part) => {
            let sheet: &[StyleProp] = ctx
                .part_styles
                .get(part)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            InheritedStyle {
                background: style_prop(sheet, "background")
                    .or_else(|| style_prop(sheet, "background-color"))
                    .and_then(qml_hex_color_or_none),
                color: style_prop(sheet, "color").and_then(qml_hex_color_or_none),
                font_family_mono: style_prop(sheet, "font-family")
                    .map(|v| v.trim() == "monospace")
                    .unwrap_or(false),
                font_pixel_size: style_prop(sheet, "font-size").and_then(qml_px_or_none),
            }
        }
        None => ctx.inherited.clone(),
    };
    let table_ctx = EmitCtx {
        col_widths_slot: discover_col_widths_slot(node),
        inherited,
        ..ctx.clone()
    };
    let ctx = &table_ctx;

    writeln!(out, "{pad}ColumnLayout {{").unwrap();
    writeln!(out, "{inner_pad}spacing: 0").unwrap();

    // UI31 §3.2 RTL contract. QML's `LayoutMirroring` attached
    // property is the canonical RTL knob: `enabled: true` flips
    // column order (so the first child appears on the right), and
    // `childrenInherit: true` propagates the flip down the
    // ColumnLayout's `RowLayout` rows so cell order matches.
    //
    // Three accepted shapes (mirrors the React/HTML/Webcomp/Flutter
    // backends shipped in #4143, #4156, #4162, #4166):
    //
    // | Source                 | Emits                                                      |
    // |------------------------|------------------------------------------------------------|
    // | `dir: rtl`             | `LayoutMirroring.enabled: true` + `.childrenInherit: true` |
    // | `dir: ltr`             | `LayoutMirroring.enabled: false`                           |
    // | `dir: auto`            | (nothing — inherit from ancestor; Qt has no auto)          |
    // | `dir: slot: layoutDir` | `LayoutMirroring.enabled: layoutDir` (slot must be bool)   |
    // | unknown keyword        | (nothing — drops silently per allow-list security gate)    |
    //
    // The allow-list (`ltr` / `rtl` / `auto`) is the security gate:
    // an attacker-controlled keyword cannot break out of the QML
    // attribute position because it never reaches the format string.
    // Slot refs run through `is_safe_identifier` so the binding
    // identifier stays syntactically clean QML.
    if let Some(slot) = find_slot_ref_prop(node, "dir") {
        let camel = to_camel_case_first_lower(slot);
        if is_safe_identifier(&camel) {
            writeln!(out, "{inner_pad}LayoutMirroring.enabled: {camel}").unwrap();
            writeln!(out, "{inner_pad}LayoutMirroring.childrenInherit: true").unwrap();
        }
    } else if let Some(kw) = find_keyword_prop(node, "dir") {
        match kw {
            "rtl" => {
                writeln!(out, "{inner_pad}LayoutMirroring.enabled: true").unwrap();
                writeln!(out, "{inner_pad}LayoutMirroring.childrenInherit: true").unwrap();
            }
            "ltr" => {
                writeln!(out, "{inner_pad}LayoutMirroring.enabled: false").unwrap();
            }
            // `auto` is the spec-mandated "let the host decide"
            // keyword. QML has no `auto` enum — the right behaviour
            // is to NOT emit the attached property so any ancestor's
            // `LayoutMirroring` (typically the Window root's) flows
            // through unchanged.
            "auto" => {}
            _ => {}
        }
    }

    // Track whether we've already emitted the head→body divider. The
    // divider sits between head and the first non-head section (body or
    // foot); we also emit one before `HostTableFoot` so foot is visually
    // separated from body. The simple rule that matches the spec sketch:
    //   - emit a divider after a head section finishes;
    //   - emit a divider before each foot section.
    for child in &node.children {
        match child.tag.as_str() {
            "HostTableHead" => {
                emit_table_section_rows(&mut out, child, depth + 1, /* bold = */ true, ctx)?;
                // Divider after the head: a 1px Rectangle.
                writeln!(out, "{inner_pad}Rectangle {{").unwrap();
                writeln!(out, "{inner_pad}    Layout.fillWidth: true").unwrap();
                writeln!(out, "{inner_pad}    height: 1").unwrap();
                writeln!(out, "{inner_pad}    color: \"#888\"").unwrap();
                writeln!(out, "{inner_pad}}}").unwrap();
            }
            "HostTableBody" => {
                emit_table_section_rows(&mut out, child, depth + 1, /* bold = */ false, ctx)?;
            }
            "HostTableFoot" => {
                // Foot is preceded by a divider so it visually separates
                // from the body. Mirrors a typical `<tfoot>` border.
                writeln!(out, "{inner_pad}Rectangle {{").unwrap();
                writeln!(out, "{inner_pad}    Layout.fillWidth: true").unwrap();
                writeln!(out, "{inner_pad}    height: 1").unwrap();
                writeln!(out, "{inner_pad}    color: \"#888\"").unwrap();
                writeln!(out, "{inner_pad}}}").unwrap();
                emit_table_section_rows(&mut out, child, depth + 1, /* bold = */ false, ctx)?;
            }
            "HostTableColGroup" => {
                // No QML analog. Emit a self-documenting comment so the
                // output is still readable but the construct is visible.
                writeln!(out, "{inner_pad}// HostTableColGroup (no QML analog)").unwrap();
            }
            _ => {
                // Anything else inside `HostTable` is walked as a normal
                // layout child. This preserves the door for future
                // additions (a `For` row, a stray `Text` caption, etc.).
                out.push_str(&emit_qml_tree(child, depth + 1, ctx)?);
            }
        }
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Emit one section (head / body / foot) of a `HostTable`. Walks the
/// section's `Row` children, emitting each as a `RowLayout { ... }`. Any
/// non-`Row` child inside the section falls back to normal walking; in
/// practice the grammar will constrain sections to `Row` children only.
///
/// When `bold` is true, every descendant `Text` inside an emitted
/// `RowLayout` carries a `font.bold: true` line. The bolding is applied
/// inline here — we generate `RowLayout`s and their `Text` cells
/// directly rather than recursing through `emit_qml_tree` — because the
/// bold flag is a property of the *section*, not of the cell. Cells are
/// shallow leaves in practice (one prop each), so the explicit code is
/// short and clear.
fn emit_table_section_rows(
    out: &mut String,
    section: &LayoutNode,
    depth: usize,
    bold: bool,
    ctx: &EmitCtx,
) -> Result<(), PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let cell_pad = "    ".repeat(depth + 1);

    for row in &section.children {
        if row.tag != "Row" {
            // Non-Row child inside a section — walk it normally and let
            // the general emitter handle it. The grammar should prevent
            // this in real input, but be permissive on the output side.
            out.push_str(&emit_qml_tree(row, depth, ctx)?);
            continue;
        }

        writeln!(out, "{pad}RowLayout {{").unwrap();
        for cell in &row.children {
            if cell.tag == "Text" {
                writeln!(out, "{cell_pad}Text {{").unwrap();
                if let Some(line) = build_text_attribute(cell) {
                    writeln!(out, "{cell_pad}    {line}").unwrap();
                }
                if bold {
                    writeln!(out, "{cell_pad}    font.bold: true").unwrap();
                }
                writeln!(out, "{cell_pad}}}").unwrap();
            } else {
                // Non-Text cell: recurse through the general walker. Bold
                // doesn't propagate here — only `Text` cells get the
                // header treatment in this first cut.
                out.push_str(&emit_qml_tree(cell, depth + 1, ctx)?);
            }
        }
        writeln!(out, "{pad}}}").unwrap();
    }
    Ok(())
}

// =====================================================================
// UI29 meta-primitive emitters — For / If / Else
// =====================================================================

/// Lower a `For` node to a QML `Repeater { model: ...; delegate: Item
/// { ... } }`.
///
/// ## Prop contract (UI29 §3.1)
///
/// | moslayout prop            | QML output                                                  |
/// |---|---|
/// | `each: slot: <name>`      | `model: <camelName>` — bare-identifier binding              |
/// | `each: <expr>`            | `model: <expr-verbatim>` — passed through to QML            |
/// | `as: <NAME>`              | `property var <NAME>: modelData` on the delegate `Item`     |
/// | `index: <NAME>` (optional)| `property int <NAME>: index` on the delegate `Item`         |
///
/// ## QML repeater shape — and why the delegate is an `Item`
///
/// QML's `Repeater` instantiates its `delegate` once per element in
/// `model`. Inside the delegate, the implicit names `modelData` (the
/// element) and `index` (the position) are in scope. The author wrote
/// `as: row, index: r` though, so we have to re-export those values
/// under the *user-chosen* names. The cleanest way to do that in QML
/// is to declare them as properties on the delegate's root `Item`,
/// which makes them visible to every descendant binding through QML's
/// normal scope rules:
///
/// ```qml
/// Repeater {
///   model: viewportRows
///   delegate: Item {
///     property var row: modelData
///     property int r: index
///     // any descendant can now refer to `row` or `r` like a slot
///     Text { text: row }
///   }
/// }
/// ```
///
/// The delegate is always an `Item` (not e.g. `RowLayout`) for the same
/// reason the component root is an `Item`: it carries the bindings
/// without taking on layout semantics. Layout primitives that the
/// author wrote as children of the `For` are placed *inside* the
/// delegate `Item`.
///
/// ## What about the iterated slot's type?
///
/// `Repeater` happily accepts a JS array as `model:`. Our slot
/// declarations lower `list<T>` to QML `var`, which is exactly that.
/// We therefore make no attempt to specialise the delegate's
/// `property var <as>` to a typed property — `var` is the right shape.
fn emit_for_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let delegate_pad = "    ".repeat(depth + 1);
    let prop_pad = "    ".repeat(depth + 2);

    let model_expr = find_each_expression(node).unwrap_or_else(|| "[]".to_string());
    let as_name = find_keyword_prop(node, "as")
        .map(to_camel_case_first_lower)
        .filter(|s| is_safe_identifier(s))
        .unwrap_or_else(|| "item".to_string());
    let index_name = find_keyword_prop(node, "index")
        .map(to_camel_case_first_lower)
        .filter(|s| is_safe_identifier(s));

    let mut out = String::new();
    writeln!(out, "{pad}Repeater {{").unwrap();
    writeln!(out, "{delegate_pad}model: {model_expr}").unwrap();
    writeln!(out, "{delegate_pad}delegate: Item {{").unwrap();
    writeln!(out, "{prop_pad}property var {as_name}: modelData").unwrap();
    if let Some(idx) = &index_name {
        writeln!(out, "{prop_pad}property int {idx}: index").unwrap();
    }
    // The delegate `Item` carries no intrinsic size, but it IS a layout
    // child of the enclosing `RowLayout` / `ColumnLayout`. Without a size
    // the whole row collapses to nothing (the original "black smudge"
    // bug). Size the delegate to its children so a fixed-size styled cell
    // `Rectangle` (which sets `implicitWidth` / `implicitHeight`) drives
    // the delegate's size, which the enclosing layout then honours.
    // `childrenRect` is loop-safe here because our children size via
    // `implicitWidth/Height`, not by anchoring back to this parent.
    writeln!(out, "{prop_pad}implicitWidth: childrenRect.width").unwrap();
    writeln!(out, "{prop_pad}implicitHeight: childrenRect.height").unwrap();

    // Children of the `For` go inside the delegate Item. We use the
    // shared children walker so nested `If`/`Else` and `For` inside the
    // loop body work without special-casing.
    //
    // The loop's `index:` binding (e.g. `c` for the inner body loop, `ch`
    // for the header loop) becomes the nearest-enclosing index for any
    // styled cell `Box` beneath it — that index drives the cell's
    // `Layout.preferredWidth: columnWidths[<index>]` column-width thread.
    // `with_for` keeps the existing index when this For has none, so a
    // styled cell still sees the closest indexed ancestor.
    let child_ctx = ctx.with_for(as_name.clone(), index_name.clone());
    out.push_str(&emit_qml_children(
        &node.children,
        depth + 2,
        false,
        &child_ctx,
    )?);

    writeln!(out, "{delegate_pad}}}").unwrap();
    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Lower an `If` (with an optional paired `Else` sibling) to one or two
/// QML `Loader` elements.
///
/// ## Lowering (UI29 §3.2)
///
/// ```qml
/// // If only:
/// Loader {
///   active: <cond>
///   sourceComponent: Component { <then-children> }
/// }
///
/// // If + Else:
/// Loader {
///   active: <cond>
///   sourceComponent: Component { <then-children> }
/// }
/// Loader {
///   active: !<cond>
///   sourceComponent: Component { <else-children> }
/// }
/// ```
///
/// ## Why `Loader`+`Component` rather than a single conditional
///
/// QML has no view-builder `if`/`else` syntax. The idiomatic
/// shape for conditional instantiation is `Loader`, which builds and
/// destroys its `sourceComponent` based on `active`. Two `Loader`s with
/// inverted `active` predicates is the natural way to express
/// `if cond { A } else { B }` — only one branch is live at a time, and
/// the `Component { ... }` wrapper provides the lazy instantiation that
/// `Loader` expects.
///
/// ## `Component` body shape
///
/// A `Component { ... }` must wrap a single top-level QML element. When
/// the `If` body contains exactly one child we emit that child
/// directly. When the body contains multiple children (or zero), we
/// wrap them in an `Item { ... }` so the `Component` stays well-formed.
fn emit_if_qml(
    if_node: &LayoutNode,
    else_node: Option<&LayoutNode>,
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);

    let cond_expr = find_when_expression(if_node).unwrap_or_else(|| "false".to_string());
    let neg_cond = negate_qml_condition(&cond_expr);

    // When the If/Else sits as the direct body of a styled cell
    // `Rectangle`, each `Loader` must fill the cell so the loaded view
    // (TextInput / Text) gets the cell's full fixed size. The flag is
    // consumed here and NOT propagated into the branch bodies (those
    // children fill their own Loader, which `cell_text_style_lines`
    // already handles via `anchors.fill: parent`).
    let fill = ctx.cell_fill_children;
    let body_ctx = EmitCtx {
        cell_fill_children: false,
        ..ctx.clone()
    };

    let mut out = String::new();

    // The `If` branch — always emitted.
    writeln!(out, "{pad}Loader {{").unwrap();
    if fill {
        writeln!(out, "{inner_pad}anchors.fill: parent").unwrap();
    }
    writeln!(out, "{inner_pad}active: {cond_expr}").unwrap();
    writeln!(out, "{inner_pad}sourceComponent: Component {{").unwrap();
    out.push_str(&emit_branch_body(&if_node.children, depth + 2, &body_ctx)?);
    writeln!(out, "{inner_pad}}}").unwrap();
    writeln!(out, "{pad}}}").unwrap();

    // The `Else` branch — only emitted when an `Else` sibling was paired.
    if let Some(else_n) = else_node {
        writeln!(out, "{pad}Loader {{").unwrap();
        if fill {
            writeln!(out, "{inner_pad}anchors.fill: parent").unwrap();
        }
        writeln!(out, "{inner_pad}active: {neg_cond}").unwrap();
        writeln!(out, "{inner_pad}sourceComponent: Component {{").unwrap();
        out.push_str(&emit_branch_body(&else_n.children, depth + 2, &body_ctx)?);
        writeln!(out, "{inner_pad}}}").unwrap();
        writeln!(out, "{pad}}}").unwrap();
    }

    Ok(out)
}

/// Emit the body of an `If`/`Else` branch, suitable for wrapping in a
/// QML `Component { ... }`.
///
/// `Component` accepts exactly one top-level child. We therefore:
///
/// - `0 children` → an empty `Item { }` (the branch is well-formed but
///   renders nothing — matches the spec's "Else can be omitted; both
///   branches can have empty bodies" wording).
/// - `1 child`    → emit the child directly. The most common shape; no
///   extra wrapper means cleaner output.
/// - `N > 1`      → wrap in an `Item { ... }` so the `Component` has a
///   single root. Inside the `Item`, the N children are emitted
///   normally.
fn emit_branch_body(
    children: &[LayoutNode],
    depth: usize,
    ctx: &EmitCtx,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    match children.len() {
        0 => Ok(format!("{pad}Item {{ }}\n")),
        1 => emit_qml_tree(&children[0], depth, ctx),
        _ => {
            let mut out = String::new();
            writeln!(out, "{pad}Item {{").unwrap();
            out.push_str(&emit_qml_children(children, depth + 1, false, ctx)?);
            writeln!(out, "{pad}}}").unwrap();
            Ok(out)
        }
    }
}

/// Build the QML expression for a `For`'s `each:` prop.
///
/// - `each: slot: foo`  → `Some("foo")` (camelCased identifier, bound
///   to the property declared on the root `Item`).
/// - `each: <expr>`     → `Some(<expr verbatim>)`. The grammar reports
///   the expression as the reconstructed source substring; QML's
///   expression syntax overlaps with JavaScript, so most reasonable
///   bindings pass through unchanged. Names from enclosing `For`
///   bindings resolve at runtime through QML's normal scope lookup.
/// - any other shape    → `None` (the caller falls back to `[]`).
fn find_each_expression(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "each")?;
    match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                Some(camel)
            } else {
                None
            }
        }
        LayoutPropValue::Expr(text) => Some(text.clone()),
        // UI29 §3.4 — bare NAME that the validator has accepted as an
        // enclosing For-binding. Lower it as the camelCased QML
        // identifier (matches how the outer For's `as:` is itself
        // camelCased and exposed as a delegate property — see
        // `emit_for_qml`).
        LayoutPropValue::Keyword(name) => {
            let camel = to_camel_case_first_lower(name);
            if is_safe_identifier(&camel) {
                Some(camel)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Build the QML expression for an `If`'s `when:` prop. Same shape as
/// [`find_each_expression`] — SlotRef → camelCased identifier, Expr →
/// passed verbatim.
fn find_when_expression(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "when")?;
    match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                Some(camel)
            } else {
                None
            }
        }
        LayoutPropValue::Expr(text) => Some(text.clone()),
        _ => None,
    }
}

/// Negate a QML boolean expression in a syntactically robust way.
///
/// For a bare identifier or a simple parenthesised expression we can
/// prefix `!`; for anything more complex (an expression with `&&`,
/// `||`, comparison operators, etc.) we wrap in parens first so the
/// negation binds to the whole expression: `!a && b` would otherwise
/// parse as `(!a) && b`, which is the wrong polarity for the else
/// branch.
fn negate_qml_condition(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return "true".to_string();
    }
    // Bare identifiers and simple member accesses are safe to prefix;
    // everything else gets wrapped in parens.
    let is_simple = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if is_simple {
        format!("!{trimmed}")
    } else {
        format!("!({trimmed})")
    }
}

/// Find a `Keyword`-typed prop on `node` and return the keyword string.
///
/// Used for `as:`/`index:` props on `For`, which the grammar lowers as
/// `Keyword(<name>)` — a bare identifier with no `slot:` prefix.
fn find_keyword_prop<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::Keyword(k) = &p.value {
                return Some(k.as_str());
            }
        }
        None
    })
}

/// Build the `text: ...` attribute for a `HostInput` from its `value` prop.
fn build_value_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "value")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => format!("text: \"{}\"", escape_qml_string(s)),
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("text: {camel}")
            } else {
                "text: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => format!("text: \"{k}\""),
        LayoutPropValue::Number(n) => format!("text: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "text: \"\"".to_string(),
        // U29-G3 expression + UI29 §3.4 scope (PR #4398) — pass the
        // Expr text verbatim into the `text:` binding. QML evaluates it
        // in the surrounding Repeater delegate's scope, where For-loop
        // bindings live as `property var` declarations. Required by
        // mosaic-pkg-grid v0.2.0's `HostInput ( value: ( v ) )` shape.
        LayoutPropValue::Expr(text) => format!("text: {text}"),
    })
}

/// Build the `readOnly: ...` attribute for a `HostInput` from its
/// `read-only` prop. Accepts a slot ref *or* the keyword literals
/// `true` / `false`.
fn build_read_only_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "read-only")?;
    Some(match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("readOnly: {camel}")
            } else {
                "readOnly: false".to_string()
            }
        }
        LayoutPropValue::Keyword(k) if k == "true" || k == "false" => {
            format!("readOnly: {k}")
        }
        // Anything else (a string literal, a number, an emit-ref) is
        // not a meaningful boolean source. Default to non-read-only.
        _ => "readOnly: false".to_string(),
    })
}

/// Build the `placeholderText: ...` attribute for a `HostInput`.
fn build_placeholder_text_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "placeholder")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => {
            format!("placeholderText: \"{}\"", escape_qml_string(s))
        }
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("placeholderText: {camel}")
            } else {
                "placeholderText: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => format!("placeholderText: \"{k}\""),
        LayoutPropValue::Number(n) => format!("placeholderText: \"{n}\""),
        LayoutPropValue::Expr(text) => format!("placeholderText: {text}"),
        LayoutPropValue::EmitRef(_) => "placeholderText: \"\"".to_string(),
    })
}

/// Build the `text: ...` attribute for a `HostButton` from its `label` prop.
fn build_label_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "label")?;
    Some(match &prop.value {
        LayoutPropValue::String(s) => format!("text: \"{}\"", escape_qml_string(s)),
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("text: {camel}")
            } else {
                "text: \"\"".to_string()
            }
        }
        LayoutPropValue::Keyword(k) => {
            let camel = to_camel_case_first_lower(k);
            if is_safe_identifier(&camel) {
                format!("text: {camel}")
            } else {
                "text: \"\"".to_string()
            }
        }
        LayoutPropValue::Number(n) => format!("text: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "text: \"\"".to_string(),
        // U29-G3 expression — same treatment as build_value_attribute.
        LayoutPropValue::Expr(text) => format!("text: {text}"),
    })
}

/// Build the `enabled: !<x>` attribute for a `HostButton` from its
/// `disabled` prop. The polarity flip happens here (QML uses
/// `enabled`, moslayout uses `disabled`).
fn build_disabled_to_enabled_attribute(node: &LayoutNode) -> Option<String> {
    let prop = node.props.iter().find(|p| p.name == "disabled")?;
    Some(match &prop.value {
        LayoutPropValue::SlotRef(s) => {
            let camel = to_camel_case_first_lower(s);
            if is_safe_identifier(&camel) {
                format!("enabled: !{camel}")
            } else {
                "enabled: true".to_string()
            }
        }
        LayoutPropValue::Keyword(k) if k == "true" || k == "false" => {
            format!("enabled: !{k}")
        }
        _ => "enabled: true".to_string(),
    })
}

/// Find a prop on `node` whose value is a `String` literal.
fn find_string_prop<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::String(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

/// Find a prop on `node` whose value is a `SlotRef`. Returns the
/// slot's original (kebab-case) name; caller is responsible for
/// camelCasing via `to_camel_case_first_lower` before interpolation.
///
/// Added in UI29-2 to support the `HostCheckbox.indeterminate` /
/// `HostRadio.group` / `HostRadio.value` slot-typed props; existing
/// emitters inline the match on `LayoutPropValue::SlotRef` so this
/// helper is purely additive.
fn find_slot_ref_prop<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::SlotRef(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

/// Load a host-supplied QML `Component` into a real Qt Quick item. The
/// surrounding Rectangle owns shared MSL sizing/paint; Loader owns the live
/// browser viewport instance and naturally stays empty in generated previews.
fn emit_host_surface_qml(
    node: &LayoutNode,
    depth: usize,
    ctx: &EmitCtx<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner = "    ".repeat(depth + 1);
    let loader_inner = "    ".repeat(depth + 2);
    let source = if let Some(slot) = find_slot_ref_prop(node, "content") {
        let field = to_camel_case_first_lower(slot);
        validate_safe_identifier(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
        format!("mosaicRoot.{field}")
    } else {
        "null".to_string()
    };

    let mut out = String::new();
    writeln!(out, "{pad}Rectangle {{").unwrap();
    writeln!(out, "{inner}objectName: \"mosaic-host-surface\"").unwrap();
    writeln!(out, "{inner}Layout.fillWidth: true").unwrap();
    writeln!(out, "{inner}Layout.fillHeight: true").unwrap();
    if let Some(props) = part_style_props(node, ctx) {
        for line in qml_layout_size_lines(props) {
            writeln!(out, "{inner}{line}").unwrap();
        }
        let paint = qml_rectangle_paint_lines(props);
        if paint.iter().all(|line| !line.starts_with("color:")) {
            writeln!(out, "{inner}color: \"transparent\"").unwrap();
        }
        for line in paint {
            writeln!(out, "{inner}{line}").unwrap();
        }
    } else {
        writeln!(out, "{inner}color: \"transparent\"").unwrap();
    }
    writeln!(out, "{inner}Loader {{").unwrap();
    writeln!(out, "{loader_inner}anchors.fill: parent").unwrap();
    writeln!(out, "{loader_inner}sourceComponent: {source}").unwrap();
    writeln!(out, "{inner}}}").unwrap();
    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Find a prop on `node` whose value is an `EmitRef`. Returns the
/// emit's original (camelCased, `on`-prefixed) name.
fn find_emit_ref_prop<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::EmitRef(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

/// Find a numeric prop on `node`. Used by `HostNumberInput` for
/// `min`/`max`/`step` compile-time numeric literals.
fn find_number_prop(node: &LayoutNode, prop_name: &str) -> Option<f64> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::Number(n) = &p.value {
                return Some(*n);
            }
        }
        None
    })
}

// =====================================================================
// Type lowering
// =====================================================================

/// Map a `SlotType` to a `(qml-type, default-literal)` pair.
///
/// The default literal must be a valid QML expression that initialises a
/// property of the given type. The QML engine accepts `""` for `string`
/// and `url`; `0` for `real`; `false` for `bool`; `"#000000"` for `color`;
/// `[]` for `var`; and `null` for `Component`.
fn slot_type_to_qml(t: &SlotType) -> (&'static str, &'static str) {
    match t {
        SlotType::Text => ("string", "\"\""),
        SlotType::Number => ("real", "0"),
        SlotType::Bool => ("bool", "false"),
        SlotType::Image => ("url", "\"\""),
        SlotType::Color => ("color", "\"#000000\""),
        SlotType::Node => ("Component", "null"),
        // QML has no statically typed generic array; `var` holds any JS array.
        // The Repeater / ListView models bind happily to a `var`-typed array.
        SlotType::List(_inner) => ("var", "[]"),
        // Component-typed slots (a named sub-component) lower to `Component`,
        // matching the same default behaviour as `node`.
        SlotType::Component(_) => ("Component", "null"),
    }
}

/// Helper for the (future) deeper list typing — unused for now but kept
/// to mirror the React emitter's shape and make the eventual switch
/// from `var` to a more specific QML type a single-site change.
#[allow(dead_code)]
fn list_inner_to_qml(t: &ListInnerType) -> &'static str {
    match t {
        ListInnerType::Text => "string",
        ListInnerType::Number => "real",
        ListInnerType::Bool => "bool",
        ListInnerType::Image => "url",
        ListInnerType::Color => "color",
        ListInnerType::Node => "Component",
        ListInnerType::Component(_) => "Component",
        ListInnerType::List(_) => "var",
    }
}

/// Map an `EmitPayloadType` to a QML parameter type.
///
/// QML's `signal` syntax takes typed parameters: `signal navigated(int row,
/// int col)`. Numbers lower to `real` (Qt's name for double-precision
/// floats); `text` to `string`; `bool` stays `bool`; `color` lowers to
/// `color`; `Component` payloads lower to `var` since we don't yet
/// resolve named sub-component types here.
fn emit_payload_to_qml(t: &EmitPayloadType) -> &'static str {
    match t {
        EmitPayloadType::Text => "string",
        EmitPayloadType::Number => "real",
        EmitPayloadType::Bool => "bool",
        EmitPayloadType::Color => "color",
        EmitPayloadType::Component(_) => "var",
    }
}

// =====================================================================
// Name conversion + safety helpers
// =====================================================================

/// Convert `kebab-case` (and `lowerCamelCase` / `PascalCase`) to
/// `lowerCamelCase`. The first character of the output is lowered
/// unconditionally so PascalCase inputs (e.g. an emit name like
/// `OnNavigate` accidentally typed) still produce a JS-style identifier.
fn to_camel_case_first_lower(s: &str) -> String {
    let mut out = String::new();
    let mut cap_next = false;
    let mut first = true;
    for ch in s.chars() {
        if ch == '-' {
            cap_next = true;
            continue;
        }
        if first {
            out.push(ch.to_ascii_lowercase());
            first = false;
        } else if cap_next {
            out.push(ch.to_ascii_uppercase());
            cap_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Strip a leading `on` prefix from an emit name, if present.
/// `onNavigate` → `Navigate`; `onEditCommit` → `EditCommit`. Leaves the
/// rest untouched. The caller then runs `to_camel_case_first_lower` to
/// lowercase the first letter.
fn strip_on_prefix(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("on") {
        if let Some(first) = rest.chars().next() {
            if first.is_ascii_uppercase() {
                return rest.to_string();
            }
        }
    }
    s.to_string()
}

/// `true` iff `s` is `[_a-zA-Z][_a-zA-Z0-9]*`. The same shape works for
/// JavaScript and QML identifiers; QML's reserved-name rules are a
/// concern at a higher level (UI21 §2.4) and out of scope for the safety
/// check here.
fn is_safe_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Result-style wrapper around `is_safe_identifier` so callers can use the
/// `?` operator with `map_err`.
fn validate_safe_identifier(s: &str) -> Result<(), String> {
    if is_safe_identifier(s) {
        Ok(())
    } else {
        Err(s.to_string())
    }
}

/// Escape a string for embedding inside a QML double-quoted string literal.
///
/// QML string literals follow JavaScript conventions: `\` and `"` must be
/// backslash-escaped; newlines pass through as a literal newline (QML's
/// parser tolerates that in multi-line strings, though most authoring
/// tools prefer to escape).
fn escape_qml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use moslayout_compiler::{LayoutNode, LayoutProp};
    use mosmodel_compiler::{EmitParam, ListInnerType};

    // -------- Test fixtures --------

    fn empty_style(component: &str) -> StyleDef {
        StyleDef {
            component_name: component.to_string(),
            parts: Vec::new(),
        }
    }

    fn single_box_layout(component: &str) -> LayoutDef {
        LayoutDef {
            component_name: component.to_string(),
            root: LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    fn component(name: &str, slots: Vec<SlotDecl>, emits: Vec<EmitDecl>) -> MosmodelComponent {
        MosmodelComponent {
            component: name.to_string(),
            slots,
            emits,
        }
    }

    fn slot(name: &str, t: SlotType, required: bool) -> SlotDecl {
        SlotDecl {
            name: name.to_string(),
            r#type: t,
            required,
            default: None,
        }
    }

    fn emit_decl(name: &str, params: Vec<EmitParam>) -> EmitDecl {
        EmitDecl {
            name: name.to_string(),
            params,
        }
    }

    fn param(name: &str, t: EmitPayloadType) -> EmitParam {
        EmitParam {
            name: name.to_string(),
            r#type: t,
        }
    }

    // -------- Test 1: empty layout produces a valid QML skeleton --------

    /// The minimum case: a component with no slots, no emits, and a single
    /// `Box` layout. Should produce a complete QML file with the
    /// auto-generated banner, both imports, and a root `Item` wrapping a
    /// child `Item`.
    #[test]
    fn empty_component_emits_valid_qml_skeleton() {
        let m = component("Empty", vec![], vec![]);
        let l = single_box_layout("Empty");
        let s = empty_style("Empty");
        let result = from_pipeline(&m, &l, &s).expect("emit ok");
        assert!(result
            .output
            .starts_with("// Auto-generated by mosaic-emit-qt"));
        assert!(result.output.contains("import QtQuick 2.15"));
        assert!(result.output.contains("import QtQuick.Layouts 1.15"));
        assert!(result.output.contains("Item {"));
        // The Box lowering → child Item.
        let item_count = result.output.matches("Item {").count();
        assert!(
            item_count >= 2,
            "expected root Item + Box Item: {}",
            result.output
        );
        assert_eq!(result.component_name, "Empty");
    }

    // -------- Test 2: slots become QML property declarations --------

    /// Each `SlotDecl` becomes one `property <type> <camelName>: <default>`
    /// line on the root `Item`. The slot names (kebab-case) are converted
    /// to camelCase to match the QML JS-style property convention.
    #[test]
    fn slots_lower_to_qml_property_declarations() {
        let m = component(
            "Card",
            vec![
                slot("display-name", SlotType::Text, true),
                slot("avatar-url", SlotType::Image, true),
                slot("is-active", SlotType::Bool, false),
            ],
            vec![],
        );
        let result = from_pipeline(&m, &single_box_layout("Card"), &empty_style("Card")).unwrap();
        assert!(
            result.output.contains("property string displayName: \"\""),
            "missing displayName property in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property url avatarUrl: \"\""),
            "missing avatarUrl property in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property bool isActive: false"),
            "missing isActive property in:\n{}",
            result.output
        );
    }

    // -------- Test 3: each slot type maps to the right QML primitive --------

    /// Sanity check for the type table: number → real, color → color,
    /// list<T> → var, node → Component, Component(X) → Component.
    #[test]
    fn slot_types_map_to_qml_types() {
        let m = component(
            "Misc",
            vec![
                slot("count", SlotType::Number, true),
                slot("tint", SlotType::Color, true),
                slot("items", SlotType::List(Box::new(ListInnerType::Text)), true),
                slot("body", SlotType::Node, true),
                slot("child", SlotType::Component("Avatar".to_string()), true),
            ],
            vec![],
        );
        let result = from_pipeline(&m, &single_box_layout("Misc"), &empty_style("Misc")).unwrap();
        assert!(result.output.contains("property real count: 0"));
        assert!(result.output.contains("property color tint: \"#000000\""));
        assert!(result.output.contains("property var items: []"));
        assert!(result.output.contains("property Component body: null"));
        assert!(result.output.contains("property Component child: null"));
    }

    // -------- Test 4: emits become QML signal declarations --------

    /// Every `EmitDecl` becomes one `signal <name>(...)` line. The `on`
    /// prefix is stripped (UI24 §5) before camelCasing the first letter,
    /// so `onNavigate` → `signal navigate()`.
    #[test]
    fn emits_lower_to_qml_signals_with_on_prefix_stripped() {
        let m = component(
            "Grid",
            vec![],
            vec![
                emit_decl("onNavigate", vec![]),
                emit_decl("onEditCommit", vec![]),
            ],
        );
        let result = from_pipeline(&m, &single_box_layout("Grid"), &empty_style("Grid")).unwrap();
        assert!(
            result.output.contains("signal navigate()"),
            "missing 'signal navigate()' in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("signal editCommit()"),
            "missing 'signal editCommit()' in:\n{}",
            result.output
        );
        assert!(result.output.contains("signal mosaicEvent(var event)"));
        assert!(
            result
                .output
                .contains("onNavigate: function() { mosaicEvent({ \"event\": \"onNavigate\" }) }"),
            "missing generic navigate Mosaic event in:\n{}",
            result.output
        );
        assert!(
            result.output.contains(
                "onEditCommit: function() { mosaicEvent({ \"event\": \"onEditCommit\" }) }"
            ),
            "missing generic editCommit Mosaic event in:\n{}",
            result.output
        );
    }

    // -------- Test 5: parameterised emits produce typed signal params --------

    /// Emit params with kebab-case names lower to camelCase QML params,
    /// each carrying a typed prefix (`int`/`real`/`string`/...). The
    /// number type lowers to QML's `real`.
    #[test]
    fn emit_params_become_typed_signal_parameters() {
        let m = component(
            "Grid",
            vec![],
            vec![emit_decl(
                "onSelect",
                vec![
                    param("start-row", EmitPayloadType::Number),
                    param("start-col", EmitPayloadType::Number),
                ],
            )],
        );
        let result = from_pipeline(&m, &single_box_layout("Grid"), &empty_style("Grid")).unwrap();
        assert!(
            result
                .output
                .contains("signal select(real startRow, real startCol)"),
            "expected typed params, got:\n{}",
            result.output
        );
        assert!(
            result.output.contains(
                "onSelect: function(startRow, startCol) { mosaicEvent({ \"event\": \"onSelect\", \"startRow\": startRow, \"startCol\": startCol }) }"
            ),
            "expected payload to flow into generic Mosaic event, got:\n{}",
            result.output
        );
    }

    // -------- Test 6: kebab→camel conversion for slots and signals --------

    /// Explicit test for the kebab→camel conversion, covering both the
    /// property and the signal halves and a multi-segment kebab name
    /// (`a11y-label-text` → `a11yLabelText`).
    #[test]
    fn kebab_names_convert_to_camel_case() {
        let m = component(
            "X",
            vec![slot("a11y-label-text", SlotType::Text, true)],
            vec![emit_decl(
                "onUserClickedHere",
                vec![param("event-row", EmitPayloadType::Number)],
            )],
        );
        let result = from_pipeline(&m, &single_box_layout("X"), &empty_style("X")).unwrap();
        assert!(result.output.contains("a11yLabelText"));
        assert!(result
            .output
            .contains("signal userClickedHere(real eventRow)"));
        assert!(result
            .output
            .contains("onUserClickedHere: function(eventRow) { mosaicEvent({ \"event\": \"onUserClickedHere\", \"eventRow\": eventRow }) }"));
    }

    // -------- Test 7: Row → RowLayout, Column → ColumnLayout --------

    /// Both moslayout container primitives lower to their QtQuick.Layouts
    /// counterparts. A `Row` containing a `Box` renders as
    /// `RowLayout { Item { } }`.
    #[test]
    fn row_and_column_lower_to_layout_managers() {
        // Row
        let m = component("X", vec![], vec![]);
        let row_layout = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Box".to_string(),
                    part_name: None,
                    props: Vec::new(),
                    children: Vec::new(),
                }],
            },
        };
        let row_result = from_pipeline(&m, &row_layout, &empty_style("X")).unwrap();
        assert!(
            row_result.output.contains("RowLayout {"),
            "got:\n{}",
            row_result.output
        );

        // Column
        let col_layout = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let col_result = from_pipeline(&m, &col_layout, &empty_style("X")).unwrap();
        assert!(
            col_result.output.contains("ColumnLayout {"),
            "got:\n{}",
            col_result.output
        );
    }

    // -------- Test 8: Text content from slot ref --------

    /// `Text { content: slot: display-name }` lowers to a `Text { text:
    /// displayName }` element — note the *bare identifier* binding, not a
    /// quoted literal. QML resolves the bare name against the surrounding
    /// `property` scope (the wrapping `Item`).
    #[test]
    fn text_content_slot_ref_emits_bare_identifier_binding() {
        let m = component(
            "Label",
            vec![slot("display-name", SlotType::Text, true)],
            vec![],
        );
        let l = LayoutDef {
            component_name: "Label".to_string(),
            root: LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::SlotRef("display-name".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("Label")).unwrap();
        assert!(
            result.output.contains("Text {"),
            "missing Text element in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: displayName"),
            "missing slot-ref text binding in:\n{}",
            result.output
        );
        // It must NOT be a quoted literal.
        assert!(
            !result.output.contains("text: \"displayName\""),
            "slot ref must be bare identifier, not string"
        );
    }

    // -------- Test 9: Text content from string literal --------

    /// `Text { content: "Hello" }` lowers to `text: "Hello"` (with the
    /// quoting). String values are escaped per QML rules so authors can
    /// safely use embedded `"` or `\`.
    #[test]
    fn text_content_string_literal_emits_quoted_text() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::String("Hello \"world\"".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("text: \"Hello \\\"world\\\"\""),
            "missing escaped string literal in:\n{}",
            result.output
        );
    }

    // -------- Test 10: Image source binding --------

    /// `Image { source: "/path/img.png" }` lowers to `Image { source:
    /// "/path/img.png" }`. Slot-ref `source` also works (bare identifier).
    #[test]
    fn image_source_lowers_to_qml_source_attribute() {
        let m = component("X", vec![slot("photo-url", SlotType::Image, true)], vec![]);
        // String-literal source
        let l_str = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Image".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "source".to_string(),
                    value: LayoutPropValue::String("/path/img.png".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r_str = from_pipeline(&m, &l_str, &empty_style("X")).unwrap();
        assert!(r_str.output.contains("Image {"));
        assert!(r_str.output.contains("source: \"/path/img.png\""));

        // Slot-ref source
        let l_slot = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Image".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "source".to_string(),
                    value: LayoutPropValue::SlotRef("photo-url".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r_slot = from_pipeline(&m, &l_slot, &empty_style("X")).unwrap();
        assert!(r_slot.output.contains("source: photoUrl"));
    }

    // -------- Test 11: Spacer carries fillWidth + fillHeight --------

    /// A `Spacer` lowers to an `Item` with `Layout.fillWidth: true` and
    /// `Layout.fillHeight: true` — the layout primitive uses whichever
    /// matches its growth axis.
    #[test]
    fn spacer_lowers_to_layout_filled_item() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Spacer".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(result.output.contains("Layout.fillWidth: true"));
        assert!(result.output.contains("Layout.fillHeight: true"));
    }

    // -------- Test 12: Divider lowers to a thin Rectangle --------

    /// A `Divider` lowers to a 1-pixel `Rectangle` with a neutral grey
    /// colour, taking the full width of its parent layout.
    #[test]
    fn divider_lowers_to_one_px_rectangle() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Divider".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(result.output.contains("Rectangle {"));
        assert!(result.output.contains("height: 1"));
        assert!(result.output.contains("color: \"#888\""));
        assert!(result.output.contains("Layout.fillWidth: true"));
    }

    // -------- Test 13: nested Row/Column tree --------

    /// Containers nest: `Column { Row { Text { content: "Hi" } } }`
    /// emits `ColumnLayout { RowLayout { Text { text: "Hi" } } }` with
    /// monotonically increasing indentation. Verify both the elements
    /// and the source-order containment by string position.
    #[test]
    fn nested_containers_emit_indented_qml_tree() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Row".to_string(),
                    part_name: None,
                    props: Vec::new(),
                    children: vec![LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("Hi".to_string()),
                        }],
                        children: Vec::new(),
                    }],
                }],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let col = result.output.find("ColumnLayout {").expect("ColumnLayout");
        let row = result.output.find("RowLayout {").expect("RowLayout");
        let txt = result.output.find("Text {").expect("Text");
        assert!(
            col < row && row < txt,
            "nesting order broken in:\n{}",
            result.output
        );
        assert!(result.output.contains("text: \"Hi\""));
    }

    // -------- Test 14: name-mismatch error --------

    /// When the mosmodel component name and the moslayout component name
    /// disagree, the emitter returns
    /// `PipelineEmitError::ComponentNameMismatch` rather than producing
    /// output. This is almost always a manifest authoring error.
    #[test]
    fn name_mismatch_between_iface_and_layout_errors() {
        let m = component("Foo", vec![], vec![]);
        let l = single_box_layout("Bar");
        let s = empty_style("Foo");
        let err = from_pipeline(&m, &l, &s).expect_err("should error on mismatch");
        match err {
            PipelineEmitError::ComponentNameMismatch {
                mosmodel,
                moslayout,
            } => {
                assert_eq!(mosmodel, "Foo");
                assert_eq!(moslayout, "Bar");
            }
            other => panic!("expected ComponentNameMismatch, got {other:?}"),
        }
    }

    // -------- Test 15: unknown-primitive error --------

    /// A moslayout node with a tag that is not in the supported primitive
    /// table is rejected with `UnknownPrimitive`. We do NOT silently fall
    /// back to a default `Item` — that would lose layout semantics.
    #[test]
    fn unknown_primitive_yields_clear_error() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "WhatIsThis".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let err =
            from_pipeline(&m, &l, &empty_style("X")).expect_err("unknown primitive must error");
        assert!(matches!(err, PipelineEmitError::UnknownPrimitive(_)));
    }

    // -------- Test 16: version --------

    /// The crate is at `0.2.0`. A trivial test, but it pairs with the
    /// CHANGELOG entry: tag bumps in the changelog without a matching
    /// `version` bump in `Cargo.toml` get caught here.
    #[test]
    fn version_is_0_2_0() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.2.0");
    }

    // -------- Test 17: imports always come before the Item --------

    /// Structural invariant: both `import` lines must appear before the
    /// root `Item {` opener. QML's parser tolerates blank lines between
    /// imports and the root element, but rejects imports *after* the
    /// root.
    #[test]
    fn imports_precede_root_item() {
        let m = component("X", vec![], vec![]);
        let result = from_pipeline(&m, &single_box_layout("X"), &empty_style("X")).unwrap();
        let qq = result.output.find("import QtQuick 2.15").expect("qtquick");
        let qql = result
            .output
            .find("import QtQuick.Layouts 1.15")
            .expect("qtquick.layouts");
        let item = result.output.find("Item {").expect("Item");
        assert!(qq < item && qql < item, "imports must precede Item");
    }

    // =================================================================
    // UI29 primitive kernel (U29-K-qt): Stack, HostInput, HostButton,
    // HostScroll, HostTable. The remaining two primitives — `If` and
    // `For` — wait on the U29-G1..U29-G2 grammar work and are not
    // covered here.
    // =================================================================

    // -------- Test 18: Stack lowers to Item with child anchors --------

    /// UI29 §3 / §4: `Stack` is a Z-axis overlay container. QML has no
    /// dedicated `ZStack`; the idiomatic shape is an `Item` whose
    /// children each set `anchors.fill: parent`. Verifies the wrapper
    /// is an `Item` and each child block carries the anchor line.
    #[test]
    fn stack_lowers_to_item_with_anchors_fill_on_children() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Stack".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Box".to_string(),
                        part_name: None,
                        props: Vec::new(),
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("overlay".to_string()),
                        }],
                        children: Vec::new(),
                    },
                ],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // Stack wrapper is an `Item` (the root Item already exists; we
        // expect *another* `Item {` for the Stack itself plus one for
        // the Box child).
        let item_count = result.output.matches("Item {").count();
        assert!(
            item_count >= 3,
            "expected root Item + Stack Item + Box Item, got:\n{}",
            result.output
        );
        // Each child must carry `anchors.fill: parent` — there are two
        // children so we expect exactly two occurrences.
        let anchor_count = result.output.matches("anchors.fill: parent").count();
        assert_eq!(
            anchor_count, 2,
            "expected 2 anchors.fill, got {anchor_count} in:\n{}",
            result.output
        );
    }

    // -------- Test 19: HostInput with value + read-only --------

    /// `HostInput` with both a `value: slot:` and `read-only: slot:`
    /// prop emits `TextInput { text: <camel>; readOnly: <camel> }`. The
    /// slot refs become bare identifier bindings (no quotes).
    #[test]
    fn host_input_with_value_and_read_only_emits_textinput() {
        let m = component(
            "X",
            vec![
                slot("user-text", SlotType::Text, true),
                slot("locked", SlotType::Bool, false),
            ],
            vec![],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: Some("address-input".to_string()),
                props: vec![
                    LayoutProp {
                        name: "value".to_string(),
                        value: LayoutPropValue::SlotRef("user-text".to_string()),
                    },
                    LayoutProp {
                        name: "read-only".to_string(),
                        value: LayoutPropValue::SlotRef("locked".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("TextInput {"),
            "missing TextInput in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("objectName: \"address-input\""),
            "missing part-backed object name in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: userText"),
            "missing text binding in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("readOnly: locked"),
            "missing readOnly binding in:\n{}",
            result.output
        );
    }

    #[test]
    fn host_input_with_placeholder_string_emits_textfield_placeholder() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "placeholder".to_string(),
                    value: LayoutPropValue::String("Search cards".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("TextField {"),
            "placeholder-backed HostInput must lower to TextField:\n{}",
            result.output
        );
        assert!(
            result.output.contains("placeholderText: \"Search cards\""),
            "missing placeholderText in:\n{}",
            result.output
        );
        assert!(
            !result.output.contains("// placeholder:"),
            "placeholder should render, not remain a comment:\n{}",
            result.output
        );
    }

    #[test]
    fn host_input_with_placeholder_slot_emits_placeholder_binding() {
        let m = component(
            "X",
            vec![slot("search-placeholder", SlotType::Text, true)],
            vec![],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "placeholder".to_string(),
                    value: LayoutPropValue::SlotRef("search-placeholder".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("TextField {"),
            "slot placeholder HostInput must lower to TextField:\n{}",
            result.output
        );
        assert!(
            result.output.contains("placeholderText: searchPlaceholder"),
            "missing slot placeholder binding in:\n{}",
            result.output
        );
    }

    // -------- Test 20a: HostInput onCommit (parameterless signal) --------

    /// `onCommit: emit: onSubmit` where `onSubmit` is declared
    /// parameterless lowers to `onAccepted: submit()` — QML signals
    /// are arity-strict, so a parameterless signal must be invoked
    /// with no args.  This is the shape used by the VisiCalc
    /// `FormulaBar`'s `onCommit ;` declaration; the prior version
    /// of the emitter unconditionally passed `(text)` and produced
    /// "too many arguments" at runtime.
    #[test]
    fn host_input_on_commit_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onSubmit", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onCommit".to_string(),
                    value: LayoutPropValue::EmitRef("onSubmit".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onAccepted: submit()"),
            "missing parameterless onAccepted in:\n{}",
            result.output
        );
        assert!(
            !result.output.contains("onAccepted: submit(text)"),
            "should NOT pass text to a zero-arity signal:\n{}",
            result.output
        );
    }

    // -------- Test 20b: HostInput onCommit (one-param signal) --------

    /// When the signal declares one parameter
    /// (e.g. `emit onSubmit ( value : text )`), the codegen passes
    /// `text` — QML's `TextInput.text` is in scope inside the
    /// signal handler.  Mirrors the React backend's
    /// `(e) => emit({value: e.target.value})` pattern.
    #[test]
    fn host_input_on_commit_one_param_passes_text() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onSubmit",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onCommit".to_string(),
                    value: LayoutPropValue::EmitRef("onSubmit".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onAccepted: submit(text)"),
            "missing onAccepted: submit(text) in:\n{}",
            result.output
        );
    }

    // -------- Test 20c: HostInput onChange (one-param signal) --------

    /// Matches the VisiCalc `FormulaBar` shape:
    /// `emit onFormulaChange ( value : text ) ;` plus
    /// `HostInput { onChange: emit: onFormulaChange }`.  Must lower
    /// to `onTextChanged: formulaChange(text)` — invoking with no
    /// args would error "Insufficient arguments" at runtime.
    #[test]
    fn host_input_on_change_one_param_passes_text() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onFormulaChange",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onChange".to_string(),
                    value: LayoutPropValue::EmitRef("onFormulaChange".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onTextChanged: formulaChange(text)"),
            "missing onTextChanged: formulaChange(text) in:\n{}",
            result.output
        );
    }

    // -------- Tests 20d–20g: extend signal-arity respect to other host emitters --------

    /// HostButton's `onTap` carries no payload by Qt convention
    /// (`Button.clicked()` has no parameters).  Authors who declare
    /// a parameterless `emit onTap ;` get `onClicked: x()` — verify
    /// no extra args slip in.
    #[test]
    fn host_button_on_tap_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onTap", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onTap".to_string(),
                    value: LayoutPropValue::EmitRef("onTap".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(r.output.contains("onClicked: tap()"));
        assert!(!r.output.contains("onClicked: tap(text)"));
    }

    /// HostCheckbox's `onToggle` invocation must follow the
    /// signal's declared arity.  Parameterless → `onToggled: x()`.
    #[test]
    fn host_button_on_click_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onClick", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onClick".to_string(),
                    value: LayoutPropValue::EmitRef("onClick".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(r.output.contains("onClicked: click()"));
        assert!(!r.output.contains("onClicked: click(text)"));
    }

    #[test]
    fn host_button_inside_indexed_for_dispatches_index_payload() {
        let m = component(
            "ListGroup",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit_decl(
                "onSelect",
                vec![param("index", EmitPayloadType::Number)],
            )],
        );
        let l = LayoutDef {
            component_name: "ListGroup".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "For".to_string(),
                    part_name: None,
                    props: vec![
                        lp("each", LayoutPropValue::SlotRef("items".to_string())),
                        lp("as", LayoutPropValue::Keyword("item".to_string())),
                        lp("index", LayoutPropValue::Keyword("i".to_string())),
                    ],
                    children: vec![LayoutNode {
                        tag: "HostButton".to_string(),
                        part_name: None,
                        props: vec![
                            lp("label", LayoutPropValue::Keyword("item".to_string())),
                            lp("onClick", LayoutPropValue::EmitRef("onSelect".to_string())),
                        ],
                        children: Vec::new(),
                    }],
                }],
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("ListGroup")).unwrap();
        assert!(r.output.contains("property int i: index"));
        assert!(
            r.output.contains("onClicked: select(i)"),
            "expected HostButton to dispatch index payload, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("text: item"),
            "expected HostButton label to use For item binding, got:\n{}",
            r.output
        );
    }

    #[test]
    fn host_button_inside_kebab_named_for_camel_cases_payload_bindings() {
        let m = component(
            "NoteEditor",
            vec![slot(
                "note-type-names",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit_decl(
                "onSelectNoteType",
                vec![param("index", EmitPayloadType::Number)],
            )],
        );
        let l = LayoutDef {
            component_name: "NoteEditor".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "For".to_string(),
                    part_name: None,
                    props: vec![
                        lp(
                            "each",
                            LayoutPropValue::SlotRef("note-type-names".to_string()),
                        ),
                        lp("as", LayoutPropValue::Keyword("note-type".to_string())),
                        lp(
                            "index",
                            LayoutPropValue::Keyword("note-type-index".to_string()),
                        ),
                    ],
                    children: vec![LayoutNode {
                        tag: "HostButton".to_string(),
                        part_name: None,
                        props: vec![
                            lp("label", LayoutPropValue::Keyword("note-type".to_string())),
                            lp(
                                "onClick",
                                LayoutPropValue::EmitRef("onSelectNoteType".to_string()),
                            ),
                        ],
                        children: Vec::new(),
                    }],
                }],
            },
        };

        let r = from_pipeline(&m, &l, &empty_style("NoteEditor")).unwrap();
        assert!(r.output.contains("property var noteType: modelData"));
        assert!(r.output.contains("property int noteTypeIndex: index"));
        assert!(
            r.output
                .contains("onClicked: selectNoteType(noteTypeIndex)"),
            "expected HostButton to dispatch camelCased index payload, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("text: noteType"),
            "expected HostButton label to use camelCased item binding, got:\n{}",
            r.output
        );
    }

    #[test]
    fn host_button_inside_for_dispatches_text_item_payload() {
        let m = component(
            "SelectMenu",
            vec![slot(
                "options",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit_decl(
                "onChange",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = LayoutDef {
            component_name: "SelectMenu".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "For".to_string(),
                    part_name: None,
                    props: vec![
                        lp("each", LayoutPropValue::SlotRef("options".to_string())),
                        lp("as", LayoutPropValue::Keyword("option".to_string())),
                    ],
                    children: vec![LayoutNode {
                        tag: "HostButton".to_string(),
                        part_name: None,
                        props: vec![
                            lp("label", LayoutPropValue::Keyword("option".to_string())),
                            lp("onClick", LayoutPropValue::EmitRef("onChange".to_string())),
                        ],
                        children: Vec::new(),
                    }],
                }],
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("SelectMenu")).unwrap();
        assert!(
            r.output.contains("onClicked: change(option)"),
            "expected HostButton to dispatch item payload, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("text: option"),
            "expected HostButton label to use For item binding, got:\n{}",
            r.output
        );
    }

    #[test]
    fn host_checkbox_on_toggle_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onToggle", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostCheckbox".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onToggle".to_string(),
                    value: LayoutPropValue::EmitRef("onToggle".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("onToggled: toggle()"),
            "missing onToggled: toggle() in:\n{}",
            r.output
        );
        assert!(!r.output.contains("onToggled: toggle(checked)"));
    }

    /// HostCheckbox with a one-param signal still emits the
    /// `checked` payload (the QML signal-handler scope's natural
    /// boolean).
    #[test]
    fn host_checkbox_on_toggle_one_param_passes_checked() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onToggle",
                vec![param("value", EmitPayloadType::Bool)],
            )],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostCheckbox".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onToggle".to_string(),
                    value: LayoutPropValue::EmitRef("onToggle".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(r.output.contains("onToggled: toggle(checked)"));
    }

    /// HostRadio with a parameterless `onSelect` must NOT pass the
    /// value payload (which is the natural Qt fallback).
    #[test]
    fn host_radio_on_select_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onSelect", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostRadio".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "value".to_string(),
                        value: LayoutPropValue::String("vanilla".to_string()),
                    },
                    LayoutProp {
                        name: "onSelect".to_string(),
                        value: LayoutPropValue::EmitRef("onSelect".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("onCheckedChanged: if (checked) select()"),
            "missing parameterless onCheckedChanged dispatch in:\n{}",
            r.output
        );
    }

    /// HostNumberInput's `onChange` must respect arity too.
    #[test]
    fn host_number_input_on_change_parameterless_emits_no_args() {
        let m = component("X", vec![], vec![emit_decl("onChange", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostNumberInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onChange".to_string(),
                    value: LayoutPropValue::EmitRef("onChange".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(r.output.contains("onTextEdited: {"));
        assert!(r
            .output
            .contains("if (text.length > 0 && !isNaN(nextValue)) { change() }"));
        assert!(!r.output.contains("change(nextValue)"));
    }

    // -------- Test 21: HostInput with onCancel emits Keys.onEscapePressed --------

    /// `onCancel: emit: onAbort` lowers to a `Keys.onEscapePressed`
    /// block that calls the signal and marks the event accepted (the
    /// `event.accepted = true` line prevents QML's default Escape
    /// handling — e.g. closing a parent dialog — from also firing).
    #[test]
    fn host_input_on_cancel_emits_keys_escape_pressed() {
        let m = component("X", vec![], vec![emit_decl("onAbort", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onCancel".to_string(),
                    value: LayoutPropValue::EmitRef("onAbort".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("Keys.onEscapePressed:"),
            "missing Keys.onEscapePressed in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("abort()"),
            "missing abort() call in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("event.accepted = true"),
            "missing event.accepted in:\n{}",
            result.output
        );
    }

    // -------- Test 22: HostButton with label + onTap --------

    /// `HostButton` lowers to `Button { text: ...; onClicked: ... }`
    /// from `QtQuick.Controls 2.15`. The `label` prop maps to QML's
    /// `text` (the canonical Button label property in Controls 2.x);
    /// `onTap` maps to `onClicked`.
    #[test]
    fn host_button_with_label_and_on_tap_emits_button_block() {
        let m = component("X", vec![], vec![emit_decl("onSave", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "label".to_string(),
                        value: LayoutPropValue::String("Save".to_string()),
                    },
                    LayoutProp {
                        name: "onTap".to_string(),
                        value: LayoutPropValue::EmitRef("onSave".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("Button {"),
            "missing Button in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"Save\""),
            "missing label text in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("onClicked: save()"),
            "missing onClicked in:\n{}",
            result.output
        );
    }

    #[test]
    fn host_button_with_label_and_on_click_emits_button_block() {
        let m = component("X", vec![], vec![emit_decl("onSave", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "label".to_string(),
                        value: LayoutPropValue::String("Save".to_string()),
                    },
                    LayoutProp {
                        name: "onClick".to_string(),
                        value: LayoutPropValue::EmitRef("onSave".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("Button {"),
            "missing Button in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"Save\""),
            "missing label text in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("onClicked: save()"),
            "missing onClicked in:\n{}",
            result.output
        );
    }

    #[test]
    fn host_button_with_part_style_emits_native_button_style() {
        let style = StyleDef {
            component_name: "X".to_string(),
            parts: vec![mosstyle_compiler::PartStyle {
                name: "danger".to_string(),
                base: vec![
                    StyleProp {
                        name: "background".to_string(),
                        value: "#f87171".to_string(),
                    },
                    StyleProp {
                        name: "color".to_string(),
                        value: "#ffffff".to_string(),
                    },
                    StyleProp {
                        name: "padding".to_string(),
                        value: "10px".to_string(),
                    },
                    StyleProp {
                        name: "border-color".to_string(),
                        value: "#7f1d1d".to_string(),
                    },
                    StyleProp {
                        name: "border-width".to_string(),
                        value: "2px".to_string(),
                    },
                    StyleProp {
                        name: "border-radius".to_string(),
                        value: "7px".to_string(),
                    },
                ],
                transitions: vec![],
                states: vec![],
            }],
        };
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: Some("danger".to_string()),
                props: vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::String("Again".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &style).unwrap();
        assert!(
            result.output.contains("objectName: \"danger\""),
            "missing part-backed object name in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("palette.buttonText: \"#ffffff\""),
            "missing foreground style in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("background: Rectangle {"),
            "missing styled background in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("color: \"#f87171\""),
            "missing background color in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("radius: 7"),
            "missing radius in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("border.color: \"#7f1d1d\""),
            "missing border color in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("border.width: 2"),
            "missing border width in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("leftPadding: 10"),
            "missing padding in:\n{}",
            result.output
        );
    }

    // -------- Test 23: HostButton with disabled flips to enabled --------

    /// `disabled: slot: x` lowers to `enabled: !x` — the polarity flip
    /// happens at lowering time because QML uses `enabled` (positive)
    /// rather than `disabled`. Verifies both a slot-ref and a literal
    /// `true` flow correctly.
    #[test]
    fn host_button_disabled_lowers_to_enabled_negated() {
        let m = component("X", vec![slot("is-saving", SlotType::Bool, false)], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::SlotRef("is-saving".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("enabled: !isSaving"),
            "missing 'enabled: !isSaving' in:\n{}",
            result.output
        );
    }

    // -------- Test 24: HostScroll emits ScrollView --------

    /// `HostScroll` wraps its children in a `ScrollView { ... }`. The
    /// children continue to lower normally inside the wrapper; the
    /// only structural change is the outer element.
    #[test]
    fn host_scroll_emits_scroll_view_wrapper() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostScroll".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Column".to_string(),
                    part_name: None,
                    props: Vec::new(),
                    children: vec![LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("Row 1".to_string()),
                        }],
                        children: Vec::new(),
                    }],
                }],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("ScrollView {"),
            "missing ScrollView wrapper in:\n{}",
            result.output
        );
        // Children still lower normally.
        assert!(
            result.output.contains("ColumnLayout {"),
            "child ColumnLayout missing in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"Row 1\""),
            "nested Text missing in:\n{}",
            result.output
        );
    }

    // -------- Test 25: QtQuick.Controls import added when needed --------

    /// The `QtQuick.Controls 2.15` import is added conditionally:
    /// only when the layout tree references a primitive that lowers
    /// to a Controls element. `HostButton` triggers it; a tree
    /// without any Host*-Controls primitive should NOT emit the
    /// import (keeping the dependency surface minimal).
    #[test]
    fn controls_import_added_only_when_button_used() {
        // With HostButton: import expected.
        let m = component("X", vec![], vec![emit_decl("onPress", vec![])]);
        let l_with = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::String("Go".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r_with = from_pipeline(&m, &l_with, &empty_style("X")).unwrap();
        assert!(
            r_with.output.contains("import QtQuick.Controls 2.15"),
            "Controls import missing when HostButton present in:\n{}",
            r_with.output
        );

        // HostInput with a placeholder lowers to TextField, so it also
        // needs QtQuick.Controls.
        let l_placeholder = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "placeholder".to_string(),
                    value: LayoutPropValue::String("Search".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r_placeholder = from_pipeline(&m, &l_placeholder, &empty_style("X")).unwrap();
        assert!(
            r_placeholder
                .output
                .contains("import QtQuick.Controls 2.15"),
            "Controls import missing when HostInput uses placeholder/TextField:\n{}",
            r_placeholder.output
        );

        // Without any Host*-Controls primitive: NO Controls import.
        let m2 = component("Y", vec![], vec![]);
        let r_without = from_pipeline(&m2, &single_box_layout("Y"), &empty_style("Y")).unwrap();
        assert!(
            !r_without.output.contains("import QtQuick.Controls"),
            "Controls import must NOT appear when no Host*-Controls primitive used:\n{}",
            r_without.output
        );
    }

    // -------- HostTable tests (U29-K-qt §HostTable) --------

    /// Build a `HostTable` layout with the given children. Used as the
    /// test fixture for the HostTable suite.
    fn host_table(children: Vec<LayoutNode>) -> LayoutDef {
        LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostTable".to_string(),
                part_name: None,
                props: Vec::new(),
                children,
            },
        }
    }

    /// Build a `Text` cell with a literal string content. Saves typing.
    fn text_cell(s: &str) -> LayoutNode {
        LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String(s.to_string()),
            }],
            children: Vec::new(),
        }
    }

    /// Build a `Row` of `Text` cells. The HostTable sub-tag grammar
    /// expects rows inside sections.
    fn row_of(cells: Vec<&str>) -> LayoutNode {
        LayoutNode {
            tag: "Row".to_string(),
            part_name: None,
            props: Vec::new(),
            children: cells.into_iter().map(text_cell).collect(),
        }
    }

    /// Build a section sub-tag (HostTableHead/Body/Foot) wrapping rows.
    fn section(tag: &str, rows: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props: Vec::new(),
            children: rows,
        }
    }

    /// Count substring occurrences (helper for "exactly N divider Rectangles"
    /// style assertions).
    fn count_occurrences(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    // -------- Test 27: empty HostTable emits ColumnLayout skeleton --------

    /// An empty `HostTable` (no sections) lowers to a bare
    /// `ColumnLayout { spacing: 0 }`. No rows, no divider — but the
    /// container is present so downstream styling can still target it.
    #[test]
    fn empty_host_table_emits_column_layout_skeleton() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("ColumnLayout {"),
            "missing ColumnLayout in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("spacing: 0"),
            "missing spacing: 0 in:\n{}",
            result.output
        );
        assert!(
            !result.output.contains("RowLayout {"),
            "unexpected RowLayout in empty HostTable:\n{}",
            result.output
        );
        // No divider Rectangle for an empty table.
        assert!(
            !result.output.contains("height: 1"),
            "unexpected divider in empty HostTable:\n{}",
            result.output
        );
    }

    // -------- Test 28: HostTableHead emits bold RowLayout cells --------

    /// `HostTableHead` rows lower to `RowLayout`s whose `Text` children
    /// each carry `font.bold: true`. Verifies both presence of the
    /// `RowLayout` and the bold attribute on the text cells.
    #[test]
    fn host_table_head_emits_bold_row_layout() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![section("HostTableHead", vec![row_of(vec!["A", "B"])])]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("RowLayout {"),
            "missing RowLayout in head:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"A\""),
            "missing first header cell text:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"B\""),
            "missing second header cell text:\n{}",
            result.output
        );
        // Both header cells must be bold — expect exactly 2 occurrences.
        assert_eq!(
            count_occurrences(&result.output, "font.bold: true"),
            2,
            "expected 2 font.bold lines in:\n{}",
            result.output
        );
    }

    // -------- Test 29: HostTableBody emits plain RowLayouts --------

    /// `HostTableBody` rows lower to `RowLayout { Text {...} ... }`
    /// without the bold flag. With no head, no divider should appear.
    #[test]
    fn host_table_body_emits_plain_row_layout() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![section(
            "HostTableBody",
            vec![row_of(vec!["1", "2"]), row_of(vec!["3", "4"])],
        )]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // Two body rows → two RowLayouts.
        assert_eq!(
            count_occurrences(&result.output, "RowLayout {"),
            2,
            "expected 2 RowLayouts in:\n{}",
            result.output
        );
        // No bolding outside head sections.
        assert!(
            !result.output.contains("font.bold: true"),
            "unexpected bold cell in body-only table:\n{}",
            result.output
        );
        // No divider Rectangle without a head/foot section.
        assert!(
            !result.output.contains("height: 1"),
            "unexpected divider with body-only table:\n{}",
            result.output
        );
    }

    // -------- Test 30: HostTableFoot is preceded by a divider --------

    /// `HostTableFoot` lowers to a `RowLayout` *preceded by* a 1px
    /// `Rectangle` divider. Verifies the divider appears literally
    /// before the foot's RowLayout in the output.
    #[test]
    fn host_table_foot_is_preceded_by_divider() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![section(
            "HostTableFoot",
            vec![row_of(vec!["total", "100"])],
        )]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // Find positions of divider and the foot's RowLayout. Divider
        // (height: 1 line) must come before the RowLayout opening.
        let divider_pos = result.output.find("height: 1").expect("divider not found");
        let row_pos = result
            .output
            .find("RowLayout {")
            .expect("foot RowLayout not found");
        assert!(
            divider_pos < row_pos,
            "divider must precede foot's RowLayout in:\n{}",
            result.output
        );
    }

    // -------- Test 31: Head + Body emit head, divider, then body --------

    /// With both a head and a body, the output order is:
    ///   head's RowLayout → divider Rectangle → body's RowLayout.
    /// Asserts the three structural positions in the output.
    #[test]
    fn host_table_head_then_body_emits_in_order() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![
            section("HostTableHead", vec![row_of(vec!["A", "B"])]),
            section("HostTableBody", vec![row_of(vec!["1", "2"])]),
        ]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let head_pos = result
            .output
            .find("text: \"A\"")
            .expect("head cell not found");
        let divider_pos = result.output.find("height: 1").expect("divider not found");
        let body_pos = result
            .output
            .find("text: \"1\"")
            .expect("body cell not found");
        assert!(
            head_pos < divider_pos && divider_pos < body_pos,
            "expected head < divider < body in:\n{}",
            result.output
        );
        // Exactly one divider between head and body.
        assert_eq!(
            count_occurrences(&result.output, "height: 1"),
            1,
            "expected exactly 1 divider in:\n{}",
            result.output
        );
    }

    // -------- Test 32: HostTableColGroup is ignored with a comment --------

    /// `HostTableColGroup` has no QML analog. It must not break
    /// emission; instead a self-documenting comment is emitted.
    #[test]
    fn host_table_col_group_emits_comment_and_does_not_break() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![
            LayoutNode {
                tag: "HostTableColGroup".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
            section("HostTableBody", vec![row_of(vec!["1"])]),
        ]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("// HostTableColGroup"),
            "missing ColGroup comment in:\n{}",
            result.output
        );
        // The body still renders normally.
        assert!(
            result.output.contains("text: \"1\""),
            "body row missing after ColGroup in:\n{}",
            result.output
        );
    }

    // -------- Test 33: orphan HostTableHead outside HostTable --------

    /// A `HostTableHead` (or any sub-tag) used as a top-level root
    /// outside a `HostTable` parent emits a self-documenting QML
    /// comment rather than erroring. Defensive — the grammar should
    /// prevent this, but the emitter should not crash on malformed
    /// trees.
    #[test]
    fn orphan_host_table_head_emits_comment_no_error() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostTableHead".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("// orphan HostTableHead"),
            "missing orphan comment in:\n{}",
            result.output
        );
        // Critically — no RowLayout or Rectangle escaping from an orphan.
        assert!(
            !result.output.contains("RowLayout {"),
            "orphan must not emit RowLayout in:\n{}",
            result.output
        );
    }

    // -------- Test 34: part_name on HostTable does not break --------

    /// Until styling integration lands for tables, a `part_name` on the
    /// `HostTable` itself is accepted silently — the emitter must not
    /// reject the node. Verifies the output still contains the
    /// `ColumnLayout` skeleton.
    #[test]
    fn host_table_part_name_does_not_break_emission() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostTable".to_string(),
                part_name: Some("data-grid".to_string()),
                props: Vec::new(),
                children: vec![section("HostTableBody", vec![row_of(vec!["x"])])],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("ColumnLayout {"),
            "missing ColumnLayout when part_name present:\n{}",
            result.output
        );
        assert!(
            result.output.contains("RowLayout {"),
            "missing body RowLayout when part_name present:\n{}",
            result.output
        );
    }

    // ================================================================
    // UI31 — HostTable a11y gate + RTL contract (Qt backend)
    //
    // Mirrors the React (#4143), HTML (#4156), WebComponent (#4162),
    // and Flutter (#4166) precedents:
    //
    // - **A11y gate**: the Qt lowering must continue to emit the
    //   semantic ColumnLayout-of-RowLayout shape (which screen
    //   readers + Qt's Accessibility framework see as a structured
    //   table), never collapse to a `Rectangle` of `Text` mess.
    // - **RTL gate**: when `dir:` is authored, the ColumnLayout
    //   carries `LayoutMirroring.enabled: ...` so column ordering
    //   flips for RTL locales. Allow-list is `ltr|rtl|auto`; unknown
    //   keywords drop silently.
    // ================================================================

    /// Helper: build a `HostTable` LayoutDef carrying a `dir:` prop.
    fn host_table_with_dir(value: LayoutPropValue) -> LayoutDef {
        LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostTable".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "dir".to_string(),
                    value,
                }],
                children: vec![section("HostTableBody", vec![row_of(vec!["x"])])],
            },
        }
    }

    /// UI31 §3.1 a11y gate — `HostTable` MUST continue to lower to
    /// the structural `ColumnLayout` + `RowLayout` shape that
    /// preserves the semantic table structure for Qt's accessibility
    /// framework. A regression to a flat `Rectangle` mess would
    /// break screen-reader navigation.
    #[test]
    fn ui31_a11y_host_table_uses_structural_columnlayout_of_rowlayouts() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![section("HostTableBody", vec![row_of(vec!["a"])])]);
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("ColumnLayout {"),
            "HostTable must lower to ColumnLayout, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("RowLayout {"),
            "HostTable body must include RowLayout rows, got:\n{}",
            r.output
        );
    }

    /// UI31 §3.2 RTL contract — `dir: rtl` keyword emits
    /// `LayoutMirroring.enabled: true` plus `childrenInherit: true`
    /// so the flip propagates into the body's `RowLayout` rows.
    #[test]
    fn ui31_rtl_host_table_dir_rtl_keyword_enables_layout_mirroring() {
        let m = component("X", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("rtl".to_string()));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("LayoutMirroring.enabled: true"),
            "expected LayoutMirroring.enabled: true, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("LayoutMirroring.childrenInherit: true"),
            "expected LayoutMirroring.childrenInherit: true, got:\n{}",
            r.output
        );
    }

    /// `dir: ltr` keyword emits the explicit-disable form. Symmetry
    /// with the `rtl` test; the explicit `false` is the right thing
    /// for an author who wants to *override* an ambient RTL
    /// ancestor — without this, `LayoutMirroring` would still
    /// inherit from a parent that set it.
    #[test]
    fn ui31_rtl_host_table_dir_ltr_keyword_explicitly_disables_mirroring() {
        let m = component("X", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("ltr".to_string()));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("LayoutMirroring.enabled: false"),
            "expected LayoutMirroring.enabled: false, got:\n{}",
            r.output
        );
    }

    /// `dir: auto` keyword is the spec-mandated "let the host
    /// decide". QML has no auto enum; the right behaviour is to NOT
    /// emit the attached property so any ancestor's `LayoutMirroring`
    /// flows through unchanged.
    #[test]
    fn ui31_rtl_host_table_dir_auto_keyword_does_not_emit_attached_property() {
        let m = component("X", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("auto".to_string()));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            !r.output.contains("LayoutMirroring"),
            "auto must NOT emit LayoutMirroring, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("ColumnLayout {"),
            "bare ColumnLayout should still render, got:\n{}",
            r.output
        );
    }

    /// `dir: slot: layout-direction` interpolates the bound slot
    /// (camel-cased to `layoutDirection`) into the
    /// `LayoutMirroring.enabled:` binding. The slot is expected to
    /// evaluate to a `bool`. Slot name goes through
    /// `is_safe_identifier` so it can't smuggle malicious QML.
    #[test]
    fn ui31_rtl_host_table_dir_slot_ref_interpolates_camel_case_identifier() {
        let m = component(
            "X",
            vec![slot("layout-direction", SlotType::Bool, true)],
            vec![],
        );
        let l = host_table_with_dir(LayoutPropValue::SlotRef("layout-direction".to_string()));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output
                .contains("LayoutMirroring.enabled: layoutDirection"),
            "expected LayoutMirroring.enabled: layoutDirection, got:\n{}",
            r.output
        );
    }

    /// Unknown `dir:` keywords (anything outside the `ltr|rtl|auto`
    /// allow-list) MUST drop silently — this is the security gate
    /// against attribute-value breakout. An attacker-controlled
    /// keyword cannot inject QML because it never reaches the format
    /// string. Test payload includes a `; pwn()` to nail down the
    /// security claim.
    #[test]
    fn ui31_rtl_host_table_unknown_dir_keyword_drops_silently() {
        let m = component("X", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword(
            "true; Component.onCompleted: pwn()".to_string(),
        ));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            !r.output.contains("pwn()"),
            "unknown keyword payload must not appear, got:\n{}",
            r.output
        );
        assert!(
            !r.output.contains("LayoutMirroring"),
            "unknown keyword must NOT emit LayoutMirroring, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("ColumnLayout {"),
            "bare ColumnLayout should still render, got:\n{}",
            r.output
        );
    }

    /// Regression guard — `HostTable` with no `dir:` prop emits no
    /// `LayoutMirroring` line. A future refactor that always-emits
    /// would break authors who rely on ambient inheritance.
    #[test]
    fn ui31_rtl_host_table_without_dir_prop_emits_no_layout_mirroring() {
        let m = component("X", vec![], vec![]);
        let l = host_table(vec![section("HostTableBody", vec![row_of(vec!["x"])])]);
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            !r.output.contains("LayoutMirroring"),
            "no LayoutMirroring expected when dir absent, got:\n{}",
            r.output
        );
    }

    // -------- Test 26: unknown primitives still produce UnknownPrimitive --------

    /// `If`, `For`, `Else`, and `HostTable` are all now lowered by
    /// dedicated emitters. Verify that the `UnknownPrimitive` error path
    /// still fires for *genuinely* unknown tags so we don't silently
    /// accept malformed input. (`HostTable`/`For`/`If` lowerings have
    /// their own dedicated tests below.)
    #[test]
    fn unknown_primitive_still_errors() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "FlibbertyJibbet".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let err =
            from_pipeline(&m, &l, &empty_style("X")).expect_err("unknown primitive must error");
        assert!(
            matches!(err, PipelineEmitError::UnknownPrimitive(ref t) if t == "FlibbertyJibbet"),
            "expected UnknownPrimitive(FlibbertyJibbet), got {err:?}"
        );
    }
    // =====================================================================
    // U29-K-qt — `For` / `If` / `Else` meta-primitive tests
    // =====================================================================
    //
    // The For/If/Else lowering is described in `emit_for_qml` and
    // `emit_if_qml`. The tests below pin every documented case.

    // ---- Test fixtures for For/If --------

    /// Build a `For` node with `each: slot: <each_slot>`, `as: <as_name>`,
    /// optional `index: <index_name>`, wrapping the given body.
    fn for_with_slot_each(
        each_slot: &str,
        as_name: &str,
        index_name: Option<&str>,
        body: Vec<LayoutNode>,
    ) -> LayoutNode {
        let mut props = vec![
            LayoutProp {
                name: "each".to_string(),
                value: LayoutPropValue::SlotRef(each_slot.to_string()),
            },
            LayoutProp {
                name: "as".to_string(),
                value: LayoutPropValue::Keyword(as_name.to_string()),
            },
        ];
        if let Some(idx) = index_name {
            props.push(LayoutProp {
                name: "index".to_string(),
                value: LayoutPropValue::Keyword(idx.to_string()),
            });
        }
        LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props,
            children: body,
        }
    }

    /// Build an `If` node with `when: slot: <slot>` wrapping the given body.
    fn if_with_slot_when(slot: &str, body: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "If".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "when".to_string(),
                value: LayoutPropValue::SlotRef(slot.to_string()),
            }],
            children: body,
        }
    }

    /// Build a bare `Else` node with the given body.
    fn else_node(body: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "Else".to_string(),
            part_name: None,
            props: Vec::new(),
            children: body,
        }
    }

    // -------- Test 35: For (each: slot) emits Repeater + camelCased model --------

    /// `For (each: slot: viewport-rows, as: row)` lowers to
    /// `Repeater { model: viewportRows; delegate: Item { property var row: modelData; ... } }`.
    /// Verifies the model name is camelCased and the delegate exposes
    /// the `as:` binding as a property.
    #[test]
    fn for_with_slot_each_emits_repeater_with_camel_model() {
        let m = component("X", vec![], vec![]);
        let row = LayoutNode {
            tag: "Row".to_string(),
            part_name: None,
            props: Vec::new(),
            children: vec![for_with_slot_each("viewport-rows", "row", None, vec![])],
        };
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: row,
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("Repeater {"),
            "missing Repeater in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("model: viewportRows"),
            "missing camelCased model in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("delegate: Item {"),
            "missing delegate Item in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property var row: modelData"),
            "missing as-binding property in:\n{}",
            result.output
        );
    }

    // -------- Test 36: For with index: binds both name properties --------

    /// `For (each: slot: rows, as: row, index: r)` produces *both*
    /// delegate properties: `property var row: modelData` and
    /// `property int r: index`. When `index:` is omitted, only the
    /// `as:` property appears.
    #[test]
    fn for_with_index_binds_both_as_and_index_properties() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: for_with_slot_each("rows", "row", Some("r"), vec![]),
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("property var row: modelData"),
            "missing as-binding in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property int r: index"),
            "missing index-binding in:\n{}",
            result.output
        );

        // Negative: dropping index: → only one property line.
        let l2 = LayoutDef {
            component_name: "X".to_string(),
            root: for_with_slot_each("rows", "row", None, vec![]),
        };
        let r2 = from_pipeline(&m, &l2, &empty_style("X")).unwrap();
        assert!(
            !r2.output.contains("property int"),
            "index property should be absent when index: is unbound; got:\n{}",
            r2.output
        );
    }

    // -------- Test 37: For with Expr each emits expression verbatim --------

    /// `For (each: <expr>, as: col)` passes the expression text through
    /// verbatim into `model:`. The `Expr` variant of `LayoutPropValue`
    /// carries the reconstructed source substring (UI29 §3.3); QML's
    /// expression grammar overlaps with JavaScript so member access /
    /// comparisons / boolean ops just pass through.
    #[test]
    fn for_with_expr_each_emits_expression_verbatim() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "For".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "each".to_string(),
                        value: LayoutPropValue::Expr("cols.visible".to_string()),
                    },
                    LayoutProp {
                        name: "as".to_string(),
                        value: LayoutPropValue::Keyword("col".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("model: cols.visible"),
            "missing verbatim Expr in model:\n{}",
            result.output
        );
    }

    // -------- Test 38: For body uses the as-bound name as bare identifier --------

    /// A `Text { content: slot: row }` *inside* a `For (..., as: row)`
    /// body lowers to `text: row` — the delegate `property var row:
    /// modelData` puts the user-chosen name in QML's scope, so the bare
    /// identifier resolves to the per-iteration value. (We treat the
    /// inner reference as a slot ref because the moslayout grammar
    /// reuses the `slot:` prefix for both real slots and `For`-bound
    /// names — see UI29 §3.4.)
    #[test]
    fn for_body_references_as_bound_name() {
        let m = component("X", vec![], vec![]);
        let inner_text = LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::SlotRef("row".to_string()),
            }],
            children: Vec::new(),
        };
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: for_with_slot_each("rows", "row", None, vec![inner_text]),
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("property var row: modelData"),
            "missing delegate property in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: row"),
            "missing bare-identifier text binding in:\n{}",
            result.output
        );
    }

    // -------- Test 39: If with then branch only emits one Loader --------

    /// A bare `If (when: slot: editing) { ... }` with no following
    /// `Else` sibling lowers to a *single* `Loader { active: editing;
    /// sourceComponent: Component { ... } }`. The negated-condition
    /// loader is omitted.
    #[test]
    fn if_with_then_only_emits_single_loader() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: if_with_slot_when(
                "editing",
                vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("on".to_string()),
                    }],
                    children: Vec::new(),
                }],
            ),
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert_eq!(
            count_occurrences(&result.output, "Loader {"),
            1,
            "expected exactly 1 Loader for bare If in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("active: editing"),
            "missing active:editing binding in:\n{}",
            result.output
        );
        assert!(
            !result.output.contains("active: !editing"),
            "unexpected negated active without Else in:\n{}",
            result.output
        );
    }

    // -------- Test 40: If + Else emits two Loaders with inverted active --------

    /// When an `If` is followed by a paired `Else` sibling, the emitter
    /// produces two Loaders: one with `active: <cond>` and one with
    /// `active: !<cond>`. The sibling pairing is performed by
    /// `emit_qml_children` on the enclosing parent.
    #[test]
    fn if_plus_else_emits_two_loaders_with_inverted_active() {
        let m = component("X", vec![], vec![]);
        // Parent Row containing If, Else as adjacent siblings.
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    if_with_slot_when(
                        "editing",
                        vec![LayoutNode {
                            tag: "Text".to_string(),
                            part_name: None,
                            props: vec![LayoutProp {
                                name: "content".to_string(),
                                value: LayoutPropValue::String("yes".to_string()),
                            }],
                            children: Vec::new(),
                        }],
                    ),
                    else_node(vec![LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("no".to_string()),
                        }],
                        children: Vec::new(),
                    }]),
                ],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert_eq!(
            count_occurrences(&result.output, "Loader {"),
            2,
            "expected 2 Loaders for paired If+Else in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("active: editing"),
            "missing then-branch active in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("active: !editing"),
            "missing else-branch negated active in:\n{}",
            result.output
        );
        // The two branch bodies should appear in source order.
        let yes_pos = result
            .output
            .find("text: \"yes\"")
            .expect("missing then body");
        let no_pos = result
            .output
            .find("text: \"no\"")
            .expect("missing else body");
        assert!(
            yes_pos < no_pos,
            "Else body must follow If body in:\n{}",
            result.output
        );
    }

    // -------- Test 41: If with Expr when: emits expression verbatim --------

    /// An `If (when: <expr>) { ... }` writes the expression text
    /// verbatim into `active:`. The negated `active:` for the Else
    /// branch wraps the whole expression in parens so operator
    /// precedence isn't disturbed.
    #[test]
    fn if_with_expr_when_emits_expression_verbatim() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "If".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "when".to_string(),
                            value: LayoutPropValue::Expr("r == editRow".to_string()),
                        }],
                        children: vec![],
                    },
                    else_node(vec![]),
                ],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("active: r == editRow"),
            "missing verbatim Expr active in:\n{}",
            result.output
        );
        // The else's `active:` must wrap the complex expression in parens
        // so the leading `!` binds correctly.
        assert!(
            result.output.contains("active: !(r == editRow)"),
            "missing parenthesised negated active in:\n{}",
            result.output
        );
    }

    // -------- Test 42: Orphan Else emits a QML comment --------

    /// A standalone `Else` at the root (no preceding `If` to pair
    /// with) emits a `// orphan Else (no preceding If)` comment
    /// rather than erroring. Same defensive shape as the orphan
    /// HostTable-sub-tag handling.
    #[test]
    fn orphan_else_emits_comment() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: else_node(vec![]),
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("// orphan Else (no preceding If)"),
            "missing orphan-Else comment in:\n{}",
            result.output
        );
        // Defensive: no Loader should be emitted for an orphan Else.
        assert!(
            !result.output.contains("Loader {"),
            "orphan Else must not emit Loader in:\n{}",
            result.output
        );
    }

    // -------- Test 43: Else not immediately following If is orphan --------

    /// `If { } Text { } Else { }` is **not** a paired chain — the
    /// pairing rule (UI29 §3.2) requires `Else` to *immediately* follow
    /// the `If`. The intervening `Text` breaks the pair, so the `Else`
    /// emits as an orphan comment and the `If` lowers as a single
    /// Loader.
    #[test]
    fn else_not_immediately_after_if_is_orphan() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    if_with_slot_when("editing", vec![]),
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("spacer".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    else_node(vec![]),
                ],
            },
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert_eq!(
            count_occurrences(&result.output, "Loader {"),
            1,
            "expected exactly 1 Loader (the If only) in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("// orphan Else"),
            "missing orphan-Else comment in:\n{}",
            result.output
        );
    }

    // -------- Test 44: Nested For loops work --------

    /// A `For` inside a `For` body produces two nested `Repeater`s,
    /// each with its own `as:` binding on its delegate. The inner
    /// delegate property uses the inner `as:` name; the outer uses
    /// the outer `as:` name. This pins the shared
    /// `emit_qml_children` walk inside the delegate.
    #[test]
    fn nested_for_loops_work() {
        let m = component("X", vec![], vec![]);
        let inner = for_with_slot_each("columns", "col", Some("c"), vec![]);
        let outer = for_with_slot_each("rows", "row", Some("r"), vec![inner]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: outer,
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert_eq!(
            count_occurrences(&result.output, "Repeater {"),
            2,
            "expected 2 Repeaters for nested For in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property var row: modelData"),
            "missing outer as-binding in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property var col: modelData"),
            "missing inner as-binding in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property int r: index"),
            "missing outer index-binding in:\n{}",
            result.output
        );
        assert!(
            result.output.contains("property int c: index"),
            "missing inner index-binding in:\n{}",
            result.output
        );
        // The outer Repeater must enclose the inner one (positionally).
        let outer_pos = result.output.find("model: rows").expect("outer model");
        let inner_pos = result.output.find("model: columns").expect("inner model");
        assert!(
            outer_pos < inner_pos,
            "outer Repeater must enclose inner in:\n{}",
            result.output
        );
    }

    // -------- Test 45: If branch body wrapped in Item when multi-child --------

    /// QML's `Component { ... }` requires exactly one top-level child.
    /// A multi-child `If` body is wrapped in an `Item { ... }` so the
    /// `Component` stays well-formed. A single-child body is emitted
    /// inline without the wrapper to keep the output clean.
    #[test]
    fn if_multi_child_body_wraps_in_item() {
        let m = component("X", vec![], vec![]);
        let text = |s: &str| LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String(s.to_string()),
            }],
            children: Vec::new(),
        };

        // Multi-child If body → wrapped in Item.
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: if_with_slot_when("show", vec![text("a"), text("b")]),
        };
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // Both children's content must appear (literal text values).
        assert!(
            result.output.contains("text: \"a\""),
            "got:\n{}",
            result.output
        );
        assert!(
            result.output.contains("text: \"b\""),
            "got:\n{}",
            result.output
        );
        // And there must be an inner `Item {` wrapping them in addition
        // to the root `Item { ... }` of the component itself, so we
        // expect at least 2 occurrences of `Item {`.
        assert!(
            count_occurrences(&result.output, "Item {") >= 2,
            "expected an inner Item wrapping multi-child If body in:\n{}",
            result.output
        );
    }

    // =====================================================================
    // U29-1-K-qt — `HostDialog` kernel primitive tests
    // =====================================================================
    //
    // The HostDialog lowering is described in `emit_host_dialog_qml`. The
    // tests below pin every documented case.

    /// Build a bare `HostDialog` node with the given props and children.
    fn host_dialog(props: Vec<LayoutProp>, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "HostDialog".to_string(),
            part_name: None,
            props,
            children,
        }
    }

    /// Wrap a HostDialog as the root of a `LayoutDef` for component "X".
    fn dialog_layout(props: Vec<LayoutProp>, children: Vec<LayoutNode>) -> LayoutDef {
        LayoutDef {
            component_name: "X".to_string(),
            root: host_dialog(props, children),
        }
    }

    // -------- Test 46: empty HostDialog emits Popup skeleton --------

    /// An empty `HostDialog` (no props, no children) lowers to a
    /// `Popup { modal: true; visible: false; closePolicy: ...;
    /// contentItem: ColumnLayout { } }`. The contentItem is always
    /// emitted so the Popup has a well-defined body to anchor styling.
    #[test]
    fn empty_host_dialog_emits_popup_skeleton() {
        let m = component("X", vec![], vec![]);
        let l = dialog_layout(vec![], vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("Popup {"),
            "missing Popup in:\n{}",
            result.output
        );
        // Default modal is true (UI29-1 §2.1).
        assert!(
            result.output.contains("modal: true"),
            "missing default modal:true in:\n{}",
            result.output
        );
        // No `open` prop → visible: false default.
        assert!(
            result.output.contains("visible: false"),
            "missing default visible:false in:\n{}",
            result.output
        );
        // contentItem: ColumnLayout always present.
        assert!(
            result.output.contains("contentItem: ColumnLayout {"),
            "missing contentItem ColumnLayout in:\n{}",
            result.output
        );
    }

    // -------- Test 47: HostDialog open slot drives visible --------

    /// `open: slot: is-open` lowers to `visible: isOpen` (bare
    /// identifier binding) — QML's `Popup.visible` is the open/close
    /// hook.
    #[test]
    fn host_dialog_open_slot_binds_visible() {
        let m = component("X", vec![slot("is-open", SlotType::Bool, false)], vec![]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "open".to_string(),
                value: LayoutPropValue::SlotRef("is-open".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("visible: isOpen"),
            "missing visible:isOpen binding in:\n{}",
            result.output
        );
        // The default-false line must NOT appear when an `open` prop binds it.
        assert!(
            !result.output.contains("visible: false"),
            "default visible:false should be suppressed when open is bound:\n{}",
            result.output
        );
    }

    // -------- Test 48: HostDialog modal: true keyword --------

    /// `modal: true` (a compile-time keyword) lowers to `modal: true`.
    /// Same as the default, but pin the path so an authoring-explicit
    /// `modal: true` doesn't accidentally regress.
    #[test]
    fn host_dialog_modal_true_keyword() {
        let m = component("X", vec![], vec![]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "modal".to_string(),
                value: LayoutPropValue::Keyword("true".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("modal: true"),
            "missing modal:true in:\n{}",
            result.output
        );
    }

    // -------- Test 49: HostDialog modal: false keyword --------

    /// `modal: false` (compile-time keyword) lowers to `modal: false`
    /// — the non-modal popover shape. UI29-1 §3.3 distinguishes this
    /// from modal explicitly.
    #[test]
    fn host_dialog_modal_false_keyword() {
        let m = component("X", vec![], vec![]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "modal".to_string(),
                value: LayoutPropValue::Keyword("false".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("modal: false"),
            "missing modal:false in:\n{}",
            result.output
        );
        // Defensive — no `modal: true` should escape from the default
        // path when `modal: false` was explicitly bound.
        assert!(
            !result.output.contains("modal: true"),
            "modal:true should not appear when modal:false was bound:\n{}",
            result.output
        );
    }

    // -------- Test 50: HostDialog onClose wires onClosed --------

    /// `onClose: emit: onDismiss` lowers to `onClosed: dismiss()`. Note
    /// the Popup signal is past-tense `closed` (QML convention); the
    /// Mosaic emit name follows the `on` + present-tense convention
    /// (UI24 §5).
    #[test]
    fn host_dialog_on_close_wires_on_closed() {
        let m = component("X", vec![], vec![emit_decl("onDismiss", vec![])]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "onClose".to_string(),
                value: LayoutPropValue::EmitRef("onDismiss".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onClosed: dismiss()"),
            "missing onClosed signal call in:\n{}",
            result.output
        );
    }

    // -------- Test 51: HostDialog children render inside contentItem --------

    /// Author-supplied children render as the body of the
    /// `contentItem: ColumnLayout { ... }`. Verifies a `Text` child
    /// appears inside the contentItem block (positionally after the
    /// `ColumnLayout` opener and before the dialog's closing brace).
    #[test]
    fn host_dialog_children_render_inside_content_item() {
        let m = component("X", vec![], vec![]);
        let child = LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String("Hello dialog".to_string()),
            }],
            children: Vec::new(),
        };
        let l = dialog_layout(vec![], vec![child]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // The contentItem ColumnLayout must precede the Text child.
        let content_pos = result
            .output
            .find("contentItem: ColumnLayout {")
            .expect("contentItem missing");
        let text_pos = result
            .output
            .find("text: \"Hello dialog\"")
            .expect("child Text missing");
        assert!(
            content_pos < text_pos,
            "child must appear inside contentItem in:\n{}",
            result.output
        );
    }

    // -------- Test 52: HostDialog title slot emits bold Text first --------

    /// A `title: slot: dialog-title` prop synthesises a bold `Text`
    /// element as the FIRST child of `contentItem`. Pinned because
    /// `Popup` has no native title slot — this is our convention.
    #[test]
    fn host_dialog_title_slot_emits_bold_text_first_child() {
        let m = component(
            "X",
            vec![slot("dialog-title", SlotType::Text, false)],
            vec![],
        );
        // Add an author child so we can pin the ordering: title Text
        // before the author's child.
        let body_child = LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String("body text".to_string()),
            }],
            children: Vec::new(),
        };
        let l = dialog_layout(
            vec![LayoutProp {
                name: "title".to_string(),
                value: LayoutPropValue::SlotRef("dialog-title".to_string()),
            }],
            vec![body_child],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();

        // The title Text must use a bare-identifier binding.
        assert!(
            result.output.contains("text: dialogTitle"),
            "missing title slot-ref binding in:\n{}",
            result.output
        );
        // The title row must carry font.bold: true.
        assert!(
            result.output.contains("font.bold: true"),
            "missing font.bold on title row in:\n{}",
            result.output
        );
        // Title Text must come before the body Text.
        let title_pos = result
            .output
            .find("text: dialogTitle")
            .expect("title Text missing");
        let body_pos = result
            .output
            .find("text: \"body text\"")
            .expect("body Text missing");
        assert!(
            title_pos < body_pos,
            "title must appear before body in:\n{}",
            result.output
        );
    }

    // -------- Test 53: dismiss-on-backdrop: false → escape-only --------

    /// `dismiss-on-backdrop: false` (compile-time keyword) lowers to
    /// `closePolicy: Popup.CloseOnEscape` — Esc still closes, but
    /// clicks outside the popup are ignored. The default (absent or
    /// `true`) keeps both Esc and outside-press handling.
    #[test]
    fn host_dialog_dismiss_on_backdrop_false_adjusts_close_policy() {
        let m = component("X", vec![], vec![]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "dismiss-on-backdrop".to_string(),
                value: LayoutPropValue::Keyword("false".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("closePolicy: Popup.CloseOnEscape"),
            "missing escape-only closePolicy in:\n{}",
            result.output
        );
        // The combined policy must NOT appear.
        assert!(
            !result.output.contains("Popup.CloseOnPressOutsideParent"),
            "outside-press must be absent when dismiss-on-backdrop:false:\n{}",
            result.output
        );

        // Sanity: the default (absent prop) keeps the combined policy.
        let l_default = dialog_layout(vec![], vec![]);
        let r_default = from_pipeline(&m, &l_default, &empty_style("X")).unwrap();
        assert!(
            r_default.output.contains("Popup.CloseOnPressOutsideParent"),
            "default closePolicy must include outside-press in:\n{}",
            r_default.output
        );
    }

    // -------- Test 54: HostDialog triggers QtQuick.Controls 2.15 import --------

    /// `HostDialog` lowers to `Popup`, which lives in
    /// `QtQuick.Controls 2.15`. Using a dialog must add the conditional
    /// Controls import — same gate as `HostButton` / `HostScroll`.
    #[test]
    fn host_dialog_triggers_qtquick_controls_import() {
        let m = component("X", vec![], vec![]);
        let l = dialog_layout(vec![], vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("import QtQuick.Controls 2.15"),
            "Controls import missing when HostDialog used in:\n{}",
            result.output
        );
    }

    // -------- Test 55: HostDialog onOpen wires onOpened --------

    /// `onOpen: emit: onShow` lowers to `onOpened: show()`. Mirrors the
    /// onClose handling. Pinned separately so a regression on one
    /// signal direction doesn't slip through.
    #[test]
    fn host_dialog_on_open_wires_on_opened() {
        let m = component("X", vec![], vec![emit_decl("onShow", vec![])]);
        let l = dialog_layout(
            vec![LayoutProp {
                name: "onOpen".to_string(),
                value: LayoutPropValue::EmitRef("onShow".to_string()),
            }],
            vec![],
        );
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onOpened: show()"),
            "missing onOpened signal call in:\n{}",
            result.output
        );
    }

    // =====================================================================
    // UI29-2 — HostCheckbox / HostRadio (QtQuick.Controls CheckBox/RadioButton)
    // =====================================================================

    /// Helper: a one-component layout def rooted at a `HostCheckbox`
    /// with the given props.
    fn checkbox_layout(props: Vec<LayoutProp>) -> LayoutDef {
        LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostCheckbox".to_string(),
                part_name: None,
                props,
                children: Vec::new(),
            },
        }
    }

    /// Helper: a one-component layout def rooted at a `HostRadio` with
    /// the given props.
    fn radio_layout(props: Vec<LayoutProp>) -> LayoutDef {
        LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostRadio".to_string(),
                part_name: None,
                props,
                children: Vec::new(),
            },
        }
    }

    /// UI29-2 Qt test 1 — bare `HostCheckbox` emits a `CheckBox { }`
    /// block. No checked/label/disabled lines because none are bound.
    #[test]
    fn host_checkbox_empty_emits_bare_checkbox_block() {
        let m = component("X", vec![], vec![]);
        let l = checkbox_layout(vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &result.output;
        assert!(
            out.contains("CheckBox {"),
            "expected CheckBox block, got:\n{out}"
        );
    }

    /// UI29-2 Qt test 2 — bare `HostCheckbox` triggers the
    /// QtQuick.Controls 2.15 import (CheckBox lives in Controls 2).
    #[test]
    fn host_checkbox_triggers_qtquick_controls_import() {
        let m = component("X", vec![], vec![]);
        let l = checkbox_layout(vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("import QtQuick.Controls 2.15"),
            "expected QtQuick.Controls import, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 3 — `checked: slot: c` binds to `checked: c`
    /// (camelCased bare identifier per QML's property binding syntax).
    #[test]
    fn host_checkbox_checked_slot_binds_to_checked_attribute() {
        let m = component("X", vec![slot("is-checked", SlotType::Bool, true)], vec![]);
        let l = checkbox_layout(vec![LayoutProp {
            name: "checked".to_string(),
            value: LayoutPropValue::SlotRef("is-checked".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("checked: isChecked"),
            "expected `checked: isChecked` binding, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 4 — `disabled: slot: d` produces `enabled: !d`
    /// (polarity flip — same pattern HostButton uses).
    #[test]
    fn host_checkbox_disabled_slot_flips_to_enabled_negated() {
        let m = component("X", vec![slot("locked", SlotType::Bool, true)], vec![]);
        let l = checkbox_layout(vec![LayoutProp {
            name: "disabled".to_string(),
            value: LayoutPropValue::SlotRef("locked".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("enabled: !locked"),
            "expected `enabled: !locked`, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 5 — `label: "Agree"` binds to `text: "Agree"`.
    #[test]
    fn host_checkbox_string_label_binds_to_text_attribute() {
        let m = component("X", vec![], vec![]);
        let l = checkbox_layout(vec![LayoutProp {
            name: "label".to_string(),
            value: LayoutPropValue::String("Agree".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("text: \"Agree\""),
            "expected `text: \"Agree\"`, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 6 — `onToggle: emit: onChange` wires to Qt's
    /// `toggled(bool)` signal as `onToggled: change(checked)`. The
    /// `checked` parameter carries Qt's new-state value to the host.
    #[test]
    fn host_checkbox_on_toggle_emits_on_toggled_handler() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onChange",
                vec![EmitParam {
                    name: "checked".to_string(),
                    r#type: EmitPayloadType::Bool,
                }],
            )],
        );
        let l = checkbox_layout(vec![LayoutProp {
            name: "onToggle".to_string(),
            value: LayoutPropValue::EmitRef("onChange".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("onToggled: change(checked)"),
            "expected `onToggled: change(checked)` handler, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 7 — `indeterminate: slot: i` adds `tristate:
    /// true` plus a `checkState:` ternary that resolves to
    /// `Qt.PartiallyChecked` when the slot is truthy.
    #[test]
    fn host_checkbox_indeterminate_slot_adds_tristate_and_check_state() {
        let m = component("X", vec![slot("is-mixed", SlotType::Bool, true)], vec![]);
        let l = checkbox_layout(vec![LayoutProp {
            name: "indeterminate".to_string(),
            value: LayoutPropValue::SlotRef("is-mixed".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &result.output;
        assert!(
            out.contains("tristate: true"),
            "expected `tristate: true`, got:\n{out}"
        );
        assert!(
            out.contains("checkState: isMixed ? Qt.PartiallyChecked"),
            "expected `checkState:` ternary, got:\n{out}"
        );
    }

    /// UI29-2 Qt test 8 — bare `HostRadio` emits a `RadioButton { }`
    /// block.
    #[test]
    fn host_radio_empty_emits_bare_radio_button_block() {
        let m = component("X", vec![], vec![]);
        let l = radio_layout(vec![]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("RadioButton {"),
            "expected RadioButton block, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 9 — `group: "flavor"` is preserved as a
    /// `// group: flavor` line comment ahead of the RadioButton (no
    /// native QtQuick.Controls grouping in v1; reserved for UI29-2.1).
    #[test]
    fn host_radio_group_string_emits_comment() {
        let m = component("X", vec![], vec![]);
        let l = radio_layout(vec![LayoutProp {
            name: "group".to_string(),
            value: LayoutPropValue::String("flavor".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("// group: flavor"),
            "expected `// group: flavor` comment, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 10 — regression: a `group:` string with a newline
    /// must not break out of the `//` line comment. Mirrors the
    /// SwiftUI backend's defense.
    #[test]
    fn host_radio_group_with_newline_is_neutralised_in_comment() {
        let m = component("X", vec![], vec![]);
        let l = radio_layout(vec![LayoutProp {
            name: "group".to_string(),
            value: LayoutPropValue::String("x\nimport Evil 1.0".to_string()),
        }]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let comment_line = result
            .output
            .lines()
            .find(|l| l.contains("// group:"))
            .expect("group comment present");
        assert!(
            comment_line.contains("import Evil"),
            "newline injection must be neutralised — `import Evil` must \
             stay on the same line as `// group:`, got line:\n{comment_line}"
        );
        assert!(
            !result.output.contains("\nimport Evil"),
            "found raw newline + `import Evil` — vector still open:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 11 — `onSelect: emit: onPick` + `value:
    /// "vanilla"` wires to Qt's `checkedChanged()` signal with a
    /// positive-transition gate. The dispatch fires `pick("vanilla")`
    /// only when `checked` is true.
    #[test]
    fn host_radio_on_select_wires_on_checked_changed_with_positive_gate() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onPick",
                vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Text,
                }],
            )],
        );
        let l = radio_layout(vec![
            LayoutProp {
                name: "value".to_string(),
                value: LayoutPropValue::String("vanilla".to_string()),
            },
            LayoutProp {
                name: "onSelect".to_string(),
                value: LayoutPropValue::EmitRef("onPick".to_string()),
            },
        ]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result
                .output
                .contains("onCheckedChanged: if (checked) pick(\"vanilla\")"),
            "expected positive-gated dispatch, got:\n{}",
            result.output
        );
    }

    /// UI29-2 Qt test 12 — `value: slot: v` flows the camelCased slot
    /// identifier directly into the dispatch payload.
    #[test]
    fn host_radio_value_slot_flows_into_dispatch_payload() {
        let m = component(
            "X",
            vec![slot("radio-value", SlotType::Text, true)],
            vec![emit_decl(
                "onPick",
                vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Text,
                }],
            )],
        );
        let l = radio_layout(vec![
            LayoutProp {
                name: "value".to_string(),
                value: LayoutPropValue::SlotRef("radio-value".to_string()),
            },
            LayoutProp {
                name: "onSelect".to_string(),
                value: LayoutPropValue::EmitRef("onPick".to_string()),
            },
        ]);
        let result = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            result.output.contains("if (checked) pick(radioValue)"),
            "expected dispatch with bare `radioValue`, got:\n{}",
            result.output
        );
    }

    // =====================================================================
    // UI29-4 — HostLink / HostTooltip / HostNumberInput (Qt)
    // =====================================================================

    /// UI29-4 Qt test 1 — bare `HostLink href + label` lowers to a
    /// rich-text `Text` element with an `<a>` tag in the body and an
    /// onLinkActivated handler that opens the URL externally.
    #[test]
    fn host_link_string_href_and_label_emits_rich_text_anchor() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostLink".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "href".to_string(),
                        value: LayoutPropValue::String("https://example.com".to_string()),
                    },
                    LayoutProp {
                        name: "label".to_string(),
                        value: LayoutPropValue::String("Click me".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("Text {"), "expected `Text {{`, got:\n{out}");
        assert!(
            out.contains("textFormat: Text.RichText"),
            "expected `textFormat: Text.RichText`, got:\n{out}"
        );
        assert!(
            out.contains("<a href="),
            "expected anchor in rich-text body, got:\n{out}"
        );
        assert!(out.contains("Click me"));
        assert!(
            out.contains("Qt.openUrlExternally(link)"),
            "expected open-external handler, got:\n{out}"
        );
    }

    /// UI29-4 Qt test 1a — SECURITY REGRESSION: a `\` or `"` in the
    /// author's `label` must not break out of the QML string literal
    /// or the inner HTML attribute. The 2-layer escape (HTML
    /// entities for `&<>"`, then escape_qml_string for backslash +
    /// double-quote at the QML layer) closes both vectors.
    #[test]
    fn host_link_with_backslash_and_quote_in_label_is_escaped() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostLink".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "href".to_string(),
                        value: LayoutPropValue::String("https://example.com".to_string()),
                    },
                    LayoutProp {
                        name: "label".to_string(),
                        value: LayoutPropValue::String("x\"\\evil".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        // The label's `"` must be HTML-entity-encoded as `&quot;`
        // (not left as `"` which would break the inner attribute
        // OR be QML-escaped as `\"` which would silently introduce
        // a literal `"` into the rendered text).
        assert!(
            out.contains("&quot;"),
            "expected `\"` to be HTML-entity encoded as &quot;, got:\n{out}"
        );
        // The label's `\` must be QML-escaped as `\\` to avoid
        // eating the closing quote of the QML literal.
        assert!(
            out.contains("\\\\"),
            "expected backslash to be QML-escaped as \\\\, got:\n{out}"
        );
    }

    /// UI29-4 Qt test 2 — `external: false` + `onActivate: emit:
    /// onNavigate` makes `onLinkActivated` dispatch the emit only
    /// (no external open), so the host's QML router takes over.
    #[test]
    fn host_link_external_false_with_on_activate_dispatches_only() {
        let m = component("X", vec![], vec![emit_decl("onNavigate", vec![])]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostLink".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "href".to_string(),
                        value: LayoutPropValue::String("/about".to_string()),
                    },
                    LayoutProp {
                        name: "external".to_string(),
                        value: LayoutPropValue::Keyword("false".to_string()),
                    },
                    LayoutProp {
                        name: "onActivate".to_string(),
                        value: LayoutPropValue::EmitRef("onNavigate".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("onLinkActivated: navigate()"),
            "expected dispatch-only handler, got:\n{}",
            r.output
        );
        assert!(
            !r.output.contains("Qt.openUrlExternally"),
            "external open must NOT be present in in-app routing mode, got:\n{}",
            r.output
        );
    }

    /// UI29-4 Qt test 3 — a link inside an indexed `For` dispatches
    /// the row index and renders the row item as its label.
    #[test]
    fn host_link_inside_indexed_for_dispatches_index_payload() {
        let m = component(
            "Nav",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit_decl(
                "onSelect",
                vec![EmitParam {
                    name: "index".to_string(),
                    r#type: EmitPayloadType::Number,
                }],
            )],
        );
        let l = LayoutDef {
            component_name: "Nav".to_string(),
            root: LayoutNode {
                tag: "For".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "each".to_string(),
                        value: LayoutPropValue::SlotRef("items".to_string()),
                    },
                    LayoutProp {
                        name: "as".to_string(),
                        value: LayoutPropValue::Keyword("item".to_string()),
                    },
                    LayoutProp {
                        name: "index".to_string(),
                        value: LayoutPropValue::Keyword("i".to_string()),
                    },
                ],
                children: vec![LayoutNode {
                    tag: "HostLink".to_string(),
                    part_name: None,
                    props: vec![
                        LayoutProp {
                            name: "href".to_string(),
                            value: LayoutPropValue::String("#".to_string()),
                        },
                        LayoutProp {
                            name: "label".to_string(),
                            value: LayoutPropValue::Keyword("item".to_string()),
                        },
                        LayoutProp {
                            name: "external".to_string(),
                            value: LayoutPropValue::Keyword("false".to_string()),
                        },
                        LayoutProp {
                            name: "onActivate".to_string(),
                            value: LayoutPropValue::EmitRef("onSelect".to_string()),
                        },
                    ],
                    children: Vec::new(),
                }],
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("Nav")).unwrap();
        assert!(
            r.output.contains("onLinkActivated: select(i)"),
            "expected HostLink to dispatch For index, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("\"<a href=\\\"#\\\">\" + String(item)"),
            "expected HostLink label to use For item binding, got:\n{}",
            r.output
        );
    }

    /// UI29-4 Qt test 4 — `HostTooltip` lowers to an `Item` wrapping
    /// the child with `ToolTip.text` + `HoverHandler` to drive
    /// visibility. The wrapped child (a Text in this fixture) is
    /// recursed-through normally.
    #[test]
    fn host_tooltip_wraps_child_in_item_with_tooltip_text() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostTooltip".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "text".to_string(),
                    value: LayoutPropValue::String("Click to submit".to_string()),
                }],
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("Submit".to_string()),
                    }],
                    children: Vec::new(),
                }],
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("Item {"), "expected `Item {{`, got:\n{out}");
        assert!(
            out.contains("ToolTip.text: \"Click to submit\""),
            "expected ToolTip.text, got:\n{out}"
        );
        assert!(
            out.contains("ToolTip.visible: hoverHandler.hovered"),
            "expected hover-driven visibility, got:\n{out}"
        );
        assert!(
            out.contains("HoverHandler { id: hoverHandler }"),
            "expected HoverHandler, got:\n{out}"
        );
    }

    /// UI29-4 Qt test 4 — bare `HostNumberInput` lowers to a
    /// decimal-safe `TextField { }` with a `DoubleValidator`.
    #[test]
    fn host_number_input_empty_emits_textfield_block() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostNumberInput".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("TextField {"),
            "expected TextField block, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("validator: DoubleValidator {"),
            "expected DoubleValidator, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("import QtQuick.Controls 2.15"),
            "expected QtQuick.Controls import, got:\n{}",
            r.output
        );
    }

    /// UI29-4 Qt test 5 — `min`/`max` numeric literals map to
    /// DoubleValidator bounds, while fractional value literals remain
    /// decimal text.
    #[test]
    fn host_number_input_value_min_max_map_to_decimal_textfield_props() {
        let m = component("X", vec![], vec![]);
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostNumberInput".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "value".to_string(),
                        value: LayoutPropValue::Number(2.5),
                    },
                    LayoutProp {
                        name: "min".to_string(),
                        value: LayoutPropValue::Number(0.0),
                    },
                    LayoutProp {
                        name: "max".to_string(),
                        value: LayoutPropValue::Number(10.5),
                    },
                ],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(
            out.contains("text: String(2.5)"),
            "expected fractional text value, got:\n{out}"
        );
        assert!(
            out.contains("bottom: 0"),
            "expected validator bottom: 0, got:\n{out}"
        );
        assert!(
            out.contains("top: 10.5"),
            "expected validator top: 10.5, got:\n{out}"
        );
    }

    /// UI29-4 Qt test 6 — `onChange: emit: onSet` wires
    /// `onTextEdited` (user-initiated only) and dispatches the
    /// parsed floating-point value.
    #[test]
    fn host_number_input_on_change_wires_parsed_text_edited_value() {
        let m = component(
            "X",
            vec![],
            vec![emit_decl(
                "onSet",
                vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Number,
                }],
            )],
        );
        let l = LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "HostNumberInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "onChange".to_string(),
                    value: LayoutPropValue::EmitRef("onSet".to_string()),
                }],
                children: Vec::new(),
            },
        };
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("onTextEdited: {"),
            "expected onTextEdited handler, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("const nextValue = Number(text)"),
            "expected parsed numeric payload, got:\n{}",
            r.output
        );
        assert!(
            r.output
                .contains("if (text.length > 0 && !isNaN(nextValue)) { set(nextValue) }"),
            "expected valid-number dispatch, got:\n{}",
            r.output
        );
    }

    // =================================================================
    // UI32-K-qt — `--emit-project` Qt6 + CMake shell tests
    //
    // Covers UI32 spec §3.1-§3.8 per-PR gates:
    //   §3.4 Composable     : default options = no project shell.
    //   §3.5 Banner          : every emitted file starts with banner.
    //   §3.1 Reproducible    : two runs produce byte-identical output.
    //   §3.6.2 Qt row        : CMake target + qmldir module name shape
    //                          (PascalCase, no hyphen).
    //   §3.6.3 Pinning       : CMakeLists.txt has pinned Qt6 + CMake +
    //                          C++ standard.
    //   §3.7 Output paths    : only the spec §2.2 enumeration.
    //   §3.8 No env reads    : no /Users/, $HOME, etc. in output.
    // =================================================================

    /// Small helper for the UI32 tests: build a minimal HostScroll-
    /// rooted layout (Qt's bare primitive set requires a root that
    /// the emitter knows; Box is fine).
    fn ui32_simple_layout(name: &str) -> LayoutDef {
        LayoutDef {
            component_name: name.to_string(),
            root: LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    #[test]
    fn ui32_emit_project_false_is_backward_compatible_with_from_pipeline() {
        let m = component("X", vec![], vec![]);
        let l = ui32_simple_layout("X");
        let s = empty_style("X");

        let legacy = from_pipeline(&m, &l, &s).unwrap();
        let extended = from_pipeline_with_options(&m, &l, &s, &EmitOptions::default()).unwrap();

        assert_eq!(legacy.output, extended.output, ".qml bytes diverged");
        assert_eq!(legacy.component_name, extended.component_name);
        assert!(
            extended.project.is_none(),
            "default options must NOT emit a project shell"
        );
    }

    #[test]
    fn ui32_emit_project_true_returns_project_files() {
        let m = component("Hello", vec![], vec![]);
        let l = ui32_simple_layout("Hello");
        let s = empty_style("Hello");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let r = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert!(
            r.project.is_some(),
            "emit_project: true must produce a shell"
        );
    }

    #[test]
    fn ui32_every_emitted_side_file_carries_auto_generated_banner() {
        let m = component("Hello", vec![], vec![]);
        let l = ui32_simple_layout("Hello");
        let s = empty_style("Hello");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .expect("project shell expected");

        // CMakeLists.txt uses `#` for comments.
        assert!(
            proj.cmake_lists.starts_with("# AUTO-GENERATED"),
            "CMakeLists.txt must START with `# AUTO-GENERATED`"
        );
        // main.cpp uses `//` for comments.
        assert!(
            proj.main_cpp.starts_with("// AUTO-GENERATED"),
            "main.cpp must START with `// AUTO-GENERATED`"
        );
        // qmldir uses `#` for comments.
        assert!(
            proj.qmldir.starts_with("# AUTO-GENERATED"),
            "qmldir must START with `# AUTO-GENERATED`"
        );
        // README.md uses HTML-comment syntax.
        assert!(
            proj.readme.starts_with("<!-- AUTO-GENERATED"),
            "README.md must START with banner"
        );
    }

    #[test]
    fn ui32_emit_project_is_byte_deterministic() {
        let m = component("Deterministic", vec![], vec![]);
        let l = ui32_simple_layout("Deterministic");
        let s = empty_style("Deterministic");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };

        let a = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        let b = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert_eq!(a.output, b.output, ".qml is not deterministic");
        assert_eq!(a.project, b.project, "project shell is not deterministic");
    }

    /// §3.6.2 Qt row: the QML module URI is `Mosaic.<Component>`,
    /// and the CMake target uses the bare component name. Both
    /// flow from the upstream-validated ASCII identifier shape.
    #[test]
    fn ui32_qmldir_and_cmake_use_pascal_component_name() {
        let m = component("ProfileCard", vec![], vec![]);
        let l = ui32_simple_layout("ProfileCard");
        let s = empty_style("ProfileCard");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        // qmldir carries the module URI + the component export.
        assert!(
            proj.qmldir.contains("module Mosaic.ProfileCard"),
            "qmldir must declare module `Mosaic.ProfileCard`, got:\n{}",
            proj.qmldir
        );
        assert!(
            proj.qmldir.contains("ProfileCard 1.0 ProfileCard.qml"),
            "qmldir must export ProfileCard at version 1.0, got:\n{}",
            proj.qmldir
        );
        // CMake target uses the component name (no hyphen — the
        // upstream validator excludes them, which keeps CMake happy).
        assert!(
            proj.cmake_lists.contains("project(ProfileCard"),
            "CMakeLists.txt must declare `project(ProfileCard ...)`"
        );
        assert!(
            proj.cmake_lists.contains("qt_add_executable(ProfileCard"),
            "CMakeLists.txt must add executable `ProfileCard`"
        );
    }

    /// §3.6.3 Pinning. CMakeLists.txt carries the default pinned
    /// Qt + CMake + C++ versions.
    #[test]
    fn ui32_cmake_lists_carries_pinned_default_versions_exactly() {
        let m = component("X", vec![], vec![]);
        let l = ui32_simple_layout("X");
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        assert!(
            proj.cmake_lists
                .contains("cmake_minimum_required(VERSION 3.21)"),
            "expected CMake minimum 3.21"
        );
        assert!(
            proj.cmake_lists.contains("find_package(Qt6 6.7 REQUIRED"),
            "expected Qt6 6.7 pin"
        );
        assert!(
            proj.cmake_lists.contains("set(CMAKE_CXX_STANDARD 17)"),
            "expected C++17 standard"
        );
    }

    /// §3.7 Output paths tripwire.
    #[test]
    fn ui32_project_files_struct_exposes_only_spec_22_qt_files() {
        let m = component("X", vec![], vec![]);
        let l = ui32_simple_layout("X");
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let ProjectFiles {
            cmake_lists,
            main_cpp,
            qmldir,
            readme,
        } = proj;
        assert!(!cmake_lists.is_empty(), "CMakeLists.txt empty");
        assert!(!main_cpp.is_empty(), "main.cpp empty");
        assert!(!qmldir.is_empty(), "qmldir empty");
        assert!(!readme.is_empty(), "README.md empty");
    }

    #[test]
    fn ui32_emitted_files_contain_no_environment_specific_strings() {
        let m = component("X", vec![], vec![]);
        let l = ui32_simple_layout("X");
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let all = format!(
            "{}\n{}\n{}\n{}",
            proj.cmake_lists, proj.main_cpp, proj.qmldir, proj.readme
        );
        for banned in ["/Users/", "/home/", "C:\\Users\\", "$HOME"] {
            assert!(
                !all.contains(banned),
                "emitted shell contains environment-specific fragment `{banned}`"
            );
        }
    }

    /// main.cpp loads the component from the embedded QML module via
    /// `qrc:/qt/qml/Mosaic/<Component>/<Component>.qml`. This is the
    /// path `qt_add_qml_module` exposes when given URI `Mosaic.<X>`.
    #[test]
    fn ui32_main_cpp_loads_component_from_embedded_qml_module() {
        let m = component("MyWidget", vec![], vec![]);
        let l = ui32_simple_layout("MyWidget");
        let s = empty_style("MyWidget");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        // The qrc path uses slashes (not dots) to navigate the
        // embedded resource tree.
        assert!(
            proj.main_cpp
                .contains("qrc:/qt/qml/Mosaic/MyWidget/MyWidget.qml"),
            "main.cpp must reference qrc:/qt/qml/Mosaic/MyWidget/MyWidget.qml, got:\n{}",
            proj.main_cpp
        );
        // Must use QApplication + QQuickView so the generated Item is hosted
        // in a visible desktop window while optional host adapters can use
        // QtWidgets file dialogs.
        assert!(
            proj.main_cpp.contains("QApplication"),
            "main.cpp must use QApplication"
        );
        assert!(
            proj.main_cpp.contains("QQuickView"),
            "main.cpp must use QQuickView"
        );
        assert!(
            proj.main_cpp.contains("#include <QQuickItem>"),
            "main.cpp must include QQuickItem so rootObject() converts to QObject*"
        );
        assert!(
            proj.main_cpp
                .contains("view.setResizeMode(QQuickView::SizeRootObjectToView);"),
            "main.cpp must size the generated Item to the view"
        );
        assert!(
            proj.main_cpp.contains("view.show();"),
            "main.cpp must show the generated desktop window"
        );
    }

    #[test]
    fn qml_root_exposes_optional_mosaic_host_bridge() {
        let m = component(
            "Review",
            vec![slot("app-title", SlotType::Text, true)],
            vec![EmitDecl {
                name: "onReveal".to_string(),
                params: vec![],
            }],
        );
        let l = ui32_simple_layout("Review");
        let s = empty_style("Review");

        let out = from_pipeline(&m, &l, &s).unwrap().output;

        assert!(
            out.contains("id: mosaicRoot"),
            "root needs a stable id for dynamic prop writes:\n{out}"
        );
        assert!(
            out.contains("property var mosaicHost: null"),
            "root should accept an optional C++ host object:\n{out}"
        );
        assert!(
            out.contains("property var lastHostIntent: null"),
            "root should retain the last structured host intent:\n{out}"
        );
        assert!(
            out.contains("function applyMosaicProps(props)"),
            "root should expose an app-agnostic prop hydrator:\n{out}"
        );
        assert!(
            out.contains("function applyMosaicResponse(response)"),
            "root should normalize host responses that include props and hostIntent:\n{out}"
        );
        assert!(
            out.contains("lastHostIntent = response.hostIntent;"),
            "root should store host intents separately from props:\n{out}"
        );
        assert!(
            out.contains("mosaicRoot[key] = props[key];"),
            "prop hydrator should assign dynamic QML properties:\n{out}"
        );
        assert!(
            out.contains(
                "onMosaicEvent: function(event) { applyMosaicResponse(mosaicHost ? mosaicHost.handleEvent(event) : null) }"
            ),
            "mosaicEvent should round-trip through the optional host:\n{out}"
        );
    }

    #[test]
    fn ui32_project_shell_exposes_optional_mosaic_host_bridge() {
        let m = component("Review", vec![], vec![]);
        let l = ui32_simple_layout("Review");
        let s = empty_style("Review");
        let opts = EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();

        assert!(
            proj.main_cpp
                .contains("#if __has_include(\"MosaicHost.h\")"),
            "main.cpp should compile with or without a host adapter:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp.contains("#include <QApplication>"),
            "main.cpp should use QApplication so host adapters can use QtWidgets dialogs:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp.contains("QApplication app(argc, argv);"),
            "main.cpp should initialize QApplication for QtWidgets-capable hosts:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp.contains("MosaicHost mosaicHost;"),
            "main.cpp should instantiate the optional host:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp.contains("MosaicHost::registerTypes();")
                && proj.main_cpp.contains("mosaicHost.attach(root);"),
            "main.cpp should register and attach native host surfaces:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp.contains("root->setProperty(\"mosaicHost\""),
            "main.cpp should expose the optional host to QML:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.main_cpp
                .contains("QMetaObject::invokeMethod(root, \"applyMosaicResponse\""),
            "main.cpp should hydrate initial props through QML:\n{}",
            proj.main_cpp
        );
        assert!(
            proj.cmake_lists
                .contains("target_sources(Review PRIVATE MosaicHost.cpp MosaicHost.h)"),
            "CMake should compile an installed host adapter when present:\n{}",
            proj.cmake_lists
        );
        assert!(
            proj.cmake_lists.contains(
                "find_package(Qt6 6.7 REQUIRED COMPONENTS Quick QmlImportScanner Widgets)"
            ),
            "CMake should resolve QtWidgets for native file picker host adapters:\n{}",
            proj.cmake_lists
        );
        assert!(
            proj.cmake_lists
                .contains("target_link_libraries(Review PRIVATE Qt6::Quick Qt6::Widgets)"),
            "CMake should link QtWidgets for native file picker host adapters:\n{}",
            proj.cmake_lists
        );
        assert!(
            proj.cmake_lists.contains("venture_browser_qt.dll")
                && proj.cmake_lists.contains("libventure_browser_qt.dylib")
                && proj.cmake_lists.contains("libventure_browser_qt.so"),
            "CMake should copy an installed native host library when present:\n{}",
            proj.cmake_lists
        );
    }

    // =================================================================
    // mosstyle inlining — styled `Box [cell]` → `Rectangle`
    //
    // These cover the UI28 §4.5 work that landed the cell-styling that
    // turns the VisiCalc-qt grid from a collapsed black smudge into a
    // real spreadsheet (borders, fixed widths, alignment, state colours).
    // =================================================================

    use mosstyle_compiler::{PartStyle, StateStyle, StyleProp};

    fn sp(name: &str, value: &str) -> StyleProp {
        StyleProp {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    /// A `cell` part mirroring `Grid.dark.msl`: border, padding, height,
    /// right text-align, plus `selected` / `editing` state blocks.
    fn cell_style(component: &str) -> StyleDef {
        StyleDef {
            component_name: component.to_string(),
            parts: vec![
                PartStyle {
                    name: "sheet".to_string(),
                    base: vec![
                        sp("background", "#1e1e1e"),
                        sp("color", "#cccccc"),
                        sp("font-family", "monospace"),
                        sp("font-size", "12px"),
                    ],
                    transitions: vec![],
                    states: vec![],
                },
                PartStyle {
                    name: "cell".to_string(),
                    base: vec![
                        sp("border-width", "1px"),
                        sp("border-color", "#3f3f46"),
                        sp("padding", "2px"),
                        sp("height", "22px"),
                        sp("text-align", "right"),
                    ],
                    transitions: vec![],
                    states: vec![
                        StateStyle {
                            state: "selected".to_string(),
                            transitions: vec![],
                            props: vec![sp("background", "#264f78"), sp("color", "#ffffff")],
                        },
                        StateStyle {
                            state: "editing".to_string(),
                            transitions: vec![],
                            props: vec![sp("background", "#1f4f3f")],
                        },
                    ],
                },
            ],
        }
    }

    fn lp(name: &str, value: LayoutPropValue) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value,
        }
    }

    /// A `For ( index: c )` whose body is a styled `Box [cell]` containing
    /// a `Text ( content: ( v ) )` — the spreadsheet body-cell shape after
    /// package resolution, wrapped in a `HostTable [sheet]` with a
    /// `HostTableColGroup` so column-width threading discovers
    /// `columnWidths`.
    fn styled_cell_table_layout(component: &str) -> LayoutDef {
        let cell_box = LayoutNode {
            tag: "Box".to_string(),
            part_name: Some("cell".to_string()),
            props: vec![
                lp(
                    "state-when-selected",
                    LayoutPropValue::Expr("( r == selectedRow && c == selectedCol )".to_string()),
                ),
                lp(
                    "state-when-editing",
                    LayoutPropValue::Expr("( r == editRow && c == editCol )".to_string()),
                ),
            ],
            children: vec![LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![lp("content", LayoutPropValue::Expr("( v )".to_string()))],
                children: vec![],
            }],
        };
        let inner_for = LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props: vec![
                lp("each", LayoutPropValue::Keyword("row".to_string())),
                lp("as", LayoutPropValue::Keyword("v".to_string())),
                lp("index", LayoutPropValue::Keyword("c".to_string())),
            ],
            children: vec![cell_box],
        };
        let data_row = LayoutNode {
            tag: "Row".to_string(),
            part_name: Some("data-row".to_string()),
            props: vec![],
            children: vec![inner_for],
        };
        let outer_for = LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props: vec![
                lp(
                    "each",
                    LayoutPropValue::SlotRef("viewport-rows".to_string()),
                ),
                lp("as", LayoutPropValue::Keyword("row".to_string())),
                lp("index", LayoutPropValue::Keyword("r".to_string())),
            ],
            children: vec![data_row],
        };
        let body = LayoutNode {
            tag: "HostTableBody".to_string(),
            part_name: None,
            props: vec![],
            children: vec![outer_for],
        };
        // ColGroup whose For-Col carries the column-widths slot.
        let colgroup = LayoutNode {
            tag: "HostTableColGroup".to_string(),
            part_name: None,
            props: vec![],
            children: vec![LayoutNode {
                tag: "For".to_string(),
                part_name: None,
                props: vec![
                    lp(
                        "each",
                        LayoutPropValue::SlotRef("column-widths".to_string()),
                    ),
                    lp("as", LayoutPropValue::Keyword("w".to_string())),
                    lp("index", LayoutPropValue::Keyword("cw".to_string())),
                ],
                children: vec![LayoutNode {
                    tag: "Col".to_string(),
                    part_name: Some("col".to_string()),
                    props: vec![lp("width", LayoutPropValue::Expr("( w )".to_string()))],
                    children: vec![],
                }],
            }],
        };
        LayoutDef {
            component_name: component.to_string(),
            root: LayoutNode {
                tag: "HostTable".to_string(),
                part_name: Some("sheet".to_string()),
                props: vec![],
                children: vec![colgroup, body],
            },
        }
    }

    fn styled_cell_component(name: &str) -> MosmodelComponent {
        component(
            name,
            vec![
                slot(
                    "viewport-rows",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
                slot(
                    "column-widths",
                    SlotType::List(Box::new(ListInnerType::Number)),
                    true,
                ),
                slot("selected-row", SlotType::Number, true),
                slot("selected-col", SlotType::Number, true),
                slot("edit-row", SlotType::Number, true),
                slot("edit-col", SlotType::Number, true),
            ],
            vec![],
        )
    }

    /// A styled `Box [cell]` lowers to a `Rectangle` carrying border,
    /// background and a `Layout.preferredWidth` driven by the discovered
    /// column-widths slot — not the old bare, sizeless `Item`.
    #[test]
    fn styled_box_cell_lowers_to_rectangle_with_border_and_width() {
        let m = styled_cell_component("Grid");
        let l = styled_cell_table_layout("Grid");
        let s = cell_style("Grid");
        let out = from_pipeline(&m, &l, &s).unwrap().output;

        assert!(
            out.contains("Rectangle {"),
            "cell must be a Rectangle:\n{out}"
        );
        assert!(
            out.contains("border.width: 1"),
            "missing border.width:\n{out}"
        );
        assert!(
            out.contains("border.color: \"#3f3f46\""),
            "missing border.color:\n{out}"
        );
        // Column width threaded from the discovered `columnWidths` slot
        // via the inner For's `c` index.
        assert!(
            out.contains("Layout.preferredWidth: columnWidths[c]"),
            "missing column-width thread:\n{out}"
        );
        // Fixed cell height from the part's `height: 22px`.
        assert!(out.contains("implicitHeight: 22"), "missing height:\n{out}");
    }

    /// The cell `Rectangle`'s `color:` is a nested conditional driven by
    /// the `state-when-*` predicates — editing wins over selected wins
    /// over the inherited sheet background.
    #[test]
    fn styled_box_cell_background_is_state_conditional() {
        let m = styled_cell_component("Grid");
        let l = styled_cell_table_layout("Grid");
        let s = cell_style("Grid");
        let out = from_pipeline(&m, &l, &s).unwrap().output;

        // editing (#1f4f3f) outermost, then selected (#264f78), then the
        // inherited sheet background (#1e1e1e) as the else branch.
        assert!(
            out.contains("color: ( ( r == editRow && c == editCol ) ) ? \"#1f4f3f\""),
            "editing branch missing:\n{out}"
        );
        assert!(
            out.contains("\"#264f78\""),
            "selected colour missing:\n{out}"
        );
        assert!(
            out.contains("\"#1e1e1e\""),
            "inherited sheet background fallback missing:\n{out}"
        );
    }

    /// The inner `Text` fills the cell, right-aligns (the part's
    /// `text-align: right`), and takes a selected-conditional colour plus
    /// the inherited monospace font.
    #[test]
    fn styled_box_cell_inner_text_aligns_and_colours() {
        let m = styled_cell_component("Grid");
        let l = styled_cell_table_layout("Grid");
        let s = cell_style("Grid");
        let out = from_pipeline(&m, &l, &s).unwrap().output;

        assert!(
            out.contains("horizontalAlignment: Text.AlignRight"),
            "inner Text must right-align:\n{out}"
        );
        assert!(
            out.contains("anchors.fill: parent"),
            "inner Text must fill the cell:\n{out}"
        );
        // selected → white, else the inherited sheet text colour.
        assert!(
            out.contains(
                "color: ( ( r == selectedRow && c == selectedCol ) ) ? \"#ffffff\" : \"#cccccc\""
            ),
            "inner Text colour conditional missing:\n{out}"
        );
        assert!(
            out.contains("font.family: \"monospace\""),
            "inherited monospace font missing:\n{out}"
        );
    }

    /// The `For` delegate `Item` sizes to its children so the styled cell
    /// `Rectangle` actually drives the row layout (the fix for the
    /// collapsed-grid bug).
    #[test]
    fn for_delegate_sizes_to_children() {
        let m = styled_cell_component("Grid");
        let l = styled_cell_table_layout("Grid");
        let s = cell_style("Grid");
        let out = from_pipeline(&m, &l, &s).unwrap().output;
        assert!(
            out.contains("implicitWidth: childrenRect.width"),
            "delegate must size to children:\n{out}"
        );
        assert!(
            out.contains("implicitHeight: childrenRect.height"),
            "delegate must size to children:\n{out}"
        );
    }

    /// A header `Box [header-cell]` (centered, gray, no own height) still
    /// becomes a styled `Rectangle` with a centered inner Text.
    #[test]
    fn styled_header_cell_centers_and_greys() {
        let style = StyleDef {
            component_name: "Grid".to_string(),
            parts: vec![PartStyle {
                name: "header-cell".to_string(),
                base: vec![
                    sp("background", "#2d2d30"),
                    sp("color", "#9d9d9d"),
                    sp("text-align", "center"),
                    sp("border-width", "1px"),
                    sp("border-color", "#3f3f46"),
                ],
                transitions: vec![],
                states: vec![],
            }],
        };
        let header_box = LayoutNode {
            tag: "Box".to_string(),
            part_name: Some("header-cell".to_string()),
            props: vec![],
            children: vec![LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![lp("content", LayoutPropValue::Expr("( h )".to_string()))],
                children: vec![],
            }],
        };
        let l = LayoutDef {
            component_name: "Grid".to_string(),
            root: header_box,
        };
        let m = component("Grid", vec![], vec![]);
        let out = from_pipeline(&m, &l, &style).unwrap().output;
        assert!(
            out.contains("Rectangle {"),
            "header must be a Rectangle:\n{out}"
        );
        assert!(
            out.contains("color: \"#2d2d30\""),
            "header bg missing:\n{out}"
        );
        assert!(
            out.contains("horizontalAlignment: Text.AlignHCenter"),
            "header must center text:\n{out}"
        );
        assert!(
            out.contains("color: \"#9d9d9d\""),
            "header text colour missing:\n{out}"
        );
    }

    #[test]
    fn styled_column_gap_and_max_width_lower_to_native_qml_layout_attrs() {
        let style = StyleDef {
            component_name: "Panel".to_string(),
            parts: vec![PartStyle {
                name: "screen".to_string(),
                base: vec![sp("gap", "14"), sp("max-width", "980px")],
                transitions: vec![],
                states: vec![],
            }],
        };
        let layout = LayoutDef {
            component_name: "Panel".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: Some("screen".to_string()),
                props: vec![],
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![lp("content", LayoutPropValue::String("Hello".to_string()))],
                    children: vec![],
                }],
            },
        };
        let out = from_pipeline(&component("Panel", vec![], vec![]), &layout, &style)
            .unwrap()
            .output;

        assert!(
            out.contains("ColumnLayout {"),
            "styled column should stay a native layout:\n{out}"
        );
        assert!(
            out.contains("spacing: 14"),
            "gap should lower to QML spacing:\n{out}"
        );
        assert!(
            out.contains("Layout.maximumWidth: 980"),
            "max-width should lower to Layout.maximumWidth:\n{out}"
        );
        assert!(
            !out.contains("max-width"),
            "CSS property name must not leak into QML:\n{out}"
        );
    }

    #[test]
    fn styled_row_with_paint_and_padding_wraps_native_qml_layout() {
        let style = StyleDef {
            component_name: "Header".to_string(),
            parts: vec![PartStyle {
                name: "app-header".to_string(),
                base: vec![
                    sp("background", "#0f172a"),
                    sp("border-color", "#334155"),
                    sp("border-radius", "8"),
                    sp("border-width", "1"),
                    sp("gap", "16"),
                    sp("padding", "12"),
                ],
                transitions: vec![],
                states: vec![],
            }],
        };
        let layout = LayoutDef {
            component_name: "Header".to_string(),
            root: LayoutNode {
                tag: "Row".to_string(),
                part_name: Some("app-header".to_string()),
                props: vec![],
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![lp("content", LayoutPropValue::String("Header".to_string()))],
                    children: vec![],
                }],
            },
        };
        let out = from_pipeline(&component("Header", vec![], vec![]), &layout, &style)
            .unwrap()
            .output;

        assert!(
            out.contains("Rectangle {"),
            "painted row should get a Rectangle wrapper:\n{out}"
        );
        assert!(
            out.contains("color: \"#0f172a\""),
            "missing background:\n{out}"
        );
        assert!(
            out.contains("border.color: \"#334155\""),
            "missing border color:\n{out}"
        );
        assert!(
            out.contains("border.width: 1"),
            "missing border width:\n{out}"
        );
        assert!(out.contains("radius: 8"), "missing radius:\n{out}");
        assert!(
            out.contains("RowLayout {"),
            "missing inner row layout:\n{out}"
        );
        assert!(out.contains("x: 12"), "missing horizontal inset:\n{out}");
        assert!(out.contains("y: 12"), "missing vertical inset:\n{out}");
        assert!(
            out.contains("spacing: 16"),
            "missing native spacing:\n{out}"
        );
    }

    #[test]
    fn styled_text_part_lowers_font_and_color_to_qml_text_attrs() {
        let style = StyleDef {
            component_name: "Title".to_string(),
            parts: vec![PartStyle {
                name: "app-title".to_string(),
                base: vec![
                    sp("color", "#f8fafc"),
                    sp("font-size", "28"),
                    sp("font-weight", "700"),
                    sp("text-align", "center"),
                ],
                transitions: vec![],
                states: vec![],
            }],
        };
        let layout = LayoutDef {
            component_name: "Title".to_string(),
            root: LayoutNode {
                tag: "Text".to_string(),
                part_name: Some("app-title".to_string()),
                props: vec![lp("content", LayoutPropValue::String("Engram".to_string()))],
                children: vec![],
            },
        };
        let out = from_pipeline(&component("Title", vec![], vec![]), &layout, &style)
            .unwrap()
            .output;

        assert!(
            out.contains("color: \"#f8fafc\""),
            "missing text color:\n{out}"
        );
        assert!(
            out.contains("font.pixelSize: 28"),
            "missing font size:\n{out}"
        );
        assert!(
            out.contains("font.bold: true"),
            "missing font weight:\n{out}"
        );
        assert!(
            out.contains("horizontalAlignment: Text.AlignHCenter"),
            "missing text alignment:\n{out}"
        );
    }

    /// An unstyled `Box` (no matching part) keeps the bare `Item` shape —
    /// the styled path must not perturb styleless emission.
    #[test]
    fn unstyled_box_still_lowers_to_item() {
        let m = component("Plain", vec![], vec![]);
        let l = single_box_layout("Plain");
        // Style declares a `cell` part, but the Box has no part_name, so
        // the styled path must not trigger.
        let s = cell_style("Plain");
        let out = from_pipeline(&m, &l, &s).unwrap().output;
        // The root wrapper Item plus the Box's own Item — no Rectangle.
        assert!(
            !out.contains("Rectangle {"),
            "unstyled Box must not become a Rectangle:\n{out}"
        );
        assert!(
            out.matches("Item {").count() >= 2,
            "Box must stay an Item:\n{out}"
        );
    }

    /// `qml_hex_color_or_none` only accepts `#RGB` / `#RRGGBB` (the
    /// security gate against attacker-controlled colour tokens reaching a
    /// QML string position).
    #[test]
    fn hex_color_gate_rejects_non_hex() {
        assert_eq!(qml_hex_color_or_none("#abc").as_deref(), Some("#aabbcc"));
        assert_eq!(qml_hex_color_or_none("#1E1E1E").as_deref(), Some("#1e1e1e"));
        assert!(qml_hex_color_or_none("red").is_none());
        assert!(qml_hex_color_or_none("rgba(0,0,0,1)").is_none());
        assert!(qml_hex_color_or_none("#12").is_none());
        assert!(qml_hex_color_or_none("#ggg").is_none());
    }
}
