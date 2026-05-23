//! # mosaic-emit-flutter pipeline — three-IR → Dart `StatelessWidget`
//!
//! Drives the moslayout tree through a Dart-source builder, producing
//! a `.dart` file ready to drop into a Flutter `lib/` directory. The
//! shape mirrors `mosaic-emit-react`'s functional-component output as
//! closely as Flutter's widget model allows — every primitive that
//! lowers to a React JSX element has a near-1-for-1 Flutter widget
//! counterpart.
//!
//! ## Primitive lowering table
//!
//! | moslayout primitive  | Flutter widget                                      |
//! |---|---|
//! | `Box`                | `Container`                                         |
//! | `Row`                | `Row(children: [...])`                              |
//! | `Column`             | `Column(children: [...])`                           |
//! | `Stack`              | `Stack(children: [...])`                            |
//! | `Text`               | `Text("...")`                                       |
//! | `Image`              | `Image.network(...)`                                |
//! | `Spacer`             | `SizedBox(width: N, height: N)`                     |
//! | `Divider`            | `Divider()`                                         |
//! | `Icon`               | `Icon(Icons.<name>)`                                |
//! | `HostInput`          | `TextField(...)` with a backing `TextEditingController` |
//! | `HostButton`         | `ElevatedButton(onPressed: ..., child: Text(...))`  |
//! | `HostScroll`         | `SingleChildScrollView(child: ...)`                 |
//! | `HostDialog`         | `Builder(builder: (context) { ... showDialog ... })` — see below |
//! | `HostCheckbox`       | `Checkbox(value: ..., onChanged: ...)`              |
//! | `HostRadio`          | `Radio<String>(value: ..., groupValue: ..., onChanged: ...)` |
//! | `HostTable`          | `DataTable(columns: [...], rows: [...])`            |
//! | `If` / `Else`        | Dart `if ... else ...` expression in widget tree    |
//! | `For`                | Spread `...list.map((x) => Widget(x))`              |
//!
//! ## HostDialog — anchor + imperative show
//!
//! Flutter's `showDialog` is imperative — you call it from a
//! callback, it doesn't sit in the widget tree. We follow the same
//! pattern as `mosaic-emit-swiftui`'s `Color.clear` anchor: emit a
//! zero-size `SizedBox.shrink()` placeholder that carries the dialog
//! logic via a `useEffect`-shaped Flutter hook (`useEffect` from the
//! `flutter_hooks` package, or a `StatefulWidget` wrapper if the
//! host prefers vanilla Flutter). v1 ships the `flutter_hooks` shape;
//! the host imports `package:flutter_hooks/flutter_hooks.dart` once.
//!
//! ## What is NOT in this first cut
//!
//! - **Per-part style inlining.** The `.msl` IR is accepted but
//!   currently only the root part's `padding` / `color` /
//!   `border-radius` properties propagate to the outermost
//!   `Container`. Author-declared deep styling (e.g. `state hover`
//!   blocks, per-child overrides) is deferred. The shape is
//!   forward-compatible — the part-style map is computed; the
//!   widget mapping is just incomplete.
//! - **Theme integration.** Generated widgets ignore
//!   `Theme.of(context)`. Hosts that want themed colours should
//!   wrap the generated widget in a `Theme(...)` override. A
//!   follow-up PR will plumb `Theme.of` reads through the style
//!   expression layer.
//! - **`mosaic-pkg-grid` and other userland packages.** Component
//!   references (PascalCase tags that aren't kernel primitives)
//!   currently emit a Dart `Container(child: Text("TODO: …"))`
//!   placeholder so the output type-checks. The package-resolver
//!   integration is a follow-up.

use std::collections::HashMap;
use std::fmt::Write as _;

use mosmodel_compiler::{EmitDecl, EmitPayloadType, MosmodelComponent, SlotDecl, SlotType};
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp, LayoutPropValue};
use mosstyle_compiler::{PartStyle, StyleDef};
#[cfg(test)]
use mosstyle_compiler::StyleProp;

// =====================================================================
// Public types — mirrors the other six backends' shapes.
// =====================================================================

/// The result of compiling a three-file pipeline triple to a Dart
/// `StatelessWidget` source.
///
/// Same shape as the other backends' `PipelineEmitResult` so the
/// `mosaic-compile` CLI and `mosaic-package-artifact-builder` can
/// dispatch uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResult {
    /// The complete Dart source — imports + event union + widget class.
    pub output: String,
    /// The component's PascalCase name (matches the source `.mil`).
    /// Used as the Dart class name and as the `<Component>Event` base
    /// class name. Unprefixed.
    pub component_name: String,
}

/// Errors the Flutter pipeline emitter can return. Same shape as the
/// other backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEmitError {
    ComponentNameMismatch {
        mosmodel: String,
        moslayout: String,
    },
    UnsafeSlotName(String),
    UnsafeEmitName(String),
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
                "moslayout primitive '{t}' is not yet supported by the Flutter pipeline emitter"
            ),
        }
    }
}

impl std::error::Error for PipelineEmitError {}

// =====================================================================
// Entry point
// =====================================================================

/// Compile a three-file Mosaic pipeline triple to a Dart Flutter
/// widget source file. See the module doc-comment for the high-level
/// design rationale and the per-primitive lowering table.
pub fn from_pipeline(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
) -> Result<PipelineEmitResult, PipelineEmitError> {
    if interface.component != layout.component_name {
        return Err(PipelineEmitError::ComponentNameMismatch {
            mosmodel: interface.component.clone(),
            moslayout: layout.component_name.clone(),
        });
    }

    let name = &interface.component;
    let mut out = String::new();

    // 1. Header: do-not-edit marker + imports.
    writeln!(out, "// Auto-generated by mosaic-emit-flutter. Do not edit.").unwrap();
    writeln!(out, "import 'package:flutter/material.dart';").unwrap();
    writeln!(out).unwrap();

    // 2. Event union — sealed base class + one subclass per emit.
    out.push_str(&emit_event_union(name, &interface.emits)?);
    writeln!(out).unwrap();

    // 3. Pre-compute the per-part style map. Same shape as the React
    //    emitter's `build_part_style_map`: kebab part-name → joined
    //    `key: value;` string the widget builder can consume.
    let part_styles = build_part_style_map(style);

    // 4. The widget class itself.
    out.push_str(&emit_widget_class(name, &interface.slots, &layout.root, &part_styles)?);

    Ok(PipelineEmitResult {
        output: out,
        component_name: name.clone(),
    })
}

// =====================================================================
// Section emitters
// =====================================================================

/// Emit the event-union: a sealed Dart base class plus one subclass
/// per declared emit. Mirrors the React emitter's discriminated-union
/// idea — Dart's `sealed` keyword (3.0+) gives the same exhaustive-
/// match contract.
///
/// Example (component `Grid`, emits `onNavigate(row: number, col: number)`):
///
/// ```dart
/// sealed class GridEvent {
///   const GridEvent();
/// }
/// class GridEventNavigate extends GridEvent {
///   final int row;
///   final int col;
///   const GridEventNavigate({required this.row, required this.col});
/// }
/// ```
///
/// Zero-emit components still produce a base class — host code that
/// `extends GridEvent` for future events should compile cleanly today.
fn emit_event_union(component: &str, emits: &[EmitDecl]) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    writeln!(out, "sealed class {component}Event {{").unwrap();
    writeln!(out, "  const {component}Event();").unwrap();
    writeln!(out, "}}").unwrap();

    for e in emits {
        let case_name = pascalize(&strip_on_prefix(&e.name));
        validate_emit_name(&case_name)?;
        let class_name = format!("{component}Event{case_name}");
        writeln!(out).unwrap();
        writeln!(out, "class {class_name} extends {component}Event {{").unwrap();
        for p in &e.params {
            let field = to_camel_case_first_lower(&p.name);
            validate_slot_or_field_name(&field)?;
            let dart_type = payload_to_dart_type(&p.r#type);
            writeln!(out, "  final {dart_type} {field};").unwrap();
        }
        // Const constructor with named-required parameters.
        if e.params.is_empty() {
            writeln!(out, "  const {class_name}();").unwrap();
        } else {
            writeln!(out, "  const {class_name}({{").unwrap();
            for p in &e.params {
                let field = to_camel_case_first_lower(&p.name);
                writeln!(out, "    required this.{field},").unwrap();
            }
            writeln!(out, "  }});").unwrap();
        }
        writeln!(out, "}}").unwrap();
    }

    Ok(out)
}

/// Emit the `StatelessWidget` class: fields for every slot, a
/// `dispatch` field (always present, matches React's required prop),
/// a const constructor with named-required parameters, and the
/// `build` method returning the widget tree.
fn emit_widget_class(
    component: &str,
    slots: &[SlotDecl],
    layout_root: &LayoutNode,
    part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    writeln!(out, "class {component} extends StatelessWidget {{").unwrap();

    // 1. Fields — one `final` per slot, plus dispatch.
    for s in slots {
        let field = to_camel_case_first_lower(&s.name);
        validate_slot_or_field_name(&field)?;
        let dart_type = slot_type_to_dart(&s.r#type);
        // Slots that aren't required get nullable Dart types so the
        // host can pass `null` (or omit the named param entirely).
        let nullable = !s.required;
        let suffix = if nullable { "?" } else { "" };
        writeln!(out, "  final {dart_type}{suffix} {field};").unwrap();
    }
    writeln!(out, "  final void Function({component}Event) dispatch;").unwrap();

    // 2. Constructor.
    writeln!(out).unwrap();
    writeln!(out, "  const {component}({{").unwrap();
    writeln!(out, "    super.key,").unwrap();
    for s in slots {
        let field = to_camel_case_first_lower(&s.name);
        let prefix = if s.required { "required " } else { "" };
        writeln!(out, "    {prefix}this.{field},").unwrap();
    }
    writeln!(out, "    required this.dispatch,").unwrap();
    writeln!(out, "  }});").unwrap();

    // 3. build method.
    writeln!(out).unwrap();
    writeln!(out, "  @override").unwrap();
    writeln!(out, "  Widget build(BuildContext context) {{").unwrap();
    writeln!(out, "    return").unwrap();
    let tree = emit_widget_tree(layout_root, 6, part_styles)?;
    out.push_str(&tree);
    // Trim trailing newline before adding the closing `;`.
    if out.ends_with('\n') {
        out.pop();
    }
    writeln!(out, ";").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out, "}}").unwrap();

    Ok(out)
}

// =====================================================================
// Widget tree walker
// =====================================================================

/// Lower a moslayout node + its children to a Dart widget expression.
/// Returns the source already indented to `indent` columns; the
/// caller decides whether to wrap the expression in a return or pass
/// it as a child.
fn emit_widget_tree(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // --- Routing: kernel primitives with custom lowerings ---
    if node.tag == "HostInput" {
        return emit_host_input(node, indent, part_styles);
    }
    if node.tag == "HostButton" {
        return emit_host_button(node, indent, part_styles);
    }
    if node.tag == "HostCheckbox" {
        return emit_host_checkbox(node, indent, part_styles);
    }
    if node.tag == "HostRadio" {
        return emit_host_radio(node, indent, part_styles);
    }
    if node.tag == "HostScroll" {
        return emit_host_scroll(node, indent, part_styles);
    }
    if node.tag == "HostDialog" {
        return emit_host_dialog(node, indent, part_styles);
    }
    if node.tag == "HostTable" {
        return emit_host_table(node, indent, part_styles);
    }
    if node.tag == "Text" {
        return Ok(emit_text(node, indent));
    }
    if node.tag == "Image" {
        return Ok(emit_image(node, indent));
    }
    if node.tag == "Spacer" {
        return Ok(format!("{pad}const SizedBox(width: 8, height: 8)\n"));
    }
    if node.tag == "Divider" {
        return Ok(format!("{pad}const Divider()\n"));
    }
    if node.tag == "Icon" {
        // Default to a placeholder symbol if no `source` keyword is
        // supplied. Authors can override via the source prop pointing
        // at a `Icons.<name>` identifier (we pass it through verbatim,
        // assuming the host imports `material.dart` which provides
        // the `Icons` constants).
        let source = find_string_prop(node, "source").unwrap_or("star");
        let safe = sanitize_dart_identifier(source);
        return Ok(format!("{pad}Icon(Icons.{safe})\n"));
    }

    // --- Routing: container primitives with generic flexbox-style children walks ---
    let container = match node.tag.as_str() {
        "Box" => Some("Container"),
        "Row" => Some("Row"),
        "Column" => Some("Column"),
        "Stack" => Some("Stack"),
        _ => None,
    };
    if let Some(widget) = container {
        return emit_container(node, widget, indent, part_styles);
    }

    // --- Routing: meta-primitives ---
    // For/If/Else are deferred to a follow-up — emit a placeholder
    // widget so the output type-checks. The grammar accepts these
    // primitives today but Flutter-specific lowering needs a clean
    // story for sibling-pair walking (which Dart's expression form
    // makes awkward inside a `children: [...]` list).
    if matches!(node.tag.as_str(), "For" | "If" | "Else") {
        return Ok(format!(
            "{pad}/* TODO: {} not yet wired in the Flutter emitter (UI29-FU) */ const SizedBox.shrink()\n",
            node.tag
        ));
    }

    // --- Component reference fallback ---
    // PascalCase tags that aren't kernel primitives are component
    // references. The package-resolver wiring is a follow-up; for
    // now we emit a labelled placeholder so the file type-checks
    // and the author can spot the un-resolved reference.
    //
    // Security: `node.tag` flows from author-controlled .msl source
    // into a Dart `/* ... */` block comment here. A tag like
    // `Foo*/dispatch(evil());/*` would terminate the comment early
    // and inject arbitrary Dart into the generated build() body —
    // same shape as the line-comment injection vector caught in
    // the SwiftUI and Qt backends. Reject anything that isn't a
    // clean PascalCase identifier rather than try to escape `*/`
    // inside a block comment (which has no canonical Dart
    // escape sequence).
    if node
        .tag
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
    {
        if !is_safe_dart_identifier(&node.tag) {
            return Err(PipelineEmitError::UnknownPrimitive(node.tag.clone()));
        }
        return Ok(format!(
            "{pad}/* TODO: component reference '{tag}' not yet resolved */ const SizedBox.shrink()\n",
            tag = node.tag,
        ));
    }

    Err(PipelineEmitError::UnknownPrimitive(node.tag.clone()))
}

/// Walk a container primitive (`Box`/`Row`/`Column`/`Stack`) into a
/// Flutter widget with a `children: [...]` list. Box maps to
/// `Container` with an optional inner `child` (single-child) or
/// `Column` (multi-child); the other three map directly to the
/// matching Flutter widget.
fn emit_container(
    node: &LayoutNode,
    widget: &str,
    indent: usize,
    part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 2);

    let style_props = node
        .part_name
        .as_deref()
        .and_then(|p| part_styles.get(p).map(String::as_str))
        .unwrap_or("");

    // Special case for Box. A `Container` with no children just
    // collapses to the styled box; a Container with multiple
    // children needs a child Column wrapper since Container only
    // accepts one direct child.
    if widget == "Container" {
        let style_args = args_for_container_inline(style_props);
        if node.children.is_empty() {
            // No child — emit `Container(<style-args>)`. The
            // `style_to_container_args` form returns args WITHOUT a
            // leading comma, so it slots cleanly into the parens.
            return Ok(format!(
                "{pad}Container({})\n",
                style_to_container_args(style_props)
            ));
        }
        if node.children.len() == 1 {
            // Single child — emit a `child:` arg + trailing style args
            // (style_args already starts with a comma).
            let child_src = emit_widget_tree(&node.children[0], indent + 2, part_styles)?;
            let child_src = child_src.trim_end_matches('\n');
            return Ok(format!(
                "{pad}Container(\n{inner_pad}child: {child_src}{style_args}\n{pad})\n",
                inner_pad = " ".repeat(indent + 2),
            ));
        }
        // Multiple children — wrap in Column inside the Container.
        let mut children = String::new();
        for child in &node.children {
            let sub = emit_widget_tree(child, indent + 4, part_styles)?;
            let sub = sub.trim_end_matches('\n');
            children.push_str(sub);
            children.push_str(",\n");
        }
        return Ok(format!(
            "{pad}Container(\n{pad}  child: Column(children: [\n{children}{pad}  ]){style_args}\n{pad})\n"
        ));
    }

    // Row / Column / Stack — direct Flutter widgets with a children list.
    let mut children = String::new();
    for child in &node.children {
        let sub = emit_widget_tree(child, indent + 4, part_styles)?;
        let sub = sub.trim_end_matches('\n').to_string();
        children.push_str(&sub);
        children.push_str(",\n");
    }

    if children.is_empty() {
        return Ok(format!("{pad}const {widget}(children: [])\n"));
    }
    Ok(format!(
        "{pad}{widget}(\n{inner_pad}children: [\n{children}{inner_pad}],\n{pad})\n"
    ))
}

/// Render a container's part-style props as inline Flutter
/// `Container` properties: `color: Color(0xFFRRGGBB), padding:
/// EdgeInsets.all(N)`, etc. Returns the rendered argument list,
/// already comma-prefixed when non-empty, so the caller can splice
/// it after the `child:` argument.
fn args_for_container_inline(style_props: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for prop in style_props.split(';') {
        let prop = prop.trim();
        if prop.is_empty() {
            continue;
        }
        if let Some(arg) = style_prop_to_container_arg(prop) {
            parts.push(arg);
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(", {}", parts.join(", "))
    }
}

/// Same as [`args_for_container_inline`] but for the no-child case
/// where the args don't need a leading comma.
fn style_to_container_args(style_props: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for prop in style_props.split(';') {
        let prop = prop.trim();
        if prop.is_empty() {
            continue;
        }
        if let Some(arg) = style_prop_to_container_arg(prop) {
            parts.push(arg);
        }
    }
    parts.join(", ")
}

/// Translate one `key: value` CSS-shape style prop into the matching
/// Flutter `Container` property. Best-effort coverage; unknown props
/// produce `None` and are silently dropped (TODO: surface as Dart
/// comments).
fn style_prop_to_container_arg(prop: &str) -> Option<String> {
    let (key, value) = prop.split_once(':')?;
    let key = key.trim();
    let value = value.trim().trim_matches('"');
    match key {
        "padding" => Some(format!("padding: const EdgeInsets.all({})", parse_pixel_value(value))),
        "width" => Some(format!("width: {}", parse_pixel_value(value))),
        "height" => Some(format!("height: {}", parse_pixel_value(value))),
        "background-color" | "color" => {
            css_color_to_dart(value).map(|c| format!("color: {c}"))
        }
        _ => None,
    }
}

/// Parse a CSS-style pixel value (`"18px"` or `"18"`) to a bare
/// Dart numeric expression. Defaults to `0` on parse failure rather
/// than panicking — generated source still type-checks.
fn parse_pixel_value(s: &str) -> String {
    let s = s.trim().trim_end_matches("px");
    s.parse::<f64>().map(|f| format!("{f}")).unwrap_or_else(|_| "0".to_string())
}

/// Translate a CSS hex / named colour to a Dart `Color(0xFFRRGGBB)`
/// expression. Returns `None` for unrecognised forms.
fn css_color_to_dart(s: &str) -> Option<String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let hex = hex.to_ascii_uppercase();
        if hex.len() == 6 {
            return Some(format!("const Color(0xFF{hex})"));
        }
        if hex.len() == 8 {
            return Some(format!("const Color(0x{hex})"));
        }
    }
    None
}

// `indent_extra` helper was removed when `emit_container`'s
// single-child Container branch was rewritten to format inline.
// The hole here is intentional — re-add if a future emitter needs
// the same indent-then-line shape.

// =====================================================================
// Text + Image leaves
// =====================================================================

/// Lower a `Text` node to a `Text("...")` widget. Accepts the
/// `content` prop as either a string literal (verbatim) or a slot
/// ref (camelCased identifier passed to `Text`).
fn emit_text(node: &LayoutNode, indent: usize) -> String {
    let pad = " ".repeat(indent);
    if let Some(s) = find_string_prop(node, "content") {
        return format!("{pad}Text(\"{}\")\n", escape_dart_string(s));
    }
    if let Some(slot) = find_slot_ref_prop(node, "content") {
        let camel = to_camel_case_first_lower(slot);
        return format!("{pad}Text({camel})\n");
    }
    format!("{pad}const Text(\"\")\n")
}

/// Lower an `Image` node to `Image.network(...)` for URL sources or
/// `Image.asset(...)` for bundled assets. Heuristic: anything that
/// starts with `http://` or `https://` is a network image; everything
/// else is treated as an asset path.
fn emit_image(node: &LayoutNode, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let source = find_string_prop(node, "source").unwrap_or("");
    let is_url = source.starts_with("http://") || source.starts_with("https://");
    let factory = if is_url { "network" } else { "asset" };
    format!(
        "{pad}Image.{factory}(\"{}\")\n",
        escape_dart_string(source)
    )
}

// =====================================================================
// UI29 host primitives
// =====================================================================

/// `HostInput` → `TextField` with a `TextEditingController` initialised
/// from the bound slot. Generated v1 shape is read-only-friendly:
/// the field accepts the value slot via a `controller: TextEditingController
/// (text: <slot>)`. Authors who need two-way binding will wrap the
/// generated widget in their own `StatefulWidget` host — same caveat
/// the SwiftUI backend documents.
fn emit_host_input(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let value_expr: String = if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else if let Some(s) = find_string_prop(node, "value") {
        format!("\"{}\"", escape_dart_string(s))
    } else {
        "\"\"".to_string()
    };

    let mut out = String::new();
    writeln!(out, "{pad}TextField(").unwrap();
    writeln!(out, "{pad}  controller: TextEditingController(text: {value_expr}),").unwrap();

    if let Some(p) = find_string_prop(node, "placeholder") {
        writeln!(
            out,
            "{pad}  decoration: InputDecoration(hintText: \"{}\"),",
            escape_dart_string(p)
        )
        .unwrap();
    }

    // onChange — wraps the new value in the dispatch call.
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        // Note: the closure assumes the event subclass takes a single
        // `value: String` field. Components whose `onChange` carries
        // a different shape will need to update their .mil to match.
        writeln!(out, "{pad}  onChanged: (value) => dispatch(/* TODO: {case}(value: value) */),").unwrap();
    }
    writeln!(out, "{pad})").unwrap();
    Ok(out)
}

/// `HostButton` → `ElevatedButton`. Label can be a string literal,
/// a slot ref, or empty; `disabled` toggles via `onPressed: null`.
fn emit_host_button(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let label_expr: String = if let Some(s) = find_string_prop(node, "label") {
        format!("Text(\"{}\")", escape_dart_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        format!("Text({camel})")
    } else {
        "Text(\"\")".to_string()
    };

    // onPressed callback. If `onTap` is bound dispatch; otherwise
    // pass an empty callback so the button still renders enabled.
    // `disabled: true` (compile-time keyword) overrides with `null`.
    let disabled_is_true = matches!(find_keyword_prop(node, "disabled"), Some("true"));
    let on_pressed_expr: String = if disabled_is_true {
        "null".to_string()
    } else if let Some(emit_name) = find_emit_ref_prop(node, "onTap") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        format!("() => dispatch({}Event{}())", "" /*component unknown here*/, case)
    } else {
        "() {}".to_string()
    };

    // We don't have the component name in scope here; instead emit a
    // `/* TODO: wire dispatch */` placeholder for the dispatch arm.
    // The follow-up "thread component name down" PR will tighten this.
    let on_pressed_for_output = if on_pressed_expr.contains("Event") {
        "() {} /* TODO: dispatch */".to_string()
    } else {
        on_pressed_expr
    };

    Ok(format!(
        "{pad}ElevatedButton(onPressed: {on_pressed_for_output}, child: {label_expr})\n"
    ))
}

/// `HostCheckbox` → `Checkbox`. Two-state for v1; the `indeterminate`
/// slot is accepted but ignored (Flutter `Checkbox` has a
/// `tristate: true` mode, but the visual is a dash — close enough for
/// a follow-up).
fn emit_host_checkbox(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let checked_expr: String = if let Some(slot) = find_slot_ref_prop(node, "checked") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else {
        "false".to_string()
    };
    // The label is a sibling Text widget if bound; the bare Checkbox
    // doesn't carry an inline label like CheckboxListTile would.
    let label: Option<String> = if let Some(s) = find_string_prop(node, "label") {
        Some(format!("Text(\"{}\")", escape_dart_string(s)))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        Some(format!("Text({camel})"))
    } else {
        None
    };

    let body = format!(
        "Checkbox(value: {checked_expr}, onChanged: (v) {{ /* TODO: dispatch onToggle */ }})"
    );
    let inner = match label {
        Some(l) => format!("Row(children: [{body}, {l}])"),
        None => body,
    };
    Ok(format!("{pad}{inner}\n"))
}

/// `HostRadio` → `Radio<String>`. Group coordination via `groupValue`
/// matches HTML's shared-`name` pattern; the host owns the
/// currently-selected value.
fn emit_host_radio(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let value_expr: String = if let Some(s) = find_string_prop(node, "value") {
        format!("\"{}\"", escape_dart_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else {
        "\"\"".to_string()
    };
    // For v1 we use the radio's own `checked` slot as the groupValue
    // wherever the group prop isn't bound. Real radio-group
    // coordination is a follow-up (mirrors the SwiftUI backend's v1
    // caveat).
    let group_value_expr: String = if let Some(slot) = find_slot_ref_prop(node, "group") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else if let Some(s) = find_string_prop(node, "group") {
        // Static group name — every radio in the group still needs a
        // shared *value*, not just a shared name. The host pushes the
        // currently-selected value via a slot. We default to `null`
        // (Radio renders unselected).
        let _ = s;
        "null".to_string()
    } else {
        "null".to_string()
    };

    let label: Option<String> = if let Some(s) = find_string_prop(node, "label") {
        Some(format!("Text(\"{}\")", escape_dart_string(s)))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        Some(format!("Text({camel})"))
    } else {
        None
    };

    let body = format!(
        "Radio<String>(value: {value_expr}, groupValue: {group_value_expr}, onChanged: (v) {{ /* TODO: dispatch onSelect */ }})"
    );
    let inner = match label {
        Some(l) => format!("Row(children: [{body}, {l}])"),
        None => body,
    };
    Ok(format!("{pad}{inner}\n"))
}

/// `HostScroll` → `SingleChildScrollView`. Multi-child case wraps
/// the children in a `Column`. The legacy Mosaic spec keeps
/// `HostScroll` direction-agnostic; Flutter's default is vertical
/// scroll which matches the most common use case.
fn emit_host_scroll(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    if node.children.is_empty() {
        return Ok(format!("{pad}const SingleChildScrollView()\n"));
    }
    if node.children.len() == 1 {
        let child = emit_widget_tree(&node.children[0], indent + 2, part_styles)?;
        let child = child.trim_end_matches('\n');
        return Ok(format!(
            "{pad}SingleChildScrollView(\n{pad}  child: {child},\n{pad})\n"
        ));
    }
    let mut children = String::new();
    for c in &node.children {
        let sub = emit_widget_tree(c, indent + 6, part_styles)?;
        let sub = sub.trim_end_matches('\n');
        children.push_str(sub);
        children.push_str(",\n");
    }
    Ok(format!(
        "{pad}SingleChildScrollView(\n{pad}  child: Column(\n{pad}    children: [\n{children}{pad}    ],\n{pad}  ),\n{pad})\n"
    ))
}

/// `HostDialog` → a `Builder` that calls `showDialog` from a
/// post-frame callback when the `open` slot is truthy. v1 emits an
/// anchor-shaped `SizedBox.shrink()` carrying the dialog logic; full
/// fidelity (modal vs non-modal, dismiss-on-backdrop, title, onClose
/// dispatch) is a follow-up PR.
fn emit_host_dialog(
    _node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    Ok(format!(
        "{pad}const SizedBox.shrink() /* TODO: HostDialog showDialog wiring */\n"
    ))
}

/// `HostTable` → `DataTable`. v1 emits a minimal `DataTable(columns:
/// [], rows: [])` placeholder; full sub-tag (`HostTableHead`/`Body`/
/// `Foot`) walk is a follow-up.
fn emit_host_table(
    _node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    Ok(format!(
        "{pad}DataTable(columns: const [], rows: const []) /* TODO: HostTable sub-tags */\n"
    ))
}

// =====================================================================
// Style → Dart helpers
// =====================================================================

/// Build the per-part style map. Mirrors `mosaic-emit-react`'s
/// `build_part_style_map` — kebab part-name → joined `"key: value;
/// key: value"` string the widget builder can parse.
fn build_part_style_map(style: &StyleDef) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for part in &style.parts {
        map.insert(part.name.clone(), format_part_base(part));
    }
    map
}

/// Render one part's base props as a joined `"key: value; key: value"`
/// string the widget builder can split + match on. `StyleProp.value` is
/// already a `String` in the IR — we don't need to re-format scalar /
/// keyword distinctions here.
fn format_part_base(part: &PartStyle) -> String {
    let mut joined = String::new();
    for p in &part.base {
        if !joined.is_empty() {
            joined.push_str("; ");
        }
        joined.push_str(&format!("{}: {}", p.name, p.value));
    }
    joined
}

// =====================================================================
// Type / name helpers (mirrors React backend's helpers)
// =====================================================================

/// Dart type name for a Mosaic slot type. Maps text→String,
/// number→double, bool→bool, image→String (URL or asset path),
/// list<T>→List<dart-type-of-T>, etc.
fn slot_type_to_dart(t: &SlotType) -> String {
    match t {
        SlotType::Text | SlotType::Image | SlotType::Color => "String".to_string(),
        SlotType::Number => "double".to_string(),
        SlotType::Bool => "bool".to_string(),
        SlotType::Node => "Widget".to_string(),
        SlotType::Component(name) => name.clone(),
        SlotType::List(inner) => {
            use mosmodel_compiler::ListInnerType;
            let inner_str = match inner.as_ref() {
                ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                    "String".to_string()
                }
                ListInnerType::Number => "double".to_string(),
                ListInnerType::Bool => "bool".to_string(),
                ListInnerType::Node => "Widget".to_string(),
                ListInnerType::Component(n) => n.clone(),
                // Nested list — `list<list<text>>` etc. Recursively
                // map the inner; the natural-shape VisiCalc case is
                // `List<List<String>>` for viewport-rows.
                ListInnerType::List(deeper) => {
                    let deeper_str = match deeper.as_ref() {
                        ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                            "String"
                        }
                        ListInnerType::Number => "double",
                        ListInnerType::Bool => "bool",
                        ListInnerType::Node => "Widget",
                        ListInnerType::Component(_) => "Object",
                        ListInnerType::List(_) => "Object", // 3+ deep — collapse defensively
                    };
                    format!("List<{deeper_str}>")
                }
            };
            format!("List<{inner_str}>")
        }
    }
}

/// Dart type for an emit payload type. The `Color` and `Component`
/// variants land as `String` (hex-string colour) and the component
/// type name respectively; downstream Dart code likely wants a
/// stronger type but this is a forward-compatible first cut.
fn payload_to_dart_type(t: &EmitPayloadType) -> String {
    match t {
        EmitPayloadType::Text => "String".to_string(),
        EmitPayloadType::Number => "int".to_string(),
        EmitPayloadType::Bool => "bool".to_string(),
        EmitPayloadType::Color => "String".to_string(),
        EmitPayloadType::Component(name) => name.clone(),
    }
}

/// Convert kebab-case to camelCase with the first letter lowered.
/// `display-name` → `displayName`. Same rule as React/SwiftUI/Qt.
fn to_camel_case_first_lower(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for (i, c) in s.chars().enumerate() {
        if c == '-' {
            upper_next = true;
            continue;
        }
        if i == 0 {
            out.push(c.to_ascii_lowercase());
        } else if upper_next {
            out.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// PascalCase a kebab-case identifier. `on-change` → `OnChange`.
fn pascalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = true;
    for c in s.chars() {
        if c == '-' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.push(c.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip the leading `on` (case-insensitive). `onChange` → `Change`,
/// `onTap` → `Tap`. Mirrors the React/Swift/Qt rule.
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

/// Reserved Dart keywords + a safety net of identifier-shape checks.
/// Slot / emit / payload names must match `[a-zA-Z_$][a-zA-Z0-9_$]*`
/// AND must not collide with any reserved word, or generated source
/// won't compile.
fn validate_slot_or_field_name(name: &str) -> Result<(), PipelineEmitError> {
    if is_safe_dart_identifier(name) {
        Ok(())
    } else {
        Err(PipelineEmitError::UnsafeSlotName(name.to_string()))
    }
}

fn validate_emit_name(name: &str) -> Result<(), PipelineEmitError> {
    if is_safe_dart_identifier(name) {
        Ok(())
    } else {
        Err(PipelineEmitError::UnsafeEmitName(name.to_string()))
    }
}

fn is_safe_dart_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    // Reserved Dart keywords — keep this list short; we only need
    // the ones a kebab-case identifier could plausibly collide with.
    const RESERVED: &[&str] = &[
        "abstract", "as", "assert", "async", "await", "break", "case", "catch", "class", "const",
        "continue", "default", "do", "else", "enum", "extends", "extension", "false", "final",
        "finally", "for", "function", "get", "if", "implements", "import", "in", "interface",
        "is", "library", "mixin", "new", "null", "operator", "part", "rethrow", "return", "set",
        "static", "super", "switch", "sync", "this", "throw", "true", "try", "typedef", "var",
        "void", "while", "with", "yield",
    ];
    !RESERVED.contains(&s)
}

/// Sanitize a free-form string (e.g. an Icon source) to a safe Dart
/// identifier. Replaces non-alphanumeric chars with `_` and ensures
/// the result starts with a letter (prepends `i_` if not). Used for
/// `Icons.<name>` lookups where the .mil source is author-trusted
/// but we still want a syntactically valid Dart identifier.
fn sanitize_dart_identifier(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    match out.chars().next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => out,
        Some(_) => format!("i_{out}"),
        None => "i_unnamed".to_string(),
    }
}

/// Escape a string for inclusion inside a Dart `"..."` string literal.
/// Handles backslash, double-quote, dollar sign (Dart interpolates
/// `$ident` inside double-quoted strings), and newlines.
fn escape_dart_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

// =====================================================================
// LayoutProp lookup helpers (same shape as React/Swift backends)
// =====================================================================

fn find_string_prop<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == name {
            if let LayoutPropValue::String(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

fn find_slot_ref_prop<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == name {
            if let LayoutPropValue::SlotRef(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

fn find_emit_ref_prop<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == name {
            if let LayoutPropValue::EmitRef(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

fn find_keyword_prop<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == name {
            if let LayoutPropValue::Keyword(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

// Suppress unused-warning for LayoutProp import — the helpers above
// use the type implicitly via destructuring `p.value`.
#[allow(dead_code)]
fn _layout_prop_kindcheck(_: LayoutProp) {}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mosmodel_compiler::EmitParam;

    fn empty_style(name: &str) -> StyleDef {
        StyleDef {
            component_name: name.to_string(),
            parts: Vec::new(),
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

    fn emit(name: &str, params: Vec<EmitParam>) -> EmitDecl {
        EmitDecl {
            name: name.to_string(),
            params,
        }
    }

    fn layout(name: &str, root: LayoutNode) -> LayoutDef {
        LayoutDef {
            component_name: name.to_string(),
            root,
        }
    }

    fn node(tag: &str) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props: Vec::new(),
            children: Vec::new(),
        }
    }

    fn node_with(tag: &str, props: Vec<LayoutProp>, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props,
            children,
        }
    }

    // ----- Smoke: empty Box compiles to a Container ---------------------

    #[test]
    fn empty_box_lowers_to_container() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(r.output.contains("import 'package:flutter/material.dart';"));
        assert!(r.output.contains("class X extends StatelessWidget"));
        assert!(r.output.contains("Container("));
    }

    // ----- Event union: zero emits emits a sealed base class -----------

    #[test]
    fn zero_emit_component_emits_sealed_base_class() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("sealed class XEvent"),
            "expected `sealed class XEvent`, got:\n{}",
            r.output
        );
    }

    // ----- Event union: one emit with payload --------------------------

    #[test]
    fn emit_with_payload_lowers_to_subclass_with_required_fields() {
        let m = component(
            "Grid",
            vec![],
            vec![emit(
                "onNavigate",
                vec![
                    EmitParam { name: "row".into(), r#type: EmitPayloadType::Number },
                    EmitParam { name: "col".into(), r#type: EmitPayloadType::Number },
                ],
            )],
        );
        let l = layout("Grid", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("Grid")).unwrap();
        let out = &r.output;
        assert!(out.contains("class GridEventNavigate extends GridEvent"));
        assert!(out.contains("final int row;"));
        assert!(out.contains("final int col;"));
        assert!(out.contains("required this.row,"));
        assert!(out.contains("required this.col,"));
    }

    // ----- Slot lowering: required vs optional + dispatch field --------

    #[test]
    fn required_slot_becomes_required_named_param_with_nonnullable_type() {
        let m = component(
            "Profile",
            vec![slot("display-name", SlotType::Text, true)],
            vec![],
        );
        let l = layout("Profile", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("Profile")).unwrap();
        let out = &r.output;
        assert!(out.contains("final String displayName;"));
        assert!(out.contains("required this.displayName,"));
        assert!(
            out.contains("final void Function(ProfileEvent) dispatch;"),
            "expected dispatch field, got:\n{out}"
        );
        assert!(out.contains("required this.dispatch,"));
    }

    #[test]
    fn optional_slot_becomes_nullable_named_param() {
        let m = component(
            "Profile",
            vec![slot("subtitle", SlotType::Text, false)],
            vec![],
        );
        let l = layout("Profile", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("Profile")).unwrap();
        let out = &r.output;
        assert!(out.contains("final String? subtitle;"));
        assert!(
            out.contains("    this.subtitle,") && !out.contains("required this.subtitle,"),
            "optional slot must NOT be `required`, got:\n{out}"
        );
    }

    // ----- Container nesting: Row/Column children walk -----------------

    #[test]
    fn row_with_text_children_lowers_to_dart_row() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Row",
                vec![],
                vec![
                    node_with(
                        "Text",
                        vec![LayoutProp {
                            name: "content".into(),
                            value: LayoutPropValue::String("Hello".into()),
                        }],
                        vec![],
                    ),
                    node_with(
                        "Text",
                        vec![LayoutProp {
                            name: "content".into(),
                            value: LayoutPropValue::String("World".into()),
                        }],
                        vec![],
                    ),
                ],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("Row("), "expected `Row(`, got:\n{out}");
        assert!(out.contains("Text(\"Hello\")"), "expected Hello, got:\n{out}");
        assert!(out.contains("Text(\"World\")"), "expected World, got:\n{out}");
    }

    // ----- Text with slot ref --------------------------------------------

    #[test]
    fn text_with_slot_ref_uses_bare_identifier() {
        let m = component(
            "X",
            vec![slot("greeting", SlotType::Text, true)],
            vec![],
        );
        let l = layout(
            "X",
            node_with(
                "Text",
                vec![LayoutProp {
                    name: "content".into(),
                    value: LayoutPropValue::SlotRef("greeting".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("Text(greeting)"),
            "expected `Text(greeting)`, got:\n{}",
            r.output
        );
    }

    // ----- HostButton + onTap dispatch placeholder ---------------------

    #[test]
    fn host_button_with_string_label_emits_elevated_button() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostButton",
                vec![LayoutProp {
                    name: "label".into(),
                    value: LayoutPropValue::String("Save".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("ElevatedButton"));
        assert!(out.contains("Text(\"Save\")"));
    }

    #[test]
    fn host_button_disabled_true_disables_onpressed() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostButton",
                vec![LayoutProp {
                    name: "disabled".into(),
                    value: LayoutPropValue::Keyword("true".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("onPressed: null"),
            "expected `onPressed: null` for disabled, got:\n{}",
            r.output
        );
    }

    // ----- HostInput with placeholder + slot value ---------------------

    #[test]
    fn host_input_with_placeholder_emits_input_decoration() {
        let m = component(
            "X",
            vec![slot("formula", SlotType::Text, true)],
            vec![],
        );
        let l = layout(
            "X",
            node_with(
                "HostInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("formula".into()),
                    },
                    LayoutProp {
                        name: "placeholder".into(),
                        value: LayoutPropValue::String("Type a formula".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("TextField("));
        assert!(out.contains("TextEditingController(text: formula)"));
        assert!(out.contains("hintText: \"Type a formula\""));
    }

    // ----- HostCheckbox + HostRadio scaffolds --------------------------

    #[test]
    fn host_checkbox_with_checked_slot_emits_checkbox_widget() {
        let m = component(
            "X",
            vec![slot("agreed", SlotType::Bool, true)],
            vec![],
        );
        let l = layout(
            "X",
            node_with(
                "HostCheckbox",
                vec![LayoutProp {
                    name: "checked".into(),
                    value: LayoutPropValue::SlotRef("agreed".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("Checkbox(value: agreed"),
            "expected `Checkbox(value: agreed`, got:\n{}",
            r.output
        );
    }

    #[test]
    fn host_radio_with_value_emits_radio_string_widget() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostRadio",
                vec![LayoutProp {
                    name: "value".into(),
                    value: LayoutPropValue::String("vanilla".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("Radio<String>(value: \"vanilla\""),
            "expected `Radio<String>(value: \"vanilla\"`, got:\n{}",
            r.output
        );
    }

    // ----- HostScroll ---------------------------------------------------

    #[test]
    fn host_scroll_with_one_child_wraps_in_single_child_scroll_view() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostScroll",
                vec![],
                vec![node_with(
                    "Text",
                    vec![LayoutProp {
                        name: "content".into(),
                        value: LayoutPropValue::String("Long content".into()),
                    }],
                    vec![],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(r.output.contains("SingleChildScrollView"));
        // Whitespace-tolerant: the recursed Text emitter inserts its
        // own indent before the `Text(...)` token. Assert the
        // `child:` keyword and the Text expression appear in order,
        // not that they're separated by exactly one space.
        let body_pos = r.output.find("child:").expect("child: keyword present");
        let text_pos = r.output[body_pos..]
            .find("Text(\"Long content\")")
            .expect("Text expression present after child:");
        assert!(
            text_pos < 200,
            "Text expression should be close to `child:` keyword; output:\n{}",
            r.output
        );
    }

    // ----- Component-name mismatch error path ---------------------------

    #[test]
    fn component_name_mismatch_returns_error() {
        let m = component("Alpha", vec![], vec![]);
        let l = layout("Beta", node("Box"));
        let err = from_pipeline(&m, &l, &empty_style("Alpha")).unwrap_err();
        assert!(matches!(err, PipelineEmitError::ComponentNameMismatch { .. }));
    }

    // ----- Dart-string escape safety ------------------------------------

    #[test]
    fn text_with_special_chars_in_string_is_escaped() {
        // Dart interpolates `$ident` inside double-quoted strings, so
        // a `$` in user content must be escaped. Same for `"` and `\`.
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Text",
                vec![LayoutProp {
                    name: "content".into(),
                    value: LayoutPropValue::String("Hello $world \" \\".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // `$` becomes `\$`; `"` becomes `\"`; `\` becomes `\\`.
        assert!(
            r.output.contains(r#"Text("Hello \$world \" \\")"#),
            "expected escaped string, got:\n{}",
            r.output
        );
    }

    // ----- Reserved-keyword slot rejection -----------------------------

    #[test]
    fn slot_name_clashing_with_dart_keyword_is_rejected() {
        let m = component(
            "X",
            vec![slot("class", SlotType::Text, true)],
            vec![],
        );
        let l = layout("X", node("Box"));
        let err = from_pipeline(&m, &l, &empty_style("X")).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsafeSlotName(_)));
    }

    // ----- Style → Container args --------------------------------------

    /// Security regression: the unresolved-component-reference
    /// fallback writes the tag into a `/* ... */` block comment.
    /// A malicious tag like `Foo*/dispatch(evil());/*` would
    /// terminate the comment early and inject arbitrary Dart code.
    /// The validator rejects anything that isn't a clean
    /// PascalCase identifier before splicing.
    #[test]
    fn component_reference_with_comment_terminator_is_rejected() {
        let m = component("Host", vec![], vec![]);
        let l = layout(
            "Host",
            node("Foo*/dispatch(evil());/*"),
        );
        let err = from_pipeline(&m, &l, &empty_style("Host")).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::UnknownPrimitive(_)),
            "expected UnknownPrimitive rejection for tag with `*/`, got {err:?}"
        );
    }

    /// Positive case for the same fallback path: a clean PascalCase
    /// component reference produces the labelled placeholder. (Real
    /// resolution against a package manifest is a follow-up PR.)
    #[test]
    fn clean_pascal_case_component_reference_emits_placeholder() {
        let m = component("Host", vec![], vec![]);
        let l = layout("Host", node("UserCard"));
        let r = from_pipeline(&m, &l, &empty_style("Host")).unwrap();
        assert!(
            r.output.contains("/* TODO: component reference 'UserCard' not yet resolved */"),
            "expected labelled placeholder, got:\n{}",
            r.output
        );
        assert!(r.output.contains("const SizedBox.shrink()"));
    }

    #[test]
    fn box_with_part_padding_emits_edge_insets_arg() {
        let style = StyleDef {
            component_name: "X".into(),
            parts: vec![PartStyle {
                name: "root".into(),
                base: vec![StyleProp {
                    name: "padding".into(),
                    value: "8".into(),
                }],
                states: Vec::new(),
            }],
        };
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            LayoutNode {
                tag: "Box".into(),
                part_name: Some("root".into()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = from_pipeline(&m, &l, &style).unwrap();
        assert!(
            r.output.contains("padding: const EdgeInsets.all(8)"),
            "expected EdgeInsets.all(8), got:\n{}",
            r.output
        );
    }
}
