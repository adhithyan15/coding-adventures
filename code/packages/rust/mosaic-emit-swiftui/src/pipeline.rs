//! # Three-file pipeline entry point for the SwiftUI backend.
//!
//! Mirrors the structural layout of `mosaic-emit-react`'s
//! [`pipeline`](https://docs.rs/mosaic-emit-react) module — same public
//! function shape (`from_pipeline`), same error variants, same section
//! emitters (event union, props/struct interface, function body, JSX/View
//! tree walker). Read alongside that crate's source if you need a side-by-
//! side comparison of how the same IR lowers to React TSX vs Swift.
//!
//! ## Why these structural choices
//!
//! - **SwiftUI uses Swift `struct`s, not closures.** The React backend
//!   emits an `export function Name({ ... }: NameProps) { return (<jsx/>); }`
//!   closure. SwiftUI's idiomatic form is a `struct NameView: View` whose
//!   stored properties are the inputs and whose `var body: some View`
//!   produces the tree. We follow that convention exactly.
//!
//! - **Slots become `let` properties.** Swift's `let` denotes an immutable
//!   stored property — a perfect match for slot semantics (the host owns
//!   the value, the component does not mutate it). For `Bool`/`Int`/`Double`
//!   primitives this is also the most efficient form (value types pass by
//!   copy, no reference counting).
//!
//! - **Event enum, not protocol or struct hierarchy.** SwiftUI events
//!   collapse cleanly into a single Swift `enum` with one `case` per emit.
//!   `case navigate(row: Int, col: Int)` matches the discriminated-union
//!   shape the React backend emits as
//!   `{ type: "navigate"; row: number; col: number }` exactly.
//!
//! - **`dispatch` is a closure property.** `let dispatch: (NameEvent) -> Void`
//!   makes the dispatch site as obvious in Swift as it is in TSX: the host
//!   provides a closure, the body invokes `dispatch(.tap)`. No delegate
//!   protocols, no `@ObservedObject` ceremony.
//!
//! ## What is NOT in this first pass
//!
//! - **`Cell` / `Column` (UI28 v3) / `Grid` v3.** Per UI28 §4.4 the
//!   SwiftUI lowering plan is `Grid → SwiftUI.Table { TableColumn(...) }`
//!   with each `Column` becoming a `TableColumn` definition and the `Cell`
//!   template becoming the row-builder closure. That entire stack is
//!   tracked as a separate follow-up PR — see the TODO at the top of
//!   `lib.rs`.
//!
//! - **Style inlining (`mosstyle::StyleDef`).** Accepted in the signature
//!   so callers can build against the stable interface, but not yet
//!   inlined into `View` modifier chains (`.background(...)`,
//!   `.padding(...)`, etc.). A future PR will read the `part` blocks and
//!   emit a modifier per CSS property using the same map shape the React
//!   backend already builds.
//!
//! - **`connects` wiring + payload-carrying emits.** Emit refs on layout
//!   nodes (`Box (onTap: emit: onTap)`) aren't yet attached to SwiftUI
//!   gesture modifiers — the `dispatch` closure is generated, but no
//!   `.onTapGesture { dispatch(.tap) }` modifier lands until the follow-up
//!   PR. Payload-carrying emits work for the *enum case shape* (so the
//!   host's switch is exhaustive) but dispatch sites still pass nothing.
//!
//! - **`Scroll`, `Stack`, `Icon`, `Grid`, `Input` primitives.** They lower
//!   to `UnknownPrimitive` errors today. Each gets a dedicated emitter in
//!   a follow-up (e.g. `Scroll → ScrollView { ... }`, `Stack → ZStack`,
//!   `Icon → Image(systemName:)`).

use std::fmt::Write as _;

use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
};
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosstyle_compiler::StyleDef;

// =====================================================================
// Public API
// =====================================================================

/// The result of compiling a three-file pipeline triple to a SwiftUI
/// `.swift` file.
///
/// Mirrors `mosaic_emit_react::pipeline::PipelineEmitResult` so callers can
/// treat both backends uniformly through generic adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResult {
    /// The complete Swift source. UTF-8, terminated by a trailing newline.
    pub output: String,
    /// The component's PascalCase name (matches the source `.mil` / `.mll`
    /// component name; the generated Swift `View` struct is named
    /// `{component_name}View`).
    pub component_name: String,
}

/// Errors the SwiftUI pipeline emitter can return.
///
/// These mirror the React backend's variants verbatim so a generic CLI can
/// print them with the same code path. Each variant carries the offending
/// identifier so the caller can include it in user-visible diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEmitError {
    /// The mosmodel component name and the moslayout component name disagree.
    ComponentNameMismatch {
        mosmodel: String,
        moslayout: String,
    },
    /// A slot name fails the safe-Swift-identifier check after camelCase
    /// conversion. Should never happen given the kebab-case grammar, but we
    /// double check defensively (mirrors UI24 §8 in the React backend).
    UnsafeSlotName(String),
    /// An emit name fails the same safe-Swift-identifier check.
    UnsafeEmitName(String),
    /// A moslayout primitive tag is not recognised by the SwiftUI backend.
    /// First-pass limitations: see the module-level "What is NOT in this
    /// first pass" section.
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
                "moslayout primitive '{t}' is not yet supported by the SwiftUI emitter"
            ),
        }
    }
}

impl std::error::Error for PipelineEmitError {}

/// Compile a three-file Mosaic pipeline triple to a SwiftUI source file.
///
/// The `style` argument is accepted to lock the signature (callers and the
/// generic pipeline driver build against it today); style inlining lands in
/// a follow-up PR.
///
/// # Errors
///
/// Returns [`PipelineEmitError`] when the three IRs disagree on the
/// component name, a slot or emit name fails the safe-identifier check, or a
/// layout node uses a primitive the SwiftUI backend does not yet support.
pub fn from_pipeline(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
) -> Result<PipelineEmitResult, PipelineEmitError> {
    // 1. Sanity check: the three IRs must agree on the component name. The
    //    style IR's name is not yet enforced (matches React backend behaviour).
    if interface.component != layout.component_name {
        return Err(PipelineEmitError::ComponentNameMismatch {
            mosmodel: interface.component.clone(),
            moslayout: layout.component_name.clone(),
        });
    }
    // `style` is accepted to lock the signature; not yet inlined.
    let _ = style;

    let name = &interface.component;
    let mut out = String::new();

    // 2. File header. `import SwiftUI` is always required because every
    //    primitive lowering names a SwiftUI type — there is no equivalent
    //    of React's "do I need to import React?" tree-shake check here.
    writeln!(out, "// Auto-generated by mosaic-emit-swiftui. Do not edit.").unwrap();
    writeln!(out, "import SwiftUI").unwrap();
    writeln!(out).unwrap();

    // 3. Event enum (analog of UI24 §3.1 event union).
    out.push_str(&emit_event_union(name, &interface.emits)?);
    writeln!(out).unwrap();

    // 4. View struct: properties + body computed property.
    out.push_str(&emit_view_struct(name, &interface.slots, &layout.root)?);

    Ok(PipelineEmitResult {
        output: out,
        component_name: name.clone(),
    })
}

// =====================================================================
// Section emitters
// =====================================================================

/// Emit the Swift `enum` modelling the discriminated event-union.
///
/// Layout for a component with zero emits:
///
/// ```swift
/// enum ButtonEvent {}
/// ```
///
/// Layout for a component with one or more emits (UI24-style, Swift form):
///
/// ```swift
/// enum GridEvent {
///     case navigate(row: Int, col: Int)
///     case editCommit(value: String)
/// }
/// ```
///
/// ## Translation rules (mirrors React backend §5 conversion)
///
/// - The case name is the emit name with a leading `on` stripped and the
///   first character lowercased (`onNavigate` → `navigate`).
/// - Payload parameters become named associated values
///   (`row: Int, col: Int`). The label is the camelCased parameter name;
///   the type follows the [`emit_payload_to_swift`] table.
/// - Void emits emit a bare `case navigate` with no parens.
fn emit_event_union(component: &str, emits: &[EmitDecl]) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    if emits.is_empty() {
        // An empty Swift enum cannot have instances — exactly what `never`
        // expresses in TypeScript. Hosts importing `{Component}Event`
        // therefore get the same "no events can fire" signal.
        writeln!(out, "enum {component}Event {{}}").unwrap();
        return Ok(out);
    }
    writeln!(out, "enum {component}Event {{").unwrap();
    for e in emits {
        let lowered = strip_on_prefix(&e.name);
        let case_name = to_camel_case_first_lower(&lowered);
        validate_emit_name(&case_name)?;
        if e.params.is_empty() {
            writeln!(out, "    case {case_name}").unwrap();
        } else {
            let mut payload = String::new();
            for (i, p) in e.params.iter().enumerate() {
                let label = to_camel_case_first_lower(&p.name);
                validate_slot_or_field_name(&label)
                    .map_err(PipelineEmitError::UnsafeSlotName)?;
                let ty = emit_payload_to_swift(&p.r#type);
                if i > 0 {
                    payload.push_str(", ");
                }
                payload.push_str(&format!("{label}: {ty}"));
            }
            writeln!(out, "    case {case_name}({payload})").unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
    Ok(out)
}

/// Emit the SwiftUI `struct {Component}View: View { ... }` declaration.
///
/// Order matches the React backend's destructuring order: slots in source
/// order, then `dispatch` last so it stands out in code review.
fn emit_view_struct(
    component: &str,
    slots: &[SlotDecl],
    layout_root: &LayoutNode,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    writeln!(out, "struct {component}View: View {{").unwrap();

    // Stored properties: slots first, then dispatch.
    for s in slots {
        let field = to_camel_case_first_lower(&s.name);
        validate_slot_or_field_name(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
        let ty = slot_type_to_swift(&s.r#type);
        writeln!(out, "    let {field}: {ty}").unwrap();
    }
    writeln!(out, "    let dispatch: ({component}Event) -> Void").unwrap();
    writeln!(out).unwrap();

    // body computed property.
    writeln!(out, "    var body: some View {{").unwrap();
    let body = emit_view_tree(layout_root, 8)?;
    if body.trim().is_empty() {
        // Empty layout — emit an EmptyView so the file still type-checks.
        // Swift's `some View` cannot resolve to "nothing"; EmptyView is the
        // canonical placeholder.
        writeln!(out, "        EmptyView()").unwrap();
    } else {
        out.push_str(&body);
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    Ok(out)
}

// =====================================================================
// View tree walker
// =====================================================================

/// Walk a `LayoutNode` and produce its SwiftUI representation.
///
/// First-pass primitive coverage:
///
/// | mosmodel tag | SwiftUI output                  |
/// |--------------|---------------------------------|
/// | Box          | `Group { ... }`                 |
/// | Row          | `HStack { ... }`                |
/// | Column       | `VStack { ... }`  (see TODO)    |
/// | Text         | `Text("...")` or `Text(slot)`   |
/// | Image        | `Image(systemName: "...")`      |
/// | Spacer       | `Spacer()`                      |
/// | Divider      | `Divider()`                     |
///
/// **TODO (UI28 §2.2):** `Column` is being repurposed as Grid metadata; this
/// first cut keeps the legacy UI14 `Column → VStack` lowering so existing
/// demos still compile. The UI28 Cell/Column-as-metadata/Grid v3 lowering
/// (`Grid → SwiftUI.Table { TableColumn(...) }`) is a separate follow-up.
fn emit_view_tree(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    match node.tag.as_str() {
        // -----------------------------------------------------------------
        // Containers — open a SwiftUI view-builder block, recurse into
        // children at +4 indentation, then close.
        // -----------------------------------------------------------------
        "Box" => container("Group", node, indent),
        "Row" => container("HStack", node, indent),
        // TODO(UI28 §2.2): Column is being repurposed as Grid-metadata
        // (carries a header label + sort key + width + per-cell template,
        // discarded as a SwiftUI view). For now we keep the legacy UI14
        // semantics: `Column → VStack`. The Cell/Column/Grid v3 SwiftUI
        // lowering lands in a separate follow-up PR.
        "Column" => container("VStack", node, indent),

        // -----------------------------------------------------------------
        // Leaf primitives — emit a single line, no children.
        // -----------------------------------------------------------------
        "Text" => {
            let expr = swift_text_expression(node);
            Ok(format!("{pad}{expr}\n"))
        }
        "Spacer" => Ok(format!("{pad}Spacer()\n")),
        "Image" => {
            // SwiftUI's `Image(systemName:)` takes an SF Symbols name. We use
            // the moslayout `source` prop if it's a string literal; if it's a
            // slot ref or missing, we fall back to a placeholder symbol so
            // the file still compiles. A real image-asset pipeline (loading
            // from URLs, bundle resources, etc.) is a follow-up.
            let symbol = find_string_prop(node, "source").unwrap_or("photo");
            let escaped = escape_swift_string(symbol);
            Ok(format!("{pad}Image(systemName: \"{escaped}\")\n"))
        }
        "Divider" => Ok(format!("{pad}Divider()\n")),

        other => Err(PipelineEmitError::UnknownPrimitive(other.to_string())),
    }
}

/// Emit a SwiftUI container (`Group`, `HStack`, `VStack`) wrapping `node`'s
/// children.
fn container(
    swiftui_view: &str,
    node: &LayoutNode,
    indent: usize,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    if node.children.is_empty() {
        // Empty containers still need a body — SwiftUI's trailing-closure
        // syntax `Group { }` is valid Swift and renders nothing.
        return Ok(format!("{pad}{swiftui_view} {{ }}\n"));
    }
    let mut out = format!("{pad}{swiftui_view} {{\n");
    for child in &node.children {
        out.push_str(&emit_view_tree(child, indent + 4)?);
    }
    out.push_str(&format!("{pad}}}\n"));
    Ok(out)
}

/// Build the SwiftUI expression for a `Text` node's content.
///
/// Three cases (mirrors React backend `jsx_text_content`):
///
/// 1. `Text { content: "literal"; }` → `Text("literal")` with quotes
///    escaped per Swift string-literal rules.
/// 2. `Text { content: @slot; }` → `Text(slotName)` referencing the let
///    property by name.
/// 3. `Text { }` → `Text("")` placeholder.
fn swift_text_expression(node: &LayoutNode) -> String {
    for prop in &node.props {
        if prop.name == "content" {
            match &prop.value {
                LayoutPropValue::String(s) => {
                    return format!("Text(\"{}\")", escape_swift_string(s));
                }
                LayoutPropValue::SlotRef(slot) => {
                    return format!("Text({})", to_camel_case_first_lower(slot));
                }
                LayoutPropValue::Keyword(k) => {
                    // Keyword values (e.g. enum members) are rare for Text
                    // content but pass through as a quoted literal so the
                    // file still compiles.
                    return format!("Text(\"{}\")", escape_swift_string(k));
                }
                LayoutPropValue::Number(n) => {
                    // Stringified numbers — SwiftUI's `Text(_:)` accepts
                    // numerics but only via specific initialisers, so wrap
                    // as a string literal for the broadest match.
                    return format!("Text(\"{n}\")");
                }
                LayoutPropValue::EmitRef(_) => {
                    // Emit refs are not valid for Text content; fall through
                    // to the empty placeholder.
                }
                LayoutPropValue::Expr(_) => {
                    // U29-G3 expression-valued content. SwiftUI emission for
                    // bound expressions requires UI29 §3.4 scope analysis;
                    // for now fall through to the empty placeholder rather
                    // than emit a Swift compile error.
                }
            }
        }
    }
    "Text(\"\")".to_string()
}

// =====================================================================
// Type mapping (mosmodel SlotType -> Swift type)
// =====================================================================

/// Map a mosmodel slot type to its Swift surface type.
///
/// | mosmodel        | Swift              |
/// |-----------------|--------------------|
/// | `text`          | `String`           |
/// | `number`        | `Double`           |
/// | `bool`          | `Bool`             |
/// | `image`         | `String` (asset name or URL) |
/// | `color`         | `String` (hex / named) — `Color` once a parser lands |
/// | `node`          | `AnyView`          |
/// | `list<T>`       | `[<inner Swift type>]` |
/// | `<Component>`   | `AnyView` (named component types pass through) |
fn slot_type_to_swift(t: &SlotType) -> String {
    match t {
        SlotType::Text => "String".to_string(),
        SlotType::Number => "Double".to_string(),
        SlotType::Bool => "Bool".to_string(),
        SlotType::Image => "String".to_string(),
        SlotType::Color => "String".to_string(),
        SlotType::Node => "AnyView".to_string(),
        SlotType::List(inner) => format!("[{}]", list_inner_to_swift(inner)),
        SlotType::Component(_) => "AnyView".to_string(),
    }
}

fn list_inner_to_swift(t: &ListInnerType) -> String {
    match t {
        ListInnerType::Text => "String".to_string(),
        ListInnerType::Number => "Double".to_string(),
        ListInnerType::Bool => "Bool".to_string(),
        ListInnerType::Image => "String".to_string(),
        ListInnerType::Color => "String".to_string(),
        ListInnerType::Node => "AnyView".to_string(),
        ListInnerType::Component(_) => "AnyView".to_string(),
        ListInnerType::List(deeper) => format!("[{}]", list_inner_to_swift(deeper)),
    }
}

/// Map a mosmodel emit payload type to its Swift type.
fn emit_payload_to_swift(t: &EmitPayloadType) -> String {
    match t {
        EmitPayloadType::Text => "String".to_string(),
        EmitPayloadType::Number => "Double".to_string(),
        EmitPayloadType::Bool => "Bool".to_string(),
        EmitPayloadType::Color => "String".to_string(),
        EmitPayloadType::Component(_) => "AnyView".to_string(),
    }
}

// =====================================================================
// Helpers — name conversion, safety, escaping
// =====================================================================

/// Find a prop on `node` whose value is a `String` literal. Returns the
/// unescaped inner text, or `None`.
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

/// Escape a string for embedding inside a Swift `"..."` literal.
///
/// Swift string literals escape `\` and `"` with a leading backslash;
/// newlines and tabs are preserved as the literal characters (Swift
/// accepts both).
fn escape_swift_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out
}

/// Strip a leading `on` (case-insensitive). Mirrors React backend §5.
fn strip_on_prefix(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'o' || bytes[0] == b'O')
        && (bytes[1] == b'n' || bytes[1] == b'N')
    {
        s[2..].to_string()
    } else {
        s.to_string()
    }
}

/// Convert kebab-case to camelCase with the first character lowered.
///
/// | Input          | Output       |
/// |----------------|--------------|
/// | `avatar-url`   | `avatarUrl`  |
/// | `display-name` | `displayName`|
/// | `Navigate`     | `navigate`   |
/// | `formula`      | `formula`    |
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

/// Validate that a slot or destructured-field name is a safe identifier.
/// Returns `Err(name)` on failure so the caller can wrap it in the
/// appropriate error variant.
fn validate_slot_or_field_name(s: &str) -> Result<(), String> {
    if is_safe_swift_identifier(s) {
        Ok(())
    } else {
        Err(s.to_string())
    }
}

/// Validate that an emit's lowered case name is a safe Swift identifier.
fn validate_emit_name(s: &str) -> Result<(), PipelineEmitError> {
    if !is_safe_swift_identifier(s) {
        return Err(PipelineEmitError::UnsafeEmitName(s.to_string()));
    }
    Ok(())
}

/// `true` iff `s` matches `[_a-zA-Z][_a-zA-Z0-9]*`. Swift allows a broader
/// identifier set (Unicode letters, etc.) but the safe ASCII subset is the
/// only thing the moslayout/mosmodel kebab-case grammar can produce after
/// camelCase conversion — anything else means the identifier was
/// hand-constructed and may be hostile.
fn is_safe_swift_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use moslayout_compiler::{LayoutNode, LayoutProp};
    use mosmodel_compiler::EmitParam;
    use mosstyle_compiler::StyleDef;

    // ---------------------------------------------------------------------
    // Test helpers — keep tests short by hiding the construction noise.
    // ---------------------------------------------------------------------

    fn empty_style(component: &str) -> StyleDef {
        StyleDef {
            component_name: component.to_string(),
            parts: Vec::new(),
        }
    }

    fn box_layout(component: &str) -> LayoutDef {
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

    fn leaf(tag: &str, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props,
            children: Vec::new(),
        }
    }

    fn container_node(tag: &str, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props: Vec::new(),
            children,
        }
    }

    fn layout_with(component: &str, root: LayoutNode) -> LayoutDef {
        LayoutDef {
            component_name: component.to_string(),
            root,
        }
    }

    fn component(
        name: &str,
        slots: Vec<SlotDecl>,
        emits: Vec<EmitDecl>,
    ) -> MosmodelComponent {
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

    fn emit(name: &str, params: Vec<EmitParam>) -> EmitDecl {
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

    fn prop_string(name: &str, value: &str) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value: LayoutPropValue::String(value.to_string()),
        }
    }

    fn prop_slot_ref(name: &str, slot: &str) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value: LayoutPropValue::SlotRef(slot.to_string()),
        }
    }

    // ---------------------------------------------------------------------
    // Test 1 — empty-layout component still produces a compilable file.
    //
    // A `Box {}` component with no slots must still emit a complete file
    // with `import SwiftUI`, the event enum, and the View struct.
    // ---------------------------------------------------------------------

    #[test]
    fn empty_box_layout_compiles_clean() {
        let m = component("Empty", vec![], vec![]);
        let l = box_layout("Empty");
        let s = empty_style("Empty");
        let result = from_pipeline(&m, &l, &s).expect("emit ok");
        assert!(result.output.contains("import SwiftUI"));
        assert!(result.output.contains("enum EmptyEvent {}"));
        assert!(result.output.contains("struct EmptyView: View {"));
        assert!(result.output.contains("var body: some View {"));
        assert!(result.output.contains("Group { }"));
        assert_eq!(result.component_name, "Empty");
    }

    // ---------------------------------------------------------------------
    // Test 2 — slots become `let` properties with the right Swift types.
    //
    // Covers all primitive slot types in one pass: text, number, bool,
    // image, color, and list<text>.
    // ---------------------------------------------------------------------

    #[test]
    fn slots_render_as_let_properties_with_swift_types() {
        let m = component(
            "Profile",
            vec![
                slot("display-name", SlotType::Text, true),
                slot("age", SlotType::Number, true),
                slot("is-active", SlotType::Bool, true),
                slot("avatar-url", SlotType::Image, true),
                slot("tint", SlotType::Color, true),
                slot(
                    "tags",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
            ],
            vec![],
        );
        let l = box_layout("Profile");
        let out = from_pipeline(&m, &l, &empty_style("Profile")).unwrap().output;

        assert!(out.contains("let displayName: String"));
        assert!(out.contains("let age: Double"));
        assert!(out.contains("let isActive: Bool"));
        assert!(out.contains("let avatarUrl: String"));
        assert!(out.contains("let tint: String"));
        assert!(out.contains("let tags: [String]"));
        // Dispatch is always last (mirrors React backend convention).
        assert!(out.contains("let dispatch: (ProfileEvent) -> Void"));
    }

    // ---------------------------------------------------------------------
    // Test 3 — event enum: empty emit list produces `enum NameEvent {}`.
    //
    // Empty Swift enums cannot be instantiated, which is the SwiftUI
    // analog of TypeScript's `type NameEvent = never`.
    // ---------------------------------------------------------------------

    #[test]
    fn empty_emit_list_emits_uninhabitable_enum() {
        let m = component("Btn", vec![], vec![]);
        let out = from_pipeline(&m, &box_layout("Btn"), &empty_style("Btn"))
            .unwrap()
            .output;
        assert!(out.contains("enum BtnEvent {}"));
    }

    // ---------------------------------------------------------------------
    // Test 4 — event enum: one case per emit, in source order, with
    // payload-typed associated values.
    // ---------------------------------------------------------------------

    #[test]
    fn event_enum_has_one_case_per_emit_with_payloads() {
        let m = component(
            "Grid",
            vec![],
            vec![
                emit(
                    "onNavigate",
                    vec![param("row", EmitPayloadType::Number), param("col", EmitPayloadType::Number)],
                ),
                emit("onEditCommit", vec![param("value", EmitPayloadType::Text)]),
                emit("onClear", vec![]),
            ],
        );
        let out = from_pipeline(&m, &box_layout("Grid"), &empty_style("Grid"))
            .unwrap()
            .output;

        assert!(out.contains("case navigate(row: Double, col: Double)"));
        assert!(out.contains("case editCommit(value: String)"));
        assert!(out.contains("case clear"));
        // Source order is preserved.
        let nav = out.find("case navigate").expect("navigate present");
        let commit = out.find("case editCommit").expect("commit present");
        let clear = out.find("case clear").expect("clear present");
        assert!(nav < commit && commit < clear);
    }

    // ---------------------------------------------------------------------
    // Test 5 — primitive lowering: Box/Row/Column/Text/Spacer/Image/Divider.
    //
    // One assertion per primitive verifies the SwiftUI surface name. Box
    // → Group, Row → HStack, Column → VStack (legacy UI14 semantics —
    // see UI28 §2.2 TODO in the source), Text/Spacer/Image/Divider →
    // their corresponding SwiftUI built-ins.
    // ---------------------------------------------------------------------

    #[test]
    fn primitives_lower_to_correct_swiftui_views() {
        let cases: &[(&str, &str)] = &[
            ("Box", "Group {"),
            ("Row", "HStack {"),
            ("Column", "VStack {"),
            ("Spacer", "Spacer()"),
            ("Divider", "Divider()"),
        ];
        for (tag, expected) in cases {
            let layout = layout_with(
                "X",
                container_node("Box", vec![leaf(tag, vec![])]),
            );
            let out = from_pipeline(
                &component("X", vec![], vec![]),
                &layout,
                &empty_style("X"),
            )
            .unwrap()
            .output;
            // Containers print `{ }` for empty bodies (one space inside the
            // braces), so we use a strict prefix to allow either form.
            assert!(
                out.contains(expected) || out.contains(&format!("{} }}", expected.trim_end_matches('{').trim())),
                "expected primitive {tag} to lower containing '{expected}', got:\n{out}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Test 6 — Text with a string-literal content prop emits a quoted
    // Swift literal with quotes / backslashes escaped.
    // ---------------------------------------------------------------------

    #[test]
    fn text_with_literal_content_emits_quoted_string() {
        let layout = layout_with(
            "Hello",
            container_node(
                "Box",
                vec![leaf("Text", vec![prop_string("content", "Hi \"world\"")])],
            ),
        );
        let out = from_pipeline(
            &component("Hello", vec![], vec![]),
            &layout,
            &empty_style("Hello"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(r#"Text("Hi \"world\"")"#),
            "expected escaped Swift literal, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 7 — Text with a slot-ref content prop references the camelCased
    // let property by name (no quotes — it's an identifier, not a literal).
    // ---------------------------------------------------------------------

    #[test]
    fn text_with_slot_ref_content_emits_identifier() {
        let layout = layout_with(
            "Label",
            container_node(
                "Box",
                vec![leaf(
                    "Text",
                    vec![prop_slot_ref("content", "display-name")],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "Label",
                vec![slot("display-name", SlotType::Text, true)],
                vec![],
            ),
            &layout,
            &empty_style("Label"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("Text(displayName)"),
            "expected Text(displayName), got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 8 — Image lowers to `Image(systemName: "...")` placeholder,
    // pulling its symbol name from the `source` string prop if present, or
    // falling back to `photo` when no source is bound.
    // ---------------------------------------------------------------------

    #[test]
    fn image_lowers_to_system_name_with_source_fallback() {
        // Bound source.
        let layout = layout_with(
            "Pic",
            container_node(
                "Box",
                vec![leaf("Image", vec![prop_string("source", "star.fill")])],
            ),
        );
        let out = from_pipeline(
            &component("Pic", vec![], vec![]),
            &layout,
            &empty_style("Pic"),
        )
        .unwrap()
        .output;
        assert!(out.contains(r#"Image(systemName: "star.fill")"#));

        // Unbound source — falls back to "photo".
        let layout2 = layout_with(
            "Pic2",
            container_node("Box", vec![leaf("Image", vec![])]),
        );
        let out2 = from_pipeline(
            &component("Pic2", vec![], vec![]),
            &layout2,
            &empty_style("Pic2"),
        )
        .unwrap()
        .output;
        assert!(out2.contains(r#"Image(systemName: "photo")"#));
    }

    // ---------------------------------------------------------------------
    // Test 9 — kebab → camelCase across slot, emit, and emit-param names.
    //
    // The conversion rule is the same one the React backend ships, so the
    // generated property names line up byte-for-byte across backends.
    // ---------------------------------------------------------------------

    #[test]
    fn kebab_to_camel_case_across_names() {
        let m = component(
            "Form",
            vec![slot("first-name", SlotType::Text, true)],
            vec![emit(
                "onSubmit",
                vec![param("first-name", EmitPayloadType::Text)],
            )],
        );
        let out = from_pipeline(&m, &box_layout("Form"), &empty_style("Form"))
            .unwrap()
            .output;
        assert!(out.contains("let firstName: String"));
        // Emit case name: `onSubmit` strips `on` and lower-camels.
        assert!(out.contains("case submit(firstName: String)"));
    }

    // ---------------------------------------------------------------------
    // Test 10 — component name mismatch across the three IRs produces a
    // structured error rather than silently emitting a malformed file.
    // ---------------------------------------------------------------------

    #[test]
    fn component_name_mismatch_returns_error() {
        let m = component("Foo", vec![], vec![]);
        let l = box_layout("Bar");
        let s = empty_style("Foo");
        let err = from_pipeline(&m, &l, &s).expect_err("should reject");
        assert!(
            matches!(
                err,
                PipelineEmitError::ComponentNameMismatch { ref mosmodel, ref moslayout }
                    if mosmodel == "Foo" && moslayout == "Bar"
            ),
            "got: {err:?}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 11 — unknown primitive (e.g. Scroll, Stack) returns
    // UnknownPrimitive rather than emitting a placeholder. The follow-up
    // PRs will add these one at a time; the error keeps callers honest
    // until then.
    // ---------------------------------------------------------------------

    #[test]
    fn unknown_primitive_returns_error() {
        let layout = layout_with(
            "X",
            container_node("Box", vec![leaf("Scroll", vec![])]),
        );
        let err = from_pipeline(
            &component("X", vec![], vec![]),
            &layout,
            &empty_style("X"),
        )
        .expect_err("Scroll is not yet supported");
        assert!(
            matches!(err, PipelineEmitError::UnknownPrimitive(ref t) if t == "Scroll"),
            "got: {err:?}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 12 — full smoke: a realistic component (Row containing Text +
    // Spacer + Text) produces a SwiftUI source string that begins with the
    // canonical header lines and contains every expected piece.
    // ---------------------------------------------------------------------

    #[test]
    fn full_smoke_test_realistic_component() {
        let m = component(
            "Header",
            vec![
                slot("title", SlotType::Text, true),
                slot("subtitle", SlotType::Text, true),
            ],
            vec![emit("onTap", vec![])],
        );
        let layout = layout_with(
            "Header",
            container_node(
                "Row",
                vec![
                    leaf("Text", vec![prop_slot_ref("content", "title")]),
                    leaf("Spacer", vec![]),
                    leaf("Text", vec![prop_slot_ref("content", "subtitle")]),
                ],
            ),
        );
        let out = from_pipeline(&m, &layout, &empty_style("Header")).unwrap().output;

        // Header lines in the expected order.
        let import_pos = out.find("import SwiftUI").expect("import present");
        let enum_pos = out.find("enum HeaderEvent {").expect("enum present");
        let struct_pos = out.find("struct HeaderView: View {").expect("struct present");
        let body_pos = out.find("var body: some View {").expect("body present");
        assert!(import_pos < enum_pos && enum_pos < struct_pos && struct_pos < body_pos);

        // Emit case (void).
        assert!(out.contains("case tap"));
        // Slot properties.
        assert!(out.contains("let title: String"));
        assert!(out.contains("let subtitle: String"));
        // Dispatch property.
        assert!(out.contains("let dispatch: (HeaderEvent) -> Void"));
        // Layout body.
        assert!(out.contains("HStack {"));
        assert!(out.contains("Text(title)"));
        assert!(out.contains("Spacer()"));
        assert!(out.contains("Text(subtitle)"));
    }

    // ---------------------------------------------------------------------
    // Test 13 — version pin: the crate version is 0.1.0. Catches accidental
    // version bumps before they merge.
    // ---------------------------------------------------------------------

    #[test]
    fn version_is_0_1_0() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }
}
