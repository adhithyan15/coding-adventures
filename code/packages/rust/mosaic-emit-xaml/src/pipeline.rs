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

    // 3. Construct the emission context — threaded through the XAML
    //    walker so `For`/`If` can register helpers, RowVms, and the
    //    converter requirement (PR-2).
    let mut ctx = EmitContext::new(name, &interface.slots);

    // 4. Emit each of the three files.
    let xaml = emit_xaml(name, &layout.root, &part_styles, options, &mut ctx)?;
    let code_behind =
        emit_code_behind(name, &interface.slots, &interface.emits, options, &ctx)?;
    let events = emit_events(name, &interface.emits, options)?;

    // 5. Assemble the result. RowVms become entries in `for_view_models`;
    //    the `if_helpers` field remains empty because the emitter inlines
    //    helper methods into the code-behind's partial class (one file
    //    per component is cleaner than scattering helper-bodies across
    //    siblings — the spec calls for separate files but PR-2 keeps
    //    them inline; see CHANGELOG for the deviation rationale).
    let for_view_models = ctx
        .row_vms
        .iter()
        .map(|vm| EmittedFile {
            filename: format!("{}.cs", vm.class_name),
            source: emit_row_vm_source(name, vm, options),
        })
        .collect();

    Ok(XamlEmitResult {
        xaml,
        code_behind,
        events,
        component_name: name.clone(),
        project: None,             // PR-5
        for_view_models,
        if_helpers: Vec::new(),    // helpers live inline in code_behind
    })
}

// =====================================================================
// EmitContext — state threaded through the XAML walker (PR-2)
// =====================================================================
//
// `If`/`Else`/`For` lowering needs information that doesn't live on any
// single node:
//
// - `For` introduces a binding into scope; nested `For`s can see outer
//   bindings; `{x:Bind}` paths need to know whether a name is a slot
//   (resolves on `this`) or a for-bound name (resolves on the
//   `DataContext` of the enclosing `<DataTemplate>`).
// - `If`/`Else` may need to generate helper methods for expressions that
//   aren't directly `{x:Bind}`-able. The helpers go into the
//   code-behind partial class.
// - `If` overall requires a `BoolToVisibilityConverter` resource added
//   exactly once per UserControl.
// - `For` requires a `RowVm` C# record per block, declared as a separate
//   file in `for_view_models`.
//
// `EmitContext` carries this state through the recursive walk and is
// consumed by `from_pipeline` when assembling the final result.

/// A single `For`-bound name in scope at the current emission point.
#[derive(Debug, Clone)]
struct ForBinding {
    /// The `as:` binding name, kebab-cased as written (e.g. `row`).
    as_name: String,
    /// The `index:` binding name when present (e.g. `r`).
    index_name: Option<String>,
    /// The C# type of the element. Derived from the iterated slot's type:
    /// `list<text>` → `string`, `list<number>` → `double`, etc.
    element_type: String,
    /// The generated RowVm class name: `{Component}_{AsName}Vm`.
    /// Stored on the binding even though the per-element binding code
    /// resolves the same value off `RowVm` — used by nested-For helper
    /// transliteration in a follow-up PR and by debug introspection.
    #[allow(dead_code)]
    vm_class: String,
}

/// A C# helper method that the emitter generates into the code-behind.
/// Used for expressions that `{x:Bind}` cannot evaluate directly
/// (indexer, comparison, logical, negation).
#[derive(Debug, Clone)]
struct HelperMethod {
    /// PascalCase method name (deterministic from the expression).
    name: String,
    /// `(parameter_name, parameter_csharp_type)` pairs.
    parameters: Vec<(String, String)>,
    /// C# return type — `bool` for predicates, `string` for indexed
    /// element accessors, `double` for numeric, etc.
    return_type: String,
    /// The C# expression body (no trailing semicolon).
    body: String,
}

/// A WinUI 3 event-handler method generated for a Host* primitive's
/// bound emits. Same lifecycle as `HelperMethod` — registered during
/// the walk, emitted inline into the code-behind partial class.
#[derive(Debug, Clone)]
struct HostHandler {
    /// Fully-qualified method name (also the XAML attribute value).
    name: String,
    /// Full C# source for the method, including signature and body.
    /// Multi-line and self-contained — emitted verbatim into the
    /// `partial class`.
    source: String,
}

/// A generated `RowVm` C# record — the typed `DataContext` for a
/// `<DataTemplate>` inside a `For` block.
#[derive(Debug, Clone)]
struct RowVm {
    /// `{Component}_{AsName}Vm` — must match the `x:DataType` reference
    /// in the matching `<DataTemplate>`.
    class_name: String,
    /// The PascalCase property name that holds the element value (e.g.
    /// `Row`, `Cell`). Derived from the `as:` binding.
    element_property: String,
    /// The C# type of the element value.
    element_type: String,
    /// `true` iff the matching `For` declared an `index:` binding.
    has_index: bool,
}

/// Mutable state threaded through the recursive XAML emission.
///
/// PR-1's emit_xaml didn't need any of this — all primitive emitters
/// were stateless. PR-2's `For`/`If` lowering does, so we collect every
/// stateful effect into one struct that the assembly step in
/// `from_pipeline` consumes.
struct EmitContext<'a> {
    /// The component name — used to namespace generated types.
    component_name: &'a str,
    /// Slot name (kebab-case) → C# type. For looking up the element
    /// type of a `For (each: slot: foo)` from `foo`'s declared type.
    slot_types: std::collections::HashMap<String, String>,
    /// `For` bindings currently in scope, innermost last. When a name
    /// resolves at expression-lowering time we walk from the back of
    /// the stack to find the closest binding.
    for_scope: Vec<ForBinding>,
    /// Helper methods to emit into the code-behind. Deduplicated by
    /// method name so two identical expressions in the same component
    /// produce only one helper.
    helpers: Vec<HelperMethod>,
    /// Tracks whether any `If` has been emitted. When `true`, the
    /// emitter writes a `BoolToVisibilityConverter` resource into the
    /// `<UserControl.Resources>` block.
    needs_bool_to_vis: bool,
    /// One `RowVm` per `For` block in the component. Becomes
    /// `XamlEmitResult::for_view_models`.
    row_vms: Vec<RowVm>,
    /// Host* event-handler method bodies registered during the walk.
    /// Each `HostInput` / `HostButton` with bound emits adds one or
    /// more entries; the assembly step emits them inline in the
    /// code-behind partial class.
    host_handlers: Vec<HostHandler>,
    /// Counter used to disambiguate Host* `x:Name`s when the node has
    /// no `part_name`. Incremented per emitted Host* primitive.
    host_counter: u32,
}

impl<'a> EmitContext<'a> {
    fn new(name: &'a str, slots: &[SlotDecl]) -> Self {
        let mut slot_types = std::collections::HashMap::new();
        for slot in slots {
            let cs = slot_type_to_csharp(&slot.r#type).unwrap_or_else(|_| "object".to_string());
            slot_types.insert(slot.name.clone(), cs);
        }
        Self {
            component_name: name,
            slot_types,
            for_scope: Vec::new(),
            helpers: Vec::new(),
            needs_bool_to_vis: false,
            row_vms: Vec::new(),
            host_handlers: Vec::new(),
            host_counter: 0,
        }
    }

    /// Allocate a unique counter for a Host* element that lacks a
    /// `part_name`. Always returns a fresh value.
    fn next_host_counter(&mut self) -> u32 {
        self.host_counter += 1;
        self.host_counter
    }

    /// Register an event-handler method. Same dedup pattern as
    /// helpers — two handlers with the same name share the same body.
    fn add_host_handler(&mut self, h: HostHandler) {
        if !self.host_handlers.iter().any(|x| x.name == h.name) {
            self.host_handlers.push(h);
        }
    }

    /// Find a `For` binding by `as:` name, searching innermost-first.
    fn lookup_for_binding(&self, as_name: &str) -> Option<&ForBinding> {
        self.for_scope.iter().rev().find(|b| b.as_name == as_name)
    }

    /// Find a `For` binding's index by name.
    fn lookup_for_index(&self, index_name: &str) -> Option<&ForBinding> {
        self.for_scope
            .iter()
            .rev()
            .find(|b| b.index_name.as_deref() == Some(index_name))
    }

    /// Add a helper method (or skip if a method by the same name already
    /// exists — assumed to be identical because helper names are a
    /// deterministic function of the expression they came from).
    fn add_helper(&mut self, helper: HelperMethod) {
        if !self.helpers.iter().any(|h| h.name == helper.name) {
            self.helpers.push(helper);
        }
    }
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
///
/// `ctx` is mutated during the walk: `For` pushes/pops bindings, `If`
/// adds helper methods, both may flip `needs_bool_to_vis`.
fn emit_xaml(
    name: &str,
    root: &LayoutNode,
    part_styles: &PartStyleMap,
    options: &EmitOptions,
    ctx: &mut EmitContext<'_>,
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

    // Walk the root node — at the moslayout level a component has
    // exactly one root, but we still pass through the children iterator
    // because `If`/`Else` pairing happens there.
    let body = emit_xaml_node(root, 4, part_styles, ctx)?;
    out.push_str(&body);

    // After walking, if any `If` was emitted we must declare the
    // converter resource. We splice it in after the open `<UserControl>`
    // by rebuilding — small enough cost given a typical component has
    // a few hundred bytes of XAML.
    if ctx.needs_bool_to_vis {
        let resources = emit_bool_to_vis_resource_block(4);
        // Insert after the `<UserControl ... >` opening tag (find the
        // `>\n` that closes the open tag).
        let split_at = out
            .find(">\n")
            .map(|p| p + 2)
            .unwrap_or(out.len());
        let (head, tail) = out.split_at(split_at);
        out = format!("{head}{resources}{tail}");
    }

    writeln!(out).unwrap();
    writeln!(out, "</UserControl>").unwrap();
    Ok(out)
}

/// Lower one moslayout node and its descendants to XAML, indented by
/// `indent` spaces.
///
/// PR-1 added the nine simple kernel primitives; PR-2 adds `For`. `If`
/// and `Else` are NOT handled here — they're consumed by
/// [`emit_xaml_children`] which pairs an `If` with the following `Else`
/// sibling. A bare `If` or `Else` reaching this function is an error
/// (they should always come through `emit_xaml_children`).
fn emit_xaml_node(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    match node.tag.as_str() {
        "Box" => emit_box(node, indent, part_styles, ctx),
        "Row" => emit_stack_panel(node, indent, part_styles, "Horizontal", ctx),
        "Column" => emit_stack_panel(node, indent, part_styles, "Vertical", ctx),
        "Stack" => emit_stack(node, indent, part_styles, ctx),
        "Text" => emit_text(node, indent, part_styles, ctx),
        "Image" => emit_image(node, indent, part_styles, ctx),
        "Spacer" => emit_spacer(node, indent, part_styles),
        "Divider" => emit_divider(node, indent, part_styles),
        "Icon" => emit_icon(node, indent, part_styles, ctx),

        // PR-2: For lowering.
        "For" => emit_for(node, indent, part_styles, ctx),

        // `If` and `Else` are paired by the children iterator. Seeing a
        // bare one here means the author wrote `If` as the root of the
        // component (no preceding sibling) — emit it as a top-level
        // conditional. The look-ahead for `Else` happens in the
        // children iterator, so the standalone case here means no
        // `Else` was paired.
        "If" => emit_if(node, None, indent, part_styles, ctx),
        // A standalone `Else` (no preceding `If`) is a moslayout-level
        // validation error per UI29 §3.2; we treat it as
        // UnsupportedPrimitive here for the second line of defence in
        // case validation was bypassed.
        "Else" => Err(PipelineEmitError::UnsupportedPrimitive(
            "Else without preceding If".to_string(),
        )),

        // PR-3: Host* primitives (single-element host-native controls).
        "HostInput" => emit_host_input(node, indent, part_styles, ctx),
        "HostButton" => emit_host_button(node, indent, part_styles, ctx),
        "HostScroll" => emit_host_scroll(node, indent, part_styles, ctx),

        // PR-4 / PR-5 territory. Recognised by name so the error message is
        // self-documenting ("not yet supported", not "unknown tag").
        "HostTable"
        | "HostTableColGroup" | "HostTableHead" | "HostTableBody" | "HostTableFoot" => {
            Err(PipelineEmitError::UnsupportedPrimitive(node.tag.clone()))
        }

        // Anything else is a component reference; will route through the
        // manifest resolver in PR-5. PR-1 simply errors.
        other => Err(PipelineEmitError::UnsupportedPrimitive(other.to_string())),
    }
}

/// Walk a slice of children, emitting each in order. Pairs an `If` with
/// a following `Else` sibling (UI29 §3.2) — that pairing is the only
/// reason this exists rather than every container directly calling
/// `emit_xaml_node` per child.
fn emit_xaml_children(
    children: &[LayoutNode],
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if child.tag == "If" {
            // Look ahead one position for an `Else` sibling.
            let else_node = if let Some(next) = children.get(i + 1) {
                if next.tag == "Else" {
                    Some(next)
                } else {
                    None
                }
            } else {
                None
            };
            out.push_str(&emit_if(child, else_node, indent, part_styles, ctx)?);
            // Skip past the consumed `Else` if we paired one.
            i += if else_node.is_some() { 2 } else { 1 };
        } else if child.tag == "Else" {
            // Should have been consumed by the preceding `If`'s look-
            // ahead; reaching here means the author wrote a standalone
            // `Else` (moslayout validation failure).
            return Err(PipelineEmitError::UnsupportedPrimitive(
                "Else without preceding If".to_string(),
            ));
        } else {
            out.push_str(&emit_xaml_node(child, indent, part_styles, ctx)?);
            i += 1;
        }
    }
    Ok(out)
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
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    emit_container(node, indent, part_styles, "Border", ctx)
}

fn emit_stack_panel(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    orientation: &str,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let mut out = format!(
        "{pad}<StackPanel Orientation=\"{orientation}\"{style}>\n"
    );
    out.push_str(&emit_xaml_children(&node.children, indent + 4, part_styles, ctx)?);
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
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    emit_container(node, indent, part_styles, "Grid", ctx)
}

fn emit_container(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    element: &str,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let mut out = format!("{pad}<{element}{style}>\n");
    out.push_str(&emit_xaml_children(&node.children, indent + 4, part_styles, ctx)?);
    write!(out, "{pad}</{element}>\n").unwrap();
    Ok(out)
}

/// `Text [name] (content: slot: foo)` → `<TextBlock Text="{x:Bind Foo}"/>`.
/// `Text [name] (content: "literal")` → `<TextBlock Text="literal"/>`.
/// `Text [name] (content: row.value)` → `<TextBlock Text="{x:Bind Row.Value}"/>`
/// when `row` is a `For`-bound name (PR-2).
fn emit_text(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
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
            // A `content: <NAME>` form. PR-2: if `k` is a for-bound
            // name, treat as `{x:Bind ForName}`; otherwise as literal
            // text (matches the React backend's behaviour pre-PR-2).
            if ctx.lookup_for_binding(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                format!(" Text=\"{{x:Bind {pascal}}}\"")
            } else if ctx.lookup_for_index(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                format!(" Text=\"{{x:Bind {pascal}}}\"")
            } else {
                let escaped = escape_xaml_attr(k);
                format!(" Text=\"{escaped}\"")
            }
        }
        Some(LayoutPropValue::Number(n)) => format!(" Text=\"{n}\""),
        Some(LayoutPropValue::Expr(src)) => {
            // PR-2: route through ExprLowerer.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => format!(" Text=\"{{x:Bind {path}}}\""),
                ExprLowering::Helper(call) => format!(" Text=\"{{x:Bind {call}}}\""),
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
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
    _ctx: &mut EmitContext<'_>,
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
    _ctx: &mut EmitContext<'_>,
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
/// Dispatch event, constructor boilerplate, and any helper methods the
/// expression lowerer registered during the XAML walk (PR-2).
fn emit_code_behind(
    name: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    options: &EmitOptions,
    ctx: &EmitContext<'_>,
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
        writeln!(out).unwrap();
    }

    // PR-2: helper methods registered by the ExprLowerer for expressions
    // that {x:Bind} cannot evaluate directly (indexer / comparison /
    // logical). Each helper is a private method on the partial class.
    for helper in &ctx.helpers {
        let params: Vec<String> = helper
            .parameters
            .iter()
            .map(|(n, t)| format!("{t} {n}"))
            .collect();
        writeln!(
            out,
            "    private {} {}({}) => {};",
            helper.return_type,
            helper.name,
            params.join(", "),
            helper.body
        )
        .unwrap();
    }

    // PR-3: event-handler methods registered by Host* primitives. These
    // are multi-line method bodies (full signature + braces + body) so
    // we write them verbatim with a leading blank line for readability.
    for h in &ctx.host_handlers {
        writeln!(out).unwrap();
        writeln!(out, "{}", h.source).unwrap();
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
// PR-2: For / If / Else lowering + ExprLowerer
// =====================================================================

/// `For (each: <expr>, as: <name>, index: <name>?) { <children> }` →
/// `<ItemsRepeater>` with a generated `<DataTemplate>` whose
/// `x:DataType` is the generated RowVm record.
fn emit_for(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    // -- 1. Extract and validate the For-required props --
    let as_name = find_prop_keyword(node, "as").ok_or_else(|| {
        PipelineEmitError::UnsupportedPrimitive(
            "For block missing required prop 'as:'".to_string(),
        )
    })?;
    let index_name = find_prop_keyword(node, "index");

    // -- 2. Resolve the `each:` source to a {x:Bind} path and an
    //    element type --
    let (items_path, element_type) = match find_prop_value(node, "each") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            if !is_safe_identifier(&pascal) {
                return Err(PipelineEmitError::UnsafeSlotName(pascal));
            }
            // Look up the slot's declared C# type to derive the element
            // type. Slots are typed as `IReadOnlyList<X>` → element is X.
            let csharp_type =
                ctx.slot_types.get(slot.as_str()).cloned().unwrap_or_else(|| "object".to_string());
            let elem_type = inner_type_of_list(&csharp_type);
            (pascal, elem_type)
        }
        Some(LayoutPropValue::Expr(expr_src)) => {
            // Could be a for-bound name's member access, e.g.
            // `row.cells`. Lower it through ExprLowerer.
            match lower_expr_for_xbind(expr_src, ctx) {
                ExprLowering::Bindable(path) => {
                    // Without further type info we can't determine the
                    // element type; default to `object` (the host's C#
                    // compiler will catch any real mismatch).
                    (path, "object".to_string())
                }
                ExprLowering::Helper(_) | ExprLowering::Unsupported(_) => {
                    return Err(PipelineEmitError::UnsupportedExpression(format!(
                        "For each: expression {expr_src:?} cannot be lowered to a binding path"
                    )));
                }
            }
        }
        _ => {
            return Err(PipelineEmitError::UnsupportedPrimitive(
                "For block must bind `each:` to a slot ref or expression".to_string(),
            ));
        }
    };

    // -- 3. Generate / register the RowVm --
    let vm_class = format!(
        "{}_{}Vm",
        ctx.component_name,
        kebab_to_pascal_case(as_name)
    );
    let element_property = kebab_to_pascal_case(as_name);
    let has_index = index_name.is_some();

    let vm = RowVm {
        class_name: vm_class.clone(),
        element_property: element_property.clone(),
        element_type: element_type.clone(),
        has_index,
    };
    if !ctx.row_vms.iter().any(|v| v.class_name == vm.class_name) {
        ctx.row_vms.push(vm);
    }

    // -- 4. Push the binding into scope, walk the body, pop. --
    ctx.for_scope.push(ForBinding {
        as_name: as_name.to_string(),
        index_name: index_name.map(String::from),
        element_type,
        vm_class: vm_class.clone(),
    });
    let body =
        emit_xaml_children(&node.children, indent + 12, part_styles, ctx)?;
    ctx.for_scope.pop();

    // -- 5. Assemble the XAML --
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let pad3 = " ".repeat(indent + 8);
    let style = part_style_attr(node, part_styles);
    let mut out = String::new();
    writeln!(out, "{pad}<ItemsRepeater ItemsSource=\"{{x:Bind {items_path}}}\"{style}>")
        .unwrap();
    writeln!(out, "{pad2}<ItemsRepeater.ItemTemplate>").unwrap();
    writeln!(
        out,
        "{pad3}<DataTemplate x:DataType=\"local:{vm_class}\">"
    )
    .unwrap();
    out.push_str(&body);
    writeln!(out, "{pad3}</DataTemplate>").unwrap();
    writeln!(out, "{pad2}</ItemsRepeater.ItemTemplate>").unwrap();
    writeln!(out, "{pad}</ItemsRepeater>").unwrap();
    Ok(out)
}

/// `If (when: <expr>) { <then> } [Else { <else> }]` → twin
/// `<ContentControl>`s whose `Visibility` is bound to the expression
/// and its negation.
fn emit_if(
    if_node: &LayoutNode,
    else_node: Option<&LayoutNode>,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    // -- 1. Lower the `when:` expression --
    let when_path = match find_prop_value(if_node, "when") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            if !is_safe_identifier(&pascal) {
                return Err(PipelineEmitError::UnsafeSlotName(pascal));
            }
            pascal
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => "True".to_string(),
        Some(LayoutPropValue::Keyword(k)) if k == "false" => "False".to_string(),
        Some(LayoutPropValue::Keyword(k)) => {
            // Treat bare keywords like for-bound names so authors can
            // write `If (when: editable) { ... }` when `editable` is
            // a `For`-bound name in scope.
            if ctx.lookup_for_binding(k).is_some() {
                kebab_to_pascal_case(k)
            } else {
                return Err(PipelineEmitError::UnsupportedExpression(format!(
                    "If when: bare name {k:?} is not a slot or for-bound name"
                )));
            }
        }
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => path,
            ExprLowering::Helper(call) => call,
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        _ => {
            return Err(PipelineEmitError::UnsupportedPrimitive(
                "If block missing required `when:` expression".to_string(),
            ));
        }
    };

    // -- 2. Flag the converter requirement (one per UserControl) --
    ctx.needs_bool_to_vis = true;

    // -- 3. Emit the then-branch wrapper. The lowered body lives
    //    inside a single `<ContentControl>` with bound Visibility. --
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let then_body = emit_xaml_children(&if_node.children, indent + 4, part_styles, ctx)?;

    let mut out = String::new();
    writeln!(
        out,
        "{pad}<ContentControl Visibility=\"{{x:Bind {when_path}, Converter={{StaticResource BoolToVisibilityConverter}}}}\">"
    )
    .unwrap();
    out.push_str(&then_body);
    writeln!(out, "{pad}</ContentControl>").unwrap();

    // -- 4. Emit the else-branch wrapper, when paired. Visibility is
    //    bound to the same expression with `ConverterParameter=invert`
    //    so the converter inverts the boolean. --
    if let Some(else_node) = else_node {
        let else_body =
            emit_xaml_children(&else_node.children, indent + 4, part_styles, ctx)?;
        writeln!(
            out,
            "{pad}<ContentControl Visibility=\"{{x:Bind {when_path}, Converter={{StaticResource BoolToVisibilityConverter}}, ConverterParameter=invert}}\">"
        )
        .unwrap();
        out.push_str(&else_body);
        writeln!(out, "{pad}</ContentControl>").unwrap();
        let _ = pad2; // silence unused if no Else
    } else {
        let _ = pad2;
    }
    Ok(out)
}

/// The `<UserControl.Resources>` block carrying the
/// `BoolToVisibilityConverter` resource. Added exactly once per
/// UserControl when any `If` is emitted.
fn emit_bool_to_vis_resource_block(indent: usize) -> String {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let mut out = String::new();
    writeln!(out, "{pad}<UserControl.Resources>").unwrap();
    writeln!(
        out,
        "{pad2}<local:BoolToVisibilityConverter x:Key=\"BoolToVisibilityConverter\"/>"
    )
    .unwrap();
    writeln!(out, "{pad}</UserControl.Resources>").unwrap();
    out
}

/// Generate the C# source for one RowVm record. Each `For` block
/// produces one of these, written into `XamlEmitResult::for_view_models`
/// as a separate `.cs` file the host project compiles alongside the
/// UserControl.
fn emit_row_vm_source(_component: &str, vm: &RowVm, options: &EmitOptions) -> String {
    let ns = &options.namespace;
    let element_type = &vm.element_type;
    let element_property = &vm.element_property;
    let class_name = &vm.class_name;
    let index_field = if vm.has_index {
        ", int Index"
    } else {
        ""
    };
    let mut out = String::new();
    writeln!(out, "// Auto-generated by mosaic-emit-xaml. Do not edit.").unwrap();
    writeln!(out, "namespace {ns};").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "/// <summary>DataTemplate context for a `For` block iterating one row.</summary>"
    )
    .unwrap();
    writeln!(
        out,
        "public sealed record {class_name}({element_type} {element_property}{index_field});"
    )
    .unwrap();
    out
}

/// Reduce a C# type like `IReadOnlyList<string>` to its inner element
/// type (`string`). For non-generic types or unmatchable strings, fall
/// back to `object`.
fn inner_type_of_list(t: &str) -> String {
    if let Some(open) = t.find('<') {
        if let Some(close) = t.rfind('>') {
            if close > open {
                return t[open + 1..close].to_string();
            }
        }
    }
    "object".to_string()
}

// ─────────────────────────────────────────────────────────────────────
// ExprLowerer — UI29 §3.3 expression source → {x:Bind} path or helper
// ─────────────────────────────────────────────────────────────────────
//
// The moslayout-compiler stores `Expr` as the source-text substring
// (tokens joined with spaces). It can be:
//
//   - bare name              `row`                                → Bindable("Row")
//   - bare slot ref          `slot: editable`                     → Bindable("Editable")
//   - boolean literal        `true` / `false`                     → Bindable("True"/"False")
//   - dotted access          `row.value` / `slot: theme.dark`     → Bindable("Row.Value" / "Theme.Dark")
//   - indexer                `row[c]` / `slot: rows[r][c]`        → Helper("GetXxx(...)")
//   - comparisons            `r == slot: edit-row`                → Helper("IsXxx(...)")
//   - logical &&/||/!        `a && b`                             → Helper("Combined(...)")
//
// The PR-2 lowerer supports the first four directly and the last three
// via generated helpers. Anything else returns `Unsupported` with a
// human-readable reason.

/// Result of lowering one moslayout `Expr` to its WinUI 3 binding form.
#[derive(Debug)]
enum ExprLowering {
    /// Direct `{x:Bind X}` path (the inner part of the markup
    /// extension, without the `{x:Bind ...}` wrapper).
    Bindable(String),
    /// A helper-method call expression in C# form — `{x:Bind GetCell(R, C)}`
    /// is the consumer; the helper itself has been registered with the
    /// EmitContext.
    Helper(String),
    /// The expression couldn't be lowered. Carries a human-readable
    /// reason for the diagnostic.
    Unsupported(String),
}

/// Lower a raw expression source string to its WinUI 3 binding form.
///
/// This is a small recursive-descent parser over the UI29 §3.3 grammar
/// (or-expr → and-expr → eq-expr → rel-expr → unary → postfix → primary).
/// We do NOT pull in a separate parser dependency; the grammar is tiny
/// and the source has already been validated by moslayout-compiler. We
/// re-tokenise here only to figure out which branch of the lowering
/// table we're in.
fn lower_expr_for_xbind(src: &str, ctx: &mut EmitContext<'_>) -> ExprLowering {
    let trimmed = src.trim();
    let tokens = match tokenise_expr(trimmed) {
        Ok(t) => t,
        Err(e) => return ExprLowering::Unsupported(e),
    };

    // Walk the token stream once with a recursive-descent parser whose
    // output is the lowered form. The parser is split into helper
    // functions; see below.
    let mut p = ExprParser::new(&tokens, ctx, src);
    match p.parse_or() {
        Ok(lowering) => {
            if p.is_done() {
                lowering
            } else {
                ExprLowering::Unsupported(format!(
                    "expression {src:?} has trailing tokens"
                ))
            }
        }
        Err(e) => ExprLowering::Unsupported(e),
    }
}

/// Tokens emitted by the tiny expression lexer.
#[derive(Debug, Clone, PartialEq)]
enum ExprTok {
    Name(String),
    SlotPrefix,  // `slot:` (yes the colon is part of the prefix as seen by the lexer)
    Number(String),
    String(String),
    True,
    False,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Not,
    Dot,
    LBracket,
    RBracket,
    LParen,
    RParen,
}

/// Tokenise an expression source string. The grammar is small enough
/// that we don't need a generated lexer — a hand-rolled one fits in a
/// few dozen lines.
fn tokenise_expr(src: &str) -> Result<Vec<ExprTok>, String> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'=' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(ExprTok::EqEq);
                i += 2;
            }
            b'!' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(ExprTok::NotEq);
                i += 2;
            }
            b'<' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(ExprTok::Le);
                i += 2;
            }
            b'>' if i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                out.push(ExprTok::Ge);
                i += 2;
            }
            b'<' => {
                out.push(ExprTok::Lt);
                i += 1;
            }
            b'>' => {
                out.push(ExprTok::Gt);
                i += 1;
            }
            b'&' if i + 1 < bytes.len() && bytes[i + 1] == b'&' => {
                out.push(ExprTok::AndAnd);
                i += 2;
            }
            b'|' if i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                out.push(ExprTok::OrOr);
                i += 2;
            }
            b'!' => {
                out.push(ExprTok::Not);
                i += 1;
            }
            b'.' => {
                out.push(ExprTok::Dot);
                i += 1;
            }
            b'[' => {
                out.push(ExprTok::LBracket);
                i += 1;
            }
            b']' => {
                out.push(ExprTok::RBracket);
                i += 1;
            }
            b'(' => {
                out.push(ExprTok::LParen);
                i += 1;
            }
            b')' => {
                out.push(ExprTok::RParen);
                i += 1;
            }
            b'"' => {
                // String literal — collect until the closing quote,
                // honouring `\"` and `\\` escapes.
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return Err(format!(
                        "unterminated string literal in expression {src:?}"
                    ));
                }
                let lit = src[start..i].to_string();
                out.push(ExprTok::String(lit));
                i += 1; // skip closing "
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_digit() || bytes[i] == b'.')
                {
                    i += 1;
                }
                out.push(ExprTok::Number(src[start..i].to_string()));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric()
                        || bytes[i] == b'_'
                        || bytes[i] == b'-')
                {
                    i += 1;
                }
                let word = &src[start..i];
                // `slot:` is two-token at the lexer level (`slot` + `:`)
                // but we collapse it to a single `SlotPrefix` token so
                // the parser doesn't have to special-case the keyword.
                if word == "slot" && i < bytes.len() && bytes[i] == b':' {
                    out.push(ExprTok::SlotPrefix);
                    i += 1; // consume the colon
                } else if word == "true" {
                    out.push(ExprTok::True);
                } else if word == "false" {
                    out.push(ExprTok::False);
                } else {
                    out.push(ExprTok::Name(word.to_string()));
                }
            }
            other => {
                return Err(format!(
                    "unexpected character {:?} in expression {:?}",
                    other as char, src
                ));
            }
        }
    }
    Ok(out)
}

/// The recursive-descent parser walking the lexed expression.
///
/// Lifetime: parser holds shared borrows of tokens + ctx for the
/// duration of one expression lowering. The result string is owned.
struct ExprParser<'a, 'b> {
    tokens: &'a [ExprTok],
    pos: usize,
    ctx: &'a mut EmitContext<'b>,
    /// The original source string — used in error messages and helper
    /// name hashing.
    src: &'a str,
}

impl<'a, 'b> ExprParser<'a, 'b> {
    fn new(tokens: &'a [ExprTok], ctx: &'a mut EmitContext<'b>, src: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            ctx,
            src,
        }
    }

    fn peek(&self) -> Option<&ExprTok> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self, expected: &ExprTok) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn is_done(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Parse the entire expression — only the recursive-descent entry
    /// point a caller invokes. Returns the lowered form for the whole
    /// expression.
    fn parse_or(&mut self) -> Result<ExprLowering, String> {
        // For PR-2 the parser is simplified: we look for the *shape* of
        // the expression and decide on a lowering strategy in one pass.
        //
        // - If every token after the first primary is `.NAME`, we have
        //   a pure member-access path → Bindable.
        // - If there's an indexer / comparison / logical / unary-not,
        //   we register a helper method and return Helper(call).
        // - The fallback is Unsupported with a clear reason.

        if self.contains_logical_or_comparison() || self.contains_indexer() || self.starts_with_not() {
            // Register a helper that evaluates the whole expression.
            let helper = self.build_predicate_helper()?;
            let call = helper_call_expression(&helper);
            self.ctx.add_helper(helper);
            // Consume all tokens (we've handled the whole expression).
            self.pos = self.tokens.len();
            return Ok(ExprLowering::Helper(call));
        }

        // Otherwise we expect a Bindable path: primary (. NAME)*
        let mut path = self.parse_primary_bindable()?;
        while self.consume(&ExprTok::Dot) {
            match self.peek().cloned() {
                Some(ExprTok::Name(n)) => {
                    path.push('.');
                    path.push_str(&kebab_to_pascal_case(&n));
                    self.pos += 1;
                }
                _ => {
                    return Err(format!(
                        "expression {:?} has '.' not followed by a name",
                        self.src
                    ));
                }
            }
        }

        Ok(ExprLowering::Bindable(path))
    }

    fn parse_primary_bindable(&mut self) -> Result<String, String> {
        let tok = self
            .peek()
            .cloned()
            .ok_or_else(|| format!("expression {:?} is empty", self.src))?;
        match tok {
            ExprTok::SlotPrefix => {
                self.pos += 1;
                let name = self.expect_name()?;
                Ok(kebab_to_pascal_case(&name))
            }
            ExprTok::Name(n) => {
                self.pos += 1;
                Ok(kebab_to_pascal_case(&n))
            }
            ExprTok::True => {
                self.pos += 1;
                Ok("True".to_string())
            }
            ExprTok::False => {
                self.pos += 1;
                Ok("False".to_string())
            }
            other => Err(format!(
                "expression {:?} has unsupported primary token {other:?}",
                self.src
            )),
        }
    }

    fn expect_name(&mut self) -> Result<String, String> {
        match self.peek().cloned() {
            Some(ExprTok::Name(n)) => {
                self.pos += 1;
                Ok(n)
            }
            other => Err(format!(
                "expression {:?} expected a name, got {other:?}",
                self.src
            )),
        }
    }

    fn contains_logical_or_comparison(&self) -> bool {
        self.tokens.iter().any(|t| {
            matches!(
                t,
                ExprTok::EqEq
                    | ExprTok::NotEq
                    | ExprTok::Lt
                    | ExprTok::Le
                    | ExprTok::Gt
                    | ExprTok::Ge
                    | ExprTok::AndAnd
                    | ExprTok::OrOr
            )
        })
    }

    fn contains_indexer(&self) -> bool {
        self.tokens.iter().any(|t| t == &ExprTok::LBracket)
    }

    fn starts_with_not(&self) -> bool {
        matches!(self.tokens.first(), Some(ExprTok::Not))
    }

    /// Build a helper method whose body evaluates the whole expression.
    /// PR-2's strategy: hash the source to produce a deterministic name,
    /// scan the expression for referenced bindings to assemble the
    /// parameter list, then transliterate the expression into C# syntax.
    fn build_predicate_helper(&self) -> Result<HelperMethod, String> {
        // Collect parameters: any `slot: X` becomes a `this.X` reference
        // (no parameter needed); any bare `X` that matches a for-bound
        // name becomes a parameter (since helpers are invoked from
        // inside a DataTemplate where the for-bound names ARE the
        // parameters); any bare `X` matching a for index also becomes a
        // parameter.
        let mut params: Vec<(String, String)> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for tok in self.tokens {
            if let ExprTok::Name(n) = tok {
                if let Some(b) = self.ctx.lookup_for_binding(n) {
                    let pname = kebab_to_pascal_case(n);
                    if seen.insert(pname.clone()) {
                        params.push((pname, b.element_type.clone()));
                    }
                } else if let Some(b) = self.ctx.lookup_for_index(n) {
                    let _ = b;
                    let pname = kebab_to_pascal_case(n);
                    if seen.insert(pname.clone()) {
                        params.push((pname, "int".to_string()));
                    }
                }
                // Otherwise: a name with no binding — leave it for
                // transliteration to surface as `this.<name>` if it's a
                // slot, or as a literal if it's something else.
            }
        }

        // Determine the return type. PR-2 supports two shapes:
        //   - logical / comparison → bool
        //   - indexer (X[idx])     → string (default; downstream type
        //                            inference is out of scope for PR-2)
        let return_type = if self.contains_logical_or_comparison() || self.starts_with_not() {
            "bool".to_string()
        } else {
            "string".to_string()
        };

        // Transliterate the source. Simple substitutions are enough
        // because the moslayout expression grammar is a subset of C#
        // operators (==/!=/<=/>=/<>/&&/||/!) and member/indexer access.
        let body = transliterate_to_csharp(self.tokens, self.ctx);

        let name = format!("Expr_{:x}", hash_expr(self.src));
        Ok(HelperMethod {
            name,
            parameters: params,
            return_type,
            body,
        })
    }
}

/// Form the C# call expression `Method(P1, P2)` for a helper.
fn helper_call_expression(helper: &HelperMethod) -> String {
    if helper.parameters.is_empty() {
        format!("{}()", helper.name)
    } else {
        let args: Vec<String> = helper.parameters.iter().map(|(n, _)| n.clone()).collect();
        format!("{}({})", helper.name, args.join(", "))
    }
}

/// Walk the lexed tokens and emit equivalent C# source. Names get
/// PascalCased, slot refs become `this.Foo`, for-bound names stay as
/// PascalCased (they're the helper's parameters), `[` / `]` /
/// `&&` / `||` / `==` / `!=` etc. pass through identically since C#
/// uses the same syntax.
fn transliterate_to_csharp(tokens: &[ExprTok], ctx: &EmitContext<'_>) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            ExprTok::SlotPrefix => {
                i += 1;
                if let Some(ExprTok::Name(n)) = tokens.get(i) {
                    out.push_str("this.");
                    out.push_str(&kebab_to_pascal_case(n));
                    i += 1;
                }
            }
            ExprTok::Name(n) => {
                let pascal = kebab_to_pascal_case(n);
                // If it's a for-bound name or index, leave it bare —
                // the helper's parameter has this exact PascalCased name.
                if ctx.lookup_for_binding(n).is_some() || ctx.lookup_for_index(n).is_some() {
                    out.push_str(&pascal);
                } else {
                    // Otherwise treat as a slot reference.
                    out.push_str("this.");
                    out.push_str(&pascal);
                }
                i += 1;
            }
            ExprTok::Number(n) => {
                out.push_str(n);
                i += 1;
            }
            ExprTok::String(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
                i += 1;
            }
            ExprTok::True => {
                out.push_str("true");
                i += 1;
            }
            ExprTok::False => {
                out.push_str("false");
                i += 1;
            }
            ExprTok::EqEq => {
                out.push_str(" == ");
                i += 1;
            }
            ExprTok::NotEq => {
                out.push_str(" != ");
                i += 1;
            }
            ExprTok::Lt => {
                out.push_str(" < ");
                i += 1;
            }
            ExprTok::Le => {
                out.push_str(" <= ");
                i += 1;
            }
            ExprTok::Gt => {
                out.push_str(" > ");
                i += 1;
            }
            ExprTok::Ge => {
                out.push_str(" >= ");
                i += 1;
            }
            ExprTok::AndAnd => {
                out.push_str(" && ");
                i += 1;
            }
            ExprTok::OrOr => {
                out.push_str(" || ");
                i += 1;
            }
            ExprTok::Not => {
                out.push('!');
                i += 1;
            }
            ExprTok::Dot => {
                out.push('.');
                i += 1;
            }
            ExprTok::LBracket => {
                out.push('[');
                i += 1;
            }
            ExprTok::RBracket => {
                out.push(']');
                i += 1;
            }
            ExprTok::LParen => {
                out.push('(');
                i += 1;
            }
            ExprTok::RParen => {
                out.push(')');
                i += 1;
            }
        }
    }
    out
}

/// Simple deterministic hash of an expression source for naming
/// helpers. We use a 32-bit FNV-1a hash because it's tiny and the
/// collision probability across a handful of expressions in one
/// component is negligible.
fn hash_expr(s: &str) -> u32 {
    let mut h: u32 = 0x811C9DC5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

/// Find a prop whose value is a `Keyword`, returning the keyword text.
/// Used by `For`'s `as:` / `index:` and `If`'s bare-keyword `when:`.
fn find_prop_keyword<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a str> {
    node.props.iter().find_map(|p| {
        if p.name == prop_name {
            if let LayoutPropValue::Keyword(s) = &p.value {
                return Some(s.as_str());
            }
        }
        None
    })
}

// =====================================================================
// PR-3: HostInput / HostButton / HostScroll
// =====================================================================
//
// These three primitives lower to native WinUI 3 controls:
//
// - HostInput  → <TextBox>     (spec §4.1)
// - HostButton → <Button>      (spec §4.2)
// - HostScroll → <ScrollViewer> (spec §4.3)
//
// Wiring an emit (e.g. `onChange: emit: onFormulaChange`) requires a
// code-behind handler method. We register them on EmitContext and emit
// them as private methods on the partial class alongside the helper
// methods PR-2 introduced.

/// Pick a deterministic XAML `x:Name` for a Host* primitive. Uses the
/// node's `part_name` PascalCased when present (idiomatic per the
/// spec's examples like `FormulaField`); otherwise allocates a
/// per-component counter so the `x:Name` is stable across rebuilds.
fn host_x_name(node: &LayoutNode, tag: &str, ctx: &mut EmitContext<'_>) -> String {
    if let Some(p) = node.part_name.as_deref() {
        let pascal = kebab_to_pascal_case(p);
        if is_safe_identifier(&pascal) {
            return pascal;
        }
    }
    let n = ctx.next_host_counter();
    format!("{tag}_{n}")
}

/// `HostInput` → `<TextBox>` per spec §4.1.
fn emit_host_input(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostInput", ctx);

    // -- Build the attribute set --
    let mut attrs = String::new();

    // value: slot/string/expr → Text binding
    match find_prop_value(node, "value") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(
                " Text=\"{{x:Bind {pascal}, Mode=TwoWay}}\""
            ));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Text=\"{}\"", escape_xaml_attr(s)));
        }
        _ => {}
    }

    // read-only: slot/keyword
    match find_prop_value(node, "read-only") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" IsReadOnly=\"{{x:Bind {pascal}}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsReadOnly=\"True\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsReadOnly=\"False\"");
        }
        _ => {}
    }

    // placeholder: literal
    if let Some(LayoutPropValue::String(s)) = find_prop_value(node, "placeholder") {
        attrs.push_str(&format!(
            " PlaceholderText=\"{}\"",
            escape_xaml_attr(s)
        ));
    }

    // max-length: number
    if let Some(LayoutPropValue::Number(n)) = find_prop_value(node, "max-length") {
        let i = *n as i64;
        attrs.push_str(&format!(" MaxLength=\"{i}\""));
    }

    // multiline: true → AcceptsReturn + TextWrapping
    if find_prop_keyword(node, "multiline") == Some("true") {
        attrs.push_str(" AcceptsReturn=\"True\" TextWrapping=\"Wrap\"");
    }

    // -- Event wiring --
    // onChange handler dispatches with the new text payload.
    if let Some(LayoutPropValue::EmitRef(emit_name)) =
        find_prop_value(node, "onChange")
    {
        let handler = format!("{x_name}_TextChanged");
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.Controls.TextChangedEventArgs e)\n    {{\n        if (sender is Microsoft.UI.Xaml.Controls.TextBox tb)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case_pascal}(tb.Text));\n        }}\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" TextChanged=\"{handler}\""));
    }

    // onCommit / onCancel → merged KeyDown handler keyed on Enter / Escape.
    let commit = match find_prop_value(node, "onCommit") {
        Some(LayoutPropValue::EmitRef(e)) => Some(e.clone()),
        _ => None,
    };
    let cancel = match find_prop_value(node, "onCancel") {
        Some(LayoutPropValue::EmitRef(e)) => Some(e.clone()),
        _ => None,
    };
    if commit.is_some() || cancel.is_some() {
        let handler = format!("{x_name}_KeyDown");
        let mut body = String::new();
        body.push_str(&format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.Input.KeyRoutedEventArgs e)\n    {{\n"
        ));
        let component = ctx.component_name;
        if let Some(emit) = &commit {
            let case = kebab_to_pascal_case(&strip_on_prefix(emit));
            body.push_str(&format!(
                "        if (e.Key == Windows.System.VirtualKey.Enter)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case}());\n        }}\n"
            ));
        }
        if let Some(emit) = &cancel {
            let case = kebab_to_pascal_case(&strip_on_prefix(emit));
            body.push_str(&format!(
                "        if (e.Key == Windows.System.VirtualKey.Escape)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case}());\n        }}\n"
            ));
        }
        body.push_str("    }");
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" KeyDown=\"{handler}\""));
    }

    // onFocus → GotFocus
    if let Some(LayoutPropValue::EmitRef(emit_name)) =
        find_prop_value(node, "onFocus")
    {
        let handler = format!("{x_name}_GotFocus");
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}());\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" GotFocus=\"{handler}\""));
    }

    Ok(format!(
        "{pad}<TextBox x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

/// `HostButton` → `<Button>` per spec §4.2.
fn emit_host_button(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostButton", ctx);

    let mut attrs = String::new();

    // label: slot/string
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
        }
        _ => {}
    }

    // disabled: slot/keyword. Polarity flip handled via a generated
    // `Not(bool)` helper on the partial class.
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            // Register the shared Not(bool) helper.
            ctx.add_helper(HelperMethod {
                name: "Not".to_string(),
                parameters: vec![("b".to_string(), "bool".to_string())],
                return_type: "bool".to_string(),
                body: "!b".to_string(),
            });
            attrs.push_str(&format!(
                " IsEnabled=\"{{x:Bind Not({pascal})}}\""
            ));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    // onClick → Click handler
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onClick") {
        let handler = format!("{x_name}_Click");
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}());\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" Click=\"{handler}\""));
    }

    Ok(format!(
        "{pad}<Button x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

/// `HostScroll` → `<ScrollViewer>` per spec §4.3.
fn emit_host_scroll(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    // direction: vertical (default) / horizontal / both
    let (v_vis, h_vis) = match find_prop_keyword(node, "direction") {
        Some("horizontal") => ("Disabled", "Auto"),
        Some("both") => ("Auto", "Auto"),
        _ => ("Auto", "Disabled"), // default: vertical
    };

    let mut out = format!(
        "{pad}<ScrollViewer VerticalScrollBarVisibility=\"{v_vis}\" HorizontalScrollBarVisibility=\"{h_vis}\"{style}>\n"
    );
    out.push_str(&emit_xaml_children(&node.children, indent + 4, part_styles, ctx)?);
    write!(out, "{pad}</ScrollViewer>\n").unwrap();
    Ok(out)
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

    /// PR-2 enables `Expr` handling. An expression containing an
    /// indexer (`row[c]`) now lowers to a helper-method `{x:Bind ...}`
    /// rather than erroring.
    ///
    /// The PR-1 version of this test expected an error; PR-2 replaces
    /// that expectation with the new lowering path.
    #[test]
    fn text_with_expr_content_lowers_to_helper_call_in_pr2() {
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
        let r = compile(&c, &l, &empty_style("Foo"));
        // The Text binding should reference a helper call.
        assert!(
            r.xaml.contains("Text=\"{x:Bind Expr_"),
            "expected helper-call binding, got:\n{}",
            r.xaml
        );
        // The helper should be inlined into the code-behind.
        assert!(
            r.code_behind.contains("private string Expr_"),
            "expected helper method in code-behind, got:\n{}",
            r.code_behind
        );
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

    /// PR-3 lowers HostInput. The PR-1 version of this test expected
    /// an `UnsupportedPrimitive` error; we now verify the actual
    /// `<TextBox>` emission instead.
    #[test]
    fn host_input_lowers_to_textbox_in_pr3() {
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
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<TextBox x:Name=\"HostInput_1\""), "got:\n{}", r.xaml);
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

    // ─────────────────────────────────────────────────────────────────
    // PR-3: HostInput / HostButton / HostScroll tests
    // ─────────────────────────────────────────────────────────────────

    fn host_input_node(part: Option<&str>, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: "HostInput".to_string(),
            part_name: part.map(String::from),
            props,
            children: Vec::new(),
        }
    }

    fn host_button_node(part: Option<&str>, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: "HostButton".to_string(),
            part_name: part.map(String::from),
            props,
            children: Vec::new(),
        }
    }

    fn host_scroll_node(direction: Option<&str>, children: Vec<LayoutNode>) -> LayoutNode {
        let props = match direction {
            Some(d) => vec![LayoutProp {
                name: "direction".to_string(),
                value: LayoutPropValue::Keyword(d.to_string()),
            }],
            None => Vec::new(),
        };
        LayoutNode {
            tag: "HostScroll".to_string(),
            part_name: None,
            props,
            children,
        }
    }

    // ── HostInput ──

    #[test]
    fn host_input_with_part_name_uses_pascal_case_as_xname() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(Some("formula-field"), Vec::new()),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<TextBox x:Name=\"FormulaField\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_with_value_slot_emits_twoway_text_binding() {
        let c = component("Foo", vec![slot("formula", SlotType::Text, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(
                None,
                vec![LayoutProp {
                    name: "value".to_string(),
                    value: LayoutPropValue::SlotRef("formula".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Text=\"{x:Bind Formula, Mode=TwoWay}\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_with_placeholder_string_emits_placeholdertext() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(
                None,
                vec![LayoutProp {
                    name: "placeholder".to_string(),
                    value: LayoutPropValue::String("Enter formula".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("PlaceholderText=\"Enter formula\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_with_max_length_emits_int_attribute() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(
                None,
                vec![LayoutProp {
                    name: "max-length".to_string(),
                    value: LayoutPropValue::Number(100.0),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("MaxLength=\"100\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_with_readonly_true_emits_literal_attribute() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(
                None,
                vec![LayoutProp {
                    name: "read-only".to_string(),
                    value: LayoutPropValue::Keyword("true".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("IsReadOnly=\"True\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_with_multiline_keyword_emits_accepts_return_and_wrap() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_input_node(
                None,
                vec![LayoutProp {
                    name: "multiline".to_string(),
                    value: LayoutPropValue::Keyword("true".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("AcceptsReturn=\"True\" TextWrapping=\"Wrap\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_input_on_change_emits_handler_with_text_payload() {
        let c = component(
            "Foo",
            vec![],
            vec![emit(
                "onFormulaChange",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = layout_with_root(
            "Foo",
            host_input_node(
                Some("formula-field"),
                vec![LayoutProp {
                    name: "onChange".to_string(),
                    value: LayoutPropValue::EmitRef("onFormulaChange".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // XAML wires TextChanged to the per-element handler.
        assert!(
            r.xaml.contains("TextChanged=\"FormulaField_TextChanged\""),
            "got:\n{}",
            r.xaml
        );
        // Code-behind has the handler implementation.
        assert!(
            r.code_behind.contains("private void FormulaField_TextChanged"),
            "got:\n{}",
            r.code_behind
        );
        // Handler dispatches FormulaChange(tb.Text).
        assert!(r.code_behind.contains("FooEvent.FormulaChange(tb.Text)"));
    }

    #[test]
    fn host_input_commit_and_cancel_emit_merged_keydown_handler() {
        let c = component(
            "Foo",
            vec![],
            vec![
                emit("onCommit", vec![]),
                emit("onCancel", vec![]),
            ],
        );
        let l = layout_with_root(
            "Foo",
            host_input_node(
                Some("formula-field"),
                vec![
                    LayoutProp {
                        name: "onCommit".to_string(),
                        value: LayoutPropValue::EmitRef("onCommit".to_string()),
                    },
                    LayoutProp {
                        name: "onCancel".to_string(),
                        value: LayoutPropValue::EmitRef("onCancel".to_string()),
                    },
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // One KeyDown handler that branches on Enter / Escape.
        assert!(
            r.xaml.contains("KeyDown=\"FormulaField_KeyDown\""),
            "got:\n{}",
            r.xaml
        );
        let cb = &r.code_behind;
        assert!(cb.contains("VirtualKey.Enter"));
        assert!(cb.contains("VirtualKey.Escape"));
        assert!(cb.contains("FooEvent.Commit()"));
        assert!(cb.contains("FooEvent.Cancel()"));
    }

    // ── HostButton ──

    #[test]
    fn host_button_lowers_to_button_with_xname() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_button_node(Some("submit"), Vec::new()),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<Button x:Name=\"Submit\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_button_with_label_slot_emits_content_binding() {
        let c = component(
            "Foo",
            vec![slot("button-label", SlotType::Text, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            host_button_node(
                None,
                vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::SlotRef("button-label".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Content=\"{x:Bind ButtonLabel}\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_button_with_label_string_emits_literal_content() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_button_node(
                None,
                vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::String("Submit".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Content=\"Submit\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_button_disabled_slot_uses_not_helper() {
        let c = component(
            "Foo",
            vec![slot("is-busy", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            host_button_node(
                None,
                vec![LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::SlotRef("is-busy".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("IsEnabled=\"{x:Bind Not(IsBusy)}\""),
            "got:\n{}",
            r.xaml
        );
        // The Not(bool) helper should be in the code-behind.
        assert!(
            r.code_behind.contains("private bool Not(bool b) => !b;"),
            "got:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn host_button_disabled_literal_true_flips_to_isenabled_false() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_button_node(
                None,
                vec![LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::Keyword("true".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("IsEnabled=\"False\""));
    }

    #[test]
    fn host_button_on_click_emits_dispatch_handler() {
        let c = component(
            "Foo",
            vec![],
            vec![emit("onSubmit", vec![])],
        );
        let l = layout_with_root(
            "Foo",
            host_button_node(
                Some("submit"),
                vec![LayoutProp {
                    name: "onClick".to_string(),
                    value: LayoutPropValue::EmitRef("onSubmit".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Click=\"Submit_Click\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("private void Submit_Click"),
            "got:\n{}",
            r.code_behind
        );
        assert!(r.code_behind.contains("FooEvent.Submit()"));
    }

    // ── HostScroll ──

    #[test]
    fn host_scroll_default_direction_is_vertical() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_scroll_node(None, Vec::new()));
        let r = compile(&c, &l, &empty_style("Foo"));
        // V=Auto, H=Disabled is the vertical default.
        assert!(
            r.xaml
                .contains("VerticalScrollBarVisibility=\"Auto\" HorizontalScrollBarVisibility=\"Disabled\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_scroll_horizontal_swaps_visibilities() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_scroll_node(Some("horizontal"), Vec::new()),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("VerticalScrollBarVisibility=\"Disabled\" HorizontalScrollBarVisibility=\"Auto\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_scroll_both_directions_both_auto() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_scroll_node(Some("both"), Vec::new()),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("VerticalScrollBarVisibility=\"Auto\" HorizontalScrollBarVisibility=\"Auto\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_scroll_wraps_children_inside_scrollviewer() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_scroll_node(
                None,
                vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("inside".to_string()),
                    }],
                    children: Vec::new(),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // The Text should sit inside the ScrollViewer.
        let sv = r.xaml.find("<ScrollViewer").unwrap();
        let txt = r.xaml.find("<TextBlock").unwrap();
        let svc = r.xaml.find("</ScrollViewer>").unwrap();
        assert!(sv < txt && txt < svc, "got:\n{}", r.xaml);
    }

    // ── Multi-Host counter ──

    #[test]
    fn multiple_unnamed_host_inputs_get_distinct_counters() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    host_input_node(None, Vec::new()),
                    host_input_node(None, Vec::new()),
                    host_input_node(None, Vec::new()),
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("x:Name=\"HostInput_1\""));
        assert!(r.xaml.contains("x:Name=\"HostInput_2\""));
        assert!(r.xaml.contains("x:Name=\"HostInput_3\""));
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

    // ─────────────────────────────────────────────────────────────────
    // PR-2: For / If / Else / ExprLowerer tests
    // ─────────────────────────────────────────────────────────────────

    fn for_node(each: LayoutPropValue, as_name: &str, index: Option<&str>, children: Vec<LayoutNode>) -> LayoutNode {
        let mut props = vec![
            LayoutProp { name: "each".to_string(), value: each },
            LayoutProp {
                name: "as".to_string(),
                value: LayoutPropValue::Keyword(as_name.to_string()),
            },
        ];
        if let Some(idx) = index {
            props.push(LayoutProp {
                name: "index".to_string(),
                value: LayoutPropValue::Keyword(idx.to_string()),
            });
        }
        LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props,
            children,
        }
    }

    fn if_node(when: LayoutPropValue, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "If".to_string(),
            part_name: None,
            props: vec![LayoutProp { name: "when".to_string(), value: when }],
            children,
        }
    }

    fn else_node(children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "Else".to_string(),
            part_name: None,
            props: Vec::new(),
            children,
        }
    }

    // ── For lowering ──

    #[test]
    fn for_with_slot_ref_lowers_to_items_repeater_with_data_template() {
        let c = component(
            "Grid",
            vec![slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                None,
                vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::Keyword("row".to_string()),
                    }],
                    children: Vec::new(),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        // ItemsRepeater bound to the slot.
        assert!(
            r.xaml.contains("<ItemsRepeater ItemsSource=\"{x:Bind Rows}\""),
            "got:\n{}",
            r.xaml
        );
        // DataTemplate with the generated RowVm typed DataContext.
        assert!(
            r.xaml.contains("<DataTemplate x:DataType=\"local:Grid_RowVm\">"),
            "got:\n{}",
            r.xaml
        );
        // Inner Text binds to the for-bound name.
        assert!(
            r.xaml.contains("Text=\"{x:Bind Row}\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn for_generates_row_vm_record() {
        let c = component(
            "Grid",
            vec![slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                None,
                vec![],
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        // One entry in for_view_models.
        assert_eq!(r.for_view_models.len(), 1, "expected one RowVm");
        let vm = &r.for_view_models[0];
        assert_eq!(vm.filename, "Grid_RowVm.cs");
        assert!(vm.source.contains("public sealed record Grid_RowVm(string Row);"));
    }

    #[test]
    fn for_with_index_adds_index_field_to_row_vm() {
        let c = component(
            "Grid",
            vec![slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                Some("r"),
                vec![],
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        let vm = &r.for_view_models[0];
        assert!(
            vm.source.contains("public sealed record Grid_RowVm(string Row, int Index);"),
            "got:\n{}",
            vm.source
        );
    }

    #[test]
    fn for_with_numeric_list_uses_double_element_type() {
        let c = component(
            "Stats",
            vec![slot("values", SlotType::List(Box::new(ListInnerType::Number)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Stats",
            for_node(
                LayoutPropValue::SlotRef("values".to_string()),
                "v",
                None,
                vec![],
            ),
        );
        let r = compile(&c, &l, &empty_style("Stats"));
        assert!(
            r.for_view_models[0]
                .source
                .contains("public sealed record Stats_VVm(double V);"),
            "got:\n{}",
            r.for_view_models[0].source
        );
    }

    #[test]
    fn for_dedupes_row_vms_within_one_component() {
        // Two For blocks binding the same `as:` produce the same VM
        // class. The emitter must register only one — the assembly step
        // in from_pipeline depends on uniqueness.
        let c = component(
            "Grid",
            vec![slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    for_node(
                        LayoutPropValue::SlotRef("rows".to_string()),
                        "row",
                        None,
                        vec![],
                    ),
                    for_node(
                        LayoutPropValue::SlotRef("rows".to_string()),
                        "row",
                        None,
                        vec![],
                    ),
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        assert_eq!(r.for_view_models.len(), 1, "expected dedup to one RowVm");
    }

    #[test]
    fn standalone_else_errors_with_unsupported_primitive() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![else_node(vec![])],
            },
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedPrimitive(_)));
    }

    // ── If / Else lowering ──

    #[test]
    fn if_with_slot_ref_lowers_to_contentcontrol_with_visibility() {
        let c = component(
            "Foo",
            vec![slot("editable", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![if_node(
                    LayoutPropValue::SlotRef("editable".to_string()),
                    vec![LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("editable!".to_string()),
                        }],
                        children: Vec::new(),
                    }],
                )],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("<ContentControl Visibility=\"{x:Bind Editable, Converter={StaticResource BoolToVisibilityConverter}}\">"),
            "got:\n{}",
            r.xaml
        );
        // Then-branch content lives inside.
        assert!(r.xaml.contains("Text=\"editable!\""));
    }

    #[test]
    fn if_emits_bool_to_visibility_converter_resource_once() {
        let c = component(
            "Foo",
            vec![slot("editable", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            if_node(
                LayoutPropValue::SlotRef("editable".to_string()),
                vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("hi".to_string()),
                    }],
                    children: Vec::new(),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<UserControl.Resources>"),
            "expected resources block, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<local:BoolToVisibilityConverter x:Key=\"BoolToVisibilityConverter\"/>"
            )
        );
        // Only one occurrence — converter is shared.
        let count = r.xaml.matches("BoolToVisibilityConverter x:Key").count();
        assert_eq!(count, 1, "expected exactly one converter resource entry");
    }

    #[test]
    fn if_without_else_does_not_emit_else_wrapper() {
        let c = component(
            "Foo",
            vec![slot("editable", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            if_node(LayoutPropValue::SlotRef("editable".to_string()), vec![]),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // Only one ContentControl in the output.
        assert_eq!(r.xaml.matches("<ContentControl").count(), 1);
    }

    #[test]
    fn if_with_else_emits_paired_contentcontrols() {
        let c = component(
            "Foo",
            vec![slot("editable", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    if_node(LayoutPropValue::SlotRef("editable".to_string()), vec![]),
                    else_node(vec![]),
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // Two ContentControls.
        assert_eq!(r.xaml.matches("<ContentControl").count(), 2);
        // Else uses ConverterParameter=invert.
        assert!(
            r.xaml.contains("ConverterParameter=invert"),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn if_when_true_keyword_uses_true_constant() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            if_node(LayoutPropValue::Keyword("true".to_string()), vec![]),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Visibility=\"{x:Bind True, Converter="),
            "got:\n{}",
            r.xaml
        );
    }

    // ── ExprLowerer ──

    #[test]
    fn expr_lowerer_bare_slot_ref_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("slot: editable", &mut ctx);
        assert!(matches!(r, ExprLowering::Bindable(ref p) if p == "Editable"));
    }

    #[test]
    fn expr_lowerer_dotted_member_access_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("slot: theme.dark.bg", &mut ctx);
        assert!(
            matches!(r, ExprLowering::Bindable(ref p) if p == "Theme.Dark.Bg"),
            "got: bindable shape mismatch"
        );
    }

    #[test]
    fn expr_lowerer_boolean_literal_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("true", &mut ctx);
        assert!(matches!(r, ExprLowering::Bindable(ref p) if p == "True"));
    }

    #[test]
    fn expr_lowerer_indexer_becomes_helper_call() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("slot: rows[r]", &mut ctx);
        match r {
            ExprLowering::Helper(call) => {
                assert!(call.starts_with("Expr_"));
                assert_eq!(ctx.helpers.len(), 1);
            }
            other => panic!("expected Helper, got {other:?}"),
        }
    }

    #[test]
    fn expr_lowerer_equality_comparison_becomes_helper() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("slot: edit-row == 0", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers.len(), 1);
        assert_eq!(ctx.helpers[0].return_type, "bool");
    }

    #[test]
    fn expr_lowerer_logical_and_becomes_helper() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers[0].return_type, "bool");
        assert!(ctx.helpers[0].body.contains(" && "));
    }

    #[test]
    fn expr_lowerer_unary_not_becomes_helper() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let r = lower_expr_for_xbind("!slot: editable", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers[0].return_type, "bool");
    }

    #[test]
    fn expr_lowerer_identical_expressions_dedupe_to_one_helper() {
        let mut ctx = EmitContext::new("Foo", &[]);
        let _ = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        let _ = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        assert_eq!(ctx.helpers.len(), 1, "expected dedup");
    }

    #[test]
    fn expr_lowerer_for_bound_name_lowers_as_parameter() {
        let mut ctx = EmitContext::new("Foo", &[]);
        // Simulate a For binding being in scope.
        ctx.for_scope.push(ForBinding {
            as_name: "row".to_string(),
            index_name: Some("r".to_string()),
            element_type: "string".to_string(),
            vm_class: "Foo_RowVm".to_string(),
        });
        let r = lower_expr_for_xbind("row == \"hello\"", &mut ctx);
        match r {
            ExprLowering::Helper(_) => {
                assert_eq!(ctx.helpers.len(), 1);
                // Helper should accept a `string Row` parameter.
                let h = &ctx.helpers[0];
                assert!(
                    h.parameters
                        .iter()
                        .any(|(n, t)| n == "Row" && t == "string"),
                    "expected (Row, string) parameter, got: {:?}",
                    h.parameters
                );
            }
            other => panic!("expected Helper, got {other:?}"),
        }
    }

    // ── End-to-end: For + If together ──

    #[test]
    fn for_body_can_contain_if_with_for_bound_name_via_expr() {
        // For (each: rows, as: row) { If (when: row.editable) { Text(...) } Else { Text(...) } }
        let c = component(
            "Grid",
            vec![slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true)],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                None,
                vec![
                    if_node(
                        LayoutPropValue::Expr("row".to_string()),
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
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        // ItemsRepeater wraps the conditional.
        assert!(r.xaml.contains("<ItemsRepeater"));
        // Two ContentControls inside the DataTemplate.
        assert_eq!(r.xaml.matches("<ContentControl").count(), 2);
        // RowVm generated.
        assert_eq!(r.for_view_models.len(), 1);
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
