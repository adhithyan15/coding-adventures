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
//! - **`Icon`, `Grid` (v2) primitives.** They still lower to
//!   `UnknownPrimitive` errors today. `Stack`, `HostScroll`, `HostInput`,
//!   and `HostButton` landed in v0.2.0 (UI29 kernel partial); the
//!   remaining primitives get dedicated emitters in follow-ups.
//!
//! ## UI29 kernel partial (v0.2.0 / v0.3.0 / v0.4.0)
//!
//! All UI29 kernel primitives now have a SwiftUI lowering. The two
//! meta-primitives (`For`, `If`/`Else`) land in v0.4.0 (U29-K-swiftui)
//! via [`emit_for_swift`] and [`emit_if_swift`].
//!
//! | UI29 primitive | SwiftUI lowering                                |
//! |----------------|-------------------------------------------------|
//! | `Stack`        | `ZStack { ... }` (z-axis / overlay container)   |
//! | `HostScroll`   | `ScrollView { ... }`                            |
//! | `HostInput`    | `TextField(placeholder, text: .constant(value))` |
//! | `HostButton`   | `Button(action:) { Text(label) }`               |
//! | `HostTable`    | `VStack { HStack { ... } }` (see [`emit_host_table`]) |
//! | `For`          | `ForEach(...) { ... }` (see [`emit_for_swift`]) |
//! | `If` / `Else`  | `if cond { ... } else { ... }` (see [`emit_if_swift`]) |
//!
//! ### `HostInput` binding choice
//!
//! SwiftUI `TextField` requires a `Binding<String>`, not a plain `String`.
//! Mosaic components receive slots as immutable `let`s, so we have two
//! options:
//!
//! - **(a) `.constant(value)` wrapper** — emit `.constant(value)` and rely
//!   on the host's flux dispatch loop to push new text back through the
//!   slot. The user-visible cost is that inline typing doesn't echo
//!   character-by-character — only `onSubmit` (Enter) carries the new
//!   buffer. UI24's dispatch-driven update pattern already matches this
//!   shape.
//! - **(b) Local `@State` proxy** — wrap the body in a `@State` buffer
//!   that initialises from the slot and dispatches `onChange` per keystroke.
//!   More complex generated code; deferred.
//!
//! This PR ships option (a). Option (b) is a future enhancement.

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
        // UI29 kernel partial — Stack is the z-axis / overlay container.
        // It is *not* a synonym for VStack: SwiftUI's `ZStack` overlays
        // children along the depth axis, which is the UI29 semantics.
        "Stack" => container("ZStack", node, indent),
        // UI29 kernel partial — `HostScroll` is the kernel form of a
        // scrollable region. SwiftUI's `ScrollView` is the direct analog;
        // it implicitly handles its own scroll-state and viewport, so we
        // do not need to thread offset/extent slots through here.
        "HostScroll" => container("ScrollView", node, indent),

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

        // UI29 kernel partial — `HostInput` and `HostButton` are leaf
        // primitives backed by SwiftUI's `TextField` and `Button`
        // respectively. They read slot/emit refs off the node props; see
        // the per-function doc comments below for the full mapping.
        "HostInput" => emit_host_input(node, indent),
        "HostButton" => emit_host_button(node, indent),

        // UI29 kernel — `HostTable` is the semantic data-table primitive.
        // See [`emit_host_table`] for the lowering rationale (VStack +
        // HStack rows, not SwiftUI.Table — for now).
        "HostTable" => emit_host_table(node, indent),

        // UI29 kernel — table sub-tags. When they appear OUTSIDE of a
        // HostTable parent they have nothing to attach to in SwiftUI;
        // we emit a self-documenting comment rather than erroring so the
        // generated file still type-checks (a Swift comment is a
        // statement-level no-op).
        "HostTableHead"
        | "HostTableBody"
        | "HostTableFoot"
        | "HostTableColGroup" => Ok(format!(
            "{pad}// {} outside HostTable — ignored\n",
            node.tag
        )),

        // `Row` outside a HostTable (and outside the normal Row container
        // path handled above) cannot appear here because `Row` is matched
        // earlier in this `match`. Sub-tag `Row` is only reached via
        // [`emit_host_table`]'s explicit recursion, not via this walker.

        // UI29 §3.1 / §3.2 — meta-primitives. `For` always lowers
        // standalone; `If` may pair with a following `Else` sibling, but
        // when reached through this single-node walker the sibling is
        // not visible — we lower as an if-only. The sibling-aware
        // pairing happens in [`emit_children`], which container-shaped
        // parents call instead of looping `emit_view_tree` directly.
        "For" => emit_for_swift(node, indent),
        "If" => emit_if_swift(node, None, indent),
        // An orphan `Else` (Else not preceded by If) is rejected by the
        // moslayout analyzer, but the emitter is defensive: rather than
        // erroring at the Swift level we emit a self-documenting Swift
        // comment so the generated file still type-checks.
        "Else" => Ok(format!("{}// orphan Else — ignored\n", " ".repeat(indent))),

        other => Err(PipelineEmitError::UnknownPrimitive(other.to_string())),
    }
}

/// Emit a SwiftUI container (`Group`, `HStack`, `VStack`) wrapping `node`'s
/// children.
///
/// Children are walked via [`emit_children`], which is sibling-aware so an
/// `If` immediately followed by an `Else` is paired into a single
/// `if/else` block rather than two stray nodes. All other primitives go
/// through `emit_view_tree` unchanged.
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
    out.push_str(&emit_children(&node.children, indent + 4)?);
    out.push_str(&format!("{pad}}}\n"));
    Ok(out)
}

/// Walk a flat list of sibling layout nodes at `indent`, with two
/// pieces of sibling-aware behaviour:
///
/// 1. An `If` node followed immediately by an `Else` is consumed as a
///    pair and lowered to a single `if cond { ... } else { ... }`
///    block (see [`emit_if_swift`]). The `Else` is then skipped on the
///    next iteration step.
/// 2. An `Else` that does *not* immediately follow an `If` is an
///    orphan; the moslayout analyzer rejects this, but we render a
///    Swift comment so the generated file still compiles.
///
/// Everything else delegates to [`emit_view_tree`].
fn emit_children(
    children: &[LayoutNode],
    indent: usize,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if child.tag == "If" {
            // Peek: if the next sibling is `Else`, consume both into a
            // paired emission and advance past the Else.
            let else_node = children.get(i + 1).filter(|n| n.tag == "Else");
            out.push_str(&emit_if_swift(child, else_node, indent)?);
            i += if else_node.is_some() { 2 } else { 1 };
            continue;
        }
        if child.tag == "Else" {
            // Reached only when an `Else` did NOT immediately follow an
            // `If`; emit a documenting comment instead of crashing.
            out.push_str(&format!(
                "{}// orphan Else — ignored\n",
                " ".repeat(indent)
            ));
            i += 1;
            continue;
        }
        out.push_str(&emit_view_tree(child, indent)?);
        i += 1;
    }
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
// UI29 kernel partial — HostInput / HostButton emitters
// =====================================================================

/// Lower a UI29 `HostInput` node to a SwiftUI `TextField`.
///
/// ## Property handling
///
/// | Moslayout prop                | SwiftUI surface                                 |
/// |-------------------------------|-------------------------------------------------|
/// | `placeholder: "..."`          | First arg of `TextField("placeholder", ...)`    |
/// | `value: slot: x`              | `text: .constant(x)` (see binding nuance below) |
/// | `read-only: slot: x`          | `.disabled(x)` modifier                         |
/// | `read-only: true` / `false`   | `.disabled(true)` / `.disabled(false)` modifier |
/// | `onChange: emit: onE`         | `.onChange(of: x) { dispatch(.e(value: x)) }`   |
/// | `onCommit: emit: onE`         | `.onSubmit { dispatch(.e(value: x)) }`          |
/// | `onCancel: emit: onE`         | `.onExitCommand { dispatch(.e) }` (macOS only)  |
///
/// ## Binding nuance — why `.constant(value)`
///
/// SwiftUI `TextField` requires `text: Binding<String>`. Mosaic components
/// receive slots as `let` properties, which cannot be the target of a
/// `Binding`. Two options exist:
///
/// 1. Wrap the slot in `.constant(value)` and accept that inline typing
///    will not echo back into the bound `let` — the host must push new
///    text back through `dispatch(.change(value: ...))`. This matches
///    UI24's dispatch-driven flux pattern.
/// 2. Wrap the body in a local `@State` buffer that proxies the slot.
///    More complex generated code; deferred to a follow-up PR.
///
/// This emitter ships option (1). The `onSubmit` path still carries the
/// (unchanged) `value` slot as a payload so the host can observe Enter
/// presses; a future `@State`-proxy lowering will swap in real
/// per-keystroke `value` payloads.
///
/// ## `onCancel` platform note
///
/// SwiftUI's `.onExitCommand` modifier is **macOS-only** (it observes
/// the Escape key via the AppKit responder chain). The same callsite on
/// iOS / iPadOS will compile but never fire; document this in the
/// crate's README as a known limitation.
fn emit_host_input(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // First arg: the placeholder string literal. We escape `\` and `"`
    // per Swift string-literal rules; everything else passes through.
    let placeholder = find_string_prop(node, "placeholder").unwrap_or("");
    let placeholder_lit = format!("\"{}\"", escape_swift_string(placeholder));

    // `value: slot: x` -> `text: .constant(x)`. If no `value` is bound we
    // synthesise an empty `.constant("")` so the file still type-checks.
    let value_expr = match find_slot_ref_prop(node, "value") {
        Some(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            camel
        }
        None => "\"\"".to_string(),
    };

    // The opening `TextField` expression.
    let mut line = format!("{pad}TextField({placeholder_lit}, text: .constant({value_expr}))");

    // Modifier chain. We deliberately keep each modifier on the same
    // line — Swift accepts chained modifiers without line breaks, and
    // the generated source stays compact and grep-friendly.

    // `.disabled(...)` — true literal, false literal, or slot-bound bool.
    if let Some(slot) = find_slot_ref_prop(node, "read-only") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        line.push_str(&format!(".disabled({camel})"));
    } else if let Some(kw) = find_keyword_prop(node, "read-only") {
        if kw == "true" || kw == "false" {
            line.push_str(&format!(".disabled({kw})"));
        }
    }

    // `.onChange(of: value) { dispatch(.e(value: value)) }`. Only fires
    // when the bound slot itself changes (which, with `.constant`, only
    // happens if the host re-renders with a new `value` slot). This is
    // intentionally a no-op on most keystrokes — full per-keystroke
    // dispatch lands with the `@State`-proxy option in a future PR.
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_emit_name(&case_name)?;
        // If no value slot is bound, the `of:` target is the empty literal
        // we synthesised above, which is invalid Swift. Skip the modifier
        // in that case — there's nothing meaningful to observe.
        if find_slot_ref_prop(node, "value").is_some() {
            line.push_str(&format!(
                ".onChange(of: {value_expr}) {{ dispatch(.{case_name}(value: {value_expr})) }}"
            ));
        }
    }

    // `.onSubmit { dispatch(.e(value: value)) }`. SwiftUI fires onSubmit
    // when the user presses Enter / Return in the TextField.
    if let Some(emit_name) = find_emit_ref_prop(node, "onCommit") {
        let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_emit_name(&case_name)?;
        if find_slot_ref_prop(node, "value").is_some() {
            line.push_str(&format!(
                ".onSubmit {{ dispatch(.{case_name}(value: {value_expr})) }}"
            ));
        } else {
            // No value slot — emit the void form.
            line.push_str(&format!(".onSubmit {{ dispatch(.{case_name}) }}"));
        }
    }

    // `.onExitCommand { dispatch(.e) }` — macOS Escape-key handler.
    if let Some(emit_name) = find_emit_ref_prop(node, "onCancel") {
        let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_emit_name(&case_name)?;
        line.push_str(&format!(".onExitCommand {{ dispatch(.{case_name}) }}"));
    }

    line.push('\n');
    Ok(line)
}

/// Lower a UI29 `HostButton` node to a SwiftUI `Button`.
///
/// ## Property handling
///
/// | Moslayout prop          | SwiftUI surface                                 |
/// |-------------------------|-------------------------------------------------|
/// | `label: "..."`          | `Text("...")` inside the label closure          |
/// | `label: slot: x`        | `Text(x)` inside the label closure              |
/// | `disabled: slot: x`     | `.disabled(x)` modifier                         |
/// | `disabled: true`/`false`| `.disabled(true)` / `.disabled(false)`          |
/// | `onTap: emit: onE`      | `action: { dispatch(.e) }`                      |
///
/// ## Generated shape
///
/// ```swift
/// Button(action: { dispatch(.tap) }) {
///     Text(label)
/// }.disabled(disabled)
/// ```
///
/// If no `onTap` emit is bound the action closure is `{ }` (a no-op);
/// the file still compiles and the button is effectively decorative.
fn emit_host_button(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);

    // Action closure body.
    let action_body = match find_emit_ref_prop(node, "onTap") {
        Some(emit_name) => {
            let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
            validate_emit_name(&case_name)?;
            format!("dispatch(.{case_name})")
        }
        None => String::new(),
    };

    // Label expression. String literal → `Text("...")`; slot ref →
    // `Text(slotName)`; nothing bound → `Text("")` placeholder.
    let label_expr = if let Some(s) = find_string_prop(node, "label") {
        format!("Text(\"{}\")", escape_swift_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        format!("Text({camel})")
    } else {
        "Text(\"\")".to_string()
    };

    let mut out = String::new();
    if action_body.is_empty() {
        // No-op action closure. Still a valid Swift Button.
        writeln!(out, "{pad}Button(action: {{ }}) {{").unwrap();
    } else {
        writeln!(out, "{pad}Button(action: {{ {action_body} }}) {{").unwrap();
    }
    writeln!(out, "{inner_pad}{label_expr}").unwrap();

    // Closing brace, then any trailing modifiers on the same line so the
    // generated source stays compact.
    let mut closing = format!("{pad}}}");
    if let Some(slot) = find_slot_ref_prop(node, "disabled") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        closing.push_str(&format!(".disabled({camel})"));
    } else if let Some(kw) = find_keyword_prop(node, "disabled") {
        if kw == "true" || kw == "false" {
            closing.push_str(&format!(".disabled({kw})"));
        }
    }
    closing.push('\n');
    out.push_str(&closing);

    Ok(out)
}

// =====================================================================
// UI29 kernel — HostTable emitter
// =====================================================================

/// Lower a UI29 `HostTable` node to a SwiftUI `VStack` of `HStack` rows.
///
/// ## Why not `SwiftUI.Table`?
///
/// SwiftUI's `Table` view is data-driven: it takes a `[RowType]` collection
/// plus `TableColumn`s with key-paths, and is not naturally produced by a
/// structural "compose from children" emitter — the data shape lives in the
/// host, not in the layout IR. The follow-up that wires `For` into
/// `HostTable` will revisit this and emit a real `Table { ... }` once the
/// IR carries the row-data shape needed to drive it.
///
/// For now we do the simpler structural thing: emit a `VStack` whose
/// children are `HStack` rows, with a `Divider` separating head from body.
/// Headers render `.bold()`. The visual result is the same as
/// `SwiftUI.Table` for static rows, just without the built-in sorting /
/// selection / column-resize behaviour Table provides.
///
/// ## Sub-tag handling
///
/// | Sub-tag             | Lowering                                    |
/// |---------------------|---------------------------------------------|
/// | `HostTableHead`     | `HStack` rows, each `Text` gets `.bold()`   |
/// | `HostTableBody`     | `HStack` rows, plain text                   |
/// | `HostTableFoot`     | `Divider` then `HStack` rows                |
/// | `HostTableColGroup` | Ignored (no SwiftUI analog) — emits comment |
///
/// Section ordering follows the IR (children are walked in source order),
/// with one structural rule: when a `HostTableHead` is followed by any
/// non-head section, a `Divider()` is inserted between them. This matches
/// the HTML `<thead>` / `<tbody>` visual convention.
///
/// `Row` children inside each section become `HStack`s; the `Row`'s
/// children are emitted via the normal walker, except inside
/// `HostTableHead` where we recursively wrap every emitted `Text(...)`
/// with `.bold()`.
///
/// ## `part_name` on the table itself
///
/// SwiftUI has no native concept matching CSS `part`. For now we emit a
/// Swift comment `// part: <name>` ahead of the VStack so the part
/// metadata survives in the generated source for downstream tooling.
/// A future PR can swap this for a SwiftUI modifier once the style
/// inlining lands.
fn emit_host_table(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);

    let mut out = String::new();
    if let Some(part) = &node.part_name {
        writeln!(out, "{pad}// part: {part}").unwrap();
    }
    // NOTE: This first cut lowers HostTable to a VStack of HStack rows
    // rather than SwiftUI.Table. SwiftUI.Table is data-driven and needs a
    // row-data shape the IR does not yet carry. See [`emit_host_table`]
    // doc comment for the rationale.
    if node.children.is_empty() {
        writeln!(out, "{pad}VStack(alignment: .leading, spacing: 0) {{ }}").unwrap();
        return Ok(out);
    }
    writeln!(out, "{pad}VStack(alignment: .leading, spacing: 0) {{").unwrap();

    // Walk sections in source order. We track whether we just emitted a
    // head section so we can insert a Divider before the first non-head
    // section that follows.
    let mut head_just_emitted = false;
    for section in &node.children {
        match section.tag.as_str() {
            "HostTableHead" => {
                emit_table_section_rows(&mut out, section, indent + 4, /*bold=*/ true)?;
                head_just_emitted = true;
            }
            "HostTableBody" => {
                if head_just_emitted {
                    writeln!(out, "{inner_pad}Divider()").unwrap();
                    head_just_emitted = false;
                }
                emit_table_section_rows(&mut out, section, indent + 4, /*bold=*/ false)?;
            }
            "HostTableFoot" => {
                // Per the brief, HostTableFoot is preceded by a Divider
                // unconditionally — it visually separates totals/summary
                // rows from the body above.
                writeln!(out, "{inner_pad}Divider()").unwrap();
                head_just_emitted = false;
                emit_table_section_rows(&mut out, section, indent + 4, /*bold=*/ false)?;
            }
            "HostTableColGroup" => {
                // No SwiftUI analog — column-width metadata is part of
                // the host's data model in SwiftUI.Table-land, but here
                // there's no column-width hook to attach it to. Emit a
                // comment so the IR's intent is visible in the output.
                writeln!(
                    out,
                    "{inner_pad}// HostTableColGroup ignored in SwiftUI"
                )
                .unwrap();
                head_just_emitted = false;
            }
            other => {
                // Anything else nested directly under HostTable is not a
                // recognised table section. Mirror the orphan-sub-tag
                // path: emit a comment, don't error.
                writeln!(out, "{inner_pad}// unexpected child '{other}' in HostTable").unwrap();
                head_just_emitted = false;
            }
        }
    }

    writeln!(out, "{pad}}}").unwrap();
    Ok(out)
}

/// Walk a `HostTableHead` / `HostTableBody` / `HostTableFoot` section,
/// emitting one `HStack { ... }` per `Row` child.
///
/// `bold` toggles whether each emitted `Text(...)` line inside the row
/// gets a `.bold()` modifier — true for header rows, false otherwise.
/// Non-`Row` children of a section are passed through the regular walker
/// so the file still compiles if an author puts (say) a `Divider` between
/// rows; an explicit comment is emitted documenting the unusual nesting.
fn emit_table_section_rows(
    out: &mut String,
    section: &LayoutNode,
    indent: usize,
    bold: bool,
) -> Result<(), PipelineEmitError> {
    let pad = " ".repeat(indent);

    for child in &section.children {
        if child.tag == "Row" {
            if child.children.is_empty() {
                writeln!(out, "{pad}HStack {{ }}").unwrap();
                continue;
            }
            writeln!(out, "{pad}HStack {{").unwrap();
            for cell in &child.children {
                let emitted = emit_view_tree(cell, indent + 4)?;
                if bold {
                    // Apply `.bold()` to every `Text(...)` line we just
                    // produced. We do this by string-rewriting the
                    // emitted child rather than threading a "bold" flag
                    // through the entire walker: header text content is
                    // the only case that needs it today, and keeping the
                    // bolding scope-local prevents the rest of the
                    // walker from carrying a styling parameter.
                    for line in emitted.lines() {
                        let trimmed = line.trim_start();
                        if trimmed.starts_with("Text(") {
                            writeln!(out, "{line}.bold()").unwrap();
                        } else {
                            writeln!(out, "{line}").unwrap();
                        }
                    }
                } else {
                    out.push_str(&emitted);
                }
                // emit_view_tree already terminates each line with '\n';
                // the bold path uses writeln! which also terminates each
                // line. No additional newline needed here.
            }
            writeln!(out, "{pad}}}").unwrap();
        } else {
            // Non-Row child inside a table section. Pass through the
            // walker so e.g. a stray Divider still compiles, and prepend
            // a comment so the unusual nesting is visible.
            writeln!(out, "{pad}// non-Row child '{}' in table section", child.tag).unwrap();
            let emitted = emit_view_tree(child, indent)?;
            out.push_str(&emitted);
        }
    }
    Ok(())
}

// =====================================================================
// UI29 §3.1 / §3.2 — `For` / `If` meta-primitive emitters
// =====================================================================

/// Lower a UI29 `For` meta-primitive (§3.1) to SwiftUI's `ForEach`.
///
/// ## Lowering by shape
///
/// `For` accepts three structural props (validated upstream in
/// `moslayout-compiler::validate_for_node`):
///
/// - `each:`  a `SlotRef` or `Expr` evaluating to a list. SwiftUI's
///   `ForEach` needs an `id:` keypath so every element has a stable
///   identity across re-renders — we use `\.self` when only `as:` is
///   bound, and `\.offset` when `index:` is also bound (since we then
///   iterate `Array(<coll>.enumerated())`).
/// - `as:`    the per-element binding NAME visible inside the body.
/// - `index:` (optional) a second NAME bound to the integer index.
///
/// Two emission shapes, mirroring UI29 §3.1's example:
///
/// | shape              | SwiftUI                                                                 |
/// |--------------------|-------------------------------------------------------------------------|
/// | `as:` only         | `ForEach(<coll>, id: \.self) { <as> in <children> }`                    |
/// | `as:` + `index:`   | `ForEach(Array(<coll>.enumerated()), id: \.offset) { (<idx>, <as>) in <children> }` |
///
/// `<coll>` is either a camelCased Swift identifier (for `SlotRef`) or
/// the expression's source text verbatim (for `Expr`). The body is
/// recursed through [`emit_children`] so nested `If`/`Else` pairing
/// still works one level down.
fn emit_for_swift(
    node: &LayoutNode,
    indent: usize,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // `each:` — required. `validate_for_node` guarantees SlotRef or Expr,
    // but the emitter is defensive: if neither, fall back to an empty
    // array literal so the generated file still type-checks.
    let coll_expr = match node.props.iter().find(|p| p.name == "each") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Expr(text) => text.clone(),
            _ => "[]".to_string(),
        },
        None => "[]".to_string(),
    };

    // `as:` — required, always a Keyword per UI29 §3.1 / `validate_for_node`.
    let as_name = find_keyword_prop(node, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());

    // `index:` — optional, always a Keyword when present.
    let index_name = find_keyword_prop(node, "index").map(to_camel_case_first_lower);

    let (header, body_indent) = match &index_name {
        Some(idx) => (
            // With index: iterate over enumerated tuples and id on the
            // offset slot of `EnumeratedSequence.Element`.
            format!(
                "{pad}ForEach(Array({coll}.enumerated()), id: \\.offset) {{ ({idx}, {asn}) in\n",
                coll = coll_expr,
                idx = idx,
                asn = as_name,
            ),
            indent + 4,
        ),
        None => (
            // Without index: iterate the collection directly, id on
            // self. `\.self` requires elements be `Hashable` — for
            // primitive Swift types this holds; for richer rows the
            // user is expected to switch to the indexed form.
            format!(
                "{pad}ForEach({coll}, id: \\.self) {{ {asn} in\n",
                coll = coll_expr,
                asn = as_name,
            ),
            indent + 4,
        ),
    };

    let mut out = header;
    if node.children.is_empty() {
        // Empty body — SwiftUI's view builder requires *something* in
        // the closure. `EmptyView()` is the canonical no-op.
        out.push_str(&format!("{}EmptyView()\n", " ".repeat(body_indent)));
    } else {
        out.push_str(&emit_children(&node.children, body_indent)?);
    }
    out.push_str(&format!("{pad}}}\n"));
    Ok(out)
}

/// Lower a UI29 `If` (§3.2) — optionally paired with a following
/// `Else` sibling — to a Swift view-builder `if`/`else`.
///
/// SwiftUI's `ViewBuilder` supports `if cond { ... } else { ... }`
/// natively as long as the branches are themselves views; we lean on
/// that and emit Swift's control-flow form directly inside the
/// surrounding view block.
///
/// | shape                   | SwiftUI                                |
/// |-------------------------|----------------------------------------|
/// | `If { then }`           | `if cond { <then> }`                   |
/// | `If { then } Else { e }`| `if cond { <then> } else { <else> }`   |
///
/// `cond` is the camelCased name for a `SlotRef`, or the expression
/// source text verbatim for an `Expr`. The branches are recursed
/// through [`emit_children`] so nested `If`/`Else` still pairs.
fn emit_if_swift(
    if_node: &LayoutNode,
    else_node: Option<&LayoutNode>,
    indent: usize,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // `when:` — required. `validate_if_node` guarantees SlotRef or Expr,
    // but again be defensive: a missing `when:` becomes `false` so the
    // file still compiles (the body is unreachable but well-typed).
    let cond_expr = match if_node.props.iter().find(|p| p.name == "when") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Expr(text) => text.clone(),
            _ => "false".to_string(),
        },
        None => "false".to_string(),
    };

    let mut out = format!("{pad}if {cond} {{\n", cond = cond_expr);
    if if_node.children.is_empty() {
        out.push_str(&format!("{}EmptyView()\n", " ".repeat(indent + 4)));
    } else {
        out.push_str(&emit_children(&if_node.children, indent + 4)?);
    }

    if let Some(en) = else_node {
        out.push_str(&format!("{pad}}} else {{\n"));
        if en.children.is_empty() {
            out.push_str(&format!("{}EmptyView()\n", " ".repeat(indent + 4)));
        } else {
            out.push_str(&emit_children(&en.children, indent + 4)?);
        }
        out.push_str(&format!("{pad}}}\n"));
    } else {
        out.push_str(&format!("{pad}}}\n"));
    }

    Ok(out)
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

/// Find a prop on `node` whose value is a `SlotRef`. Returns the slot's
/// kebab-case name (e.g. `display-name`), or `None`.
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

/// Find a prop on `node` whose value is an `EmitRef`. Returns the emit's
/// camelCased name (e.g. `onTap`), or `None`.
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

/// Find a prop on `node` whose value is a `Keyword`. Returns the keyword
/// string (e.g. `"true"`, `"false"`), or `None`.
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

    fn prop_emit_ref(name: &str, emit_name: &str) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value: LayoutPropValue::EmitRef(emit_name.to_string()),
        }
    }

    fn prop_keyword(name: &str, keyword: &str) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value: LayoutPropValue::Keyword(keyword.to_string()),
        }
    }

    fn prop_expr(name: &str, text: &str) -> LayoutProp {
        LayoutProp {
            name: name.to_string(),
            value: LayoutPropValue::Expr(text.to_string()),
        }
    }

    fn node_with_props(
        tag: &str,
        props: Vec<LayoutProp>,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props,
            children,
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
    // Test 13 — version pin: the crate version is 0.4.0. Catches accidental
    // version bumps before they merge.
    // ---------------------------------------------------------------------

    #[test]
    fn version_is_0_4_0() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.4.0");
    }

    // ---------------------------------------------------------------------
    // UI29 kernel partial — tests 14 through 21.
    //
    // These exercise the four primitives added in v0.2.0:
    // Stack → ZStack, HostScroll → ScrollView, HostInput → TextField,
    // HostButton → Button.
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Test 14 — `Stack` primitive lowers to `ZStack { ... }`.
    //
    // Empty-children case mirrors the `Box → Group { }` empty-container
    // shape so the file still type-checks under SwiftUI's `some View`.
    // ---------------------------------------------------------------------

    #[test]
    fn stack_lowers_to_zstack() {
        let layout = layout_with(
            "S",
            container_node("Box", vec![leaf("Stack", vec![])]),
        );
        let out = from_pipeline(
            &component("S", vec![], vec![]),
            &layout,
            &empty_style("S"),
        )
        .unwrap()
        .output;
        assert!(out.contains("ZStack { }"), "expected ZStack, got:\n{out}");
    }

    // ---------------------------------------------------------------------
    // Test 15 — `Stack` with children emits children inside the ZStack.
    //
    // Verifies the container path (not just the leaf path) and that the
    // child indent is +4 inside the ZStack body.
    // ---------------------------------------------------------------------

    #[test]
    fn stack_with_children_emits_children_inside() {
        let layout = layout_with(
            "Layered",
            container_node(
                "Stack",
                vec![
                    leaf("Text", vec![prop_string("content", "back")]),
                    leaf("Text", vec![prop_string("content", "front")]),
                ],
            ),
        );
        let out = from_pipeline(
            &component("Layered", vec![], vec![]),
            &layout,
            &empty_style("Layered"),
        )
        .unwrap()
        .output;
        assert!(out.contains("ZStack {"));
        assert!(out.contains(r#"Text("back")"#));
        assert!(out.contains(r#"Text("front")"#));
        // Ordering — back is layered first (z-order bottom), front on top.
        let back = out.find(r#"Text("back")"#).unwrap();
        let front = out.find(r#"Text("front")"#).unwrap();
        assert!(back < front);
    }

    // ---------------------------------------------------------------------
    // Test 16 — `HostInput` with `value` + `placeholder` emits
    // `TextField("placeholder", text: .constant(value))`.
    //
    // Documents the option-(a) binding choice: the slot flows through
    // `.constant(...)` so the host's flux dispatch loop owns updates.
    // ---------------------------------------------------------------------

    #[test]
    fn host_input_emits_textfield_with_constant_binding() {
        let layout = layout_with(
            "Form",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_string("placeholder", "Search…"),
                        prop_slot_ref("value", "query"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "Form",
                vec![slot("query", SlotType::Text, true)],
                vec![],
            ),
            &layout,
            &empty_style("Form"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(r#"TextField("Search…", text: .constant(query))"#),
            "expected TextField with .constant binding, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 17 — `HostInput` with `read-only: true` emits the `.disabled(true)`
    // modifier on the TextField.
    //
    // Also exercises the slot-bound form (`read-only: slot: locked`) so
    // both branches of the `read-only` lookup are covered.
    // ---------------------------------------------------------------------

    #[test]
    fn host_input_read_only_emits_disabled_modifier() {
        // Literal `true` keyword form.
        let layout = layout_with(
            "F",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_slot_ref("value", "q"),
                        prop_keyword("read-only", "true"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component("F", vec![slot("q", SlotType::Text, true)], vec![]),
            &layout,
            &empty_style("F"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".disabled(true)"),
            "expected .disabled(true), got:\n{out}"
        );

        // Slot-bound form.
        let layout2 = layout_with(
            "G",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_slot_ref("value", "q"),
                        prop_slot_ref("read-only", "locked"),
                    ],
                )],
            ),
        );
        let out2 = from_pipeline(
            &component(
                "G",
                vec![
                    slot("q", SlotType::Text, true),
                    slot("locked", SlotType::Bool, true),
                ],
                vec![],
            ),
            &layout2,
            &empty_style("G"),
        )
        .unwrap()
        .output;
        assert!(
            out2.contains(".disabled(locked)"),
            "expected .disabled(locked), got:\n{out2}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 18 — `HostInput` with `onCommit: emit: onCommit` emits
    // `.onSubmit { dispatch(.commit(value: value)) }`.
    //
    // The `on` prefix is stripped + lower-camelCased, and the bound
    // `value` slot is threaded into the dispatch payload.
    // ---------------------------------------------------------------------

    #[test]
    fn host_input_on_commit_emits_on_submit_dispatch() {
        let layout = layout_with(
            "F",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_slot_ref("value", "value"),
                        prop_emit_ref("onCommit", "onCommit"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "F",
                vec![slot("value", SlotType::Text, true)],
                vec![emit(
                    "onCommit",
                    vec![param("value", EmitPayloadType::Text)],
                )],
            ),
            &layout,
            &empty_style("F"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".onSubmit { dispatch(.commit(value: value)) }"),
            "expected .onSubmit dispatch site, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 19 — `HostButton` with `label` + `onTap` produces the right
    // SwiftUI Button structure.
    //
    // The action closure dispatches `.tap`; the label closure contains a
    // `Text(label)` slot reference.
    // ---------------------------------------------------------------------

    #[test]
    fn host_button_emits_button_with_action_and_label() {
        let layout = layout_with(
            "Bar",
            container_node(
                "Box",
                vec![leaf(
                    "HostButton",
                    vec![
                        prop_slot_ref("label", "caption"),
                        prop_emit_ref("onTap", "onTap"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "Bar",
                vec![slot("caption", SlotType::Text, true)],
                vec![emit("onTap", vec![])],
            ),
            &layout,
            &empty_style("Bar"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("Button(action: { dispatch(.tap) }) {"),
            "expected Button(action:) opener, got:\n{out}"
        );
        assert!(
            out.contains("Text(caption)"),
            "expected label closure with Text(caption), got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 20 — `HostScroll` lowers to `ScrollView { ... }`.
    //
    // Like the other containers, the empty-children form prints
    // `ScrollView { }` so it still type-checks under SwiftUI's `some View`.
    // ---------------------------------------------------------------------

    #[test]
    fn host_scroll_lowers_to_scroll_view() {
        let layout = layout_with(
            "S",
            container_node(
                "HostScroll",
                vec![leaf("Text", vec![prop_string("content", "row")])],
            ),
        );
        let out = from_pipeline(
            &component("S", vec![], vec![]),
            &layout,
            &empty_style("S"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("ScrollView {"),
            "expected ScrollView, got:\n{out}"
        );
        assert!(out.contains(r#"Text("row")"#));
    }

    // ---------------------------------------------------------------------
    // Test 21 — the UI29 kernel primitives recognised through this PR
    // (Stack, HostScroll, HostInput, HostButton, HostTable, For, If, Else)
    // must NOT fire `UnknownPrimitive`. With U29-K-swiftui (this PR) the
    // For/If/Else meta-primitives now have lowerings too, so the
    // "deferred set" is empty. We still assert the recognised set so a
    // regression that drops a kernel tag is caught.
    // ---------------------------------------------------------------------

    #[test]
    fn ui29_kernel_recognised_set_and_deferred_set() {
        // Recognised leaf-shaped primitives — must NOT return
        // UnknownPrimitive. (For/If/Else require props to lower
        // cleanly, so they are exercised in their dedicated tests
        // above; here we just confirm the leaf-shaped kernel.)
        for tag in ["Stack", "HostScroll", "HostInput", "HostButton", "HostTable"] {
            let layout = layout_with(
                "X",
                container_node("Box", vec![leaf(tag, vec![])]),
            );
            let r = from_pipeline(
                &component("X", vec![], vec![]),
                &layout,
                &empty_style("X"),
            );
            assert!(
                r.is_ok(),
                "expected primitive {tag} to lower without error, got: {r:?}"
            );
        }

        // For/If/Else are now recognised; they no longer fire
        // UnknownPrimitive. We confirm propless For/If still lower
        // (defensively — `validate_for_node`/`validate_if_node` would
        // have flagged them upstream) and that orphan Else emits a
        // documenting comment instead of erroring.
        for tag in ["For", "If", "Else"] {
            let layout = layout_with(
                "X",
                container_node("Box", vec![leaf(tag, vec![])]),
            );
            let r = from_pipeline(
                &component("X", vec![], vec![]),
                &layout,
                &empty_style("X"),
            );
            assert!(
                r.is_ok(),
                "expected meta-primitive {tag} to lower (defensively) without error, got: {r:?}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // UI29 kernel — HostTable tests (22 through 29).
    //
    // HostTable lowers to a `VStack(alignment: .leading, spacing: 0)` of
    // `HStack` rows. SwiftUI's data-driven `Table` view is deferred to a
    // follow-up that wires `For`-inside-table — see the doc comment on
    // [`emit_host_table`] for the rationale.
    // ---------------------------------------------------------------------

    /// Build a HostTable section node (`HostTableHead` / `HostTableBody` /
    /// `HostTableFoot`) from a list of `Row` children, where each row is
    /// itself a list of `Text` content cells.
    fn table_section(tag: &str, rows: Vec<Vec<&str>>) -> LayoutNode {
        let row_nodes = rows
            .into_iter()
            .map(|cells| {
                container_node(
                    "Row",
                    cells
                        .into_iter()
                        .map(|c| leaf("Text", vec![prop_string("content", c)]))
                        .collect(),
                )
            })
            .collect();
        container_node(tag, row_nodes)
    }

    // ---------------------------------------------------------------------
    // Test 22 — empty HostTable emits an empty VStack.
    //
    // The `alignment: .leading, spacing: 0` arguments are part of the
    // generated source even for empty bodies, so downstream tooling can
    // grep for the canonical opener.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_empty_emits_vstack() {
        let layout = layout_with(
            "T",
            container_node("Box", vec![leaf("HostTable", vec![])]),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("VStack(alignment: .leading, spacing: 0) { }"),
            "expected empty VStack, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 23 — HostTable with a HostTableHead emits an HStack whose Text
    // children carry a `.bold()` modifier.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_head_emits_bold_hstack() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section("HostTableHead", vec![vec!["A", "B"]])],
                )],
            ),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        assert!(out.contains("HStack {"), "expected HStack opener, got:\n{out}");
        assert!(
            out.contains(r#"Text("A")"#) && out.contains(".bold()"),
            "expected bold header text, got:\n{out}"
        );
        // Every header Text must carry `.bold()`.
        assert!(
            out.contains(r#"Text("A")"#) && out.contains(r#"Text("B")"#),
            "expected both header cells, got:\n{out}"
        );
        let bold_count = out.matches(".bold()").count();
        assert!(
            bold_count >= 2,
            "expected at least 2 .bold() modifiers (one per header cell), got {bold_count}:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 24 — HostTable with a HostTableBody emits HStack rows without
    // `.bold()`. Body rows are plain.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_body_emits_plain_hstack() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section(
                        "HostTableBody",
                        vec![vec!["r1c1", "r1c2"], vec!["r2c1", "r2c2"]],
                    )],
                )],
            ),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        // Two HStacks (one per Row), four Texts total, no .bold().
        let hstack_count = out.matches("HStack {").count();
        assert_eq!(hstack_count, 2, "expected 2 HStacks, got {hstack_count}:\n{out}");
        assert!(out.contains(r#"Text("r1c1")"#));
        assert!(out.contains(r#"Text("r2c2")"#));
        assert!(
            !out.contains(".bold()"),
            "body rows must not be bolded, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 25 — HostTable with a HostTableFoot is preceded by a `Divider`.
    //
    // The foot section is unconditionally separated from whatever came
    // before it (head, body, or nothing) by a visual divider.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_foot_is_preceded_by_divider() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section("HostTableFoot", vec![vec!["total"]])],
                )],
            ),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        let divider_pos = out.find("Divider()").expect("Divider present");
        let foot_text = out.find(r#"Text("total")"#).expect("foot text present");
        assert!(
            divider_pos < foot_text,
            "Divider must precede foot row, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 26 — head + body together emit Head, then Divider, then Body.
    //
    // The Divider auto-inserts between head and the first non-head
    // section that follows it.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_head_then_body_emits_divider_between() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        table_section("HostTableHead", vec![vec!["Name", "Age"]]),
                        table_section(
                            "HostTableBody",
                            vec![vec!["Alice", "30"], vec!["Bob", "25"]],
                        ),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        let head_pos = out.find(r#"Text("Name")"#).expect("head present");
        let divider_pos = out.find("Divider()").expect("divider present");
        let body_pos = out.find(r#"Text("Alice")"#).expect("body present");
        assert!(
            head_pos < divider_pos && divider_pos < body_pos,
            "expected Head < Divider < Body ordering, got:\n{out}"
        );
        // Headers bolded, body rows not.
        assert!(out.contains(r#"Text("Name")"#) && out.contains(".bold()"));
        // The two body rows should not be bolded.
        let bold_count = out.matches(".bold()").count();
        assert_eq!(
            bold_count, 2,
            "expected 2 .bold() (one per header cell), got {bold_count}:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 27 — HostTableColGroup nested under HostTable emits a Swift
    // comment (no SwiftUI analog) and does not break compilation.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_col_group_emits_comment() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        container_node("HostTableColGroup", vec![]),
                        table_section("HostTableBody", vec![vec!["x"]]),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("// HostTableColGroup ignored in SwiftUI"),
            "expected ColGroup comment, got:\n{out}"
        );
        assert!(out.contains(r#"Text("x")"#));
    }

    // ---------------------------------------------------------------------
    // Test 28 — orphan HostTableHead (used outside any HostTable) emits a
    // self-documenting Swift comment rather than erroring. The Swift
    // comment is a statement-level no-op, so the file still type-checks.
    // ---------------------------------------------------------------------

    #[test]
    fn orphan_host_table_head_emits_comment_no_error() {
        let layout = layout_with(
            "T",
            container_node("Box", vec![leaf("HostTableHead", vec![])]),
        );
        let result = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        );
        let out = result.expect("orphan sub-tag must not error").output;
        assert!(
            out.contains("// HostTableHead outside HostTable — ignored"),
            "expected orphan sub-tag comment, got:\n{out}"
        );

        // Sanity: HostTableBody / HostTableFoot / HostTableColGroup
        // orphans take the same path.
        for tag in ["HostTableBody", "HostTableFoot", "HostTableColGroup"] {
            let layout = layout_with(
                "T",
                container_node("Box", vec![leaf(tag, vec![])]),
            );
            let out = from_pipeline(
                &component("T", vec![], vec![]),
                &layout,
                &empty_style("T"),
            )
            .expect("orphan sub-tag must not error")
            .output;
            assert!(
                out.contains(&format!("// {tag} outside HostTable — ignored")),
                "expected orphan comment for {tag}, got:\n{out}"
            );
        }
    }

    // ---------------------------------------------------------------------
    // Test 29 — `part_name` on a HostTable surfaces in the generated
    // source as a Swift comment `// part: <name>` directly preceding the
    // VStack. SwiftUI has no native equivalent of CSS `part`; a future
    // style-inlining PR can swap the comment for a real modifier.
    // ---------------------------------------------------------------------

    #[test]
    fn host_table_part_name_emits_comment_before_vstack() {
        let table_node = LayoutNode {
            tag: "HostTable".to_string(),
            part_name: Some("data-table".to_string()),
            props: Vec::new(),
            children: vec![table_section("HostTableBody", vec![vec!["x"]])],
        };
        let layout = layout_with(
            "T",
            container_node("Box", vec![table_node]),
        );
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        let part_pos = out.find("// part: data-table").expect("part comment present");
        let vstack_pos = out
            .find("VStack(alignment: .leading, spacing: 0)")
            .expect("VStack present");
        assert!(
            part_pos < vstack_pos,
            "expected // part comment to precede VStack, got:\n{out}"
        );
    }

    // =================================================================
    // UI29 §3.1 — `For` meta-primitive tests
    // =================================================================

    // -----------------------------------------------------------------
    // Test 30 — `For (each: slot: rows, as: row) { Text "x" }` lowers
    // to `ForEach(rows, id: \.self) { row in Text("x") }`. The id
    // keypath is `\.self` (not `.offset`) because no `index:` is bound.
    // -----------------------------------------------------------------

    #[test]
    fn for_with_slot_each_emits_foreach_id_self() {
        let for_node = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row"),
            ],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("ForEach(rows, id: \\.self) { row in"),
            "expected ForEach with \\.self id, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"x\")"),
            "expected body Text, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 31 — Adding `index: r` switches the lowering to
    // `ForEach(Array(rows.enumerated()), id: \.offset) { (r, row) in
    // ... }`. The enumerated form is the only way to get an integer
    // index out of SwiftUI's ForEach without a separate Range.
    // -----------------------------------------------------------------

    #[test]
    fn for_with_index_emits_enumerated_array() {
        let for_node = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row"),
                prop_keyword("index", "r"),
            ],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("ForEach(Array(rows.enumerated()), id: \\.offset) { (r, row) in"),
            "expected enumerated ForEach with \\.offset id, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 32 — `each: <expr>` passes the expression source text
    // through verbatim. UI29 §3.3 stores Expr-valued props as the
    // reconstructed source substring; the backend cannot interpret
    // them, only embed them.
    // -----------------------------------------------------------------

    #[test]
    fn for_with_expr_each_emits_verbatim() {
        let for_node = node_with_props(
            "For",
            vec![
                prop_expr("each", "cols.visible"),
                prop_keyword("as", "col"),
            ],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("ForEach(cols.visible, id: \\.self) { col in"),
            "expected expr passed verbatim into ForEach, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 33 — A body that references the `as`-bound name (here as a
    // `Text content: slot: row` placeholder, where the slot name
    // happens to match) emits the camelCased identifier. The point of
    // this test is to confirm the binding name `row` appears in the
    // generated closure header and is therefore usable from the body.
    // -----------------------------------------------------------------

    #[test]
    fn for_body_uses_as_bound_name_as_identifier() {
        let for_node = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row-item"),
            ],
            vec![leaf("Text", vec![prop_slot_ref("content", "row-item")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        // `row-item` camelCases to `rowItem` for both the closure
        // parameter and the body reference.
        assert!(
            out.contains("{ rowItem in"),
            "expected camelCased as-binding in closure header, got:\n{out}"
        );
        assert!(
            out.contains("Text(rowItem)"),
            "expected camelCased as-binding referenced in body, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 34 — Nested `For`s. The outer loop's body contains an inner
    // For, and both should appear in the generated source with proper
    // nesting and distinct bindings.
    // -----------------------------------------------------------------

    #[test]
    fn nested_for_loops_compose() {
        let inner_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "cols"),
                prop_keyword("as", "col"),
            ],
            vec![leaf("Text", vec![prop_string("content", "c")])],
        );
        let outer_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row"),
            ],
            vec![inner_for],
        );
        let layout = layout_with("C", container_node("Box", vec![outer_for]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        let outer = out
            .find("ForEach(rows, id: \\.self) { row in")
            .expect("outer ForEach present");
        let inner = out
            .find("ForEach(cols, id: \\.self) { col in")
            .expect("inner ForEach present");
        assert!(
            outer < inner,
            "expected outer ForEach to precede inner, got:\n{out}"
        );
    }

    // =================================================================
    // UI29 §3.2 — `If` / `Else` meta-primitive tests
    // =================================================================

    // -----------------------------------------------------------------
    // Test 35 — `If (when: slot: editing) { Text "x" }` with no Else
    // sibling lowers to a single `if cond { ... }` block.
    // -----------------------------------------------------------------

    #[test]
    fn if_without_else_emits_if_only() {
        let if_node = node_with_props(
            "If",
            vec![prop_slot_ref("when", "editing")],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![if_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("if editing {"),
            "expected `if editing {{` header, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"x\")"),
            "expected then-branch Text, got:\n{out}"
        );
        assert!(
            !out.contains("} else {"),
            "expected no else clause, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 36 — `If { then } Else { else }` siblings are paired by
    // `emit_children` into a single `if cond { ... } else { ... }`
    // block. The Else is consumed when the If is processed; the
    // `emit_view_tree` walker never sees it as a standalone node.
    // -----------------------------------------------------------------

    #[test]
    fn if_with_else_emits_paired_block() {
        let if_node = node_with_props(
            "If",
            vec![prop_slot_ref("when", "editing")],
            vec![leaf("Text", vec![prop_string("content", "edit")])],
        );
        let else_node = node_with_props(
            "Else",
            vec![],
            vec![leaf("Text", vec![prop_string("content", "view")])],
        );
        let layout = layout_with(
            "C",
            container_node("Box", vec![if_node, else_node]),
        );
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("if editing {"),
            "expected if header, got:\n{out}"
        );
        assert!(
            out.contains("} else {"),
            "expected else clause, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"edit\")"),
            "expected then-branch Text, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"view\")"),
            "expected else-branch Text, got:\n{out}"
        );
        // No "orphan Else" comment should appear — the Else was
        // consumed by its If sibling.
        assert!(
            !out.contains("orphan Else"),
            "expected no orphan-Else comment, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 37 — `If (when: <expr>) { ... }` emits the expression
    // source text verbatim as the Swift condition. This mirrors
    // For's Expr passthrough (Test 32).
    // -----------------------------------------------------------------

    #[test]
    fn if_with_expr_when_emits_verbatim() {
        let if_node = node_with_props(
            "If",
            vec![prop_expr("when", "editing && row == 0")],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![if_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("if editing && row == 0 {"),
            "expected expr passed verbatim into if header, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 38 — Orphan `Else` (Else not preceded by an If sibling)
    // is normally rejected by the moslayout analyzer, but if it
    // reaches the emitter we render a documenting Swift comment so
    // the generated file still compiles.
    // -----------------------------------------------------------------

    #[test]
    fn orphan_else_emits_swift_comment() {
        let else_node = node_with_props(
            "Else",
            vec![],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![else_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("// orphan Else — ignored"),
            "expected orphan-Else comment, got:\n{out}"
        );
        // The orphan Else's body must NOT be emitted as Swift code,
        // since there is no surrounding if-block.
        assert!(
            !out.contains("Text(\"x\")"),
            "expected orphan Else body to be dropped, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 39 — An `If` whose Else sibling carries an Expr-valued
    // `when:` on the If (combined check that pairing + Expr both
    // work together, not redundant with 36 or 37).
    // -----------------------------------------------------------------

    #[test]
    fn if_else_pair_keeps_expr_condition() {
        let if_node = node_with_props(
            "If",
            vec![prop_expr("when", "r == 0")],
            vec![leaf("Text", vec![prop_string("content", "hdr")])],
        );
        let else_node = node_with_props(
            "Else",
            vec![],
            vec![leaf("Text", vec![prop_string("content", "body")])],
        );
        let layout = layout_with(
            "C",
            container_node("Box", vec![if_node, else_node]),
        );
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("if r == 0 {"),
            "expected expr condition, got:\n{out}"
        );
        assert!(
            out.contains("} else {"),
            "expected paired else, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"hdr\")") && out.contains("Text(\"body\")"),
            "expected both branches present, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 40 — Two consecutive If-without-Else siblings each emit
    // their own standalone `if` block. The walker must NOT
    // accidentally pair the second If's "Else-or-nothing" against
    // the first If's "Else-or-nothing".
    // -----------------------------------------------------------------

    #[test]
    fn two_consecutive_ifs_each_emit_separate_blocks() {
        let if1 = node_with_props(
            "If",
            vec![prop_slot_ref("when", "a")],
            vec![leaf("Text", vec![prop_string("content", "1")])],
        );
        let if2 = node_with_props(
            "If",
            vec![prop_slot_ref("when", "b")],
            vec![leaf("Text", vec![prop_string("content", "2")])],
        );
        let layout = layout_with("C", container_node("Box", vec![if1, if2]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("if a {"),
            "expected first if, got:\n{out}"
        );
        assert!(
            out.contains("if b {"),
            "expected second if, got:\n{out}"
        );
        assert!(
            !out.contains("} else {"),
            "expected no else clauses, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 41 — `For` body containing an `If`/`Else` pair. The
    // pairing must work one level down inside the For closure.
    // -----------------------------------------------------------------

    #[test]
    fn for_body_pairs_inner_if_else() {
        let if_node = node_with_props(
            "If",
            vec![prop_slot_ref("when", "editing")],
            vec![leaf("Text", vec![prop_string("content", "edit")])],
        );
        let else_node = node_with_props(
            "Else",
            vec![],
            vec![leaf("Text", vec![prop_string("content", "view")])],
        );
        let for_node = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row"),
            ],
            vec![if_node, else_node],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(
            &component("C", vec![], vec![]),
            &layout,
            &empty_style("C"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("ForEach(rows, id: \\.self) { row in"),
            "expected ForEach header, got:\n{out}"
        );
        assert!(
            out.contains("if editing {") && out.contains("} else {"),
            "expected paired if/else inside For body, got:\n{out}"
        );
    }
}
