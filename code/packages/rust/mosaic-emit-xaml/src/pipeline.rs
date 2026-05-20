//! # Three-file pipeline entry point for the WinUI 3 / XAML backend.
//!
//! Mirrors the public function shape of `mosaic-emit-react`'s and
//! `mosaic-emit-swiftui`'s `pipeline` modules — same [`from_pipeline`]
//! signature, same error variants, same section emitters. Read alongside
//! those crates' source if you need a side-by-side comparison of how the
//! same IR lowers to JSX vs. Swift vs. XAML.
//!
//! ## Why XAML emission needs three output files
//!
//! WinUI 3's XAML compiler parses the markup file at build time and
//! generates a partial class. The user-authored partial class lives in
//! `{Component}.xaml.cs` and must match the markup's class name + base
//! type. Squashing both into one file is not possible at the WinUI 3
//! level — the XAML compiler refuses files that don't match its expected
//! shape. Splitting the event-union out into a third file is a Mosaic
//! convention (keeps `.xaml.cs` lean and lets hosts import event types in
//! isolation).
//!
//! ## Design choices
//!
//! - **Slots become `DependencyProperty`.** WinUI 3's binding system
//!   requires dependency properties for `{x:Bind}` / `{Binding}` change
//!   notifications. The hand-rolled DP registration is a fixed pattern
//!   per type; the emitter writes one block per slot.
//! - **Emits become a single `Dispatch` event** carrying the discriminated
//!   `{Component}Event` record. This matches UI24 §3.1's React shape
//!   (`event GridEvent = ...`) exactly — host code subscribes via
//!   `grid.Dispatch += (s, e) => state.HandleEvent(e);`.
//! - **`Box` without padding/background lowers to `<ContentPresenter>`**
//!   instead of `<Border>`. A `<Border>` always paints (even with zero
//!   thickness and transparent brush) — `<ContentPresenter>` is the
//!   zero-cost option. The emitter picks the right one by inspecting the
//!   resolved mosstyle for the box's part name. PR-1 doesn't yet inline
//!   per-element styles, so today every `Box` lowers to `<Border>`
//!   defensively. A follow-up swaps in `<ContentPresenter>` when no
//!   `Background`/`BorderThickness`/`Padding` are set.

use std::fmt::Write as _;

use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
};
use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosstyle_compiler::StyleDef;

// =====================================================================
// Public API
// =====================================================================

/// The result of compiling a three-file pipeline triple to a WinUI 3
/// UserControl.
///
/// Mirrors `mosaic_emit_react::pipeline::PipelineEmitResult` and
/// `mosaic_emit_swiftui::pipeline::PipelineEmitResult` so a generic CLI
/// driver can treat all three backends uniformly — except that XAML
/// returns three separate source strings (one per output file) rather
/// than one combined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XamlEmitResult {
    /// The XAML markup: `<UserControl>` + lowered moslayout tree.
    /// Destined for `{Component}.xaml`.
    pub xaml: String,

    /// The C# code-behind: `partial class {Component} : UserControl` with
    /// dependency properties, the `Dispatch` event, and any
    /// `InitializeComponent()` boilerplate.
    /// Destined for `{Component}.xaml.cs`.
    pub code_behind: String,

    /// The discriminated event-union: `partial class {Component}Event`
    /// with one nested `sealed record` per declared emit.
    /// Destined for `{Component}.Event.cs`.
    pub events: String,

    /// The component's PascalCase name (matches the source `.mil` /
    /// `.mll`). Unprefixed; the generated UserControl's class name is
    /// exactly this string.
    pub component_name: String,

    /// Project-shaped artifacts (`.csproj`, `App.xaml(.cs)`,
    /// `MainWindow.xaml(.cs)`, `Package.appxmanifest`) when
    /// `EmitOptions::emit_project` is on. `None` otherwise.
    ///
    /// PR-1 always returns `None`; the project triple lands with
    /// PR-5.
    pub project: Option<ProjectFiles>,

    /// One entry per `For` block — the generated `RowVm` C# source.
    ///
    /// PR-1 always returns an empty `Vec`; `For` lowering lands with
    /// PR-2.
    pub for_view_models: Vec<EmittedFile>,

    /// One entry per `If` whose expression is not `{x:Bind}`-able — the
    /// computed-property helper C# source.
    ///
    /// PR-1 always returns an empty `Vec`; `If` lowering lands with
    /// PR-2.
    pub if_helpers: Vec<EmittedFile>,
}

/// A generated source file with a filename and its UTF-8 source text.
/// Used for the `for_view_models` / `if_helpers` / `project` fields where
/// one logical artifact corresponds to multiple physical files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedFile {
    /// The filename relative to the component directory. No path
    /// separators; flat filenames only.
    pub filename: String,
    /// The complete source text.
    pub source: String,
}

/// Project-shaped artifacts emitted when `EmitOptions::emit_project` is on.
///
/// PR-1 never populates this — the field is on `XamlEmitResult` to lock
/// the API shape for PR-5's project-mode work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFiles {
    pub csproj: String,
    pub app_xaml: String,
    pub app_xaml_cs: String,
    pub main_window_xaml: String,
    pub main_window_cs: String,
    pub package_manifest: String,
}

/// Options controlling the emitter's behaviour.
///
/// Default: produces only the component triple (`xaml`, `code_behind`,
/// `events`); no project artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Also emit `.csproj` + `App.xaml(.cs)` + `MainWindow.xaml(.cs)` +
    /// `Package.appxmanifest`. Default `false`. PR-1 ignores this flag —
    /// the project triple lands in PR-5.
    pub emit_project: bool,

    /// Top-level C# namespace for emitted types. Default
    /// `"Mosaic.Generated"`.
    pub namespace: String,

    /// Windows App SDK version to pin in the emitted `.csproj` (only used
    /// when `emit_project` is on). Default `"1.5"`.
    pub windows_app_sdk: String,

    /// Lower `HostTable` to `controls:DataGrid` from the Community
    /// Toolkit rather than a hand-rolled `<Grid>`. PR-1 ignores this
    /// flag — `HostTable` is unsupported until PR-4.
    pub use_community_datagrid: bool,

    /// Treat the input as a UI29 userland package. PR-1 ignores this
    /// flag — `--package-mode` lands in PR-5.
    pub package_mode: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            emit_project: false,
            namespace: "Mosaic.Generated".to_string(),
            windows_app_sdk: "1.5".to_string(),
            use_community_datagrid: false,
            package_mode: false,
        }
    }
}

/// Errors the WinUI 3 pipeline emitter can return.
///
/// PR-1 only fires the first three plus the safe-identifier checks; the
/// rest become reachable in PR-2..PR-5. They are on the enum today so
/// downstream consumers (the `mosaic-compile` CLI) can build against the
/// full set without churn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEmitError {
    /// The mosmodel component name and the moslayout component name
    /// disagree. The mosstyle name is allowed to differ per UI23 §4 (a
    /// style file can target a layout variant).
    ComponentNameMismatch { mosmodel: String, moslayout: String },

    /// A moslayout primitive tag is not in the UI29 kernel and not a
    /// recognised sub-tag, and the resolver could not find it in the
    /// active manifest (PR-5).
    ///
    /// PR-1 fires this for: `HostInput`, `HostButton`, `HostScroll`,
    /// `HostTable`, `HostTableColGroup`, `HostTableHead`,
    /// `HostTableBody`, `HostTableFoot`, `If`, `Else`, `For`, and every
    /// component-reference tag. The error message embeds the tag so the
    /// caller can include it in user-visible diagnostics.
    UnsupportedPrimitive(String),

    /// An expression form (`row[c]`, `slot: a && slot: b`, …) is not yet
    /// lowered by the WinUI 3 ExprLowerer. PR-1 doesn't yet attempt
    /// expression lowering; PR-2 introduces this error path.
    UnsupportedExpression(String),

    /// A component reference (non-kernel tag) was not found in the
    /// manifest's `[dependencies]`. PR-5 introduces this path.
    UnknownComponent(String),

    /// A slot name fails the safe-C#-identifier check after PascalCase
    /// conversion. Should never happen given the kebab-case grammar; the
    /// check is defense-in-depth.
    UnsafeSlotName(String),

    /// An emit name fails the same safe-identifier check.
    UnsafeEmitName(String),

    /// A mosmodel slot type has no WinUI 3 mapping (per spec §8). Only
    /// possible if mosmodel grows new slot types ahead of this backend.
    UnmappableSlotType(String),

    /// A mosstyle CSS property has no XAML setter mapping. PR-1's
    /// limited style-inlining doesn't fire this; later PRs do.
    UnmappableStyleProperty(String),

    /// A `HostTable` carries two of the same section sub-tag. PR-4
    /// detects and fires this.
    DuplicateTableSection(String),
}

impl std::fmt::Display for PipelineEmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineEmitError::ComponentNameMismatch { mosmodel, moslayout } => write!(
                f,
                "component name mismatch: mosmodel says '{mosmodel}', moslayout says '{moslayout}'"
            ),
            PipelineEmitError::UnsupportedPrimitive(t) => write!(
                f,
                "moslayout primitive '{t}' is not yet supported by the WinUI 3 / XAML emitter (deferred per spec PR sequence)"
            ),
            PipelineEmitError::UnsupportedExpression(e) => write!(
                f,
                "expression form not yet supported on the WinUI 3 backend: {e}"
            ),
            PipelineEmitError::UnknownComponent(c) => write!(
                f,
                "unknown component reference '{c}' (not in manifest dependencies)"
            ),
            PipelineEmitError::UnsafeSlotName(n) => {
                write!(f, "unsafe slot name '{n}' (post PascalCase conversion)")
            }
            PipelineEmitError::UnsafeEmitName(n) => {
                write!(f, "unsafe emit name '{n}' (post PascalCase conversion)")
            }
            PipelineEmitError::UnmappableSlotType(t) => {
                write!(f, "slot type {t:?} has no WinUI 3 mapping")
            }
            PipelineEmitError::UnmappableStyleProperty(p) => {
                write!(f, "style property {p:?} has no XAML setter mapping")
            }
            PipelineEmitError::DuplicateTableSection(s) => {
                write!(f, "HostTable has duplicate section sub-tag '{s}'")
            }
        }
    }
}

impl std::error::Error for PipelineEmitError {}

/// Compile a three-file Mosaic pipeline triple to a WinUI 3 UserControl
/// triple.
///
/// The `_manifest` argument carries the resolved package dependencies
/// (UI29 §4.4). PR-1 ignores it — every non-kernel tag is currently
/// reported as `UnsupportedPrimitive`. PR-5 lands the resolver and
/// switches a manifest-known tag to a `<pkg:ComponentName/>` reference.
///
/// # Errors
///
/// See `PipelineEmitError` for the full set.
pub fn from_pipeline(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
    _manifest: Option<&()>,
    options: &EmitOptions,
) -> Result<XamlEmitResult, PipelineEmitError> {
    // 1. The three IRs must agree on the component name. The style IR's
    //    `component_name` is allowed to differ when the style targets a
    //    specific layout variant (UI23 §4).
    if interface.component != layout.component_name {
        return Err(PipelineEmitError::ComponentNameMismatch {
            mosmodel: interface.component.clone(),
            moslayout: layout.component_name.clone(),
        });
    }

    let name = &interface.component;

    // 2. Build a part-name → CSS-fragment map from the mosstyle source.
    //    Used by the style inliner inside each primitive emitter. PR-1's
    //    inliner only consumes base props; state blocks and the full
    //    UserControl.Resources cascade land in later PRs.
    let part_styles = build_part_style_map(style);

    // 3. Emit each of the three files.
    let xaml = emit_xaml(name, &layout.root, &part_styles, options)?;
    let code_behind = emit_code_behind(name, &interface.slots, &interface.emits, options)?;
    let events = emit_events(name, &interface.emits, options)?;

    Ok(XamlEmitResult {
        xaml,
        code_behind,
        events,
        component_name: name.clone(),
        project: None,             // PR-5
        for_view_models: Vec::new(), // PR-2
        if_helpers: Vec::new(),    // PR-2
    })
}

// =====================================================================
// Part-style map (mosstyle → flat property fragments)
// =====================================================================

/// A part-style entry: the joined CSS fragment for one `part` block's
/// base properties. State blocks are deferred to a later PR; today only
/// the base goes into the map.
type PartStyleMap = std::collections::HashMap<String, String>;

/// Walk the `StyleDef` and produce a flat `part_name -> css_fragment`
/// map. The fragment is a comma-separated `key: "value"` list ready to
/// embed in a XAML setter chain.
fn build_part_style_map(style: &StyleDef) -> PartStyleMap {
    let mut out = PartStyleMap::with_capacity(style.parts.len());
    for part in &style.parts {
        let frag = build_style_fragment(&part.base);
        if !frag.is_empty() {
            out.insert(part.name.clone(), frag);
        }
        // State blocks not yet wired — see CHANGELOG known-limitations.
    }
    out
}

fn build_style_fragment(props: &[mosstyle_compiler::StyleProp]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(props.len());
    for p in props {
        let key = css_property_to_xaml_setter(&p.name);
        let escaped = p.value.replace('\\', "\\\\").replace('"', "\\\"");
        parts.push(format!("{key}=\"{escaped}\""));
    }
    parts.join(" ")
}

/// Map a mosstyle CSS property name to its XAML setter property name.
/// The table is intentionally small in PR-1 — only what the nine simple
/// primitives need. PR-3..PR-6 grow it.
fn css_property_to_xaml_setter(name: &str) -> String {
    match name {
        "background"     => "Background".to_string(),
        "color"          => "Foreground".to_string(),
        "font-family"    => "FontFamily".to_string(),
        "font-size"      => "FontSize".to_string(),
        "font-weight"    => "FontWeight".to_string(),
        "padding"        => "Padding".to_string(),
        "margin"         => "Margin".to_string(),
        "width"          => "Width".to_string(),
        "height"         => "Height".to_string(),
        "border-width"   => "BorderThickness".to_string(),
        "border-color"   => "BorderBrush".to_string(),
        // Anything else passes through PascalCased so we don't crash; the
        // emitter prefers a stale-but-running output to a hard error in
        // PR-1. A real CSS → XAML completeness check lands later.
        other => kebab_to_pascal_case(other),
    }
}

// =====================================================================
// Identifier conversions
// =====================================================================

/// `column-headers` → `ColumnHeaders` (XAML `DependencyProperty` names,
/// C# property names). PascalCase.
fn kebab_to_pascal_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut capitalise = true;
    for c in s.chars() {
        if c == '-' || c == '_' {
            capitalise = true;
        } else if capitalise {
            out.push(c.to_ascii_uppercase());
            capitalise = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// `column-headers` → `columnHeaders`. Used for `{x:Bind}` paths and
/// local C# helpers. camelCase with first letter lowered.
///
/// Currently unused — PR-2's `For` lowering will reach for it when it
/// generates `{x:Bind}` paths into `For`-bound row variables.
#[allow(dead_code)]
fn kebab_to_camel_case(s: &str) -> String {
    let mut p = kebab_to_pascal_case(s);
    if let Some(first) = p.get_mut(0..1) {
        first.make_ascii_lowercase();
    }
    p
}

/// Verify a converted identifier is a valid C# / XAML name: ASCII alpha
/// followed by ASCII alphanumeric. The mosmodel grammar already produces
/// safe kebab-case names so this is defense-in-depth.
fn is_safe_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// =====================================================================
// File 1: XAML markup
// =====================================================================

/// Emit `{Component}.xaml` — the markup file. Wraps the lowered
/// moslayout tree in a `<UserControl>` root.
fn emit_xaml(
    name: &str,
    root: &LayoutNode,
    part_styles: &PartStyleMap,
    options: &EmitOptions,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let ns = &options.namespace;

    writeln!(out, "<!-- Auto-generated by mosaic-emit-xaml. Do not edit. -->")
        .unwrap();
    writeln!(out, "<UserControl").unwrap();
    writeln!(out, "    x:Class=\"{ns}.{name}\"").unwrap();
    writeln!(out, "    xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"")
        .unwrap();
    writeln!(out, "    xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"").unwrap();
    writeln!(out, "    xmlns:local=\"using:{ns}\">").unwrap();
    writeln!(out).unwrap();

    let body = emit_xaml_node(root, 4, part_styles)?;
    out.push_str(&body);

    writeln!(out).unwrap();
    writeln!(out, "</UserControl>").unwrap();
    Ok(out)
}

/// Lower one moslayout node and its descendants to XAML, indented by
/// `indent` spaces. PR-1 handles the nine simple kernel primitives;
/// everything else surfaces as a clear `UnsupportedPrimitive` error.
fn emit_xaml_node(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    match node.tag.as_str() {
        "Box" => emit_box(node, indent, part_styles),
        "Row" => emit_stack_panel(node, indent, part_styles, "Horizontal"),
        "Column" => emit_stack_panel(node, indent, part_styles, "Vertical"),
        "Stack" => emit_stack(node, indent, part_styles),
        "Text" => emit_text(node, indent, part_styles),
        "Image" => emit_image(node, indent, part_styles),
        "Spacer" => emit_spacer(node, indent, part_styles),
        "Divider" => emit_divider(node, indent, part_styles),
        "Icon" => emit_icon(node, indent, part_styles),

        // PR-2..PR-5 territory. Recognised by name so the error message is
        // self-documenting ("not yet supported", not "unknown tag").
        "If" | "Else" | "For"
        | "HostInput" | "HostButton" | "HostScroll" | "HostTable"
        | "HostTableColGroup" | "HostTableHead" | "HostTableBody" | "HostTableFoot" => {
            Err(PipelineEmitError::UnsupportedPrimitive(node.tag.clone()))
        }

        // Anything else is a component reference; will route through the
        // manifest resolver in PR-5. PR-1 simply errors.
        other => Err(PipelineEmitError::UnsupportedPrimitive(other.to_string())),
    }
}

/// Build the optional `Style="..."` attribute fragment from a part name.
/// Returns the full attribute (e.g. ` Background="#1e1e1e" FontSize="12"`)
/// or an empty string when no style applies.
fn part_style_attr(node: &LayoutNode, part_styles: &PartStyleMap) -> String {
    if let Some(part) = node.part_name.as_deref() {
        if let Some(frag) = part_styles.get(part) {
            // Each fragment is space-separated `Key="Value"` pairs ready
            // to splice straight into the opening tag.
            return format!(" {frag}");
        }
    }
    String::new()
}

// ---------------------------------------------------------------------
// Primitive emitters (the nine simple kernel primitives — PR-1)
// ---------------------------------------------------------------------

/// `Box [name] { children }` → `<Border>...</Border>`.
///
/// PR-1 always emits `<Border>` even when no style applies. A later PR
/// swaps to `<ContentPresenter>` when the resolved style has no
/// background / border / padding — `<ContentPresenter>` is zero-cost
/// while `<Border>` always allocates a brush.
fn emit_box(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    emit_container(node, indent, part_styles, "Border")
}

fn emit_stack_panel(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    orientation: &str,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let mut out = format!(
        "{pad}<StackPanel Orientation=\"{orientation}\"{style}>\n"
    );
    for child in &node.children {
        out.push_str(&emit_xaml_node(child, indent + 4, part_styles)?);
    }
    write!(out, "{pad}</StackPanel>\n").unwrap();
    Ok(out)
}

/// `Stack [name] { children }` → `<Grid>...</Grid>`.
///
/// XAML `<Grid>` is the z-axis container — children at the same row/col
/// stack visually with later children drawn on top. The Mosaic `Stack`
/// primitive (UI29 §2.1) is exactly this shape.
fn emit_stack(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    emit_container(node, indent, part_styles, "Grid")
}

fn emit_container(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    element: &str,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let mut out = format!("{pad}<{element}{style}>\n");
    for child in &node.children {
        out.push_str(&emit_xaml_node(child, indent + 4, part_styles)?);
    }
    write!(out, "{pad}</{element}>\n").unwrap();
    Ok(out)
}

/// `Text [name] (content: slot: foo)` → `<TextBlock Text="{x:Bind Foo}"/>`.
/// `Text [name] (content: "literal")` → `<TextBlock Text="literal"/>`.
fn emit_text(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let text_attr = match find_prop_value(node, "content") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            if !is_safe_identifier(&pascal) {
                return Err(PipelineEmitError::UnsafeSlotName(pascal));
            }
            format!(" Text=\"{{x:Bind {pascal}}}\"")
        }
        Some(LayoutPropValue::String(s)) => {
            let escaped = escape_xaml_attr(s);
            format!(" Text=\"{escaped}\"")
        }
        Some(LayoutPropValue::Keyword(k)) => {
            // A `content: <NAME>` form — treat as literal text. Matches
            // how the React backend handles bare-name content.
            let escaped = escape_xaml_attr(k);
            format!(" Text=\"{escaped}\"")
        }
        Some(LayoutPropValue::Number(n)) => format!(" Text=\"{n}\""),
        Some(LayoutPropValue::Expr(_)) => {
            return Err(PipelineEmitError::UnsupportedExpression(
                "Text content expression (PR-2)".to_string(),
            ));
        }
        Some(LayoutPropValue::EmitRef(_)) | None => String::new(),
    };

    Ok(format!("{pad}<TextBlock{text_attr}{style}/>\n"))
}

/// `Image [name] (source: slot: foo)` → `<Image Source="{x:Bind Foo}"/>`.
fn emit_image(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let source_attr = match find_prop_value(node, "source").or_else(|| find_prop_value(node, "src")) {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            if !is_safe_identifier(&pascal) {
                return Err(PipelineEmitError::UnsafeSlotName(pascal));
            }
            format!(" Source=\"{{x:Bind {pascal}}}\"")
        }
        Some(LayoutPropValue::String(s)) => format!(" Source=\"{}\"", escape_xaml_attr(s)),
        _ => String::new(),
    };
    Ok(format!("{pad}<Image{source_attr}{style}/>\n"))
}

/// `Spacer` → `<Rectangle/>` with default Width/Height that flex the layout.
/// In a StackPanel a `<Rectangle Width="0" Height="0"/>` collapses; a more
/// useful default is `Width="Auto"` so the parent layout can drive size.
fn emit_spacer(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    Ok(format!(
        "{pad}<Rectangle Width=\"Auto\" Height=\"Auto\"{style}/>\n"
    ))
}

/// `Divider` → a thin `<Border>` band. WinUI 3 has no `<Separator>` in the
/// base SDK; the conventional pattern is a `<Border BorderThickness="..."
/// BorderBrush="..."/>` line.
fn emit_divider(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    // Default 1px horizontal divider. A future PR reads
    // `direction: vertical` for a vertical line.
    Ok(format!(
        "{pad}<Border BorderThickness=\"0,0,0,1\" BorderBrush=\"#80808080\"{style}/>\n"
    ))
}

/// `Icon [name] (glyph: "...")` → `<FontIcon Glyph="..."/>` against
/// Segoe Fluent Icons (the WinUI 3 default icon font).
fn emit_icon(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let glyph_attr = match find_prop_value(node, "glyph").or_else(|| find_prop_value(node, "name")) {
        Some(LayoutPropValue::String(s)) => format!(" Glyph=\"{}\"", escape_xaml_attr(s)),
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            format!(" Glyph=\"{{x:Bind {pascal}}}\"")
        }
        _ => String::new(),
    };
    Ok(format!("{pad}<FontIcon{glyph_attr}{style}/>\n"))
}

// =====================================================================
// File 2: code-behind (.xaml.cs)
// =====================================================================

/// Emit `{Component}.xaml.cs` — the partial class with DPs, the
/// Dispatch event, and constructor boilerplate.
fn emit_code_behind(
    name: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    options: &EmitOptions,
) -> Result<String, PipelineEmitError> {
    let ns = &options.namespace;
    let mut out = String::new();

    writeln!(out, "// Auto-generated by mosaic-emit-xaml. Do not edit.").unwrap();
    writeln!(out, "using Microsoft.UI.Xaml;").unwrap();
    writeln!(out, "using Microsoft.UI.Xaml.Controls;").unwrap();
    writeln!(out, "using System;").unwrap();
    writeln!(out, "using System.Collections.Generic;").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "namespace {ns};").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "public sealed partial class {name} : UserControl").unwrap();
    writeln!(out, "{{").unwrap();

    // Constructor: `InitializeComponent()`. The XAML compiler generates
    // the actual `InitializeComponent` method at build time from the
    // matching `.xaml` file.
    writeln!(out, "    public {name}()").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        this.InitializeComponent();").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // One DependencyProperty per declared slot (spec §8).
    for slot in slots {
        out.push_str(&emit_dependency_property(slot, name)?);
        writeln!(out).unwrap();
    }

    // The UI24 Dispatch event. Always emitted (even with zero emits) so
    // host code can subscribe uniformly.
    writeln!(
        out,
        "    /// <summary>Fires once for every emit declared in the .mil interface.</summary>"
    )
    .unwrap();
    writeln!(out, "    public event EventHandler<{name}Event>? Dispatch;").unwrap();
    writeln!(out).unwrap();

    // Helper to invoke Dispatch from generated handlers — used by future
    // PRs (PR-3 wires HostButton's Click etc.). Today it's just here as a
    // no-warn unused method to lock the API shape.
    if !emits.is_empty() {
        writeln!(out, "    private void RaiseDispatch({name}Event ev)").unwrap();
        writeln!(out, "    {{").unwrap();
        writeln!(out, "        Dispatch?.Invoke(this, ev);").unwrap();
        writeln!(out, "    }}").unwrap();
    }

    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn emit_dependency_property(
    slot: &SlotDecl,
    component: &str,
) -> Result<String, PipelineEmitError> {
    let pascal = kebab_to_pascal_case(&slot.name);
    if !is_safe_identifier(&pascal) {
        return Err(PipelineEmitError::UnsafeSlotName(pascal));
    }
    let csharp_type = slot_type_to_csharp(&slot.r#type)?;

    let mut out = String::new();
    writeln!(out, "    public {csharp_type} {pascal}").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        get => ({csharp_type})GetValue({pascal}Property);").unwrap();
    writeln!(out, "        set => SetValue({pascal}Property, value);").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(
        out,
        "    public static readonly DependencyProperty {pascal}Property ="
    )
    .unwrap();
    writeln!(
        out,
        "        DependencyProperty.Register(nameof({pascal}), typeof({csharp_type}), typeof({component}), new PropertyMetadata(default({csharp_type})));"
    )
    .unwrap();
    Ok(out)
}

/// Translate a mosmodel slot type to its C# property type per spec §8.
fn slot_type_to_csharp(t: &SlotType) -> Result<String, PipelineEmitError> {
    Ok(match t {
        SlotType::Text => "string".to_string(),
        SlotType::Number => "double".to_string(),
        SlotType::Bool => "bool".to_string(),
        SlotType::Color => "Windows.UI.Color".to_string(),
        SlotType::Image => "Microsoft.UI.Xaml.Media.Imaging.ImageSource".to_string(),
        SlotType::Node => "Microsoft.UI.Xaml.UIElement".to_string(),
        SlotType::List(inner) => format!("IReadOnlyList<{}>", list_inner_to_csharp(inner)?),
        // Component slots carry a host-defined named type. PR-5 (the
        // component-reference resolver) will look the name up against
        // the manifest and produce a proper C# type reference; for
        // now we forward the name verbatim so the DP at least compiles
        // when the host declares a matching record.
        SlotType::Component(type_name) => type_name.clone(),
    })
}

fn list_inner_to_csharp(t: &ListInnerType) -> Result<String, PipelineEmitError> {
    Ok(match t {
        ListInnerType::Text => "string".to_string(),
        ListInnerType::Number => "double".to_string(),
        ListInnerType::Bool => "bool".to_string(),
        ListInnerType::Color => "Windows.UI.Color".to_string(),
        ListInnerType::Image => "Microsoft.UI.Xaml.Media.Imaging.ImageSource".to_string(),
        ListInnerType::Node => "Microsoft.UI.Xaml.UIElement".to_string(),
        ListInnerType::Component(name) => name.clone(),
        ListInnerType::List(inner) => {
            format!("IReadOnlyList<{}>", list_inner_to_csharp(inner)?)
        }
    })
}

// =====================================================================
// File 3: event union (.Event.cs)
// =====================================================================

/// Emit `{Component}.Event.cs` — the discriminated record union for the
/// UI24 dispatch contract.
fn emit_events(
    name: &str,
    emits: &[EmitDecl],
    options: &EmitOptions,
) -> Result<String, PipelineEmitError> {
    let ns = &options.namespace;
    let mut out = String::new();

    writeln!(out, "// Auto-generated by mosaic-emit-xaml. Do not edit.").unwrap();
    writeln!(out, "namespace {ns};").unwrap();
    writeln!(out).unwrap();

    if emits.is_empty() {
        // Zero-emit components still produce the abstract record so host
        // code that pattern-matches on `{Component}Event` compiles.
        writeln!(out, "public abstract record {name}Event;").unwrap();
        return Ok(out);
    }

    writeln!(out, "public abstract record {name}Event").unwrap();
    writeln!(out, "{{").unwrap();
    for emit in emits {
        let case_name = strip_on_prefix(&emit.name);
        let case_pascal = kebab_to_pascal_case(&case_name);
        if !is_safe_identifier(&case_pascal) {
            return Err(PipelineEmitError::UnsafeEmitName(case_pascal));
        }

        if emit.params.is_empty() {
            writeln!(
                out,
                "    public sealed record {case_pascal}() : {name}Event;"
            )
            .unwrap();
        } else {
            let mut params: Vec<String> = Vec::with_capacity(emit.params.len());
            for p in &emit.params {
                let pname = kebab_to_pascal_case(&p.name);
                if !is_safe_identifier(&pname) {
                    return Err(PipelineEmitError::UnsafeEmitName(pname));
                }
                let ptype = emit_payload_to_csharp(&p.r#type);
                params.push(format!("{ptype} {pname}"));
            }
            writeln!(
                out,
                "    public sealed record {case_pascal}({}) : {name}Event;",
                params.join(", ")
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn emit_payload_to_csharp(t: &EmitPayloadType) -> String {
    match t {
        EmitPayloadType::Text => "string".to_string(),
        EmitPayloadType::Number => "double".to_string(),
        EmitPayloadType::Bool => "bool".to_string(),
        EmitPayloadType::Color => "Windows.UI.Color".to_string(),
        // Component-typed emit payloads forward the C# type name verbatim
        // (same shape as component-typed slots — the host declares a
        // matching record type and the resolver in PR-5 wires it up).
        EmitPayloadType::Component(type_name) => type_name.clone(),
    }
}

fn strip_on_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("on") {
        if rest.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) {
            // `onNavigate` → `navigate` (lower the first char so kebab
            // conversion gives `Navigate`).
            let mut chars = rest.chars();
            let mut s = String::with_capacity(rest.len());
            if let Some(c) = chars.next() {
                s.push(c.to_ascii_lowercase());
            }
            s.extend(chars);
            return s;
        }
    }
    name.to_string()
}

// =====================================================================
// Prop helpers
// =====================================================================

/// Look up a prop by name. Returns the prop's value if present, else None.
fn find_prop_value<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a LayoutPropValue> {
    node.props.iter().find(|p| p.name == prop_name).map(|p| &p.value)
}

/// Escape characters that would break a XAML attribute value. Quotes and
/// ampersand are the only ones strictly required; newlines pass through as
/// literal newlines.
fn escape_xaml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
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
    use mosmodel_compiler::{
        EmitParam, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
    };
    use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp};
    use mosstyle_compiler::{PartStyle, StyleDef, StyleProp};

    // ── helpers ──

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

    fn param(name: &str, t: EmitPayloadType) -> EmitParam {
        EmitParam {
            name: name.to_string(),
            r#type: t,
        }
    }

    fn layout_with_root(name: &str, root: LayoutNode) -> LayoutDef {
        LayoutDef {
            component_name: name.to_string(),
            root,
        }
    }

    fn box_root() -> LayoutNode {
        LayoutNode {
            tag: "Box".to_string(),
            part_name: None,
            props: Vec::new(),
            children: Vec::new(),
        }
    }

    fn empty_style(name: &str) -> StyleDef {
        StyleDef {
            component_name: name.to_string(),
            parts: Vec::new(),
        }
    }

    fn opts() -> EmitOptions {
        EmitOptions::default()
    }

    fn compile(c: &MosmodelComponent, l: &LayoutDef, s: &StyleDef) -> XamlEmitResult {
        from_pipeline(c, l, s, None, &opts()).expect("emit ok")
    }

    // ── version ──

    #[test]
    fn version_is_0_1_0() {
        assert_eq!(crate::VERSION, "0.1.0");
    }

    // ── kebab → casing ──

    #[test]
    fn pascal_case_handles_single_segment() {
        assert_eq!(kebab_to_pascal_case("name"), "Name");
    }

    #[test]
    fn pascal_case_handles_multi_segment() {
        assert_eq!(kebab_to_pascal_case("column-headers"), "ColumnHeaders");
        assert_eq!(kebab_to_pascal_case("on-edit-commit"), "OnEditCommit");
    }

    #[test]
    fn camel_case_lowers_first_char() {
        assert_eq!(kebab_to_camel_case("column-headers"), "columnHeaders");
        assert_eq!(kebab_to_camel_case("name"), "name");
    }

    #[test]
    fn is_safe_identifier_accepts_normal_names() {
        assert!(is_safe_identifier("Foo"));
        assert!(is_safe_identifier("ColumnHeaders"));
        assert!(is_safe_identifier("X1"));
    }

    #[test]
    fn is_safe_identifier_rejects_leading_digit() {
        assert!(!is_safe_identifier("1Foo"));
    }

    #[test]
    fn is_safe_identifier_rejects_punctuation() {
        assert!(!is_safe_identifier("Foo-Bar"));
        assert!(!is_safe_identifier(""));
    }

    // ── component name mismatch ──

    #[test]
    fn component_name_mismatch_errors() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Bar", box_root());
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::ComponentNameMismatch { .. }));
    }

    // ── XAML root shape ──

    #[test]
    fn xaml_root_has_usercontrol_with_class_and_namespaces() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<UserControl"), "got:\n{}", r.xaml);
        assert!(
            r.xaml.contains("x:Class=\"Mosaic.Generated.Foo\""),
            "got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\""));
        assert!(r.xaml.contains("xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\""));
        assert!(r.xaml.contains("</UserControl>"));
    }

    #[test]
    fn xaml_uses_custom_namespace_from_options() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let s = empty_style("Foo");
        let mut o = opts();
        o.namespace = "Acme.Widgets".to_string();
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        assert!(
            r.xaml.contains("x:Class=\"Acme.Widgets.Foo\""),
            "got:\n{}",
            r.xaml
        );
    }

    // ── code-behind shape ──

    #[test]
    fn code_behind_has_partial_class_and_init_call() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.code_behind.contains("public sealed partial class Foo : UserControl"));
        assert!(r.code_behind.contains("this.InitializeComponent();"));
        assert!(r.code_behind.contains("public event EventHandler<FooEvent>? Dispatch;"));
    }

    #[test]
    fn code_behind_emits_one_dependency_property_per_slot() {
        let c = component(
            "Foo",
            vec![
                slot("name", SlotType::Text, true),
                slot("count", SlotType::Number, true),
            ],
            vec![],
        );
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.code_behind.contains("public string Name"));
        assert!(r.code_behind.contains("DependencyProperty.Register(nameof(Name), typeof(string), typeof(Foo)"));
        assert!(r.code_behind.contains("public double Count"));
        assert!(r.code_behind.contains("DependencyProperty.Register(nameof(Count), typeof(double), typeof(Foo)"));
    }

    #[test]
    fn code_behind_dp_for_list_uses_ireadonlylist() {
        let c = component(
            "Foo",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.code_behind.contains("public IReadOnlyList<string> Rows"),
            "got:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn code_behind_dp_for_nested_list_uses_nested_ireadonlylist() {
        let c = component(
            "Foo",
            vec![slot(
                "cells",
                SlotType::List(Box::new(ListInnerType::List(Box::new(ListInnerType::Text)))),
                true,
            )],
            vec![],
        );
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.code_behind
                .contains("public IReadOnlyList<IReadOnlyList<string>> Cells"),
            "got:\n{}",
            r.code_behind
        );
    }

    // ── event union ──

    #[test]
    fn empty_emit_union_is_abstract_record_with_no_body() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.events.contains("public abstract record FooEvent;"));
    }

    #[test]
    fn populated_emit_union_has_one_record_per_emit() {
        let c = component(
            "Grid",
            vec![],
            vec![
                emit(
                    "onNavigate",
                    vec![param("row", EmitPayloadType::Number), param("col", EmitPayloadType::Number)],
                ),
                emit("onEditCommit", vec![param("value", EmitPayloadType::Text)]),
                emit("onCancel", vec![]),
            ],
        );
        let l = layout_with_root("Grid", box_root());
        let r = compile(&c, &l, &empty_style("Grid"));
        assert!(
            r.events.contains("public sealed record Navigate(double Row, double Col) : GridEvent;"),
            "got:\n{}",
            r.events
        );
        assert!(r.events.contains("public sealed record EditCommit(string Value) : GridEvent;"));
        assert!(r.events.contains("public sealed record Cancel() : GridEvent;"));
    }

    #[test]
    fn raise_dispatch_helper_present_when_emits_exist() {
        let c = component("Foo", vec![], vec![emit("onClick", vec![])]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.code_behind.contains("RaiseDispatch"));
    }

    #[test]
    fn raise_dispatch_helper_absent_when_no_emits() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(!r.code_behind.contains("RaiseDispatch"));
    }

    // ── primitive lowering: Box / containers ──

    #[test]
    fn box_lowers_to_border() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<Border"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("</Border>"));
    }

    #[test]
    fn row_lowers_to_horizontal_stackpanel() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<StackPanel Orientation=\"Horizontal\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn column_lowers_to_vertical_stackpanel() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<StackPanel Orientation=\"Vertical\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn stack_lowers_to_grid() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Stack".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // <Grid> in XAML is the z-axis container (matches UI29 §2.1 `Stack`).
        assert!(r.xaml.contains("<Grid>"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("</Grid>"));
    }

    #[test]
    fn nested_containers_indent_correctly() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
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
                            value: LayoutPropValue::String("hi".to_string()),
                        }],
                        children: Vec::new(),
                    }],
                }],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // The Text should be nested under the Column under the Row.
        let row_pos = r.xaml.find("<StackPanel Orientation=\"Horizontal\"").unwrap();
        let col_pos = r.xaml.find("<StackPanel Orientation=\"Vertical\"").unwrap();
        let txt_pos = r.xaml.find("<TextBlock").unwrap();
        assert!(row_pos < col_pos && col_pos < txt_pos);
    }

    // ── primitive lowering: Text / Image / Spacer / Divider / Icon ──

    #[test]
    fn text_with_literal_content_emits_text_attribute() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::String("Hello".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<TextBlock Text=\"Hello\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn text_with_slot_ref_content_uses_xbind() {
        let c = component("Foo", vec![slot("greeting", SlotType::Text, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::SlotRef("greeting".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<TextBlock Text=\"{x:Bind Greeting}\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn text_with_kebab_slot_pascal_cases_in_xbind() {
        let c = component(
            "Foo",
            vec![slot("display-name", SlotType::Text, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::SlotRef("display-name".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("{x:Bind DisplayName}"));
    }

    #[test]
    fn text_with_quote_in_literal_escapes_xaml_attribute() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::String("say \"hi\"".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Text=\"say &quot;hi&quot;\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn text_with_expr_content_errors_with_unsupported_expression() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::Expr("row[c]".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedExpression(_)));
    }

    #[test]
    fn image_with_slot_ref_uses_xbind() {
        let c = component("Foo", vec![slot("avatar-url", SlotType::Image, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Image".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "source".to_string(),
                    value: LayoutPropValue::SlotRef("avatar-url".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<Image Source=\"{x:Bind AvatarUrl}\""));
    }

    #[test]
    fn spacer_lowers_to_rectangle_with_auto_size() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Spacer".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<Rectangle Width=\"Auto\" Height=\"Auto\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn divider_lowers_to_thin_border() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Divider".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("<Border BorderThickness=\"0,0,0,1\" BorderBrush=\"#80808080\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn icon_with_glyph_string_emits_fonticon() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Icon".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("\u{E700}".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<FontIcon"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Glyph="));
    }

    // ── unsupported primitives surface clearly ──

    #[test]
    fn host_input_errors_with_unsupported_primitive() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::UnsupportedPrimitive(ref t) if t == "HostInput"),
            "got: {err:?}"
        );
    }

    #[test]
    fn host_table_errors_with_unsupported_primitive() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "HostTable".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::UnsupportedPrimitive(ref t) if t == "HostTable")
        );
    }

    #[test]
    fn for_errors_with_unsupported_primitive() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "For".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedPrimitive(_)));
    }

    #[test]
    fn unknown_tag_errors_with_unsupported_primitive() {
        // Until PR-5 lands the component resolver, any non-kernel tag
        // looks the same to the emitter.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "WhateverComponent".to_string(),
                part_name: None,
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::UnsupportedPrimitive(ref t) if t == "WhateverComponent")
        );
    }

    // ── part-style application ──

    #[test]
    fn part_style_attaches_setters_to_container_opening_tag() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("root".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = StyleDef {
            component_name: "Foo".to_string(),
            parts: vec![PartStyle {
                name: "root".to_string(),
                base: vec![
                    StyleProp {
                        name: "background".to_string(),
                        value: "#1e1e1e".to_string(),
                    },
                    StyleProp {
                        name: "padding".to_string(),
                        value: "8".to_string(),
                    },
                ],
                states: Vec::new(),
            }],
        };
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("Background=\"#1e1e1e\""),
            "got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Padding=\"8\""), "got:\n{}", r.xaml);
    }

    // ── unused-flag placeholders ──

    #[test]
    fn project_field_is_none_in_pr1() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.project.is_none());
    }

    #[test]
    fn for_view_models_field_is_empty_in_pr1() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.for_view_models.is_empty());
    }

    #[test]
    fn if_helpers_field_is_empty_in_pr1() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.if_helpers.is_empty());
    }
}
