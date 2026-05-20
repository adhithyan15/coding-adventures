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
//! | `HostInput`   | `TextInput { text: ...; readOnly: ...; onAccepted/Keys.onEscapePressed }`   |
//! | `HostButton`  | `Button { text: ...; enabled: ...; onClicked: ... }` (Controls 2.15)        |
//! | `HostScroll`  | `ScrollView { ... children ... }`                                           |
//!
//! ## What this emitter does NOT yet do
//!
//! See the module-level doc on the crate root (`src/lib.rs`). The short list:
//! Cell/data-Column/Grid v3 primitives (UI28 §2); `connects` wiring from
//! `EmitRef` props to `signal` emissions inside the tree; and mosstyle
//! inlining into element attributes. The remaining UI29 kernel primitives
//! (`If`, `For`, `HostTable`) wait on the U29-G1..U29-G3 grammar work.
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

use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
};
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosstyle_compiler::StyleDef;

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
    ComponentNameMismatch {
        mosmodel: String,
        moslayout: String,
    },
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
            PipelineEmitError::ComponentNameMismatch { mosmodel, moslayout } => write!(
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

/// Compile a three-file Mosaic pipeline triple to a QML source file.
///
/// The `style` argument is accepted now (rather than added later) so that
/// downstream callers — chiefly `mosaic-compile` — can build against the
/// stable signature. The style IR is not yet inlined into the QML; see the
/// module-level doc.
pub fn from_pipeline(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    _style: &StyleDef,
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
    writeln!(out, "    // Component: {name}").unwrap();

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

    // 6. The layout tree.
    writeln!(out).unwrap();
    out.push_str(&emit_qml_tree(&layout.root, 1)?);

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

// =====================================================================
// Layout tree walker
// =====================================================================

/// Walk a `LayoutNode` and produce the corresponding indented QML source.
///
/// Indentation is in units of 4 spaces per level (Qt Creator's default).
/// The `depth` argument is the *block depth from the root Item* — so the
/// outermost layout element starts at depth 1 (one level inside the root
/// wrapper).
fn emit_qml_tree(node: &LayoutNode, depth: usize) -> Result<String, PipelineEmitError> {
    // The UI29 *host* primitives (`HostInput`, `HostButton`) need
    // attribute lowering that depends on the moslayout props — slot refs
    // for `value`, emit refs for `onCommit`, etc. — none of which the
    // structural walker below can express. Those primitives get their
    // own emitter functions, mirroring the React backend's
    // `emit_input_jsx` carve-out for `Input`.
    match node.tag.as_str() {
        "HostInput" => return emit_host_input_qml(node, depth),
        "HostButton" => return emit_host_button_qml(node, depth),
        _ => {}
    }

    let pad = "    ".repeat(depth);

    // Decompose the primitive into its QML element name and any built-in
    // properties (the small chunks of QML that always go on this element
    // regardless of the moslayout props — e.g. Spacer's `Layout.fillWidth`).
    let QmlElement {
        element_name,
        builtin_lines,
        is_text,
        is_image,
    } = primitive_to_qml(&node.tag)?;

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
    for child in &node.children {
        let child_qml = emit_qml_tree(child, depth + 1)?;
        if is_stack {
            out.push_str(&inject_anchors_fill_parent(&child_qml, depth + 1));
        } else {
            out.push_str(&child_qml);
        }
    }

    writeln!(out, "{pad}}}").unwrap();
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
        // `HostInput` and `HostButton` are handled by their own
        // emitters earlier in `emit_qml_tree`; reaching this branch
        // would be an internal logic error.
        "HostInput" | "HostButton" => unreachable!(
            "HostInput/HostButton are handled by dedicated emitters; should not reach primitive_to_qml"
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
        // U29-G3 expression. Surface as an empty text — Qt/QML emission for
        // bound expressions needs UI29 §3.4 scope analysis to know how to
        // map names to QML property bindings.
        LayoutPropValue::Expr(_) => "text: \"\"".to_string(),
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
        // U29-G3 expression. Surface as an empty source — Qt/QML emission
        // for bound expressions needs UI29 §3.4 scope analysis.
        LayoutPropValue::Expr(_) => "source: \"\"".to_string(),
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
/// | `placeholder: "..."`    | A `// placeholder: ...` comment line. `TextInput` has no    |
/// |                         | placeholder attribute — the QML idiom is a sibling `Text`   |
/// |                         | element shown when `text.length === 0`, but that requires   |
/// |                         | a richer node-emission path; deferred to a follow-up PR.    |
/// | `onChange: emit: onE`   | `onTextChanged: e()`                                        |
/// | `onCommit: emit: onE`   | `onAccepted: e(text)` — fires on Enter                      |
/// | `onCancel: emit: onE`   | `Keys.onEscapePressed: { e(); event.accepted = true }`      |
///
/// The Enter / Escape mapping mirrors UI25 §10 (the React backend
/// merges both into a single `onKeyDown` handler; QML has dedicated
/// signal handlers for both, so we use them directly).
fn emit_host_input_qml(node: &LayoutNode, depth: usize) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();
    writeln!(out, "{pad}TextInput {{").unwrap();

    // text: <slot or literal>
    if let Some(line) = build_value_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // readOnly: <slot or literal>
    if let Some(line) = build_read_only_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // placeholder: emit as a comment only — see the table above.
    if let Some(s) = find_string_prop(node, "placeholder") {
        writeln!(
            out,
            "{inner_pad}// placeholder: \"{}\" (not yet rendered — see backend doc)",
            escape_qml_string(s)
        )
        .unwrap();
    }

    // onTextChanged: e()
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(out, "{inner_pad}onTextChanged: {camel}()").unwrap();
    }

    // onAccepted: e(text)
    if let Some(emit_name) = find_emit_ref_prop(node, "onCommit") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(out, "{inner_pad}onAccepted: {camel}(text)").unwrap();
    }

    // Keys.onEscapePressed: { e(); event.accepted = true }
    if let Some(emit_name) = find_emit_ref_prop(node, "onCancel") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(
            out,
            "{inner_pad}Keys.onEscapePressed: {{ {camel}(); event.accepted = true }}"
        )
        .unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
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
fn emit_host_button_qml(node: &LayoutNode, depth: usize) -> Result<String, PipelineEmitError> {
    let pad = "    ".repeat(depth);
    let inner_pad = "    ".repeat(depth + 1);
    let mut out = String::new();
    writeln!(out, "{pad}Button {{").unwrap();

    // text: <slot or literal> — sourced from the `label` prop.
    if let Some(line) = build_label_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // enabled: !<slot or literal> — inverted from `disabled`.
    if let Some(line) = build_disabled_to_enabled_attribute(node) {
        writeln!(out, "{inner_pad}{line}").unwrap();
    }

    // onClicked: e()
    if let Some(emit_name) = find_emit_ref_prop(node, "onTap") {
        let camel = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_safe_identifier(&camel).map_err(PipelineEmitError::UnsafeEmitName)?;
        writeln!(out, "{inner_pad}onClicked: {camel}()").unwrap();
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// True iff any node in the layout tree lowers to a `QtQuick.Controls`
/// element. Today: `HostButton` → `Button`, `HostScroll` →
/// `ScrollView`. Used to decide whether to emit
/// `import QtQuick.Controls 2.15` at the top of the file.
fn tree_needs_controls_import(node: &LayoutNode) -> bool {
    matches!(node.tag.as_str(), "HostButton" | "HostScroll")
        || node.children.iter().any(tree_needs_controls_import)
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
        LayoutPropValue::Keyword(k) => format!("text: \"{k}\""),
        LayoutPropValue::Number(n) => format!("text: \"{n}\""),
        LayoutPropValue::EmitRef(_) => "text: \"\"".to_string(),
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
        assert!(result.output.starts_with("// Auto-generated by mosaic-emit-qt"));
        assert!(result.output.contains("import QtQuick 2.15"));
        assert!(result.output.contains("import QtQuick.Layouts 1.15"));
        assert!(result.output.contains("Item {"));
        // The Box lowering → child Item.
        let item_count = result.output.matches("Item {").count();
        assert!(item_count >= 2, "expected root Item + Box Item: {}", result.output);
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
        let result =
            from_pipeline(&m, &single_box_layout("Card"), &empty_style("Card")).unwrap();
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
        let result =
            from_pipeline(&m, &single_box_layout("Misc"), &empty_style("Misc")).unwrap();
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
        let result =
            from_pipeline(&m, &single_box_layout("Grid"), &empty_style("Grid")).unwrap();
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
        let result =
            from_pipeline(&m, &single_box_layout("Grid"), &empty_style("Grid")).unwrap();
        assert!(
            result.output.contains("signal select(real startRow, real startCol)"),
            "expected typed params, got:\n{}",
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
        assert!(result.output.contains("signal userClickedHere(real eventRow)"));
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
        assert!(row_result.output.contains("RowLayout {"), "got:\n{}", row_result.output);

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
        let m = component(
            "X",
            vec![slot("photo-url", SlotType::Image, true)],
            vec![],
        );
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
        assert!(col < row && row < txt, "nesting order broken in:\n{}", result.output);
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
            PipelineEmitError::ComponentNameMismatch { mosmodel, moslayout } => {
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
        let err = from_pipeline(&m, &l, &empty_style("X"))
            .expect_err("unknown primitive must error");
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
        let result =
            from_pipeline(&m, &single_box_layout("X"), &empty_style("X")).unwrap();
        let qq = result.output.find("import QtQuick 2.15").expect("qtquick");
        let qql = result
            .output
            .find("import QtQuick.Layouts 1.15")
            .expect("qtquick.layouts");
        let item = result.output.find("Item {").expect("Item");
        assert!(qq < item && qql < item, "imports must precede Item");
    }

    // =================================================================
    // UI29 primitive kernel (U29-K-qt partial): Stack, HostInput,
    // HostButton, HostScroll. The remaining three primitives — `If`,
    // `For`, `HostTable` — wait on the U29-G1..U29-G3 grammar work and
    // are not covered here.
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
                part_name: None,
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
        assert!(result.output.contains("TextInput {"), "missing TextInput in:\n{}", result.output);
        assert!(result.output.contains("text: userText"), "missing text binding in:\n{}", result.output);
        assert!(result.output.contains("readOnly: locked"), "missing readOnly binding in:\n{}", result.output);
    }

    // -------- Test 20: HostInput with onCommit emits onAccepted --------

    /// `onCommit: emit: onSubmit` lowers to `onAccepted: submit(text)`
    /// — QML's `TextInput.onAccepted` fires on Enter, matching the
    /// UI25 §10 semantics of `onCommit`. The signal call passes the
    /// current `text` value (Qt's `TextInput.text` is in scope inside
    /// the signal handler).
    #[test]
    fn host_input_on_commit_emits_on_accepted() {
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
            result.output.contains("onAccepted: submit(text)"),
            "missing onAccepted signal call in:\n{}",
            result.output
        );
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
        assert!(result.output.contains("Button {"), "missing Button in:\n{}", result.output);
        assert!(result.output.contains("text: \"Save\""), "missing label text in:\n{}", result.output);
        assert!(result.output.contains("onClicked: save()"), "missing onClicked in:\n{}", result.output);
    }

    // -------- Test 23: HostButton with disabled flips to enabled --------

    /// `disabled: slot: x` lowers to `enabled: !x` — the polarity flip
    /// happens at lowering time because QML uses `enabled` (positive)
    /// rather than `disabled`. Verifies both a slot-ref and a literal
    /// `true` flow correctly.
    #[test]
    fn host_button_disabled_lowers_to_enabled_negated() {
        let m = component(
            "X",
            vec![slot("is-saving", SlotType::Bool, false)],
            vec![],
        );
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

        // Without any Host*-Controls primitive: NO Controls import.
        let m2 = component("Y", vec![], vec![]);
        let r_without = from_pipeline(
            &m2,
            &single_box_layout("Y"),
            &empty_style("Y"),
        )
        .unwrap();
        assert!(
            !r_without.output.contains("import QtQuick.Controls"),
            "Controls import must NOT appear when no Host*-Controls primitive used:\n{}",
            r_without.output
        );
    }

    // -------- Test 26: For/If still produce UnknownPrimitive errors --------

    /// The remaining UI29 kernel primitives `If` and `For` are
    /// out-of-scope for this PR (they wait on grammar PRs). Verify the
    /// existing error path still works: a layout node with these tags
    /// produces `UnknownPrimitive` rather than silently being accepted.
    #[test]
    fn deferred_if_for_and_host_table_still_error_as_unknown() {
        for tag in ["If", "For", "HostTable"] {
            let m = component("X", vec![], vec![]);
            let l = LayoutDef {
                component_name: "X".to_string(),
                root: LayoutNode {
                    tag: tag.to_string(),
                    part_name: None,
                    props: Vec::new(),
                    children: Vec::new(),
                },
            };
            let err = from_pipeline(&m, &l, &empty_style("X"))
                .expect_err(&format!("{tag} must still be UnknownPrimitive"));
            assert!(
                matches!(err, PipelineEmitError::UnknownPrimitive(ref t) if t == tag),
                "expected UnknownPrimitive({tag}), got {err:?}"
            );
        }
    }
}
