//! # Three-file pipeline entry point for the WinUI 3 / XAML backend.
//!
//! Mirrors the public function shape of `mosaic-emit-react`'s and
//! `mosaic-emit-swiftui`'s `pipeline` modules â€” same [`from_pipeline`]
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
//! level â€” the XAML compiler refuses files that don't match its expected
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
//!   `{Component}Event` record. This matches UI24 Â§3.1's React shape
//!   (`event GridEvent = ...`) exactly â€” host code subscribes via
//!   `grid.Dispatch += (s, e) => state.HandleEvent(e);`.
//! - **`Box` without padding/background lowers to `<ContentPresenter>`**
//!   instead of `<Border>`. A `<Border>` always paints (even with zero
//!   thickness and transparent brush) â€” `<ContentPresenter>` is the
//!   zero-cost option. The emitter picks the right one by inspecting the
//!   resolved mosstyle for the box's part name. PR-1 doesn't yet inline
//!   per-element styles, so today every `Box` lowers to `<Border>`
//!   defensively. A follow-up swaps in `<ContentPresenter>` when no
//!   `Background`/`BorderThickness`/`Padding` are set.

use std::fmt::Write as _;

use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotType,
};
use mosstyle_compiler::StyleDef;

// =====================================================================
// Public API
// =====================================================================

/// The result of compiling a three-file pipeline triple to a WinUI 3
/// UserControl.
///
/// Mirrors `mosaic_emit_react::pipeline::PipelineEmitResult` and
/// `mosaic_emit_swiftui::pipeline::PipelineEmitResult` so a generic CLI
/// driver can treat all three backends uniformly â€” except that XAML
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

    /// One entry per `For` block â€” the generated `RowVm` C# source.
    ///
    /// PR-1 always returns an empty `Vec`; `For` lowering lands with
    /// PR-2.
    pub for_view_models: Vec<EmittedFile>,

    /// One entry per `If` whose expression is not `{x:Bind}`-able â€” the
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

/// Project-shaped artifacts emitted when `EmitOptions::emit_project` is
/// on. With these in addition to the per-component triple, the output
/// directory is a buildable WinUI 3 project â€” `dotnet build` produces a
/// runnable .exe (modulo the well-documented bare-SDK MSBuild error,
/// see lessons.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFiles {
    /// `<Component>.csproj` â€” MSBuild project file. Targets net9.0-windows,
    /// references WindowsAppSDK + Microsoft.Windows.SDK.BuildTools,
    /// declares the project unpackaged + self-contained, includes a
    /// post-build target that flattens the native runtime DLLs (Fix B2).
    pub csproj: String,
    /// `App.xaml` â€” application resource dictionary (Fluent / Mica
    /// styles).
    pub app_xaml: String,
    /// `App.xaml.cs` â€” application code-behind: instantiates
    /// MainWindow on OnLaunched.
    pub app_xaml_cs: String,
    /// `MainWindow.xaml` â€” host window. Layout depends on the chosen
    /// `RootShape`:
    ///   - `UserControl` root: hosts the component directly in the
    ///     Grid (full-window placement).
    ///   - `ContentDialog` root: hosts a "Show dialog" button which
    ///     spawns the dialog on click, plus a status bar that echoes
    ///     dispatched events.
    pub main_window_xaml: String,
    /// `MainWindow.xaml.cs` â€” host code-behind. Wires the component's
    /// Dispatch event to a stub handler the user fills in with their
    /// business logic.
    pub main_window_cs: String,
    /// `app.manifest` â€” Win32 app manifest with DPI awareness +
    /// supported-OS GUID.
    pub package_manifest: String,
    /// `build.ps1` â€” driver script that runs `mosaic-compile` over
    /// each `.mil/.mll/.msl` triple, then `dotnet build`, then
    /// optionally launches the .exe. Fix B3.
    pub build_script: String,
    /// `README.md` for the emitted project â€” describes prerequisites
    /// (Windows App Runtime install), the build command, and the
    /// known MSBuild error from bare-SDK environments.
    pub readme: String,
}

/// Registry of components that the emitter is allowed to reference as
/// non-kernel tags (UI29 Â§4.4 component references).
///
/// The CLI builds this by walking the active manifest's
/// `[dependencies]`, parsing each one's `mosaic-package.toml`, and
/// registering every exported component name â†’ its
/// (xmlns_prefix, xmlns_value, package_name) tuple.
///
/// Tests build the registry inline with synthetic entries.
///
/// PR-5 lands the registry type and the lookup logic; the actual
/// file-walking from disk is the CLI's responsibility (mosaic-compile's
/// run_pipeline) and is wired up in the same PR series.
#[derive(Debug, Clone, Default)]
pub struct ComponentRegistry {
    /// component_name â†’ resolution info. Keyed by the PascalCase tag
    /// that appears in `.mll` source.
    entries: std::collections::HashMap<String, ComponentRef>,
}

/// One entry in the [`ComponentRegistry`] â€” the metadata needed to
/// emit a `<{prefix}:{Tag} ... />` XAML reference plus the
/// `xmlns:prefix="using:Namespace"` declaration on the `<UserControl>`
/// root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRef {
    /// The xmlns prefix that appears in the `<{prefix}:{Tag}/>`
    /// reference and in the matching `xmlns:{prefix}` declaration on
    /// the `<UserControl>` root. Conventionally derived from the
    /// package name (`mosaic-pkg-grid` â†’ `grid`).
    pub xmlns_prefix: String,
    /// The value of the xmlns declaration, e.g. `using:Mosaic.Package.Grid`.
    /// Derived from the package's C# namespace.
    pub xmlns_value: String,
    /// The package name (used in diagnostics).
    pub package_name: String,
}

impl ComponentRegistry {
    /// Construct an empty registry. The emitter treats this the same
    /// way it treats `None` â€” every non-kernel tag becomes
    /// [`PipelineEmitError::UnknownComponent`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one component reference.
    ///
    /// `tag` is the PascalCase identifier that appears in `.mll`
    /// source (matches the package's `[components].exports`).
    pub fn register(
        &mut self,
        tag: impl Into<String>,
        xmlns_prefix: impl Into<String>,
        xmlns_value: impl Into<String>,
        package_name: impl Into<String>,
    ) {
        let entry = ComponentRef {
            xmlns_prefix: xmlns_prefix.into(),
            xmlns_value: xmlns_value.into(),
            package_name: package_name.into(),
        };
        self.entries.insert(tag.into(), entry);
    }

    /// Look up a tag. Returns `None` when the tag isn't registered.
    pub fn lookup(&self, tag: &str) -> Option<&ComponentRef> {
        self.entries.get(tag)
    }

    /// Returns `true` when no entries are registered. Equivalent to
    /// passing `None` for the registry argument of `from_pipeline`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Options controlling the emitter's behaviour.
///
/// Default: produces only the component triple (`xaml`, `code_behind`,
/// `events`); no project artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Also emit `.csproj` + `App.xaml(.cs)` + `MainWindow.xaml(.cs)` +
    /// `Package.appxmanifest`. Default `false`. PR-1 ignores this flag â€”
    /// the project triple lands in PR-5.
    pub emit_project: bool,

    /// Top-level C# namespace for emitted types. Default
    /// `"Mosaic.Generated"`.
    pub namespace: String,

    /// Windows App SDK version to pin in the emitted `.csproj` (only used
    /// when `emit_project` is on). Default `"1.7.250606001"` â€” a known-
    /// good full version. A bare `"1.5"` or `"1.6"` doesn't pin enough
    /// for NuGet to resolve a build-able combination on every machine.
    pub windows_app_sdk: String,

    /// Lower `HostTable` to `controls:DataGrid` from the Community
    /// Toolkit rather than a hand-rolled `<Grid>`. PR-1 ignores this
    /// flag â€” `HostTable` is unsupported until PR-4.
    pub use_community_datagrid: bool,

    /// Treat the input as a UI29 userland package. PR-1 ignores this
    /// flag â€” `--package-mode` lands in PR-5.
    pub package_mode: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            emit_project: false,
            namespace: "Mosaic.Generated".to_string(),
            windows_app_sdk: "1.7.250606001".to_string(),
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
    /// disagree. The mosstyle name is allowed to differ per UI23 Â§4 (a
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

    /// An expression form (`row[c]`, `slot: a && slot: b`, â€¦) is not yet
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

    /// A mosmodel slot type has no WinUI 3 mapping (per spec Â§8). Only
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
/// (UI29 Â§4.4). PR-1 ignores it â€” every non-kernel tag is currently
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
    registry: Option<&ComponentRegistry>,
    options: &EmitOptions,
) -> Result<XamlEmitResult, PipelineEmitError> {
    // 1. The three IRs must agree on the component name. The style IR's
    //    `component_name` is allowed to differ when the style targets a
    //    specific layout variant (UI23 Â§4).
    if interface.component != layout.component_name {
        return Err(PipelineEmitError::ComponentNameMismatch {
            mosmodel: interface.component.clone(),
            moslayout: layout.component_name.clone(),
        });
    }

    let name = &interface.component;

    // 2. Build a part-name â†’ CSS-fragment map from the mosstyle source.
    //    Used by the style inliner inside each primitive emitter. PR-1's
    //    inliner only consumes base props; state blocks and the full
    //    UserControl.Resources cascade land in later PRs.
    let part_styles = build_part_style_map(style);

    // 3. Construct the emission context â€” threaded through the XAML
    //    walker so `For`/`If` can register helpers, RowVms, and the
    //    converter requirement (PR-2).
    let mut ctx = EmitContext::new(name, &interface.slots, &interface.emits);
    ctx.registry = registry;

    // 3a. Pick the XAML root shape (UserControl vs ContentDialog)
    //     based on the moslayout root. HostDialog-rooted layouts
    //     produce a ContentDialog root + partial class. Fix A1.
    let shape = pick_root_shape(&layout.root);
    populate_slot_aliases(&mut ctx, shape, &interface.slots);

    // 4. Emit each of the three files.
    let xaml = emit_xaml(name, &layout.root, &part_styles, options, &mut ctx)?;
    let code_behind = emit_code_behind(
        name,
        &interface.slots,
        &interface.emits,
        options,
        &ctx,
        shape,
    )?;
    let events = emit_events(name, &interface.emits, options)?;

    // 5. Assemble the result. RowVms become entries in `for_view_models`;
    //    the `if_helpers` field remains empty because the emitter inlines
    //    helper methods into the code-behind's partial class (one file
    //    per component is cleaner than scattering helper-bodies across
    //    siblings â€” the spec calls for separate files but PR-2 keeps
    //    them inline; see CHANGELOG for the deviation rationale).
    let for_view_models = ctx
        .row_vms
        .iter()
        .map(|vm| EmittedFile {
            filename: format!("{}.cs", vm.class_name),
            source: emit_row_vm_source(name, vm, options),
        })
        .collect();

    // Fix A5: when `If` or HostDialog-with-bound-open emitted a
    // `{StaticResource BoolToVisibilityConverter}` reference, ship
    // the converter's C# source as a side-file. Same lifecycle as
    // RowVms; lands in `if_helpers` (per the spec's original intent
    // for that field).
    let mut if_helpers: Vec<EmittedFile> = Vec::new();
    if ctx.needs_bool_to_vis {
        if_helpers.push(EmittedFile {
            filename: "BoolToVisibilityConverter.cs".to_string(),
            source: emit_bool_to_vis_converter_source(&options.namespace),
        });
    }

    // Fix B1: when --emit-project is on, populate the full project
    // shell (csproj + App + MainWindow + manifest + build.ps1 + README).
    // The CLI then writes them next to the component triple.
    let project = if options.emit_project {
        Some(build_project_files(
            name,
            &interface.slots,
            &interface.emits,
            shape,
            options,
        ))
    } else {
        None
    };

    Ok(XamlEmitResult {
        xaml,
        code_behind,
        events,
        component_name: name.clone(),
        project,
        for_view_models,
        if_helpers,
    })
}

// =====================================================================
// EmitContext â€” state threaded through the XAML walker (PR-2)
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
    /// `list<text>` â†’ `string`, `list<number>` â†’ `double`, etc.
    element_type: String,
    /// The generated RowVm class name: `{Component}_{AsName}Vm`.
    /// Stored on the binding even though the per-element binding code
    /// resolves the same value off `RowVm` â€” used by nested-For helper
    /// transliteration in a follow-up PR and by debug introspection.
    #[allow(dead_code)]
    vm_class: String,
    /// Code-behind property that projects the source list into row VMs,
    /// when the source is a component slot. Template predicates can use
    /// this to attach extra row-local state such as `IsSelected`.
    projection_property: Option<String>,
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
    /// C# return type â€” `bool` for predicates, `string` for indexed
    /// element accessors, `double` for numeric, etc.
    return_type: String,
    /// The C# expression body (no trailing semicolon).
    body: String,
}

/// A WinUI 3 event-handler method generated for a Host* primitive's
/// bound emits. Same lifecycle as `HelperMethod` â€” registered during
/// the walk, emitted inline into the code-behind partial class.
#[derive(Debug, Clone)]
struct HostHandler {
    /// Fully-qualified method name (also the XAML attribute value).
    name: String,
    /// Full C# source for the method, including signature and body.
    /// Multi-line and self-contained â€” emitted verbatim into the
    /// `partial class`.
    source: String,
}

/// A generated `RowVm` C# record â€” the typed `DataContext` for a
/// `<DataTemplate>` inside a `For` block.
#[derive(Debug, Clone)]
struct RowVm {
    /// `{Component}_{AsName}Vm` â€” must match the `x:DataType` reference
    /// in the matching `<DataTemplate>`.
    class_name: String,
    /// The PascalCase property name that holds the element value (e.g.
    /// `Row`, `Cell`). Derived from the `as:` binding.
    element_property: String,
    /// The C# type of the element value.
    element_type: String,
    /// `true` iff the matching `For` declared an `index:` binding.
    has_index: bool,
    /// GROUP C: `true` iff this VM is the per-column cell loop (a `For`
    /// whose `each:` is an enclosing For binding â€” UI29 Â§3.4). Such VMs
    /// carry an extra `double Width` field so the host can thread the
    /// matching column's fixed pixel width onto every cell, and the
    /// generated cell element binds `Width="{x:Bind Width}"`.
    has_width: bool,
    /// True when a template-local predicate such as `i == selectedIndex`
    /// is lowered into a row-local boolean instead of a page helper call.
    has_is_selected: bool,
}

/// A code-behind property that projects a slot list into generated row VMs.
#[derive(Debug, Clone)]
struct RowProjection {
    property_name: String,
    source_path: String,
    vm_class: String,
    has_index: bool,
    has_width: bool,
    selected_index_path: Option<String>,
}

/// Mutable state threaded through the recursive XAML emission.
///
/// PR-1's emit_xaml didn't need any of this â€” all primitive emitters
/// were stateless. PR-2's `For`/`If` lowering does, so we collect every
/// stateful effect into one struct that the assembly step in
/// `from_pipeline` consumes.
struct EmitContext<'a> {
    /// The component name â€” used to namespace generated types.
    component_name: &'a str,
    /// Slot name (kebab-case) â†’ C# type. For looking up the element
    /// type of a `For (each: slot: foo)` from `foo`'s declared type.
    slot_types: std::collections::HashMap<String, String>,
    emit_payloads: std::collections::HashMap<String, Vec<(String, String)>>,
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
    /// Code-behind row-VM projections used as ItemsSource values for
    /// slot-backed `For` templates.
    row_projections: Vec<RowProjection>,
    /// Host* event-handler method bodies registered during the walk.
    /// Each `HostInput` / `HostButton` with bound emits adds one or
    /// more entries; the assembly step emits them inline in the
    /// code-behind partial class.
    host_handlers: Vec<HostHandler>,
    /// Counter used to disambiguate Host* `x:Name`s when the node has
    /// no `part_name`. Incremented per emitted Host* primitive.
    host_counter: u32,
    /// Optional registry of resolvable component references. Populated
    /// by `from_pipeline`'s `registry` argument. PR-5 uses this to
    /// emit `<{prefix}:{Tag}/>` references for non-kernel tags.
    registry: Option<&'a ComponentRegistry>,
    /// xmlns declarations that need to land on the `<UserControl>`
    /// root after the walk completes. One entry per distinct package
    /// referenced. Keyed by xmlns prefix to dedupe.
    used_xmlns: std::collections::BTreeMap<String, String>,
    /// Slot names (kebab-case) whose PascalCased form collides with a
    /// property on the chosen base class (ContentDialog has `Title`,
    /// etc.). The DP generator renames these to `<BaseName>{Slot}`;
    /// the in-XAML `{x:Bind}` paths consult this map to use the alias.
    /// Fix A4.
    slot_aliases: std::collections::HashMap<String, String>,
    /// When the XAML root is a HostDialog hoisted to `<ContentDialog>`
    /// (Fix A1), this carries the dialog's attributes (Title, Closed
    /// handler, etc.) for `emit_xaml` to splice into the open tag.
    /// `None` for the standard UserControl root.
    root_extra_attrs: Option<String>,
}

impl<'a> EmitContext<'a> {
    fn new(name: &'a str, slots: &[SlotDecl], emits: &[EmitDecl]) -> Self {
        let mut slot_types = std::collections::HashMap::new();
        for slot in slots {
            let cs = slot_type_to_csharp(&slot.r#type).unwrap_or_else(|_| "object".to_string());
            slot_types.insert(slot.name.clone(), cs);
        }
        let mut emit_payloads = std::collections::HashMap::new();
        for emit in emits {
            let payloads = emit
                .params
                .iter()
                .map(|param| (param.name.clone(), emit_payload_to_csharp(&param.r#type)))
                .collect();
            emit_payloads.insert(emit.name.clone(), payloads);
        }
        Self {
            component_name: name,
            slot_types,
            emit_payloads,
            for_scope: Vec::new(),
            helpers: Vec::new(),
            needs_bool_to_vis: false,
            row_vms: Vec::new(),
            row_projections: Vec::new(),
            host_handlers: Vec::new(),
            host_counter: 0,
            registry: None,
            used_xmlns: std::collections::BTreeMap::new(),
            slot_aliases: std::collections::HashMap::new(),
            root_extra_attrs: None,
        }
    }

    /// PascalCased slot name (PR-1 default), unless the slot collides
    /// with a property on the chosen base class â€” in which case the
    /// alias from `slot_aliases` wins. `{x:Bind}` paths route through
    /// this so a slot named `title` on a ContentDialog-rooted
    /// component resolves to `DialogTitle`, not the shadowed
    /// `Title`. Fix A4.
    fn slot_xbind_path(&self, slot_name: &str) -> String {
        if let Some(alias) = self.slot_aliases.get(slot_name) {
            return alias.clone();
        }
        kebab_to_pascal_case(slot_name)
    }

    /// Allocate a unique counter for a Host* element that lacks a
    /// `part_name`. Always returns a fresh value.
    fn next_host_counter(&mut self) -> u32 {
        self.host_counter += 1;
        self.host_counter
    }

    /// Register an event-handler method. Same dedup pattern as
    /// helpers â€” two handlers with the same name share the same body.
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
    /// exists â€” assumed to be identical because helper names are a
    /// deterministic function of the expression they came from).
    fn add_helper(&mut self, helper: HelperMethod) {
        if !self.helpers.iter().any(|h| h.name == helper.name) {
            self.helpers.push(helper);
        }
    }
}

// =====================================================================
// Part-style map (mosstyle â†’ flat property fragments)
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
        // State blocks not yet wired â€” see CHANGELOG known-limitations.
    }
    out
}

fn build_style_fragment(props: &[mosstyle_compiler::StyleProp]) -> String {
    let mut parts: Vec<(String, String)> = Vec::with_capacity(props.len());
    for p in props {
        if let Some((key, value)) = css_side_spacing_to_xaml_attr(&p.name, &p.value) {
            upsert_style_attr(&mut parts, key, value);
            continue;
        }

        // X5: `css_property_to_xaml_setter` now returns `None` for
        // CSS-only properties with no WinUI analog (`border-collapse`,
        // `border-style`, `outline`, `text-decoration`, `box-shadow`).
        // Dropping them here means they never reach the XAML markup
        // compiler as invalid attributes / `<Setter>`s.
        let key = match css_property_to_xaml_setter(&p.name) {
            Some(k) => k,
            None => continue,
        };
        // X5: translate the *value* into the form the WinUI 3 markup
        // compiler accepts. `translate_xaml_value` may return `None`
        // when the whole property must be dropped (e.g. a percentage
        // `Width="100%"` â€” WinUI's `Width` is a `Double`, not a
        // percentage). `{x:Bind â€¦}` / `{Binding â€¦}` markup extensions
        // pass through untouched (never px-stripped or case-mangled).
        let value = match translate_xaml_value(&key, &p.value) {
            Some(v) => v,
            None => continue,
        };
        upsert_style_attr(&mut parts, key, value);
    }
    parts
        .into_iter()
        .map(|(key, value)| {
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            format!("{key}=\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn upsert_style_attr(attrs: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some((_, existing)) = attrs
        .iter_mut()
        .find(|(existing_key, _)| *existing_key == key)
    {
        *existing = value;
    } else {
        attrs.push((key, value));
    }
}

fn css_side_spacing_to_xaml_attr(name: &str, raw: &str) -> Option<(String, String)> {
    let (setter, edge) = match name {
        "padding-top" => ("Padding", "top"),
        "padding-right" => ("Padding", "right"),
        "padding-bottom" => ("Padding", "bottom"),
        "padding-left" => ("Padding", "left"),
        "margin-top" => ("Margin", "top"),
        "margin-right" => ("Margin", "right"),
        "margin-bottom" => ("Margin", "bottom"),
        "margin-left" => ("Margin", "left"),
        _ => return None,
    };
    let value = css_single_side_length(raw)?;
    Some((setter.to_string(), edge_thickness(edge, &value)))
}

fn css_single_side_length(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') || trimmed.ends_with('%') {
        return None;
    }
    let value = strip_px_units(trimmed);
    if value.contains(',') || value.split_whitespace().nth(1).is_some() {
        return None;
    }
    Some(value)
}

fn edge_thickness(edge: &str, value: &str) -> String {
    match edge {
        "top" => format!("0,{value},0,0"),
        "right" => format!("0,0,{value},0"),
        "bottom" => format!("0,0,0,{value}"),
        "left" => format!("{value},0,0,0"),
        _ => value.to_string(),
    }
}

/// Translate a mosstyle CSS *value* into the form the WinUI 3 XAML
/// markup compiler accepts for the given setter `key`. Returns `None`
/// when the whole property must be dropped (e.g. a percentage width).
///
/// This is the X5 value-translation layer. It sits below the X1
/// name-mapping (`css_property_to_xaml_setter`) and the X4 color
/// PascalCasing (`normalize_xaml_color_value`), and handles the value
/// shapes the earlier layers didn't:
///
/// | CSS source            | WinUI key       | Emitted value |
/// |-----------------------|-----------------|---------------|
/// | `font-size: 12px`     | `FontSize`      | `12`          |
/// | `border-width: 0,0,0,1px` | `BorderThickness` | `0,0,0,1`|
/// | `width: 100%`         | `Width`         | (dropped)     |
/// | `text-align: center`  | `TextAlignment` | `Center`      |
/// | `font-weight: 600`    | `FontWeight`    | `SemiBold`    |
/// | `background: red`     | `Background`    | `Red`         |
///
/// `{x:Bind â€¦}` / `{Binding â€¦}` values pass through verbatim â€” a
/// binding expression is not a literal and must never be mangled.
fn translate_xaml_value(key: &str, raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    // Markup extensions (`{x:Bind â€¦}`, `{Binding â€¦}`, `{StaticResource â€¦}`)
    // pass through untouched â€” they are not literal values.
    if trimmed.starts_with('{') {
        return Some(raw.to_string());
    }

    // Color setters: hand off to the X4 PascalCasing pass.
    if is_color_setter(key) {
        return Some(normalize_xaml_color_value(raw));
    }

    // Length setters: strip CSS `px` units (and reject percentages,
    // which WinUI's `Double`-typed length properties can't express).
    if is_length_setter(key) {
        // `100%` (or any percentage) â€” WinUI lengths are absolute
        // Doubles. Drop the whole property; the layout container
        // (StackPanel / Grid `*`) sizes the element instead.
        if trimmed.ends_with('%') {
            return None;
        }
        return Some(strip_px_units(trimmed));
    }

    // `text-align` â†’ WinUI `TextAlignment` enum (PascalCase value).
    if key == "TextAlignment" {
        return Some(pascalcase_text_alignment(trimmed));
    }

    // `font-weight` â†’ WinUI `FontWeight` named-constant (PascalCase).
    if key == "FontWeight" {
        return Some(pascalcase_font_weight(trimmed));
    }

    // Everything else passes through verbatim.
    Some(raw.to_string())
}

/// Which XAML setter properties take an absolute length (a `Double` or
/// a `Thickness` / `CornerRadius` built from Doubles). These are the
/// setters whose values must have CSS `px` units stripped and reject
/// percentages.
fn is_length_setter(setter: &str) -> bool {
    matches!(
        setter,
        "FontSize"
            | "Height"
            | "Width"
            | "MaxHeight"
            | "MaxWidth"
            | "MinHeight"
            | "MinWidth"
            | "Padding"
            | "Margin"
            | "BorderThickness"
            | "CornerRadius"
            | "Spacing"
    )
}

/// Strip CSS `px` suffixes from a length value, preserving the
/// comma-separated `Thickness` shape WinUI uses for multi-edge values.
///
/// - `12px`          â†’ `12`
/// - `0,0,0,1px`     â†’ `0,0,0,1`
/// - `8px 4px`       â†’ `8 4`   (space-separated multi-value)
/// - `12`            â†’ `12`    (already clean)
///
/// Only a trailing `px` on each component is removed; the numeric body
/// is left exactly as written so the host's XAML `Double` / `Thickness`
/// parser does the final conversion.
fn strip_px_units(value: &str) -> String {
    // Pick the separator the source used so the WinUI shape is
    // preserved: commas (`0,0,0,1`) form a `Thickness`; a single space
    // (`8 4`) is the space-separated `Thickness` shorthand.
    let sep = if value.contains(',') { ',' } else { ' ' };
    value
        .split(sep)
        .map(|seg| {
            let s = seg.trim();
            s.strip_suffix("px").unwrap_or(s)
        })
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

/// `center` â†’ `Center`, `right` â†’ `Right`, `left` â†’ `Left`, `justify`
/// â†’ `Justify`. Maps a CSS `text-align` value to the WinUI
/// `TextAlignment` enum member (PascalCase). Unknown values are
/// PascalCased generically so a typo surfaces at the markup compiler
/// rather than silently mangling.
fn pascalcase_text_alignment(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "center" => "Center".to_string(),
        "right" => "Right".to_string(),
        "left" => "Left".to_string(),
        "justify" => "Justify".to_string(),
        // `start` / `end` are logical CSS values; WinUI's
        // TextAlignment has `Start`/`End` too (since the 2020 SDK).
        "start" => "Start".to_string(),
        "end" => "End".to_string(),
        other => kebab_to_pascal_case(other),
    }
}

/// Map a CSS `font-weight` value (a keyword or a 100â€“900 numeric) to
/// the WinUI `FontWeights` named constant. WinUI's `FontWeight` setter
/// accepts the named constants (`Normal`, `Bold`, `SemiBold`, â€¦) but
/// NOT the bare CSS keyword `normal`/`bold` in lowercase, and not the
/// numeric `600` form in a `<Setter>`.
fn pascalcase_font_weight(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "100" | "thin" => "Thin".to_string(),
        "200" | "extralight" | "ultralight" => "ExtraLight".to_string(),
        "300" | "light" => "Light".to_string(),
        "400" | "normal" | "regular" => "Normal".to_string(),
        "500" | "medium" => "Medium".to_string(),
        "600" | "semibold" | "demibold" => "SemiBold".to_string(),
        "700" | "bold" => "Bold".to_string(),
        "800" | "extrabold" | "ultrabold" => "ExtraBold".to_string(),
        "900" | "black" | "heavy" => "Black".to_string(),
        // Unknown â€” PascalCase generically so the markup compiler flags
        // it rather than us silently emitting an invalid lowercase form.
        other => kebab_to_pascal_case(other),
    }
}

/// Which XAML setter properties take a `Brush` (the WinUI color type).
/// Used by `build_style_fragment` to scope `normalize_xaml_color_value`
/// to the values that actually flow to a brush â€” everything else
/// (lengths, fonts, weights) passes through verbatim.
fn is_color_setter(setter: &str) -> bool {
    matches!(setter, "Background" | "Foreground" | "BorderBrush")
}

/// Normalize a mosstyle color value into a form the WinUI 3 XAML markup
/// compiler accepts.
///
/// Mosstyle's `.msl` source treats colors as opaque strings.  CSS-style
/// lowercase names (`"transparent"`, `"red"`) are valid input, but XAML
/// expects either a hex literal (`#RRGGBB` / `#AARRGGBB`) or a *Pascal-
/// cased* named color (`Transparent`, `Red`).  This function:
///
/// - Pass-through for hex literals (`#â€¦`) â€” XAML accepts these as-is.
/// - Pass-through for already-PascalCased names (first letter upper)
///   â€” assumed XAML-native.
/// - PascalCase known CSS color names â€” `transparent`â†’`Transparent`,
///   etc.
/// - For anything else, return unchanged.  Better to emit a stale
///   value the markup compiler can flag than to silently mangle a
///   user-supplied identifier.
///
/// The named-color table mirrors the CSS3 / SVG palette intersected
/// with the WinUI 3 `Microsoft.UI.Colors` set.  Most of them are
/// identical PascalCased forms (`red`â†’`Red`); a few â€” `darkgray`
/// vs `DarkGray` â€” also normalise to PascalCase.
fn normalize_xaml_color_value(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('#') {
        return s.to_string();
    }
    // `{x:Bind â€¦}` / `{Binding â€¦}` markup extensions or any string with
    // braces â€” keep verbatim.  These aren't color literals.
    if trimmed.starts_with('{') {
        return s.to_string();
    }
    // Already PascalCased (or starts with an uppercase letter)?  Treat
    // as XAML-native and pass through.
    let first = trimmed.chars().next();
    if matches!(first, Some(c) if c.is_ascii_uppercase()) {
        return s.to_string();
    }
    // All-lowercase identifier â€” PascalCase it.  `transparent` â†’
    // `Transparent`, `red` â†’ `Red`, etc.  We don't gate on a known
    // CSS-color whitelist: the markup compiler will reject anything
    // that isn't a real named color, and over-pascalCasing is the
    // failure mode we want (it just shifts which compiler complains).
    if trimmed.chars().all(|c| c.is_ascii_lowercase()) {
        let mut chars = trimmed.chars();
        match chars.next() {
            Some(c) => return c.to_ascii_uppercase().to_string() + chars.as_str(),
            None => return s.to_string(),
        }
    }
    s.to_string()
}

/// Map a mosstyle CSS property name to its XAML setter property name.
/// The table is intentionally small in PR-1 â€” only what the nine simple
/// primitives need. PR-3..PR-6 grow it.
fn css_property_to_xaml_setter(name: &str) -> Option<String> {
    match name {
        "background" => Some("Background".to_string()),
        "background-color" => Some("Background".to_string()),
        "color" => Some("Foreground".to_string()),
        "font-family" => Some("FontFamily".to_string()),
        "font-size" => Some("FontSize".to_string()),
        "font-weight" => Some("FontWeight".to_string()),
        "gap" => Some("Spacing".to_string()),
        "padding" => Some("Padding".to_string()),
        "margin" => Some("Margin".to_string()),
        "width" => Some("Width".to_string()),
        "height" => Some("Height".to_string()),
        "max-width" => Some("MaxWidth".to_string()),
        "max-height" => Some("MaxHeight".to_string()),
        "min-width" => Some("MinWidth".to_string()),
        "min-height" => Some("MinHeight".to_string()),
        "border-width" => Some("BorderThickness".to_string()),
        "border-color" => Some("BorderBrush".to_string()),
        // X1 fix: mosstyle's `border-radius` maps to WinUI's
        // `CornerRadius` (UIElement.CornerRadius), NOT `BorderRadius`.
        // The latter isn't a real WinUI 3 property; the XAML markup
        // compiler rejects it silently (XamlCompiler.exe exits 1 with
        // no diagnostic). Caught by the toolkit Button + Alert + Badge
        // demo (#4548).
        "border-radius" => Some("CornerRadius".to_string()),
        // X5: `text-align` maps to WinUI's `TextAlignment` (a
        // `TextBlock` enum property), NOT `TextAlign`. The value side
        // is PascalCased by `translate_xaml_value`
        // (`center`â†’`Center`). The old `kebab_to_pascal_case` fallback
        // produced `TextAlign` â€” a property that doesn't exist â€” so the
        // setter was silently dropped by the markup compiler.
        "text-align" => Some("TextAlignment".to_string()),
        // X5/X6: CSS-only or flex-only properties with NO direct WinUI
        // attribute analog in the current lowering. Returning `None`
        // omits them entirely rather than emitting an invalid attribute
        // / `<Setter>` the markup compiler rejects.
        //
        //   border-collapse â€” WinUI has no table model; gridlines are
        //                     drawn by per-cell BorderThickness.
        //   border-style    â€” WinUI borders are always solid; there is
        //                     no dashed/dotted `BorderStyle` property.
        //   outline         â€” no WinUI equivalent (focus visuals use
        //                     the FocusVisual* attached properties).
        //   text-decoration â€” TextBlock uses the `TextDecorations`
        //                     property with a different value shape;
        //                     not wired yet, so drop rather than emit
        //                     an invalid literal.
        //   box-shadow      â€” WinUI shadows use `<ThemeShadow>` /
        //                     translation Z, not a CSS-shaped property.
        //   align-items     — requires child-level alignment, not a
        //                     property on StackPanel/Border/TextBlock.
        //   justify-content — StackPanel has no space-between/around
        //                     distribution property.
        //   flex-wrap       — WinUI has no built-in WrapPanel in WinUI 3.
        "align-items" | "border-collapse" | "border-style" | "box-shadow" | "flex-wrap"
        | "justify-content" | "outline" | "text-decoration" => None,
        // Unknown properties must not be PascalCased into fake WinUI
        // setters. That path let Lattice/web layout names such as
        // `gap` and `flex-wrap` leak into TextBlock styles and made
        // XamlCompiler.exe fail with code 1 and no useful diagnostic.
        _ => None,
    }
}

/// Returns `true` for the XAML setter properties that belong on a
/// `<Border>` (and other container elements like `<Grid>`, `<StackPanel>`)
/// â€” i.e. properties governing the box's own paint, not the text content
/// inside it. Used by `emit_box` to partition style props between the
/// `<Border>` itself and its inner `<TextBlock>` child.
///
/// `<Border>` accepts: Background, BorderBrush, BorderThickness,
/// CornerRadius, Padding, Margin, Width, Height. It does NOT accept
/// Foreground, FontSize, FontWeight, FontFamily â€” those belong on the
/// inner content. Caught by the toolkit Alert + Badge demo (#4548).
fn is_container_style_attr(setter: &str) -> bool {
    matches!(
        setter,
        "Background"
            | "BorderBrush"
            | "BorderThickness"
            | "CornerRadius"
            | "Padding"
            | "Margin"
            | "Width"
            | "Height"
            | "MaxWidth"
            | "MaxHeight"
            | "MinWidth"
            | "MinHeight"
            | "HorizontalAlignment"
            | "VerticalAlignment"
    )
}

fn is_stack_panel_style_attr(setter: &str) -> bool {
    matches!(
        setter,
        "Spacing"
            | "Margin"
            | "Width"
            | "Height"
            | "MaxWidth"
            | "MaxHeight"
            | "MinWidth"
            | "MinHeight"
            | "HorizontalAlignment"
            | "VerticalAlignment"
    )
}

fn is_text_style_attr(setter: &str) -> bool {
    matches!(
        setter,
        "Foreground" | "FontFamily" | "FontSize" | "FontWeight" | "TextAlignment"
    )
}

// =====================================================================
// Identifier conversions
// =====================================================================

/// `column-headers` â†’ `ColumnHeaders` (XAML `DependencyProperty` names,
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

/// `column-headers` â†’ `columnHeaders`. Used for `{x:Bind}` paths and
/// local C# helpers. camelCase with first letter lowered.
///
/// Currently unused â€” PR-2's `For` lowering will reach for it when it
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

/// Emit `{Component}.xaml` â€” the markup file. Wraps the lowered
/// moslayout tree in a `<UserControl>` root.
///
/// `ctx` is mutated during the walk: `For` pushes/pops bindings, `If`
/// adds helper methods, both may flip `needs_bool_to_vis`.
/// Identifies which XAML root the emitter is producing. Most
/// components lower to a `<UserControl>` root; HostDialog-rooted
/// components hoist to a `<ContentDialog>` root (Fix A1).
///
/// The base class affects:
///   - the XAML root element name
///   - the partial class's base type (emit_code_behind)
///   - whether slot DP names collide with inherited properties
///     (emit_dependency_property uses ctx.slot_aliases)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootShape {
    /// Standard component â€” `<UserControl>` root, `: UserControl` C#.
    UserControl,
    /// HostDialog-rooted â€” `<ContentDialog>` root, `: ContentDialog`
    /// C#. The moslayout root's HostDialog props become attributes on
    /// the ContentDialog itself; its children become the Content.
    ContentDialog,
}

impl RootShape {
    fn xaml_tag(self) -> &'static str {
        match self {
            RootShape::UserControl => "UserControl",
            RootShape::ContentDialog => "ContentDialog",
        }
    }
    fn csharp_base(self) -> &'static str {
        match self {
            RootShape::UserControl => "UserControl",
            RootShape::ContentDialog => "ContentDialog",
        }
    }
    /// The set of inherited property names that the slot-DP generator
    /// must alias around so the author's `slot title : text` doesn't
    /// shadow `ContentDialog.Title`. Fix A4. PascalCased.
    fn inherited_properties(self) -> &'static [&'static str] {
        match self {
            // UserControl has very few user-facing properties; the
            // generated DPs almost never collide.
            RootShape::UserControl => &[],
            // ContentDialog has `Title`, `PrimaryButtonText`,
            // `SecondaryButtonText`, `CloseButtonText`,
            // `IsPrimaryButtonEnabled`, etc. List the ones authors
            // are most likely to step on; expand as we discover
            // more.
            RootShape::ContentDialog => &[
                "Title",
                "PrimaryButtonText",
                "SecondaryButtonText",
                "CloseButtonText",
                "IsPrimaryButtonEnabled",
                "IsSecondaryButtonEnabled",
                "DefaultButton",
            ],
        }
    }
}

/// Choose the root shape for the component's emitted XAML.
///
/// HostDialog as the moslayout root â†’ ContentDialog root + partial
/// class extends ContentDialog. This matches the WinUI 3 idiom that
/// ContentDialog is a top-layer popup primitive: it can't be embedded
/// inside a UserControl and then shown via ShowAsync(); the parented
/// child can't be re-parented for the modal popup.
///
/// Discovered in `code/programs/csharp/hello-dialog-xaml/ISSUES.md` A1.
fn pick_root_shape(root: &LayoutNode) -> RootShape {
    match root.tag.as_str() {
        "HostDialog" => RootShape::ContentDialog,
        _ => RootShape::UserControl,
    }
}

/// Populate `ctx.slot_aliases` with `slot_name â†’ AliasedDpName` entries
/// for every slot whose PascalCased name collides with a property on
/// the chosen base class. Fix A4.
///
/// Aliasing rule: `BaseTypeName + PascalCasedSlotName`. So a
/// `slot title : text` on a ContentDialog-rooted component generates a
/// DP named `DialogTitle`. The XAML `{x:Bind}` paths route through
/// `EmitContext::slot_xbind_path` to use the alias.
fn populate_slot_aliases(ctx: &mut EmitContext<'_>, shape: RootShape, slots: &[SlotDecl]) {
    let inherited = shape.inherited_properties();
    if inherited.is_empty() {
        return;
    }
    let base_prefix = match shape {
        RootShape::ContentDialog => "Dialog",
        RootShape::UserControl => "Control",
    };
    for slot in slots {
        let pascal = kebab_to_pascal_case(&slot.name);
        if inherited.iter().any(|p| *p == pascal) {
            let alias = format!("{base_prefix}{pascal}");
            ctx.slot_aliases.insert(slot.name.clone(), alias);
        }
    }
}

fn emit_xaml(
    name: &str,
    root: &LayoutNode,
    part_styles: &PartStyleMap,
    options: &EmitOptions,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let ns = &options.namespace;
    let shape = pick_root_shape(root);
    let root_tag = shape.xaml_tag();

    writeln!(
        out,
        "<!-- Auto-generated by mosaic-emit-xaml. Do not edit. -->"
    )
    .unwrap();
    writeln!(out, "<{root_tag}").unwrap();
    writeln!(out, "    x:Class=\"{ns}.{name}\"").unwrap();
    writeln!(
        out,
        "    xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\""
    )
    .unwrap();
    writeln!(
        out,
        "    xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\""
    )
    .unwrap();
    writeln!(out, "    xmlns:local=\"using:{ns}\">").unwrap();
    writeln!(out).unwrap();

    // Walk the root node â€” at the moslayout level a component has
    // exactly one root, but we still pass through the children iterator
    // because `If`/`Else` pairing happens there.
    //
    // For a ContentDialog-rooted component (HostDialog) the
    // `emit_host_dialog` emitter knows to emit the dialog's *contents*
    // (its children) without re-wrapping in another `<ContentDialog>`.
    // That's done via `EmitContext::root_already_emitted`, set below.
    let body = if shape == RootShape::ContentDialog {
        emit_host_dialog_as_root(root, 4, part_styles, ctx)?
    } else {
        emit_xaml_node(root, 4, part_styles, ctx)?
    };
    out.push_str(&body);

    // ---- Locate the root open tag for splicing ----
    //
    // The output begins with an auto-generated comment whose own `>`
    // would otherwise mislead `find(">\n")`. We anchor on the literal
    // `<{root_tag}` substring to find the actual root open tag, then
    // search forward for its closing `>\n`. Helper closure for the
    // multiple splice sites below.
    let find_root_open_close = |s: &str| -> Option<usize> {
        let start = format!("<{root_tag}");
        s.find(&start)
            .and_then(|p| s[p..].find(">\n").map(|q| p + q))
    };

    // After walking, if any `If` was emitted we must declare the
    // converter resource. We splice it in after the open root tag.
    if ctx.needs_bool_to_vis {
        let resources_tag = match shape {
            RootShape::UserControl => "UserControl.Resources",
            RootShape::ContentDialog => "ContentDialog.Resources",
        };
        let resources = emit_bool_to_vis_resource_block(4, resources_tag);
        let split_at = find_root_open_close(&out)
            .map(|p| p + 2)
            .unwrap_or(out.len());
        let (head, tail) = out.split_at(split_at);
        out = format!("{head}{resources}{tail}");
    }

    // PR-5: inject `xmlns:{prefix}="{value}"` declarations for any
    // resolved component references onto the root open tag, just
    // before its closing `>`.
    if !ctx.used_xmlns.is_empty() {
        let mut xmlns_lines = String::new();
        for (prefix, value) in &ctx.used_xmlns {
            xmlns_lines.push_str(&format!(
                "\n    xmlns:{prefix}=\"{}\"",
                escape_xaml_attr(value)
            ));
        }
        if let Some(close) = find_root_open_close(&out) {
            let (head, tail) = out.split_at(close);
            out = format!("{head}{xmlns_lines}{tail}");
        }
    }

    // Fix A1 ContentDialog-root path: splice the HostDialog
    // attributes that `emit_host_dialog_as_root` stashed onto the
    // open ContentDialog tag, just before its closing `>`.
    if let Some(extra) = ctx.root_extra_attrs.clone() {
        if let Some(close) = find_root_open_close(&out) {
            let (head, tail) = out.split_at(close);
            out = format!("{head}{extra}{tail}");
        }
    }

    writeln!(out).unwrap();
    writeln!(out, "</{root_tag}>").unwrap();
    Ok(out)
}

/// Lower one moslayout node and its descendants to XAML, indented by
/// `indent` spaces.
///
/// PR-1 added the nine simple kernel primitives; PR-2 adds `For`. `If`
/// and `Else` are NOT handled here â€” they're consumed by
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
        // component (no preceding sibling) â€” emit it as a top-level
        // conditional. The look-ahead for `Else` happens in the
        // children iterator, so the standalone case here means no
        // `Else` was paired.
        "If" => emit_if(node, None, indent, part_styles, ctx),
        // A standalone `Else` (no preceding `If`) is a moslayout-level
        // validation error per UI29 Â§3.2; we treat it as
        // UnsupportedPrimitive here for the second line of defence in
        // case validation was bypassed.
        "Else" => Err(PipelineEmitError::UnsupportedPrimitive(
            "Else without preceding If".to_string(),
        )),

        // PR-3: Host* primitives (single-element host-native controls).
        "HostInput" => emit_host_input(node, indent, part_styles, ctx),
        "HostButton" => emit_host_button(node, indent, part_styles, ctx),

        // UI29-2 â€” `HostCheckbox` lowers to WinUI/WPF `<CheckBox>` and
        // `HostRadio` lowers to `<RadioButton>`. Both controls share
        // the `IsChecked` / `IsEnabled` / `Content` property surface
        // with `<Button>`, plus their own checked-state events.
        "HostCheckbox" => emit_host_checkbox(node, indent, part_styles, ctx),
        "HostRadio" => emit_host_radio(node, indent, part_styles, ctx),

        // UI29-4 â€” HostLink lowers to a `<HyperlinkButton NavigateUri=
        // "..." Content="...">` (WinUI 3's first-class clickable
        // hyperlink). HostTooltip uses the `ToolTipService.ToolTip`
        // attached property on the wrapped child. HostNumberInput
        // uses `<NumberBox>` (WinUI 3 numeric input with built-in Â±
        // stepper).
        "HostLink" => emit_host_link(node, indent, part_styles, ctx),
        "HostTooltip" => emit_host_tooltip(node, indent, part_styles, ctx),
        "HostNumberInput" => emit_host_number_input(node, indent, part_styles, ctx),

        "HostScroll" => emit_host_scroll(node, indent, part_styles, ctx),

        // PR-4: HostTable.
        "HostTable" => emit_host_table(node, indent, part_styles, ctx),

        // U29-1-K-xaml: HostDialog kernel primitive (UI29-1 Â§3.6).
        "HostDialog" => emit_host_dialog(node, indent, part_styles, ctx),

        // The four section sub-tags are recognised only as children of
        // HostTable. Encountering them as direct nodes here means the
        // author wrote them at the wrong level (outside a HostTable);
        // surface as a clear UnsupportedPrimitive with the offending
        // tag name.
        "HostTableColGroup" | "HostTableHead" | "HostTableBody" | "HostTableFoot" => Err(
            PipelineEmitError::UnsupportedPrimitive(format!("{} outside HostTable", node.tag)),
        ),

        // Anything else is a component reference (UI29 Â§4.4). PR-5
        // resolves it through the optional `ComponentRegistry`. When the
        // registry is absent or the tag isn't registered, the error
        // path makes the failure clear: a missing manifest dependency
        // is `UnknownComponent`; a registry-less invocation falls back
        // to `UnsupportedPrimitive` for parity with the pre-PR-5 shape.
        other => emit_component_reference(other, node, indent, ctx),
    }
}

/// Walk a slice of children, emitting each in order. Pairs an `If` with
/// a following `Else` sibling (UI29 Â§3.2) â€” that pairing is the only
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
            let else_node = children.get(i + 1).filter(|next| next.tag == "Else");
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

fn emitted_content_child_count(children: &[LayoutNode]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if child.tag == "If" {
            let has_else = children.get(i + 1).is_some_and(|next| next.tag == "Else");
            count += if has_else { 2 } else { 1 };
            i += if has_else { 2 } else { 1 };
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

/// Emit children for XAML hosts that accept exactly one content object.
/// If Mosaic lowered the child list to multiple sibling elements, wrap
/// them in a neutral vertical StackPanel instead of emitting invalid
/// direct children under ContentControl, DataTemplate, Border, etc.
fn emit_xaml_single_content_children(
    children: &[LayoutNode],
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    if emitted_content_child_count(children) <= 1 {
        return emit_xaml_children(children, indent, part_styles, ctx);
    }

    let pad = " ".repeat(indent);
    let mut out = String::new();
    writeln!(out, "{pad}<StackPanel Orientation=\"Vertical\">").unwrap();
    out.push_str(&emit_xaml_children(children, indent + 4, part_styles, ctx)?);
    writeln!(out, "{pad}</StackPanel>").unwrap();
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

/// Parse a joined style fragment (the value side of `PartStyleMap`)
/// back into individual `(setter, value)` pairs. Round-trips
/// `build_style_fragment`'s output. Used by `emit_container` to
/// partition style props between the wrapping element and its
/// inner text content. Caught by the toolkit Alert + Badge demo
/// (#4548).
fn parse_style_fragment(frag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut chars = frag.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace.
        while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
            chars.next();
        }
        // Read the key up to '='.
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' {
                break;
            }
            key.push(c);
            chars.next();
        }
        if chars.peek() != Some(&'=') {
            break;
        }
        chars.next(); // consume '='
        if chars.peek() != Some(&'"') {
            break;
        }
        chars.next(); // consume opening '"'
                      // Read the value up to the next un-escaped '"'. Values that
                      // contain `\"` (escaped) are unusual but handled.
        let mut value = String::new();
        while let Some(&c) = chars.peek() {
            if c == '"' {
                chars.next();
                break;
            }
            if c == '\\' {
                chars.next();
                if let Some(&n) = chars.peek() {
                    value.push(n);
                    chars.next();
                }
            } else {
                value.push(c);
                chars.next();
            }
        }
        if !key.is_empty() {
            out.push((key, value));
        }
    }
    out
}

/// Partition a style fragment into:
///   - `container_attrs`: a leading-space-prefixed attribute string
///     containing only the setters valid on `<Border>` / `<Grid>` /
///     `<StackPanel>` (paint, padding, sizing â€” see
///     `is_container_style_attr`).
///   - `text_setters`: the remaining `(setter, value)` pairs that
///     belong on text content (Foreground, FontSize, FontWeight,
///     FontFamily). Empty when no text-style props were present.
///
/// `emit_container` uses this to put the box's own paint on the
/// outer element and emit a scoped `Style TargetType="TextBlock"`
/// resource carrying the inheritable text style. WinUI's implicit-
/// style mechanism then applies it to every `TextBlock` descendant
/// inside the container.
fn partition_box_style(
    part: Option<&str>,
    part_styles: &PartStyleMap,
) -> (String, Vec<(String, String)>) {
    let frag = match part.and_then(|p| part_styles.get(p)) {
        Some(f) => f.as_str(),
        None => return (String::new(), Vec::new()),
    };
    let mut container = String::new();
    let mut text = Vec::new();
    for (setter, value) in parse_style_fragment(frag) {
        if is_container_style_attr(&setter) {
            container.push(' ');
            container.push_str(&setter);
            container.push_str("=\"");
            container.push_str(&value);
            container.push('"');
        } else if is_text_style_attr(&setter) {
            text.push((setter, value));
        }
    }
    (container, text)
}

fn partition_stack_panel_style(
    part: Option<&str>,
    part_styles: &PartStyleMap,
) -> (String, String, Vec<(String, String)>) {
    let frag = match part.and_then(|p| part_styles.get(p)) {
        Some(f) => f.as_str(),
        None => return (String::new(), String::new(), Vec::new()),
    };
    let mut wrapper = String::new();
    let mut stack = String::new();
    let mut text = Vec::new();
    for (setter, value) in parse_style_fragment(frag) {
        if setter == "Spacing" {
            stack.push(' ');
            stack.push_str(&setter);
            stack.push_str("=\"");
            stack.push_str(&value);
            stack.push('"');
        } else if is_container_style_attr(&setter) && !is_stack_panel_style_attr(&setter) {
            wrapper.push(' ');
            wrapper.push_str(&setter);
            wrapper.push_str("=\"");
            wrapper.push_str(&value);
            wrapper.push('"');
        } else if is_stack_panel_style_attr(&setter) {
            stack.push(' ');
            stack.push_str(&setter);
            stack.push_str("=\"");
            stack.push_str(&value);
            stack.push('"');
        } else if is_text_style_attr(&setter) {
            text.push((setter, value));
        }
    }
    (wrapper, stack, text)
}

// ---------------------------------------------------------------------
// Primitive emitters (the nine simple kernel primitives â€” PR-1)
// ---------------------------------------------------------------------

/// `Box [name] { children }` â†’ `<Border>...</Border>`.
///
/// PR-1 always emits `<Border>` even when no style applies. A later PR
/// swaps to `<ContentPresenter>` when the resolved style has no
/// background / border / padding â€” `<ContentPresenter>` is zero-cost
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
    let inner_pad = " ".repeat(indent + 4);
    let (container_attrs, stack_attrs, text_setters) =
        partition_stack_panel_style(node.part_name.as_deref(), part_styles);
    let mut out = if container_attrs.is_empty() && text_setters.is_empty() {
        format!("{pad}<StackPanel Orientation=\"{orientation}\"{stack_attrs}>\n")
    } else {
        let mut wrapped = format!("{pad}<Border{container_attrs}>\n");
        emit_text_style_resources(&mut wrapped, "Border", indent + 4, &text_setters);
        writeln!(
            wrapped,
            "{inner_pad}<StackPanel Orientation=\"{orientation}\"{stack_attrs}>"
        )
        .unwrap();
        wrapped
    };
    let child_indent = if container_attrs.is_empty() && text_setters.is_empty() {
        indent + 4
    } else {
        indent + 8
    };
    out.push_str(&emit_xaml_children(
        &node.children,
        child_indent,
        part_styles,
        ctx,
    )?);
    if container_attrs.is_empty() && text_setters.is_empty() {
        writeln!(out, "{pad}</StackPanel>").unwrap();
    } else {
        writeln!(out, "{inner_pad}</StackPanel>").unwrap();
        writeln!(out, "{pad}</Border>").unwrap();
    }
    Ok(out)
}

/// `Stack [name] { children }` â†’ `<Grid>...</Grid>`.
///
/// XAML `<Grid>` is the z-axis container â€” children at the same row/col
/// stack visually with later children drawn on top. The Mosaic `Stack`
/// primitive (UI29 Â§2.1) is exactly this shape.
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
    let inner_pad = " ".repeat(indent + 4);

    // X3 fix: `<Border>`, `<Grid>`, `<StackPanel>` don't have
    // `Foreground` / `FontSize` / `FontWeight` / `FontFamily`. When
    // the part style includes those, the XAML markup compiler
    // rejects them. Partition the style props: keep container-paint
    // attrs on the opening tag, push text-style attrs into a scoped
    // `<Element.Resources>` block as a TargetType="TextBlock" implicit
    // style. WinUI's implicit-style resolution then applies them to
    // every TextBlock descendant inside. Caught by the toolkit Alert
    // + Badge demo (#4548).
    let (container_attrs, text_setters) =
        partition_box_style(node.part_name.as_deref(), part_styles);

    if element != "Border" && (!container_attrs.is_empty() || !text_setters.is_empty()) {
        let mut out = format!("{pad}<Border{container_attrs}>\n");
        emit_text_style_resources(&mut out, "Border", indent + 4, &text_setters);
        writeln!(out, "{inner_pad}<{element}>").unwrap();
        out.push_str(&emit_xaml_children(
            &node.children,
            indent + 8,
            part_styles,
            ctx,
        )?);
        writeln!(out, "{inner_pad}</{element}>").unwrap();
        writeln!(out, "{pad}</Border>").unwrap();
        return Ok(out);
    }

    let mut out = format!("{pad}<{element}{container_attrs}>\n");

    emit_text_style_resources(&mut out, element, indent + 4, &text_setters);

    if element == "Border" {
        out.push_str(&emit_xaml_single_content_children(
            &node.children,
            indent + 4,
            part_styles,
            ctx,
        )?);
    } else {
        out.push_str(&emit_xaml_children(
            &node.children,
            indent + 4,
            part_styles,
            ctx,
        )?);
    }
    writeln!(out, "{pad}</{element}>").unwrap();
    Ok(out)
}

fn emit_text_style_resources(
    out: &mut String,
    element: &str,
    indent: usize,
    text_setters: &[(String, String)],
) {
    if text_setters.is_empty() {
        return;
    }
    let pad = " ".repeat(indent);
    writeln!(out, "{pad}<{element}.Resources>").unwrap();
    writeln!(out, "{pad}    <Style TargetType=\"TextBlock\">").unwrap();
    for (setter, value) in text_setters {
        writeln!(
            out,
            "{pad}        <Setter Property=\"{setter}\" Value=\"{value}\"/>"
        )
        .unwrap();
    }
    writeln!(out, "{pad}    </Style>").unwrap();
    writeln!(out, "{pad}</{element}.Resources>").unwrap();
}

/// `Text [name] (content: slot: foo)` â†’ `<TextBlock Text="{x:Bind Foo}"/>`.
/// `Text [name] (content: "literal")` â†’ `<TextBlock Text="literal"/>`.
/// `Text [name] (content: row.value)` â†’ `<TextBlock Text="{x:Bind Row.Value}"/>`
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
            // name (either the element or the index binding), treat as
            // `{x:Bind ForName}`; otherwise as literal text (matches
            // the React backend's behaviour pre-PR-2).
            if ctx.lookup_for_binding(k).is_some() || ctx.lookup_for_index(k).is_some() {
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

/// `Image [name] (source: slot: foo)` â†’ `<Image Source="{x:Bind Foo}"/>`.
fn emit_image(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    _ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let source_attr = match find_prop_value(node, "source").or_else(|| find_prop_value(node, "src"))
    {
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

/// `Spacer` â†’ `<Rectangle/>` with default Width/Height that flex the layout.
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

/// `Divider` â†’ a thin `<Border>` band. WinUI 3 has no `<Separator>` in the
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

/// `Icon [name] (glyph: "...")` â†’ `<FontIcon Glyph="..."/>` against
/// Segoe Fluent Icons (the WinUI 3 default icon font).
///
/// Exception: when the `glyph` value is a *semantic* name (today only
/// `"spinner"`) the lowering switches to the WinUI-native widget that
/// expresses that semantic â€” `<ProgressRing IsActive="True"/>` for
/// `"spinner"`.  This is X5 Path A from
/// `code/programs/csharp/toolkit-multi-demo/ISSUES.md`: Segoe Fluent has no glyph
/// literally named `"spinner"`, and even if it did, `FontIcon` only
/// renders a static character â€” the toolkit's `Spinner` component
/// wants the animated spinning ring that `ProgressRing` provides.
///
/// The semantic-name list is intentionally tiny (start with `spinner`,
/// grow case-by-case as toolkit authors invent new ones).  A future
/// kernel-vocabulary cycle (UI34 / UI35) may promote `Spinner` to a
/// first-class primitive; until then, this targeted lowering is the
/// minimum-coupling fix.
fn emit_icon(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    _ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    // X5: semantic-glyph lowering.  Only fires for literal string
    // values â€” slot-bound glyphs (`{x:Bind GlyphProp}`) stay on the
    // FontIcon path because we can't statically tell what the runtime
    // value will be.
    if let Some(LayoutPropValue::String(s)) =
        find_prop_value(node, "glyph").or_else(|| find_prop_value(node, "name"))
    {
        if let Some(replacement) = semantic_glyph_xaml_element(s) {
            return Ok(format!("{pad}<{replacement}{style}/>\n"));
        }
    }

    let glyph_attr = match find_prop_value(node, "glyph").or_else(|| find_prop_value(node, "name"))
    {
        Some(LayoutPropValue::String(s)) => format!(" Glyph=\"{}\"", escape_xaml_attr(s)),
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            format!(" Glyph=\"{{x:Bind {pascal}}}\"")
        }
        _ => String::new(),
    };
    Ok(format!("{pad}<FontIcon{glyph_attr}{style}/>\n"))
}

/// Map a semantic glyph name to a WinUI 3 element name + attribute
/// fragment that expresses that semantic natively.  Returns `None`
/// for any name not in the table â€” the caller then falls back to
/// the standard `<FontIcon Glyph="..."/>` lowering.
///
/// Currently recognized:
///
/// | semantic name | XAML element                  |
/// |---|---|
/// | `"spinner"`   | `ProgressRing IsActive="True"`|
///
/// New entries land case-by-case as the toolkit demo surfaces them.
fn semantic_glyph_xaml_element(name: &str) -> Option<&'static str> {
    match name {
        "spinner" => Some("ProgressRing IsActive=\"True\""),
        _ => None,
    }
}

// =====================================================================
// File 2: code-behind (.xaml.cs)
// =====================================================================

/// Emit `{Component}.xaml.cs` â€” the partial class with DPs, the
/// Dispatch event, constructor boilerplate, and any helper methods the
/// expression lowerer registered during the XAML walk (PR-2).
fn emit_code_behind(
    name: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    options: &EmitOptions,
    ctx: &EmitContext<'_>,
    shape: RootShape,
) -> Result<String, PipelineEmitError> {
    let ns = &options.namespace;
    let base_class = shape.csharp_base();
    let mut out = String::new();

    writeln!(out, "// Auto-generated by mosaic-emit-xaml. Do not edit.").unwrap();
    writeln!(out, "using Microsoft.UI.Xaml;").unwrap();
    writeln!(out, "using Microsoft.UI.Xaml.Controls;").unwrap();
    writeln!(out, "using System;").unwrap();
    writeln!(out, "using System.Collections.Generic;").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "namespace {ns};").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "public sealed partial class {name} : {base_class}").unwrap();
    writeln!(out, "{{").unwrap();

    // Constructor: `InitializeComponent()`. The XAML compiler generates
    // the actual `InitializeComponent` method at build time from the
    // matching `.xaml` file.
    writeln!(out, "    public {name}()").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        this.InitializeComponent();").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // One DependencyProperty per declared slot (spec Â§8). Slots whose
    // PascalCased name collides with a property on the chosen base
    // class are renamed to `<BaseName>{Slot}` via `ctx.slot_aliases`
    // (Fix A4 â€” e.g. `slot title : text` on a ContentDialog-rooted
    // component becomes `DialogTitle`).
    for slot in slots {
        out.push_str(&emit_dependency_property(slot, name, ctx)?);
        writeln!(out).unwrap();
    }

    for projection in &ctx.row_projections {
        out.push_str(&emit_row_projection_property(projection));
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

    // Helper to invoke Dispatch from generated handlers â€” used by future
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
    ctx: &EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    // Fix A4: use the alias if this slot's PascalCased name would
    // collide with an inherited property on the chosen base class
    // (e.g. ContentDialog.Title).
    let pascal = ctx.slot_xbind_path(&slot.name);
    if !is_safe_identifier(&pascal) {
        return Err(PipelineEmitError::UnsafeSlotName(pascal));
    }
    let csharp_type = slot_type_to_csharp(&slot.r#type)?;

    let mut out = String::new();
    writeln!(out, "    public {csharp_type} {pascal}").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(
        out,
        "        get => ({csharp_type})GetValue({pascal}Property);"
    )
    .unwrap();
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

/// Translate a mosmodel slot type to its C# property type per spec Â§8.
fn emit_row_projection_property(projection: &RowProjection) -> String {
    let property_name = &projection.property_name;
    let source_path = &projection.source_path;
    let vm_class = &projection.vm_class;

    let mut args = vec!["source[i]".to_string()];
    if projection.has_index {
        args.push("i".to_string());
    }
    if projection.has_width {
        args.push("0".to_string());
    }
    if let Some(selected_index_path) = &projection.selected_index_path {
        args.push(format!("i == {selected_index_path}"));
    }
    let ctor_args = args.join(", ");

    let mut out = String::new();
    writeln!(out, "    public IReadOnlyList<{vm_class}> {property_name}").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        get").unwrap();
    writeln!(out, "        {{").unwrap();
    writeln!(out, "            var source = {source_path};").unwrap();
    writeln!(out, "            var rows = new List<{vm_class}>();").unwrap();
    writeln!(out, "            if (source is null) return rows;").unwrap();
    writeln!(out, "            for (var i = 0; i < source.Count; i++)").unwrap();
    writeln!(out, "            {{").unwrap();
    writeln!(
        out,
        "                rows.Add(new {vm_class}({ctor_args}));"
    )
    .unwrap();
    writeln!(out, "            }}").unwrap();
    writeln!(out, "            return rows;").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    out
}

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

/// Emit `{Component}.Event.cs` â€” the discriminated record union for the
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
    writeln!(out, "    public abstract string MosaicName {{ get; }}").unwrap();
    writeln!(
        out,
        "    public virtual System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicPayload => new System.Collections.Generic.Dictionary<string, object?>();"
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "    public System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicEnvelope"
    )
    .unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        get").unwrap();
    writeln!(out, "        {{").unwrap();
    writeln!(
        out,
        "            var envelope = new System.Collections.Generic.Dictionary<string, object?>(MosaicPayload);"
    )
    .unwrap();
    writeln!(out, "            envelope[\"event\"] = MosaicName;").unwrap();
    writeln!(out, "            return envelope;").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    for emit in emits {
        let case_name = strip_on_prefix(&emit.name);
        let case_pascal = kebab_to_pascal_case(&case_name);
        if !is_safe_identifier(&case_pascal) {
            return Err(PipelineEmitError::UnsafeEmitName(case_pascal));
        }
        let event_name = escape_csharp_string(&emit.name);

        if emit.params.is_empty() {
            writeln!(
                out,
                "    public sealed record {case_pascal}() : {name}Event\n    {{\n        public override string MosaicName => \"{event_name}\";\n    }}"
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
            let payload = csharp_event_payload_dictionary(emit)?;
            writeln!(
                out,
                "    public sealed record {case_pascal}({}) : {name}Event\n    {{\n        public override string MosaicName => \"{event_name}\";\n        public override System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicPayload => {payload};\n    }}",
                params.join(", "),
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn csharp_event_payload_dictionary(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    let mut entries = Vec::with_capacity(emit.params.len());
    for param in &emit.params {
        let key = kebab_to_camel_case(&param.name);
        let property_name = kebab_to_pascal_case(&param.name);
        if !is_safe_identifier(&property_name) {
            return Err(PipelineEmitError::UnsafeEmitName(property_name));
        }
        entries.push(format!(
            "[\"{}\"] = {property_name}",
            escape_csharp_string(&key)
        ));
    }
    Ok(format!(
        "new System.Collections.Generic.Dictionary<string, object?> {{ {} }}",
        entries.join(", ")
    ))
}

fn emit_payload_to_csharp(t: &EmitPayloadType) -> String {
    match t {
        EmitPayloadType::Text => "string".to_string(),
        EmitPayloadType::Number => "double".to_string(),
        EmitPayloadType::Bool => "bool".to_string(),
        EmitPayloadType::Color => "Windows.UI.Color".to_string(),
        // Component-typed emit payloads forward the C# type name verbatim
        // (same shape as component-typed slots â€” the host declares a
        // matching record type and the resolver in PR-5 wires it up).
        EmitPayloadType::Component(type_name) => type_name.clone(),
    }
}

fn strip_on_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("on") {
        if rest
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
        {
            // `onNavigate` â†’ `navigate` (lower the first char so kebab
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
    node.props
        .iter()
        .find(|p| p.name == prop_name)
        .map(|p| &p.value)
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

/// `For (each: <expr>, as: <name>, index: <name>?) { <children> }` â†’
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
        PipelineEmitError::UnsupportedPrimitive("For block missing required prop 'as:'".to_string())
    })?;
    let index_name = find_prop_keyword(node, "index");

    // -- 2. Resolve the `each:` source to a {x:Bind} path and an
    //    element type --
    // `is_cell_loop` is `true` only for the per-column cell loop â€”
    // a `For` whose `each:` is an enclosing For binding (UI29 Â§3.4,
    // the inner `For (each: row, â€¦)`). GROUP C threads the colgroup
    // width onto that loop's value VM (a `double Width` field) so each
    // column renders at a fixed pixel width.
    let (items_path, element_type, is_cell_loop, slot_backed_items_source) =
        match find_prop_value(node, "each") {
            Some(LayoutPropValue::SlotRef(slot)) => {
                let pascal = ctx.slot_xbind_path(slot);
                if !is_safe_identifier(&pascal) {
                    return Err(PipelineEmitError::UnsafeSlotName(pascal));
                }
                // Look up the slot's declared C# type to derive the element
                // type. Slots are typed as `IReadOnlyList<X>` â†’ element is X.
                let csharp_type = ctx
                    .slot_types
                    .get(slot.as_str())
                    .cloned()
                    .unwrap_or_else(|| "object".to_string());
                let elem_type = inner_type_of_list(&csharp_type);
                (pascal.clone(), elem_type, false, Some(pascal))
            }
            Some(LayoutPropValue::Expr(expr_src)) => {
                // Could be a for-bound name's member access, e.g.
                // `row.cells`. Lower it through ExprLowerer.
                match lower_expr_for_xbind(expr_src, ctx) {
                    ExprLowering::Bindable(path) => {
                        // Without further type info we can't determine the
                        // element type; default to `object` (the host's C#
                        // compiler will catch any real mismatch).
                        (path, "object".to_string(), false, None)
                    }
                    ExprLowering::Helper(_) | ExprLowering::Unsupported(_) => {
                        return Err(PipelineEmitError::UnsupportedExpression(format!(
                            "For each: expression {expr_src:?} cannot be lowered to a binding path"
                        )));
                    }
                }
            }
            // UI29 Â§3.4 â€” `each: <NAME>` where NAME is an enclosing For's
            // binding. The moslayout validator has already verified the
            // name is in scope. XAML's existing `ctx.for_scope` tracks the
            // matching `ForBinding`, so we can look up its `element_type`
            // for richer downstream binding info. If the name isn't found
            // in the scope (shouldn't happen since the validator gates it),
            // default to `object` the same way the Expr path does.
            Some(LayoutPropValue::Keyword(name)) => {
                let pascal = kebab_to_pascal_case(name);
                if !is_safe_identifier(&pascal) {
                    return Err(PipelineEmitError::UnsafeSlotName(pascal));
                }
                // GROUP B FIX (element-type peel). `each: <NAME>` where NAME
                // is an enclosing `For`'s `as:` binding (UI29 Â§3.4 â€” the
                // nested cell loop `For (each: row, as: v)`). The enclosing
                // binding's `element_type` is the type of NAME *itself*
                // (e.g. `row` is `IReadOnlyList<string>`). But THIS `For`
                // iterates over NAME's elements, so each `as:` element is
                // one level deeper â€” `v` is `string`, not the whole row
                // list. We must peel exactly one `List<>` level.
                //
                // The pre-fix code used `fb.element_type` verbatim, so the
                // inner value VM (`Grid_VVm`) typed its value field as
                // `IReadOnlyList<string>`. The cell then bound
                // `<TextBlock Text="{x:Bind V}"/>` â€” a `string` Text bound
                // to a list â€” which BLOCKS `dotnet build`. Peeling one
                // level types `V` as `string`, so the bind type-checks.
                let outer_type = ctx
                    .for_scope
                    .iter()
                    .rev()
                    .find(|fb| fb.as_name == *name)
                    .map(|fb| fb.element_type.clone())
                    .unwrap_or_else(|| "object".to_string());
                let elem_type = inner_type_of_list(&outer_type);
                (pascal, elem_type, true, None)
            }
            _ => {
                return Err(PipelineEmitError::UnsupportedPrimitive(
                    "For block must bind `each:` to a slot ref, an enclosing For binding, \
                 or an expression"
                        .to_string(),
                ));
            }
        };

    // -- 3. Generate / register the RowVm --
    let vm_class = format!("{}_{}Vm", ctx.component_name, kebab_to_pascal_case(as_name));
    let element_property = kebab_to_pascal_case(as_name);
    let has_index = index_name.is_some();

    let vm = RowVm {
        class_name: vm_class.clone(),
        element_property: element_property.clone(),
        element_type: element_type.clone(),
        has_index,
        // GROUP C: only the per-column cell loop's VM carries `Width`.
        has_width: is_cell_loop,
        has_is_selected: false,
    };
    if !ctx.row_vms.iter().any(|v| v.class_name == vm.class_name) {
        ctx.row_vms.push(vm);
    }

    let projection_property = slot_backed_items_source.as_ref().map(|source| {
        let prop = format!("{}Rows", vm_class.replace('_', ""));
        if !ctx
            .row_projections
            .iter()
            .any(|projection| projection.property_name == prop)
        {
            ctx.row_projections.push(RowProjection {
                property_name: prop.clone(),
                source_path: source.clone(),
                vm_class: vm_class.clone(),
                has_index,
                has_width: is_cell_loop,
                selected_index_path: None,
            });
        }
        prop
    });

    // -- 4. Push the binding into scope, walk the body, pop. --
    ctx.for_scope.push(ForBinding {
        as_name: as_name.to_string(),
        index_name: index_name.map(String::from),
        element_type,
        vm_class: vm_class.clone(),
        projection_property: projection_property.clone(),
    });
    let mut body =
        emit_xaml_single_content_children(&node.children, indent + 12, part_styles, ctx)?;
    ctx.for_scope.pop();

    // GROUP C: bind the fixed per-column width onto the rendered cell.
    // The cell element is the first opening tag of this loop's body â€”
    // either a kernel `<Border â€¦>` (when Cell.mll resolved inline) or
    // a component reference `<grid:Cell â€¦>` (both are FrameworkElements
    // and so have a `Width` property). Inject `Width="{x:Bind Width}"`
    // into that opening tag so the column renders at the colgroup's
    // fixed pixel width regardless of cell content.
    if is_cell_loop {
        body = inject_attr_into_first_element(&body, "Width=\"{x:Bind Width}\"");
    }

    // -- 5. Assemble the XAML --
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let pad3 = " ".repeat(indent + 8);
    let style = part_style_attr(node, part_styles);
    let items_source = projection_property.as_deref().unwrap_or(&items_path);
    let mut out = String::new();
    writeln!(
        out,
        "{pad}<ItemsRepeater ItemsSource=\"{{x:Bind {items_source}}}\"{style}>"
    )
    .unwrap();
    writeln!(out, "{pad2}<ItemsRepeater.ItemTemplate>").unwrap();
    writeln!(out, "{pad3}<DataTemplate x:DataType=\"local:{vm_class}\">").unwrap();
    out.push_str(&body);
    writeln!(out, "{pad3}</DataTemplate>").unwrap();
    writeln!(out, "{pad2}</ItemsRepeater.ItemTemplate>").unwrap();
    writeln!(out, "{pad}</ItemsRepeater>").unwrap();
    Ok(out)
}

/// Inject an extra attribute into the opening tag of the first XML
/// element in `body`. Used by GROUP C to bind `Width="{x:Bind Width}"`
/// onto a per-column cell element without re-plumbing every cell
/// emitter to thread a width argument.
///
/// The first element's opening tag is the first `<` that starts a tag
/// name (not a comment `<!--` and not a closing `</`). The attribute is
/// spliced just before that tag's terminating `>` (or `/>`), preserving
/// the existing attributes. If no suitable element is found the body is
/// returned unchanged.
///
/// Example: `inject_attr_into_first_element("  <Border A=\"1\">\nâ€¦", "W=\"2\"")`
/// â†’ `"  <Border A=\"1\" W=\"2\">\nâ€¦"`.
fn inject_attr_into_first_element(body: &str, attr: &str) -> String {
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Skip comments (`<!-- â€¦ -->`) and closing tags (`</â€¦>`).
            let next = bytes.get(i + 1).copied();
            if next == Some(b'!') || next == Some(b'/') {
                // Advance past this `<` and continue scanning.
                i += 1;
                continue;
            }
            // Found an opening tag at `i`. Locate its terminating `>`.
            if let Some(rel_close) = body[i..].find('>') {
                let close = i + rel_close;
                // `/>` self-closing: splice before the `/`.
                let insert_at = if close > 0 && bytes[close - 1] == b'/' {
                    close - 1
                } else {
                    close
                };
                let mut out = String::with_capacity(body.len() + attr.len() + 1);
                out.push_str(&body[..insert_at]);
                // Ensure a single separating space before the new attr.
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(attr);
                out.push_str(&body[insert_at..]);
                return out;
            }
            return body.to_string();
        }
        i += 1;
    }
    body.to_string()
}

/// `If (when: <expr>) { <then> } [Else { <else> }]` â†’ twin
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
        Some(LayoutPropValue::Expr(src)) => {
            if let Some(path) = try_lower_for_template_predicate(src, ctx) {
                path
            } else {
                match lower_expr_for_xbind(src, ctx) {
                    ExprLowering::Bindable(path) => path,
                    ExprLowering::Helper(call) => call,
                    ExprLowering::Unsupported(reason) => {
                        return Err(PipelineEmitError::UnsupportedExpression(reason));
                    }
                }
            }
        }
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
    let then_body =
        emit_xaml_single_content_children(&if_node.children, indent + 4, part_styles, ctx)?;

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
            emit_xaml_single_content_children(&else_node.children, indent + 4, part_styles, ctx)?;
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
fn emit_bool_to_vis_resource_block(indent: usize, resources_tag: &str) -> String {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let mut out = String::new();
    writeln!(out, "{pad}<{resources_tag}>").unwrap();
    writeln!(
        out,
        "{pad2}<local:BoolToVisibilityConverter x:Key=\"BoolToVisibilityConverter\"/>"
    )
    .unwrap();
    writeln!(out, "{pad}</{resources_tag}>").unwrap();
    out
}

/// C# source for the `BoolToVisibilityConverter` class. Emitted as a
/// sibling file in `XamlEmitResult::if_helpers` whenever the
/// generator wrote `{StaticResource BoolToVisibilityConverter}` into
/// the XAML (any `If` use, or any HostDialog with bound `open`). Fix
/// A5 from `code/programs/csharp/hello-dialog-xaml/ISSUES.md`.
///
/// Implements `IValueConverter` with optional `ConverterParameter`
/// support: passing `"invert"` flips the boolean before converting
/// to `Visibility`. That matches the `If`/`Else` lowering in Â§6.2.
fn emit_bool_to_vis_converter_source(namespace: &str) -> String {
    format!(
        "// Auto-generated by mosaic-emit-xaml. Do not edit.\n\
         //\n\
         // Bool â†’ Visibility converter. Used by every `If` / `Else` lowering and by\n\
         // HostDialog `open: slot:` bindings. ConverterParameter=\"invert\" flips the\n\
         // boolean before mapping (used by the Else branch of an If/Else pair).\n\
         using System;\n\
         using Microsoft.UI.Xaml;\n\
         using Microsoft.UI.Xaml.Data;\n\
         \n\
         namespace {namespace};\n\
         \n\
         public sealed class BoolToVisibilityConverter : IValueConverter\n\
         {{\n    \
             public object Convert(object value, Type targetType, object parameter, string language)\n    \
             {{\n        \
                 var b = value is bool x && x;\n        \
                 if (parameter is string p && p == \"invert\") b = !b;\n        \
                 return b ? Visibility.Visible : Visibility.Collapsed;\n    \
             }}\n\n    \
             public object ConvertBack(object value, Type targetType, object parameter, string language)\n    \
             {{\n        \
                 throw new NotImplementedException();\n    \
             }}\n\
         }}\n"
    )
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
    let index_field = if vm.has_index { ", int Index" } else { "" };
    // GROUP C: the per-column cell loop's VM carries a `double Width`
    // field. The generated cell element binds `Width="{x:Bind Width}"`,
    // so the host must populate Width with the matching column's pixel
    // width when it builds the VM instances.
    let width_field = if vm.has_width { ", double Width" } else { "" };
    let selected_field = if vm.has_is_selected {
        ", bool IsSelected"
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
    if vm.has_width {
        // The host-side VM-builder that POPULATES these instances (zipping
        // each cell value with its column index â†’ width) is host code the
        // emitter doesn't generate. Tell the Windows dev exactly how, in a
        // `<remarks>` the IDE surfaces on hover.
        writeln!(
            out,
            "/// <remarks>\n\
             /// GROUP C â€” fixed per-column widths. This VM carries a `Width`\n\
             /// (double) the cell element binds via `Width=\"{{x:Bind Width}}\"`.\n\
             /// The emitter does NOT generate the code that fills it â€” the\n\
             /// host builds these VMs per row and must zip each cell value\n\
             /// with its column index to look up the column's pixel width.\n\
             /// Example (inside the per-row VM builder):\n\
             /// <code>\n\
             /// for (int col = 0; col &lt; row.Count; col++)\n\
             ///     cells.Add(new {class_name}(row[col], col, ColumnWidths[col]));\n\
             /// </code>\n\
             /// where `ColumnWidths` is the host's `column-widths` slot\n\
             /// (e.g. [48, 96, 96, 96, 96, 96] for a gutter + five data\n\
             /// columns).\n\
             /// </remarks>"
        )
        .unwrap();
    }
    writeln!(
        out,
        "public sealed record {class_name}({element_type} {element_property}{index_field}{width_field}{selected_field});"
    )
    .unwrap();
    out
}

// =====================================================================
// --emit-project: generate a full WinUI 3 project shell (Fix B1)
// =====================================================================
//
// The CLI flag `--emit-project` flips `EmitOptions::emit_project`. When
// on, `from_pipeline` populates `XamlEmitResult::project` with a full
// set of host-project files (csproj, App.xaml(.cs), MainWindow.xaml(.cs),
// app.manifest, build.ps1, README.md). The CLI writes them next to the
// component triple. Result: a directory you can `dotnet build && run`
// to see the component on screen.
//
// What the host MainWindow does depends on the component's RootShape:
//   - `RootShape::UserControl` â†’ MainWindow's Grid hosts the component
//     directly as its content (full-window placement).
//   - `RootShape::ContentDialog` â†’ MainWindow has a button that
//     constructs the dialog (a ContentDialog under the hood), sets
//     its XamlRoot from the button (Fix D1), and ShowAsync's it.
//
// The component's slot DPs become host-set values in the generated
// MainWindow code; the user replaces these stubs with real values
// when filling in business logic. Sensible defaults:
//   - text slot â†’ "Sample <SlotName>"
//   - number slot â†’ 0
//   - bool slot â†’ false
//   - color slot â†’ /* TODO */ Windows.UI.Colors.Gray
//   - image slot â†’ null
//   - node slot â†’ null
//   - list<T> slot â†’ an empty array
//
// The component's emits are wired to a single `OnComponentDispatch`
// handler that pattern-matches the event union and updates a status
// TextBlock. The user replaces the match arms' bodies with real
// business logic.

/// Build the full `ProjectFiles` set for a component.
fn build_project_files(
    name: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    shape: RootShape,
    options: &EmitOptions,
) -> ProjectFiles {
    ProjectFiles {
        csproj: emit_csproj(name, options),
        app_xaml: emit_app_xaml(options),
        app_xaml_cs: emit_app_xaml_cs(options),
        main_window_xaml: emit_main_window_xaml(name, options, shape),
        main_window_cs: emit_main_window_cs(name, slots, emits, options, shape),
        package_manifest: emit_app_manifest(name),
        build_script: emit_build_script(name),
        readme: emit_project_readme(name, shape),
    }
}

fn emit_csproj(_name: &str, options: &EmitOptions) -> String {
    let ns = &options.namespace;
    let sdk_ver = if options.windows_app_sdk.is_empty() {
        "1.7.250606001"
    } else {
        options.windows_app_sdk.as_str()
    };
    format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n\
         \n\
           <!-- Auto-generated by mosaic-emit-xaml in emit-project mode. -->\n\
           <PropertyGroup>\n\
             <OutputType>WinExe</OutputType>\n\
             <TargetFramework>net9.0-windows10.0.19041.0</TargetFramework>\n\
             <RootNamespace>{ns}</RootNamespace>\n\
             <ApplicationManifest>app.manifest</ApplicationManifest>\n\
             <Platforms>x64</Platforms>\n\
             <RuntimeIdentifier>win-x64</RuntimeIdentifier>\n\
             <UseWinUI>true</UseWinUI>\n\
             <EnableMsixTooling>false</EnableMsixTooling>\n\
             <WindowsPackageType>None</WindowsPackageType>\n\
             <EnableCoreMrtTooling>false</EnableCoreMrtTooling>\n\
             <Nullable>enable</Nullable>\n\
             <ImplicitUsings>enable</ImplicitUsings>\n\
             <LangVersion>latest</LangVersion>\n\
             <!-- Framework-dependent: the Windows App Runtime must be installed\n\
                  system-wide (`winget install Microsoft.WindowsAppRuntime.1.7`).\n\
                  Self-contained bundling (<WindowsAppSDKSelfContained>true) is\n\
                  available, but the bundled Microsoft.UI.Xaml.dll 3.1.7.0 in the\n\
                  1.7 NuGet currently crashes on initialization (0xc000027b in\n\
                  Microsoft.UI.Xaml.dll). Use framework-dependent until that's\n\
                  resolved upstream. -->\n\
             <WindowsAppSDKSelfContained>false</WindowsAppSDKSelfContained>\n\
             <SelfContained>false</SelfContained>\n\
             <!-- WindowsAppSDK uses the legacy `win10-*` RIDs that .NET 8+\n\
                  removed from the default graph. UseRidGraph=true restores\n\
                  support for them. -->\n\
             <UseRidGraph>true</UseRidGraph>\n\
             <!-- Unpackaged WinUI does not need MrtCore PRI generation or MSIX\n\
                  packaging targets for this host shell. Keeping them disabled\n\
                  lets `dotnet build` work on SDK-only machines. -->\n\
             <AppxGeneratePriEnabled>false</AppxGeneratePriEnabled>\n\
             <EnableDefaultPriItems>false</EnableDefaultPriItems>\n\
           </PropertyGroup>\n\
         \n\
           <ItemGroup>\n\
             <PackageReference Include=\"Microsoft.WindowsAppSDK\" Version=\"{sdk_ver}\" />\n\
             <PackageReference Include=\"Microsoft.Windows.SDK.BuildTools\" Version=\"10.0.22621.756\" />\n\
           </ItemGroup>\n\
         \n\
           <!-- Fix B2: dotnet build leaves the native runtime DLLs in\n\
                runtimes/win-x64/native/. Flatten them next to the .exe so\n\
                the unpackaged bootstrap finds them at launch. -->\n\
           <Target Name=\"FlattenNativeRuntimeDlls\" AfterTargets=\"Build\">\n\
             <ItemGroup>\n\
               <_NativeRuntimeDlls Include=\"$(OutDir)runtimes/win-x64/native/*.dll\" />\n\
             </ItemGroup>\n\
             <Copy SourceFiles=\"@(_NativeRuntimeDlls)\" DestinationFolder=\"$(OutDir)\"\n\
                   SkipUnchangedFiles=\"true\" />\n\
           </Target>\n\
         \n\
           <!-- Mosaic host adapters can carry app-specific native DLLs, such as\n\
                an FFI bridge generated by the app's Rust core. If a package build\n\
                places those DLLs beside this project file, copy them next to the\n\
                executable so DllImport/NativeLibrary can resolve them at launch. -->\n\
           <Target Name=\"CopyMosaicNativeHostLibraries\" AfterTargets=\"Build\">\n\
             <ItemGroup>\n\
               <_MosaicNativeHostLibraries Include=\"$(MSBuildProjectDirectory)\\*.dll\" />\n\
             </ItemGroup>\n\
             <Copy SourceFiles=\"@(_MosaicNativeHostLibraries)\" DestinationFolder=\"$(OutDir)\"\n\
                   SkipUnchangedFiles=\"true\" />\n\
           </Target>\n\
         \n\
         </Project>\n"
    )
}

fn emit_app_xaml(options: &EmitOptions) -> String {
    let ns = &options.namespace;
    format!(
        "<!-- Auto-generated by mosaic-emit-xaml in emit-project mode. -->\n\
         <Application\n    \
             x:Class=\"{ns}.App\"\n    \
             xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"\n    \
             xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">\n    \
             <Application.Resources>\n        \
                 <ResourceDictionary>\n            \
                     <ResourceDictionary.MergedDictionaries>\n                \
                         <XamlControlsResources xmlns=\"using:Microsoft.UI.Xaml.Controls\"/>\n            \
                     </ResourceDictionary.MergedDictionaries>\n        \
                 </ResourceDictionary>\n    \
             </Application.Resources>\n\
         </Application>\n"
    )
}

fn emit_app_xaml_cs(options: &EmitOptions) -> String {
    let ns = &options.namespace;
    format!(
        "// Auto-generated by mosaic-emit-xaml in emit-project mode.\n\
         using Microsoft.UI.Xaml;\n\
         \n\
         namespace {ns};\n\
         \n\
         public partial class App : Application\n\
         {{\n    \
             private Window? _window;\n\
         \n    \
             public App()\n    \
             {{\n        \
                 this.InitializeComponent();\n    \
             }}\n\
         \n    \
             protected override void OnLaunched(LaunchActivatedEventArgs args)\n    \
             {{\n        \
                 _window = new MainWindow();\n        \
                 _window.Activate();\n    \
             }}\n\
         }}\n"
    )
}

fn emit_main_window_xaml(name: &str, options: &EmitOptions, shape: RootShape) -> String {
    let ns = &options.namespace;
    match shape {
        RootShape::ContentDialog => {
            // For a HostDialog-rooted component, the MainWindow has a
            // button and a status text. Click â†’ spawn the dialog.
            format!(
                "<!-- Auto-generated by mosaic-emit-xaml in emit-project mode. -->\n\
                 <Window\n    \
                     x:Class=\"{ns}.MainWindow\"\n    \
                     xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"\n    \
                     xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"\n    \
                     xmlns:local=\"using:{ns}\"\n    \
                     Title=\"{name} â€” Mosaic â†’ XAML demo\">\n    \
                     <Grid>\n        \
                         <Grid.RowDefinitions>\n            \
                             <RowDefinition Height=\"*\"/>\n            \
                             <RowDefinition Height=\"Auto\"/>\n        \
                         </Grid.RowDefinitions>\n        \
                         <TextBlock Grid.Row=\"0\" Margin=\"40\" FontSize=\"18\" TextWrapping=\"Wrap\"\n                   \
                                    Text=\"Mosaic-authored {name} dialog. Click the button to open it.\"/>\n        \
                         <TextBlock Grid.Row=\"1\" Margin=\"40,0,40,20\" x:Name=\"StatusText\" Foreground=\"#888\"\n                   \
                                    Text=\"Status: waiting for dispatchâ€¦\"/>\n        \
                         <Button Grid.Row=\"1\" HorizontalAlignment=\"Right\" Margin=\"0,0,40,20\"\n                \
                                 x:Name=\"OpenButton\" Content=\"Open the dialog\" Click=\"OnOpenButtonClick\"/>\n    \
                     </Grid>\n\
                 </Window>\n"
            )
        }
        RootShape::UserControl => {
            // Hosts the component directly as the window's content.
            format!(
                "<!-- Auto-generated by mosaic-emit-xaml in emit-project mode. -->\n\
                 <Window\n    \
                     x:Class=\"{ns}.MainWindow\"\n    \
                     xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"\n    \
                     xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"\n    \
                     xmlns:gen=\"using:{ns}\"\n    \
                     Title=\"{name} â€” Mosaic â†’ XAML demo\">\n    \
                     <Grid>\n        \
                         <Grid.RowDefinitions>\n            \
                             <RowDefinition Height=\"*\"/>\n            \
                             <RowDefinition Height=\"Auto\"/>\n        \
                         </Grid.RowDefinitions>\n        \
                         <gen:{name} Grid.Row=\"0\" x:Name=\"Component\"/>\n        \
                         <TextBlock Grid.Row=\"1\" Margin=\"20\" x:Name=\"StatusText\" Foreground=\"#888\"\n                   \
                                    Text=\"Status: waiting for dispatchâ€¦\"/>\n    \
                     </Grid>\n\
                 </Window>\n"
            )
        }
    }
}

fn emit_main_window_cs(
    name: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    options: &EmitOptions,
    shape: RootShape,
) -> String {
    let ns = &options.namespace;
    let component_ctor = build_component_constructor(name, slots);
    let dispatch_match = build_dispatch_match(name, emits);
    let host_helpers = build_optional_host_helpers(name, ns);

    match shape {
        RootShape::ContentDialog => {
            format!(
                "// Auto-generated by mosaic-emit-xaml in emit-project mode.\n\
                 //\n\
                 // STUB host for the {name} dialog. Replace the slot values in\n\
                 // ShowMosaicDialog() and the body of each match arm in\n\
                 // OnComponentDispatch with your real business logic.\n\
                 //\n\
                 using Microsoft.UI.Xaml;\n\
                 using Microsoft.UI.Xaml.Controls;\n\
                 \n\
                 namespace {ns};\n\
                 \n\
                 public sealed partial class MainWindow : Window\n\
                 {{\n    \
                     public MainWindow()\n    \
                     {{\n        \
                         this.InitializeComponent();\n    \
                     }}\n\
                 \n    \
                     private async void OnOpenButtonClick(object sender, RoutedEventArgs e)\n    \
                     {{\n        \
                         // Fix D1: use the button's XamlRoot â€” it's guaranteed in-tree at click time.\n        \
                         var xamlRoot = (sender as FrameworkElement)?.XamlRoot;\n        \
                         if (xamlRoot is null) {{ this.StatusText.Text = \"No XamlRoot on click sender\"; return; }}\n        \
                         try\n        \
                         {{\n            \
                             var dlg = {component_ctor};\n            \
                             var hostStatus = TryApplyMosaicHostProps(dlg);\n            \
                             if (hostStatus is not null) {{ this.StatusText.Text = hostStatus; }}\n            \
                             dlg.XamlRoot = xamlRoot;\n            \
                             dlg.Dispatch += OnComponentDispatch;\n            \
                             await dlg.ShowAsync();\n        \
                         }}\n        \
                         catch (System.Exception ex)\n        \
                         {{\n            \
                             this.StatusText.Text = $\"Exception: {{ex.GetType().Name}}: {{ex.Message}}\";\n        \
                         }}\n    \
                     }}\n\
                 \n    \
                     /// <summary>\n    \
                     /// Receives Mosaic Dispatch events. Replace each arm's body with the\n    \
                     /// business logic that should run when that event fires.\n    \
                     /// </summary>\n    \
                     private async void OnComponentDispatch(object? sender, {name}Event ev)\n    \
                     {{\n        \
                         var component = sender as {name};\n        \
                         var hostStatus = component is not null ? await TryHandleMosaicHostEvent(component, ev) : null;\n        \
                         if (hostStatus is not null) {{ this.StatusText.Text = hostStatus; return; }}\n        \
                         {dispatch_match}\n    \
                     }}\n\
                 \n    \
                     {host_helpers}\n\
                 }}\n"
            )
        }
        RootShape::UserControl => {
            format!(
                "// Auto-generated by mosaic-emit-xaml in emit-project mode.\n\
                 //\n\
                 // STUB host for the {name} component. The component is placed in the\n\
                 // window's Grid as `x:Name=\"Component\"`. Set its slot values in the\n\
                 // constructor below and fill in the OnComponentDispatch match arms.\n\
                 //\n\
                 using Microsoft.UI.Xaml;\n\
                 \n\
                 namespace {ns};\n\
                 \n\
                 public sealed partial class MainWindow : Window\n\
                 {{\n    \
                     public MainWindow()\n    \
                     {{\n        \
                         this.InitializeComponent();\n        \
                         var hostStatus = TryApplyMosaicHostProps(this.Component);\n        \
                         if (hostStatus is null)\n        \
                         {{\n            \
                             // Wire slot values: replace the stub defaults with your real data.\n            \
                             {component_ctor_inline}\n            \
                             hostStatus = \"Status: sample props loaded\";\n        \
                         }}\n        \
                         this.StatusText.Text = hostStatus;\n        \
                         this.Component.Dispatch += OnComponentDispatch;\n    \
                     }}\n\
                 \n    \
                     /// <summary>\n    \
                     /// Receives Mosaic Dispatch events. Replace each arm's body with the\n    \
                     /// business logic that should run when that event fires.\n    \
                     /// </summary>\n    \
                     private async void OnComponentDispatch(object? sender, {name}Event ev)\n    \
                     {{\n        \
                         var hostStatus = await TryHandleMosaicHostEvent(this.Component, ev);\n        \
                         if (hostStatus is not null) {{ this.StatusText.Text = hostStatus; return; }}\n        \
                         {dispatch_match}\n    \
                     }}\n\
                 \n    \
                     {host_helpers}\n\
                 }}\n",
                component_ctor_inline = build_component_inline_setup(slots),
            )
        }
    }
}

fn build_optional_host_helpers(name: &str, namespace: &str) -> String {
    let host_type = escape_csharp_string(&format!("{namespace}.MosaicHost"));
    format!(
        "private string? TryApplyMosaicHostProps({name} component)\n    \
         {{\n        \
             var method = FindMosaicHostMethod(\"ApplyProps\", typeof({name}));\n        \
             if (method is null) {{ return null; }}\n        \
             try\n        \
             {{\n            \
                 return CoerceMosaicHostResult(\n                \
                     method.Invoke(null, new object[] {{ component }}),\n                \
                     \"Status: Mosaic host props loaded\");\n        \
             }}\n        \
             catch (System.Reflection.TargetInvocationException ex) when (ex.InnerException is not null)\n        \
             {{\n            \
                 return $\"Mosaic host failed: {{ex.InnerException.GetType().Name}}: {{ex.InnerException.Message}}\";\n        \
             }}\n        \
             catch (System.Exception ex)\n        \
             {{\n            \
                 return $\"Mosaic host failed: {{ex.GetType().Name}}: {{ex.Message}}\";\n        \
             }}\n    \
         }}\n\
         \n    \
         private async System.Threading.Tasks.Task<string?> TryHandleMosaicHostEvent({name} component, {name}Event ev)\n    \
         {{\n        \
             var method = FindMosaicHostMethod(\"HandleEvent\", typeof({name}), typeof({name}Event));\n        \
             if (method is null) {{ return null; }}\n        \
             try\n        \
             {{\n            \
                 var result = await UnwrapMosaicHostResultAsync(method.Invoke(null, new object[] {{ component, ev }}));\n            \
                 var status = CoerceMosaicHostResult(result, $\"Status: Mosaic host handled {{ev.MosaicName}}\");\n            \
                 var intent = GetMosaicHostIntent(result);\n            \
                 if (intent is not null)\n            \
                 {{\n                \
                     var intentStatus = await TryHandleMosaicHostIntent(component, intent);\n                \
                     if (intentStatus is not null) {{ return intentStatus; }}\n            \
                 }}\n            \
                 return status;\n        \
             }}\n        \
             catch (System.Reflection.TargetInvocationException ex) when (ex.InnerException is not null)\n        \
             {{\n            \
                 return $\"Mosaic host failed: {{ex.InnerException.GetType().Name}}: {{ex.InnerException.Message}}\";\n        \
             }}\n        \
             catch (System.Exception ex)\n        \
             {{\n            \
                 return $\"Mosaic host failed: {{ex.GetType().Name}}: {{ex.Message}}\";\n        \
             }}\n    \
         }}\n\
         \n    \
         private async System.Threading.Tasks.Task<string?> TryHandleMosaicHostIntent({name} component, object hostIntent)\n    \
         {{\n        \
             var hostType = FindMosaicHostType();\n        \
             if (hostType is null) {{ return null; }}\n        \
             var method = FindMosaicHostIntentMethod(hostType, hostIntent.GetType(), typeof({name}));\n        \
             if (method is null) {{ return null; }}\n        \
             try\n        \
             {{\n            \
                 var result = await UnwrapMosaicHostResultAsync(method.Invoke(null, new object[] {{ this, component, hostIntent }}));\n            \
                 return CoerceMosaicHostResult(result, \"Status: Mosaic host handled host intent\");\n        \
             }}\n        \
             catch (System.Reflection.TargetInvocationException ex) when (ex.InnerException is not null)\n        \
             {{\n            \
                 return $\"Mosaic host intent failed: {{ex.InnerException.GetType().Name}}: {{ex.InnerException.Message}}\";\n        \
             }}\n        \
             catch (System.Exception ex)\n        \
             {{\n            \
                 return $\"Mosaic host intent failed: {{ex.GetType().Name}}: {{ex.Message}}\";\n        \
             }}\n    \
         }}\n\
         \n    \
         private static async System.Threading.Tasks.Task<object?> UnwrapMosaicHostResultAsync(object? result)\n    \
         {{\n        \
             if (result is System.Threading.Tasks.Task task)\n        \
             {{\n            \
                 await task.ConfigureAwait(true);\n            \
                 return task.GetType().GetProperty(\"Result\")?.GetValue(task);\n        \
             }}\n        \
             return result;\n    \
         }}\n\
         \n    \
         private static object? GetMosaicHostIntent(object? result)\n    \
         {{\n        \
             if (result is null || result is string) {{ return null; }}\n        \
             return result.GetType().GetProperty(\n            \
                 \"HostIntent\",\n            \
                 System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public)?.GetValue(result);\n    \
         }}\n\
         \n    \
         private static string CoerceMosaicHostResult(object? result, string fallbackStatus)\n    \
         {{\n        \
             if (result is null) {{ return fallbackStatus; }}\n        \
             if (result is string status) {{ return status; }}\n        \
             var statusProperty = result.GetType().GetProperty(\n            \
                 \"Status\",\n            \
                 System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public);\n        \
             return statusProperty?.GetValue(result) as string ?? fallbackStatus;\n    \
         }}\n\
         \n    \
         private static System.Type? FindMosaicHostType()\n    \
         {{\n        \
             return System.Type.GetType(\"{host_type}\");\n    \
         }}\n\
         \n    \
         private static System.Reflection.MethodInfo? FindMosaicHostMethod(string methodName, params System.Type[] parameterTypes)\n    \
         {{\n        \
             var hostType = FindMosaicHostType();\n        \
             if (hostType is null) {{ return null; }}\n        \
             return hostType.GetMethod(\n            \
                 methodName,\n            \
                 System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static,\n            \
                 binder: null,\n            \
                 types: parameterTypes,\n            \
                 modifiers: null);\n    \
         }}\n\
         \n    \
         private static System.Reflection.MethodInfo? FindMosaicHostIntentMethod(\n        \
             System.Type hostType,\n        \
             System.Type hostIntentType,\n        \
             System.Type componentType)\n    \
         {{\n        \
             foreach (var method in hostType.GetMethods(System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static))\n        \
             {{\n            \
                 if (method.Name != \"HandleHostIntent\") {{ continue; }}\n            \
                 var parameters = method.GetParameters();\n            \
                 if (parameters.Length != 3) {{ continue; }}\n            \
                 if (!parameters[0].ParameterType.IsAssignableFrom(typeof(Window))) {{ continue; }}\n            \
                 if (!parameters[1].ParameterType.IsAssignableFrom(componentType)) {{ continue; }}\n            \
                 if (!parameters[2].ParameterType.IsAssignableFrom(hostIntentType)) {{ continue; }}\n            \
                 return method;\n        \
             }}\n        \
             return null;\n    \
         }}"
    )
}

/// Build a `new ComponentName { Slot = default, ... }` initializer
/// for the ContentDialog-rooted MainWindow path.
fn build_component_constructor(name: &str, slots: &[SlotDecl]) -> String {
    if slots.is_empty() {
        return format!("new {name}()");
    }
    let mut out = format!("new {name}\n            {{\n");
    let ctx_stub = EmitContext::new("", &[], &[]);
    for slot in slots {
        // Use the same aliased PascalCase the DP generator emits.
        let pascal = if let Some(alias) = ctx_stub.slot_aliases.get(&slot.name) {
            alias.clone()
        } else {
            kebab_to_pascal_case(&slot.name)
        };
        let value = stub_value_for_slot(&slot.r#type, &slot.name);
        out.push_str(&format!("                {pascal} = {value},\n"));
    }
    out.push_str("            }");
    out
}

/// Build `this.Component.Slot = default;` statements for the
/// UserControl-rooted MainWindow path.
fn build_component_inline_setup(slots: &[SlotDecl]) -> String {
    if slots.is_empty() {
        return String::from("// (no slots)");
    }
    let mut lines: Vec<String> = Vec::with_capacity(slots.len());
    for slot in slots {
        let pascal = kebab_to_pascal_case(&slot.name);
        let value = stub_value_for_slot(&slot.r#type, &slot.name);
        lines.push(format!("this.Component.{pascal} = {value};"));
    }
    lines.join("\n        ")
}

/// Pick a reasonable stub literal for a slot's C# value. Used to
/// pre-populate the host MainWindow's component instance.
fn stub_value_for_slot(t: &SlotType, slot_name: &str) -> String {
    match t {
        SlotType::Text => format!("\"Sample {}\"", kebab_to_pascal_case(slot_name)),
        SlotType::Number => "0".to_string(),
        SlotType::Bool => "false".to_string(),
        SlotType::Color => "Microsoft.UI.Colors.Gray".to_string(),
        SlotType::Image => "null!".to_string(),
        SlotType::Node => "null!".to_string(),
        SlotType::List(inner) => format!(
            "new System.Collections.Generic.List<{}>()",
            list_inner_csharp(inner)
        ),
        // Component slots (rare) â€” surface a stub null for now.
        _ => "null!".to_string(),
    }
}

fn list_inner_csharp(t: &ListInnerType) -> String {
    match t {
        ListInnerType::Text => "string".to_string(),
        ListInnerType::Number => "double".to_string(),
        ListInnerType::Bool => "bool".to_string(),
        ListInnerType::Color => "Windows.UI.Color".to_string(),
        ListInnerType::Image => "Microsoft.UI.Xaml.Media.Imaging.ImageSource".to_string(),
        ListInnerType::Node => "Microsoft.UI.Xaml.UIElement".to_string(),
        ListInnerType::Component(c) => c.clone(),
        ListInnerType::List(inner) => format!(
            "System.Collections.Generic.IReadOnlyList<{}>",
            list_inner_csharp(inner)
        ),
    }
}

/// Build the body of `OnComponentDispatch` â€” a `switch (ev) { ... }`
/// over the emit cases. Each arm sets `this.StatusText.Text` to a
/// stub label and has a `/* TODO: business logic */` comment.
fn build_dispatch_match(name: &str, emits: &[EmitDecl]) -> String {
    if emits.is_empty() {
        return format!(
            "// {name} declares no emits â€” Dispatch never fires.\n        \
             this.StatusText.Text = $\"Dispatched (no emits declared): {{ev}}\";"
        );
    }
    let mut out = String::from("switch (ev)\n        {\n");
    for emit in emits {
        let case_name = kebab_to_pascal_case(&strip_on_prefix(&emit.name));
        if emit.params.is_empty() {
            out.push_str(&format!(
                "            case {name}Event.{case_name}:\n                \
                     this.StatusText.Text = \"Dispatch: {case_name}\";\n                \
                     // TODO: business logic for {case_name}\n                \
                     break;\n"
            ));
        } else {
            let pattern_args: Vec<String> = emit
                .params
                .iter()
                .enumerate()
                .map(|(idx, _)| format!("var payload{idx}"))
                .collect();
            let pattern = pattern_args.join(", ");
            out.push_str(&format!(
                "            case {name}Event.{case_name}({pattern}) c:\n                \
                     this.StatusText.Text = $\"Dispatch: {case_name}({{c}})\";\n                \
                     // TODO: business logic for {case_name}\n                \
                     break;\n"
            ));
        }
    }
    out.push_str("        }");
    out
}

fn emit_app_manifest(_name: &str) -> String {
    // DPI awareness + supported-OS GUID for Windows 10 / 11.
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
     <assembly manifestVersion=\"1.0\" xmlns=\"urn:schemas-microsoft-com:asm.v1\">\n  \
       <application xmlns=\"urn:schemas-microsoft-com:asm.v3\">\n    \
         <windowsSettings>\n      \
           <dpiAware xmlns=\"http://schemas.microsoft.com/SMI/2005/WindowsSettings\">true/pm</dpiAware>\n      \
           <dpiAwareness xmlns=\"http://schemas.microsoft.com/SMI/2016/WindowsSettings\">PerMonitorV2, PerMonitor</dpiAwareness>\n    \
         </windowsSettings>\n  \
       </application>\n  \
       <compatibility xmlns=\"urn:schemas-microsoft-com:compatibility.v1\">\n    \
         <application>\n      \
           <supportedOS Id=\"{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}\"/>\n    \
         </application>\n  \
       </compatibility>\n\
     </assembly>\n"
        .to_string()
}

fn emit_build_script(name: &str) -> String {
    format!(
        "# Auto-generated by mosaic-emit-xaml in emit-project mode.\n\
         #\n\
         # Driver for the {name} WinUI 3 host project. Regenerates the per-component\n\
         # triple from the Mosaic sources (if mosaic-compile is on PATH), then runs\n\
         # `dotnet build`.\n\
         #\n\
         # Prerequisite:\n\
         #   - The generated project is framework-dependent, so a system-wide\n\
         #     install of the Windows App Runtime is required to run it:\n\
         #         winget install Microsoft.WindowsAppRuntime.1.7\n\
         #\n\
         param([switch]$Clean, [switch]$Run)\n\
         \n\
         $ErrorActionPreference = \"Stop\"\n\
         $proj = Join-Path $PSScriptRoot \"{name}.csproj\"\n\
         $dotnet = (Get-Command dotnet -ErrorAction SilentlyContinue).Source\n\
         if (-not $dotnet) {{\n    \
             $defaultDotnet = Join-Path $env:ProgramFiles \"dotnet\\dotnet.exe\"\n    \
             if (Test-Path $defaultDotnet) {{\n        \
                 $dotnet = $defaultDotnet\n    \
             }}\n\
         }}\n\
         if (-not $dotnet) {{\n    \
             Write-Error \"dotnet was not found on PATH or at $env:ProgramFiles\\dotnet\\dotnet.exe\"\n    \
             exit 127\n\
         }}\n\
         \n\
         if ($Clean) {{\n    \
             Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $PSScriptRoot \"bin\")\n    \
             Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $PSScriptRoot \"obj\")\n\
         }}\n\
         \n\
         # The Platform=x64 arg is required because WindowsAppSDK's\n\
         # self-contained mode rejects AnyCPU. The csproj's <Platforms>x64</Platforms>\n\
         # only declares the SET of platforms; the ACTIVE one comes from this arg.\n\
         & $dotnet build $proj -c Debug -p:Platform=x64 --nologo\n\
         $buildExitCode = $LASTEXITCODE\n\
         if ($buildExitCode -ne 0) {{\n    \
             exit $buildExitCode\n\
         }}\n\
         \n\
         if ($Run) {{\n    \
             # x64 Debug bin path. .NET 9 puts the platform in the path\n    \
             # segment when -p:Platform=x64 is set.\n    \
             $exe = Join-Path $PSScriptRoot \"bin\\x64\\Debug\\net9.0-windows10.0.19041.0\\win-x64\\{name}.exe\"\n    \
             if (-not (Test-Path $exe)) {{\n        \
                 # Fallback to the non-platform-segment path for builds that\n        \
                 # left it out.\n        \
                 $exe = Join-Path $PSScriptRoot \"bin\\Debug\\net9.0-windows10.0.19041.0\\win-x64\\{name}.exe\"\n    \
             }}\n    \
             if (Test-Path $exe) {{\n        \
                 & $exe\n    \
                 $runExitCode = $LASTEXITCODE\n        \
                 if ($runExitCode -ne 0) {{\n            \
                     exit $runExitCode\n        \
                 }}\n    \
             }} else {{\n        \
                 Write-Host \"No .exe at $exe -- build must have failed.\" -ForegroundColor Red\n    \
                 exit 1\n    \
             }}\n\
         }}\n"
    )
}

fn emit_project_readme(name: &str, shape: RootShape) -> String {
    let shape_blurb = match shape {
        RootShape::ContentDialog => {
            "This project hosts a Mosaic-authored **dialog component** ({name}).\n\
             The MainWindow contains a button â€” click it to display the dialog\n\
             (a `<ContentDialog>` underneath).\n"
        }
        RootShape::UserControl => {
            "This project hosts a Mosaic-authored **UserControl** ({name}) placed\n\
             directly in the MainWindow as the full content area.\n"
        }
    };
    let shape_blurb = shape_blurb.replace("{name}", name);
    format!(
        "# {name} â€” WinUI 3 host project\n\
         \n\
         Auto-generated by `mosaic-compile --backend xaml --emit-project`.\n\
         \n\
         {shape_blurb}\n\
         ## Prerequisites\n\
         \n\
         1. **.NET 9.0 SDK** â€” `dotnet --list-sdks` should list one matching `9.0.*`.\n\
         2. The Windows App Runtime 1.7 installed system-wide:\n\
            `winget install Microsoft.WindowsAppRuntime.1.7`.\n\
         3. Visual Studio Build Tools 2022 is useful when opening the project in\n\
            Visual Studio, but `.\\build.ps1` uses `dotnet build` and keeps the\n\
            unpackaged MSIX / PRI tooling disabled.\n\
         \n\
         ## Build\n\
         \n\
         ```powershell\n\
         .\\build.ps1            # builds\n\
         .\\build.ps1 -Run       # builds + runs\n\
         .\\build.ps1 -Clean     # deletes bin/ + obj/\n\
         ```\n\
         \n\
         The build emits a framework-dependent .exe at\n\
         `bin\\x64\\Debug\\net9.0-windows10.0.19041.0\\win-x64\\{name}.exe`. The native\n\
         WindowsAppRuntime DLLs are auto-flattened next to the .exe by an\n\
         MSBuild post-build target (see the project's `.csproj`).\n\
         \n\
         ## Where to add business logic\n\
         \n\
         - **`MainWindow.xaml.cs`** has stub slot values for the component and a\n\
           stub `OnComponentDispatch` handler. Replace the stub values with your\n\
           real data, and fill in the body of each match arm with the logic that\n\
           should run when each Mosaic emit fires.\n\
         - The `{name}.xaml.cs` and `{name}.xaml` files are auto-generated\n\
           from the Mosaic sources and **should NOT be edited by hand** â€” they\n\
           get overwritten on the next `mosaic-compile` run.\n\
         \n\
         ## Files\n\
         \n\
         | File | Source | Edit by hand? |\n\
         |---|---|---|\n\
         | `{name}.xaml` | mosaic-compile | No |\n\
         | `{name}.xaml.cs` | mosaic-compile | No |\n\
         | `{name}.Event.cs` | mosaic-compile | No |\n\
         | `MainWindow.xaml(.cs)` | --emit-project | **Yes** â€” your host |\n\
         | `App.xaml(.cs)` | --emit-project | Rare |\n\
         | `{name}.csproj` | --emit-project | Rare |\n\
         | `app.manifest` | --emit-project | Rare |\n\
         | `build.ps1` | --emit-project | Rare |\n"
    )
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// ExprLowerer â€” UI29 Â§3.3 expression source â†’ {x:Bind} path or helper
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// The moslayout-compiler stores `Expr` as the source-text substring
// (tokens joined with spaces). It can be:
//
//   - bare name              `row`                                â†’ Bindable("Row")
//   - bare slot ref          `slot: editable`                     â†’ Bindable("Editable")
//   - boolean literal        `true` / `false`                     â†’ Bindable("True"/"False")
//   - dotted access          `row.value` / `slot: theme.dark`     â†’ Bindable("Row.Value" / "Theme.Dark")
//   - indexer                `row[c]` / `slot: rows[r][c]`        â†’ Helper("GetXxx(...)")
//   - comparisons            `r == slot: edit-row`                â†’ Helper("IsXxx(...)")
//   - logical &&/||/!        `a && b`                             â†’ Helper("Combined(...)")
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
    /// A helper-method call expression in C# form â€” `{x:Bind GetCell(R, C)}`
    /// is the consumer; the helper itself has been registered with the
    /// EmitContext.
    Helper(String),
    /// The expression couldn't be lowered. Carries a human-readable
    /// reason for the diagnostic.
    Unsupported(String),
}

fn try_lower_for_template_predicate(src: &str, ctx: &mut EmitContext<'_>) -> Option<String> {
    let tokens = tokenise_expr(src.trim()).ok()?;
    let selected_path = match tokens.as_slice() {
        [ExprTok::Name(left), ExprTok::EqEq, ExprTok::Name(right)]
            if ctx.lookup_for_index(left).is_some() =>
        {
            kebab_to_pascal_case(right)
        }
        [ExprTok::Name(left), ExprTok::EqEq, ExprTok::SlotPrefix, ExprTok::Name(right)]
            if ctx.lookup_for_index(left).is_some() =>
        {
            ctx.slot_xbind_path(right)
        }
        [ExprTok::Name(left), ExprTok::EqEq, ExprTok::Name(right)]
            if ctx.lookup_for_index(right).is_some() =>
        {
            kebab_to_pascal_case(left)
        }
        [ExprTok::SlotPrefix, ExprTok::Name(left), ExprTok::EqEq, ExprTok::Name(right)]
            if ctx.lookup_for_index(right).is_some() =>
        {
            ctx.slot_xbind_path(left)
        }
        _ => return None,
    };

    let binding = ctx.for_scope.last()?;
    let projection_property = binding.projection_property.clone()?;
    if let Some(vm) = ctx
        .row_vms
        .iter_mut()
        .find(|vm| vm.class_name == binding.vm_class)
    {
        vm.has_is_selected = true;
    }
    if let Some(projection) = ctx
        .row_projections
        .iter_mut()
        .find(|projection| projection.property_name == projection_property)
    {
        projection.selected_index_path = Some(selected_path);
        return Some("IsSelected".to_string());
    }
    None
}

/// Lower a raw expression source string to its WinUI 3 binding form.
///
/// This is a small recursive-descent parser over the UI29 Â§3.3 grammar
/// (or-expr â†’ and-expr â†’ eq-expr â†’ rel-expr â†’ unary â†’ postfix â†’ primary).
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
                ExprLowering::Unsupported(format!("expression {src:?} has trailing tokens"))
            }
        }
        Err(e) => ExprLowering::Unsupported(e),
    }
}

/// Tokens emitted by the tiny expression lexer.
#[derive(Debug, Clone, PartialEq)]
enum ExprTok {
    Name(String),
    SlotPrefix, // `slot:` (yes the colon is part of the prefix as seen by the lexer)
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
/// that we don't need a generated lexer â€” a hand-rolled one fits in a
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
                // String literal â€” collect until the closing quote,
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
                    return Err(format!("unterminated string literal in expression {src:?}"));
                }
                let lit = src[start..i].to_string();
                out.push(ExprTok::String(lit));
                i += 1; // skip closing "
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                out.push(ExprTok::Number(src[start..i].to_string()));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
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
    /// The original source string â€” used in error messages and helper
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

    /// Parse the entire expression â€” only the recursive-descent entry
    /// point a caller invokes. Returns the lowered form for the whole
    /// expression.
    fn parse_or(&mut self) -> Result<ExprLowering, String> {
        // For PR-2 the parser is simplified: we look for the *shape* of
        // the expression and decide on a lowering strategy in one pass.
        //
        // - If every token after the first primary is `.NAME`, we have
        //   a pure member-access path â†’ Bindable.
        // - If there's an indexer / comparison / logical / unary-not,
        //   we register a helper method and return Helper(call).
        // - The fallback is Unsupported with a clear reason.

        if self.contains_logical_or_comparison()
            || self.contains_indexer()
            || self.starts_with_not()
        {
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
            // UI28-1 / U29-D1 â€” accept a parenthesised primary so that
            // single-NAME grouping like `( h )` (used by mosaic-pkg-grid
            // v0.2.0's Grid.mll and the VisiCalc demo's inlined copy)
            // resolves cleanly through the XAML {x:Bind} path. The
            // moslayout parser turns `( h )` into Expr because the
            // `(...)` grouping triggers the Expr branch (UI29 Â§3.3);
            // the XAML emitter previously rejected the resulting
            // LParen primary as "unsupported primary token". A
            // parenthesised primary is just its inner primary â€”
            // recurse and consume the matching RParen.
            ExprTok::LParen => {
                self.pos += 1;
                let inner = self.parse_primary_bindable()?;
                if !self.consume(&ExprTok::RParen) {
                    return Err(format!(
                        "expression {:?} has unmatched LParen â€” expected RParen after {:?}",
                        self.src, inner
                    ));
                }
                Ok(inner)
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
                // Otherwise: a name with no binding â€” leave it for
                // transliteration to surface as `this.<name>` if it's a
                // slot, or as a literal if it's something else.
            }
        }

        // Determine the return type. PR-2 supports two shapes:
        //   - logical / comparison â†’ bool
        //   - indexer (X[idx])     â†’ string (default; downstream type
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
                // If it's a for-bound name or index, leave it bare â€”
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
// - HostInput  â†’ <TextBox>     (spec Â§4.1)
// - HostButton â†’ <Button>      (spec Â§4.2)
// - HostScroll â†’ <ScrollViewer> (spec Â§4.3)
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
            // X2 fix: when the pascal-cased part name collides with the
            // enclosing component class name (e.g. component `Button`
            // with part `button`), WinUI's XAML compiler generates a
            // `private â€¦ {pascal} {pascal};` field that triggers C#
            // error CS0542 ("member names cannot be the same as their
            // enclosing type"). Suffix `Element` to disambiguate; the
            // `_Click` handler stem is derived from `x_name` so the
            // .xaml.cs stays consistent automatically. Caught by the
            // toolkit Button + Checkbox + Input + Radio demo (#4548).
            if pascal == ctx.component_name {
                return format!("{pascal}Element");
            }
            return pascal;
        }
    }
    let n = ctx.next_host_counter();
    format!("{tag}_{n}")
}

/// `HostInput` â†’ `<TextBox>` per spec Â§4.1.
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

    // value: slot/string/expr â†’ Text binding
    match find_prop_value(node, "value") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" Text=\"{{x:Bind {pascal}, Mode=TwoWay}}\""));
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
        attrs.push_str(&format!(" PlaceholderText=\"{}\"", escape_xaml_attr(s)));
    }

    // max-length: number
    if let Some(LayoutPropValue::Number(n)) = find_prop_value(node, "max-length") {
        let i = *n as i64;
        attrs.push_str(&format!(" MaxLength=\"{i}\""));
    }

    // multiline: true â†’ AcceptsReturn + TextWrapping
    if find_prop_keyword(node, "multiline") == Some("true") {
        attrs.push_str(" AcceptsReturn=\"True\" TextWrapping=\"Wrap\"");
    }

    // -- Event wiring --
    // onChange handler dispatches with the new text payload.
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onChange") {
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

    // onCommit / onCancel â†’ merged KeyDown handler keyed on Enter / Escape.
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

    // onFocus â†’ GotFocus
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onFocus") {
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

/// `HostButton` â†’ `<Button>` per spec Â§4.2.
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
            if ctx.lookup_for_binding(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}}}\""));
            } else if ctx.lookup_for_index(k).is_some() {
                attrs.push_str(" Content=\"{x:Bind Index}\"");
            } else {
                attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
            }
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
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind Not({pascal})}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    // onClick â†’ Click handler
    if let Some(LayoutPropValue::EmitRef(emit_name)) =
        find_prop_value(node, "onClick").or_else(|| find_prop_value(node, "onTap"))
    {
        let handler = format!("{x_name}_Click");
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let event_ctor = if let Some(payload_expr) = host_button_click_payload_expr(emit_name, ctx)
        {
            format!("new {component}Event.{case_pascal}({payload_expr})")
        } else {
            format!("new {component}Event.{case_pascal}()")
        };
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, {event_ctor});\n    }}"
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

fn host_button_click_payload_expr(emit_name: &str, ctx: &EmitContext<'_>) -> Option<String> {
    let params = ctx.emit_payloads.get(emit_name)?;
    if params.len() != 1 {
        return None;
    }

    let (param_name, param_type) = &params[0];
    let binding = ctx.for_scope.iter().rev().find(|binding| {
        binding.index_name.is_some() || matches!(param_type.as_str(), "string" | "double" | "bool")
    })?;
    let vm_class = &binding.vm_class;
    let element_property = kebab_to_pascal_case(&binding.as_name);
    match (param_name.as_str(), param_type.as_str()) {
        ("index", "double") if binding.index_name.is_some() => Some(format!(
            "(sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is {vm_class} row ? (double)row.Index : -1.0"
        )),
        (_, "string") => Some(format!(
            "(sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is {vm_class} row ? row.{element_property} : string.Empty"
        )),
        (_, "double") if binding.element_type == "double" => Some(format!(
            "(sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is {vm_class} row ? row.{element_property} : 0.0"
        )),
        (_, "bool") if binding.element_type == "bool" => Some(format!(
            "(sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is {vm_class} row && row.{element_property}"
        )),
        _ => None,
    }
}

/// `HostCheckbox` â†’ WinUI / WPF `<CheckBox>` per UI29-2.
///
/// ## Property handling
///
/// | moslayout prop          | XAML                                                        |
/// |---|---|
/// | `checked: slot: c`      | `IsChecked="{x:Bind C, Mode=OneWay}"`                       |
/// | `checked: true/false`   | `IsChecked="True"` / `IsChecked="False"`                    |
/// | `disabled: slot: d`     | `IsEnabled="{x:Bind Not(D)}"` (shared Not(bool) helper)     |
/// | `disabled: true/false`  | `IsEnabled="False"` / `IsEnabled="True"`                    |
/// | `indeterminate: slot:i` | `IsThreeState="True"` + binding via code-behind             |
/// | `label: str / slot`     | `Content="..."` / `Content="{x:Bind Label}"`                |
/// | `onToggle: emit: onX`   | `Checked="X_Checked" Unchecked="X_Unchecked"` handler pair  |
///
/// ## Checked vs. Unchecked event split
///
/// WinUI's `<CheckBox>` has separate `Checked(object, RoutedEventArgs)`
/// and `Unchecked(object, RoutedEventArgs)` events â€” there is no
/// "toggled with new value" combined event. We register **two**
/// code-behind handlers per `onToggle` binding: `<X>_Checked` fires
/// `Dispatch(.x(checked: true))` and `<X>_Unchecked` fires
/// `Dispatch(.x(checked: false))`. This matches the kernel-canonical
/// `onToggle(checked: bool)` signature exactly.
///
/// (`Indeterminate` event is intentionally NOT wired â€” the tri-state
/// case only fires `Indeterminate` when the user clicks through to the
/// third state, which is a UX choice the host can drive via the
/// `indeterminate:` slot. v1 ignores `Indeterminate` events.)
fn host_link_click_payload_expr(
    emit_name: &str,
    node: &LayoutNode,
    ctx: &EmitContext<'_>,
) -> Option<String> {
    let params = ctx.emit_payloads.get(emit_name)?;
    if params.is_empty() {
        return None;
    }
    if params.len() != 1 {
        return None;
    }

    let (param_name, param_type) = &params[0];
    if param_name == "href" && param_type == "string" {
        return Some(host_link_href_payload_expr(node));
    }
    if let Some(expr) = host_button_click_payload_expr(emit_name, ctx) {
        return Some(expr);
    }
    if param_type == "string" {
        return Some(host_link_href_payload_expr(node));
    }
    None
}

fn host_link_href_payload_expr(node: &LayoutNode) -> String {
    match find_prop_value(node, "href") {
        Some(LayoutPropValue::String(s)) => {
            format!("\"{}\"", escape_csharp_string(s))
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            format!("this.{pascal}")
        }
        _ => "\"\"".to_string(),
    }
}

fn emit_host_checkbox(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostCheckbox", ctx);

    let mut attrs = String::new();

    // Content (label).
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

    // IsChecked from `checked:`.
    match find_prop_value(node, "checked") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" IsChecked=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsChecked=\"True\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsChecked=\"False\"");
        }
        _ => {}
    }

    // IsEnabled from `disabled:` (polarity flip; reuses HostButton's
    // Not(bool) helper).
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            ctx.add_helper(HelperMethod {
                name: "Not".to_string(),
                parameters: vec![("b".to_string(), "bool".to_string())],
                return_type: "bool".to_string(),
                body: "!b".to_string(),
            });
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind Not({pascal})}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    // IsThreeState + tri-state from `indeterminate:`. The visual
    // tri-state is enabled by `IsThreeState="True"`; the actual
    // "show as indeterminate" toggle is driven via code-behind reading
    // the bound slot. v1 emits the bare IsThreeState attr; the host
    // owns IsChecked transitions.
    let enable_three_state = match find_prop_value(node, "indeterminate") {
        Some(LayoutPropValue::SlotRef(_)) => true,
        Some(LayoutPropValue::Keyword(k)) if k == "true" => true,
        _ => false,
    };
    if enable_three_state {
        attrs.push_str(" IsThreeState=\"True\"");
    }

    // Checked + Unchecked handlers from `onToggle:`. WinUI splits the
    // toggle into two events; we wire both to dispatch with the
    // matching `checked: bool` payload value (true for Checked, false
    // for Unchecked) so the kernel-canonical UI29-2 Â§2.2 emit signature
    // is satisfied.
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onToggle") {
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;

        let checked_handler = format!("{x_name}_Checked");
        let checked_body = format!(
            "    private void {checked_handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}(true));\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: checked_handler.clone(),
            source: checked_body,
        });

        let unchecked_handler = format!("{x_name}_Unchecked");
        let unchecked_body = format!(
            "    private void {unchecked_handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}(false));\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: unchecked_handler.clone(),
            source: unchecked_body,
        });

        attrs.push_str(&format!(
            " Checked=\"{checked_handler}\" Unchecked=\"{unchecked_handler}\""
        ));
    }

    Ok(format!(
        "{pad}<CheckBox x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

/// `HostRadio` â†’ WinUI / WPF `<RadioButton>` per UI29-2.
///
/// ## Property handling
///
/// | moslayout prop          | XAML                                                       |
/// |---|---|
/// | `checked: slot: c`      | `IsChecked="{x:Bind C, Mode=OneWay}"`                      |
/// | `group: "..."`          | `GroupName="..."` â€” WinUI's native radio-mutex attribute   |
/// | `group: slot: g`        | `GroupName="{x:Bind G}"`                                   |
/// | `value: ... / slot:`    | recorded in source as a `<!-- value: ... -->` annotation   |
/// | `disabled: ...`         | `IsEnabled` â€” same shape as HostCheckbox                   |
/// | `label: ...`            | `Content=...` â€” same shape as HostCheckbox                 |
/// | `onSelect: emit: onX`   | `Checked="X_Checked"` (only â€” Unchecked is silent)         |
///
/// ## Group mutex
///
/// WinUI's `<RadioButton GroupName="...">` provides true radio-group
/// behavior at the XAML level: setting `IsChecked="True"` on any
/// member automatically deselects the other members. This matches
/// UI29-2's design exactly (browser-level mutex in HTML, ButtonGroup
/// in QtQuick, etc.). No `RadioGroup` synthesis needed.
///
/// ## `onSelect` fires only on Checked
///
/// Per UI29-2 Â§2.2, `onSelect = "this radio was chosen"`. We wire
/// **only** the `Checked` event â€” `Unchecked` (sibling-caused
/// deselect) is intentionally not handled.
///
/// ## `value` is recorded as a comment for v1
///
/// WinUI's `<RadioButton>` has no built-in `Value` property. The
/// emitted code-behind handler dispatches `.x(value: "<lit>")` (string
/// literal) or `.x(value: this.<Pascal>)` (slot ref) directly â€” see
/// the handler emission below.
fn emit_host_radio(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostRadio", ctx);

    let mut attrs = String::new();

    // Content (label).
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

    // GroupName from `group:` â€” native WinUI radio-mutex.
    match find_prop_value(node, "group") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" GroupName=\"{{x:Bind {pascal}}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" GroupName=\"{}\"", escape_xaml_attr(s)));
        }
        _ => {}
    }

    // IsChecked from `checked:`.
    match find_prop_value(node, "checked") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" IsChecked=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsChecked=\"True\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsChecked=\"False\"");
        }
        _ => {}
    }

    // IsEnabled from `disabled:` (same as HostCheckbox / HostButton).
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            ctx.add_helper(HelperMethod {
                name: "Not".to_string(),
                parameters: vec![("b".to_string(), "bool".to_string())],
                return_type: "bool".to_string(),
                body: "!b".to_string(),
            });
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind Not({pascal})}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    // Checked handler from `onSelect:`. Pre-compute the value: payload
    // (C# string literal or property reference) so the handler body
    // dispatches the right shape.
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onSelect") {
        let value_expr: String = match find_prop_value(node, "value") {
            Some(LayoutPropValue::String(s)) => format!("\"{}\"", escape_csharp_string(s)),
            Some(LayoutPropValue::SlotRef(slot)) => {
                let pascal = kebab_to_pascal_case(slot);
                format!("this.{pascal}")
            }
            _ => "\"\"".to_string(),
        };

        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let handler = format!("{x_name}_Checked");
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}({value_expr}));\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" Checked=\"{handler}\""));
    }

    Ok(format!(
        "{pad}<RadioButton x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

/// Escape a Rust string for embedding inside a C# double-quoted
/// string literal. Same minimal rule as `escape_swift_string`:
/// backslash and double-quote.
///
/// This is the C# escaper used by the HostRadio handler body
/// generator; existing XAML emitters that escape for XML attributes
/// (`escape_xaml_attr`) don't fit here because the target is a C#
/// string literal in code-behind, not an XML attribute value.
fn escape_csharp_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            // C# string literals do allow embedded \n in non-verbatim
            // strings, but they translate to actual newlines at runtime
            // and don't pose an injection risk inside a string-literal
            // context. We leave them alone.
            other => out.push(other),
        }
    }
    out
}

// =====================================================================
// UI29-4 â€” HostLink / HostTooltip / HostNumberInput emitters
// =====================================================================

/// `HostLink` â†’ WinUI 3 `<HyperlinkButton>` per UI29-4.
///
/// WinUI 3 ships HyperlinkButton specifically for "clickable hyperlink"
/// (vs `<Hyperlink>` which is the inline-text variant used inside
/// `RichTextBlock`). HyperlinkButton has native a11y role + visited-
/// state styling + Ctrl+click new-window support.
///
/// ## Property handling
///
/// | moslayout prop      | XAML                                                         |
/// |---|---|
/// | `href: "..."`       | `NavigateUri="..."` (XAML-attr-escaped)                     |
/// | `href: slot: u`     | `NavigateUri="{x:Bind U}"`                                   |
/// | `label: ..." / slot`| `Content="..."` / `Content="{x:Bind Label}"`                 |
/// | `target: new-tab`   | (no extra attr â€” WinUI HyperlinkButton always opens via OS) |
/// | `external: false` + `onActivate` | swaps to `<Button>` with `Click` handler so the host can route in-app |
/// | `onActivate: emit`  | Click handler when external:false; otherwise dropped (v1) |
///
/// ## Generated shape
///
/// ```xml
/// <HyperlinkButton x:Name="link0" NavigateUri="https://example.com" Content="Click me"/>
/// ```
///
/// or, with `external: false`:
///
/// ```xml
/// <Button x:Name="link0" Content="Click me" Click="link0_Click"/>
/// ```
/// + a `link0_Click` code-behind handler that dispatches the named emit.
fn emit_host_link(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostLink", ctx);

    let external_false = matches!(find_prop_value(node, "external"), Some(LayoutPropValue::Keyword(k)) if k == "false");
    let on_activate = match find_prop_value(node, "onActivate") {
        Some(LayoutPropValue::EmitRef(s)) => Some(s.as_str()),
        _ => None,
    };

    // Content (label) â€” shared between Button and HyperlinkButton.
    let mut content_attr = String::new();
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            content_attr.push_str(&format!(" Content=\"{{x:Bind {pascal}}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            content_attr.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            if ctx.lookup_for_binding(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                content_attr.push_str(&format!(" Content=\"{{x:Bind {pascal}}}\""));
            } else if ctx.lookup_for_index(k).is_some() {
                content_attr.push_str(" Content=\"{x:Bind Index}\"");
            } else {
                content_attr.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
            }
        }
        _ => {
            // No label â€” fall back to href as the visible text.
            if let Some(LayoutPropValue::String(s)) = find_prop_value(node, "href") {
                content_attr.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
            }
        }
    }

    if external_false {
        // In-app routing path: Button + Click handler that dispatches.
        let mut attrs = content_attr;
        if let Some(emit_name) = on_activate {
            let handler = format!("{x_name}_Click");
            let case_pascal = kebab_to_pascal_case(&strip_on_prefix(emit_name));
            let component = ctx.component_name;
            let event_ctor =
                if let Some(payload_expr) = host_link_click_payload_expr(emit_name, node, ctx) {
                    format!("new {component}Event.{case_pascal}({payload_expr})")
                } else {
                    format!("new {component}Event.{case_pascal}()")
                };
            let body = format!(
                "    private void {handler}(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)\n    {{\n        Dispatch?.Invoke(this, {event_ctor});\n    }}"
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
    } else {
        // Default external-open path: HyperlinkButton with NavigateUri.
        let mut attrs = String::new();
        match find_prop_value(node, "href") {
            Some(LayoutPropValue::String(s)) => {
                attrs.push_str(&format!(" NavigateUri=\"{}\"", escape_xaml_attr(s)));
            }
            Some(LayoutPropValue::SlotRef(slot)) => {
                let pascal = kebab_to_pascal_case(slot);
                attrs.push_str(&format!(" NavigateUri=\"{{x:Bind {pascal}}}\""));
            }
            _ => {}
        }
        attrs.push_str(&content_attr);
        Ok(format!(
            "{pad}<HyperlinkButton x:Name=\"{x_name}\"{attrs}{style}/>\n"
        ))
    }
}

/// `HostTooltip` â†’ wrap the single child with WinUI's
/// `ToolTipService.ToolTip` attached property.
///
/// ## Generated shape
///
/// ```xml
/// <Border ToolTipService.ToolTip="...">
///   <!-- child(ren) -->
/// </Border>
/// ```
///
/// A `Border` wrapper (rather than e.g. a `Grid`) keeps the layout
/// flat â€” Border with no padding/margin/background is functionally a
/// pass-through. The ToolTipService attached property surfaces the
/// tooltip on hover with proper a11y wiring.
fn emit_host_tooltip(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 2);
    let _ = part_styles;

    let text = match find_prop_value(node, "text") {
        Some(LayoutPropValue::String(s)) => {
            format!(" ToolTipService.ToolTip=\"{}\"", escape_xaml_attr(s))
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            format!(" ToolTipService.ToolTip=\"{{x:Bind {pascal}}}\"")
        }
        _ => String::new(),
    };

    if node.children.is_empty() {
        return Ok(format!("{pad}<Border{text}/>\n"));
    }

    let mut out = format!("{pad}<Border{text}>\n");
    for child in &node.children {
        out.push_str(&emit_xaml_node(child, indent + 2, part_styles, ctx)?);
    }
    out.push_str(&format!("{pad}</Border>\n"));
    let _ = inner_pad;
    Ok(out)
}

/// `HostNumberInput` â†’ WinUI 3 `<NumberBox>` per UI29-4.
///
/// NumberBox is WinUI 3's native numeric input with built-in Â±
/// stepper buttons, min/max validation, and locale-aware decimal
/// parsing. Perfect cross-mapping for HostNumberInput's slot
/// surface.
///
/// ## Property handling
///
/// | moslayout prop  | XAML                                                      |
/// |---|---|
/// | `value: slot: v`| `Value="{x:Bind V, Mode=TwoWay}"`                         |
/// | `min: <n>`      | `Minimum="<n>"`                                            |
/// | `max: <n>`      | `Maximum="<n>"`                                            |
/// | `step: <n>`     | `SmallChange="<n>"`                                        |
/// | `placeholder`   | `PlaceholderText="..."` / bound                            |
/// | `disabled`      | `IsEnabled="{x:Bind Not(D)}"` (polarity flip via Not helper) |
/// | `onChange: emit`| `ValueChanged="X_ValueChanged"` (only fires on commit, not per-keystroke) |
fn emit_host_number_input(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostNumberInput", ctx);

    let mut attrs = String::new();

    // value: slot ref TwoWay binding; numeric literal as a Value attr.
    match find_prop_value(node, "value") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" Value=\"{{x:Bind {pascal}, Mode=TwoWay}}\""));
        }
        Some(LayoutPropValue::Number(n)) => {
            attrs.push_str(&format!(" Value=\"{n}\""));
        }
        _ => {}
    }

    // min/max/step â†’ Minimum/Maximum/SmallChange numeric literals.
    if let Some(LayoutPropValue::Number(n)) = find_prop_value(node, "min") {
        attrs.push_str(&format!(" Minimum=\"{n}\""));
    }
    if let Some(LayoutPropValue::Number(n)) = find_prop_value(node, "max") {
        attrs.push_str(&format!(" Maximum=\"{n}\""));
    }
    if let Some(LayoutPropValue::Number(n)) = find_prop_value(node, "step") {
        attrs.push_str(&format!(" SmallChange=\"{n}\""));
    }

    // placeholder: string or slot.
    match find_prop_value(node, "placeholder") {
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" PlaceholderText=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            attrs.push_str(&format!(" PlaceholderText=\"{{x:Bind {pascal}}}\""));
        }
        _ => {}
    }

    // disabled: slot polarity-flip (Not helper) or literal keyword.
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            ctx.add_helper(HelperMethod {
                name: "Not".to_string(),
                parameters: vec![("b".to_string(), "bool".to_string())],
                return_type: "bool".to_string(),
                body: "!b".to_string(),
            });
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind Not({pascal})}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    // onChange â†’ ValueChanged code-behind handler.
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onChange") {
        let handler = format!("{x_name}_ValueChanged");
        let case_pascal = kebab_to_pascal_case(&strip_on_prefix(emit_name));
        let component = ctx.component_name;
        // NumberBox.ValueChanged fires NumberBoxValueChangedEventArgs;
        // the new value is at args.NewValue (double).
        let body = format!(
            "    private void {handler}(Microsoft.UI.Xaml.Controls.NumberBox sender, Microsoft.UI.Xaml.Controls.NumberBoxValueChangedEventArgs args)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}(args.NewValue));\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" ValueChanged=\"{handler}\""));
    }

    Ok(format!(
        "{pad}<NumberBox x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

/// `HostScroll` â†’ `<ScrollViewer>` per spec Â§4.3.
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
    out.push_str(&emit_xaml_single_content_children(
        &node.children,
        indent + 4,
        part_styles,
        ctx,
    )?);
    writeln!(out, "{pad}</ScrollViewer>").unwrap();
    Ok(out)
}

// =====================================================================
// U29-1-K-xaml: HostDialog (UI29-1 Â§3.6)
// =====================================================================
//
// `HostDialog` lowers to WinUI 3's `ContentDialog` (modal: true, the
// default) or `Flyout` (modal: false). Both are platform-level
// top-layer primitives that provide modal blocking / focus trap /
// dismiss handling out of the box â€” exactly the properties UI29-1 Â§1
// identified as impossible to compose from `<div>`/`<Border>`.
//
// Lifecycle (ShowAsync / Hide) requires C# code-behind on the host
// side: ContentDialog is not driven by a simple `IsOpen` DP â€” the
// caller must `await dialog.ShowAsync()` to present it. The emitter
// therefore takes the documented "code-behind stub" path:
//
//   - `open: slot: x` lands as a XAML comment + a `{Binding x}`
//     attribute on a Mosaic attached property name
//     (`mos:Dialog.IsOpen`). The host project's code-behind is
//     expected to either implement that attached property or watch
//     the slot's DP and call `ShowAsync()` / `Hide()` itself. The
//     comment makes that contract visible in the emitted XAML.
//   - `onClose: emit: onX` wires a `Closed="OnHostDialogClose_N"`
//     event handler. The handler body is generated into the
//     code-behind (same shape as HostButton's Click handler) and
//     dispatches the named emit.
//
// Modal selection is a compile-time keyword (`modal: true|false`)
// matching the spec's compile-time-only choice; we route to two
// different XAML elements at lowering time rather than emitting one
// element and toggling a runtime property.
//
// `dismiss-on-backdrop` is documented as not-yet-bindable here:
// ContentDialog's nearest analogue is `LightDismissOverlayMode`
// (an enum, not a bool) and Flyout's is `LightDismissOverlayMode`
// + `ShouldConstrainToRootBounds` â€” neither maps cleanly to the
// spec's boolean. A keyword-true (the default) becomes a no-op; a
// keyword-false surfaces as an emitted XAML comment so the gap is
// visible in diffs without breaking the compile.
/// Build the attribute/comment/handler bundle shared between the two
/// HostDialog emission paths (nested or root). Returns
/// `(attrs_string, comment_lines)` where `attrs_string` is spliced
/// into the opening tag and `comment_lines` is emitted just above it.
///
/// Fix A2: dropped the `mos:Dialog.IsOpen` attribute entirely. The
/// open-state still surfaces as a comment (the host code-behind
/// remains responsible for calling ShowAsync()/Hide() â€” same contract
/// as before, minus the undeclared-namespace XAML).
///
/// Fix A3: `Title="{Binding X}"` â†’ `Title="{x:Bind X, Mode=OneWay}"`
/// to match the rest of the emitter. The `{Binding}` form silently
/// failed because nothing sets DataContext.
///
/// Fix A4: routes the `title` slot through `ctx.slot_xbind_path()` so
/// a ContentDialog-rooted component uses the `DialogTitle` alias.
fn build_host_dialog_attrs(
    node: &LayoutNode,
    ctx: &mut EmitContext<'_>,
    counter: u32,
) -> Result<(String, Vec<String>), PipelineEmitError> {
    let mut attrs = String::new();
    let mut comments: Vec<String> = Vec::new();

    // title: slot/string â€” Fix A3 + A4.
    match find_prop_value(node, "title") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let path = ctx.slot_xbind_path(slot);
            if !is_safe_identifier(&path) {
                return Err(PipelineEmitError::UnsafeSlotName(path));
            }
            attrs.push_str(&format!(" Title=\"{{x:Bind {path}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Title=\"{}\"", escape_xaml_attr(s)));
        }
        _ => {}
    }

    // open: slot/keyword â€” Fix A2: NO `mos:Dialog.IsOpen` emission.
    // The lifecycle contract lives in a doc comment for the host.
    match find_prop_value(node, "open") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let path = ctx.slot_xbind_path(slot);
            if !is_safe_identifier(&path) {
                return Err(PipelineEmitError::UnsafeSlotName(path));
            }
            comments.push(format!(
                "<!-- HostDialog #{counter} open-state: bind '{path}'; host code-behind watches this DP and calls ShowAsync()/Hide() accordingly. -->"
            ));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            comments.push(format!(
                "<!-- HostDialog #{counter} open-state: literal true; host code-behind calls ShowAsync() once on load. -->"
            ));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            comments.push(format!(
                "<!-- HostDialog #{counter} open-state: literal false; dialog stays hidden until host code-behind calls ShowAsync(). -->"
            ));
        }
        _ => {}
    }

    // dismiss-on-backdrop: WinUI 3 has no clean boolean analogue.
    if let Some("false") = find_prop_keyword(node, "dismiss-on-backdrop") {
        comments.push(format!(
            "<!-- HostDialog #{counter} dismiss-on-backdrop: false â€” XAML's ContentDialog has no boolean equivalent (only LightDismissOverlayMode enum). Host must override the dismiss behaviour in code-behind. -->"
        ));
    }

    // onClose â†’ Closed handler. Handler dispatches the declared emit
    // case with no payload.
    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onClose") {
        let handler = format!("OnHostDialogClose_{counter}");
        let emit_case = strip_on_prefix(emit_name);
        let case_pascal = kebab_to_pascal_case(&emit_case);
        let component = ctx.component_name;
        let body = format!(
            "    private void {handler}(object sender, object e)\n    {{\n        Dispatch?.Invoke(this, new {component}Event.{case_pascal}());\n    }}"
        );
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: body,
        });
        attrs.push_str(&format!(" Closed=\"{handler}\""));
        comments.push(format!(
            "<!-- HostDialog #{counter} onClose: dispatches {case_pascal}; handler wired in code-behind. -->"
        ));
    }

    Ok((attrs, comments))
}

/// HostDialog as a NESTED layout primitive (the rare case â€” most
/// HostDialog uses are at the moslayout root). Emits a
/// `<ContentDialog>` or `<Flyout>` element with its own attributes
/// and children.
fn emit_host_dialog(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let modal_keyword = find_prop_keyword(node, "modal");
    let element = match modal_keyword {
        Some("false") => "Flyout",
        _ => "ContentDialog",
    };
    let counter = ctx.next_host_counter();
    let (attrs, comments) = build_host_dialog_attrs(node, ctx, counter)?;

    let mut out = String::new();
    for c in &comments {
        writeln!(out, "{pad}{c}").unwrap();
    }
    writeln!(out, "{pad}<{element}{attrs}{style}>").unwrap();
    out.push_str(&emit_xaml_single_content_children(
        &node.children,
        indent + 4,
        part_styles,
        ctx,
    )?);
    writeln!(out, "{pad}</{element}>").unwrap();
    Ok(out)
}

/// HostDialog as the moslayout ROOT â€” the common case after Fix A1.
/// The component's XAML root IS the `<ContentDialog>`, so this
/// emitter writes the dialog's *attributes* and *children* only,
/// without wrapping in another `<ContentDialog>` (that wrapping is
/// done by `emit_xaml` at the outer level).
///
/// The attributes (Title, Closed handler, â€¦) need to land on the
/// outer ContentDialog tag. We emit them by SPLICING into the
/// already-written `<ContentDialog>` open tag â€” that's the only way
/// to keep one source of truth for the attribute list across the
/// nested vs root paths.
///
/// We return:
///   1. the comments to emit above the root (open-state, dismiss,
///      close-handler docs)
///   2. the child markup
///   3. the attribute string, packaged into a sentinel comment line
///      that `emit_xaml` looks for and splices into the open tag.
///
/// The sentinel approach is fragile â€” a cleaner refactor is on the
/// to-do list â€” but it keeps the diff small and gets the demo green.
fn emit_host_dialog_as_root(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    // Reserve the counter at #1 to match the standalone HostDialog
    // numbering convention.
    let counter = ctx.next_host_counter();
    let (attrs, comments) = build_host_dialog_attrs(node, ctx, counter)?;
    let style = part_style_attr(node, part_styles);

    // The root's attributes need to live on the outer ContentDialog
    // tag written by `emit_xaml`. Stash them in `ctx.used_xmlns` â€”
    // no, that's xmlns prefixes only. Use a side channel.
    ctx.root_extra_attrs = Some(format!("{attrs}{style}"));

    let mut out = String::new();
    for c in &comments {
        writeln!(out, "{}{c}", " ".repeat(indent)).unwrap();
    }
    out.push_str(&emit_xaml_single_content_children(
        &node.children,
        indent,
        part_styles,
        ctx,
    )?);
    Ok(out)
}

// =====================================================================
// PR-4: HostTable + section sub-tags
// =====================================================================
//
// `HostTable` is the only kernel primitive WinUI 3 has no idiomatic
// native control for. Per spec Â§5, the lowering is a hand-rolled
// `<Grid>` (XAML's primitive!) with `Grid.RowDefinitions` driven by
// the present section sub-tags and each section's `Row` children
// becoming a `<StackPanel Orientation="Horizontal">`.
//
// Four section sub-tags are recognised:
//   - HostTableColGroup â€” UI29 Â§2.1 (deferred to a later PR â€” see Â§5.2
//                         caveat about column-widths layout)
//   - HostTableHead     â€” header row(s), Grid.Row="0"
//   - HostTableBody     â€” data row(s), wrapped in ScrollViewer for
//                         vertical overflow, Grid.Row="<head?1:0>"
//   - HostTableFoot     â€” footer row(s), Grid.Row="<...>"
//
// Each section appears at most once per HostTable; a duplicate is a
// `DuplicateTableSection` error (the spec's debug_assert pattern lifted
// to a fatal error here â€” XAML doesn't have a defensible fallback for
// an extra `<Grid.RowDefinitions>` row).

/// `HostTable [name] { section sub-tags... }` per spec Â§5.
fn emit_host_table(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let style = part_style_attr(node, part_styles);

    // UI31 Â§3.2 RTL contract. WinUI's `FrameworkElement.FlowDirection`
    // is the canonical RTL knob: setting it to `RightToLeft` on the
    // root `<Grid>` flips the column ordering of all descendant rows
    // automatically by the WinUI layout pass. Inherited by descendants.
    //
    // Three accepted shapes (mirrors the React/HTML/Webcomp/Flutter/
    // Qt/SwiftUI backends in #4143, #4156, #4162, #4166, #4185, #4194):
    //
    // | Source                 | Emits                                                |
    // |------------------------|------------------------------------------------------|
    // | `dir: rtl`             | ` FlowDirection="RightToLeft"`                       |
    // | `dir: ltr`             | ` FlowDirection="LeftToRight"`                       |
    // | `dir: auto`            | (nothing â€” inherit from ancestor; WinUI has no auto) |
    // | `dir: slot: layout-dir`| ` FlowDirection="{x:Bind LayoutDir}"`                |
    // | unknown keyword        | (nothing â€” drops silently per allow-list)            |
    //
    // The allow-list (`ltr` / `rtl` / `auto`) is the security gate.
    // Slot refs go through `kebab_to_pascal_case` + `is_safe_identifier`
    // so the binding path stays a clean XAML identifier â€” an
    // attacker-controlled slot name can't break out of the
    // `{x:Bind ...}` attribute value.
    let flow_direction_attr: String = match find_prop_value(node, "dir") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = kebab_to_pascal_case(slot);
            if is_safe_identifier(&pascal) {
                format!(" FlowDirection=\"{{x:Bind {pascal}}}\"")
            } else {
                String::new()
            }
        }
        Some(LayoutPropValue::Keyword(k)) => match k.as_str() {
            "rtl" => " FlowDirection=\"RightToLeft\"".to_string(),
            "ltr" => " FlowDirection=\"LeftToRight\"".to_string(),
            // `auto` is the spec-mandated "let the host decide"
            // keyword. WinUI has no `Auto` enum value for
            // FlowDirection â€” the right behaviour is to NOT emit the
            // attribute so any ancestor's FlowDirection (typically
            // the root `Page`'s, set from CultureInfo) flows through.
            "auto" => String::new(),
            _ => String::new(),
        },
        _ => String::new(),
    };

    // -- 1. Find each section sub-tag at most once. --
    let mut colgroup: Option<&LayoutNode> = None;
    let mut head: Option<&LayoutNode> = None;
    let mut body: Option<&LayoutNode> = None;
    let mut foot: Option<&LayoutNode> = None;

    for child in &node.children {
        match child.tag.as_str() {
            "HostTableColGroup" => {
                if colgroup.is_some() {
                    return Err(PipelineEmitError::DuplicateTableSection(
                        "HostTableColGroup".to_string(),
                    ));
                }
                colgroup = Some(child);
            }
            "HostTableHead" => {
                if head.is_some() {
                    return Err(PipelineEmitError::DuplicateTableSection(
                        "HostTableHead".to_string(),
                    ));
                }
                head = Some(child);
            }
            "HostTableBody" => {
                if body.is_some() {
                    return Err(PipelineEmitError::DuplicateTableSection(
                        "HostTableBody".to_string(),
                    ));
                }
                body = Some(child);
            }
            "HostTableFoot" => {
                if foot.is_some() {
                    return Err(PipelineEmitError::DuplicateTableSection(
                        "HostTableFoot".to_string(),
                    ));
                }
                foot = Some(child);
            }
            other => {
                return Err(PipelineEmitError::UnsupportedPrimitive(format!(
                    "{other} is not a HostTable section sub-tag"
                )));
            }
        }
    }

    // colgroup is recognised but not yet rendered â€” the column-widths
    // story needs more design (Â§5.2 caveat). PR-4 silently ignores it.
    let _ = colgroup;

    // -- 2. Empty HostTable â†’ empty `<Grid/>`. Preserves part style. --
    if head.is_none() && body.is_none() && foot.is_none() {
        return Ok(format!("{pad}<Grid{flow_direction_attr}{style}></Grid>\n"));
    }

    // -- 3. Build RowDefinitions list. Each present section gets one
    //       row; head and foot are Auto-sized, body is `*` (fills). --
    let mut row_defs: Vec<&'static str> = Vec::with_capacity(3);
    if head.is_some() {
        row_defs.push("Auto");
    }
    if body.is_some() {
        row_defs.push("*");
    }
    if foot.is_some() {
        row_defs.push("Auto");
    }

    // -- 4. Assemble the XAML. --
    let mut out = String::new();
    writeln!(out, "{pad}<Grid{flow_direction_attr}{style}>").unwrap();
    writeln!(out, "{pad2}<Grid.RowDefinitions>").unwrap();
    for r in &row_defs {
        writeln!(out, "{pad2}    <RowDefinition Height=\"{r}\"/>").unwrap();
    }
    writeln!(out, "{pad2}</Grid.RowDefinitions>").unwrap();

    // -- 5. Per-section content. Assign Grid.Row indices in source order. --
    let mut row_index = 0u32;
    if let Some(h) = head {
        out.push_str(&emit_host_table_section(
            h,
            row_index,
            indent + 4,
            part_styles,
            ctx,
            false, // header doesn't wrap in ScrollViewer
        )?);
        row_index += 1;
    }
    if let Some(b) = body {
        out.push_str(&emit_host_table_section(
            b,
            row_index,
            indent + 4,
            part_styles,
            ctx,
            true, // body wraps in ScrollViewer for vertical overflow
        )?);
        row_index += 1;
    }
    if let Some(f) = foot {
        out.push_str(&emit_host_table_section(
            f,
            row_index,
            indent + 4,
            part_styles,
            ctx,
            false, // footer doesn't wrap
        )?);
    }

    writeln!(out, "{pad}</Grid>").unwrap();
    Ok(out)
}

/// Emit one section (Head / Body / Foot) of a HostTable. The section's
/// `Row` children become `<StackPanel Orientation="Horizontal">` of
/// cell children; the section itself becomes a
/// `<StackPanel Orientation="Vertical">` (wrapped in a `<ScrollViewer>`
/// when `scrollable` is `true` â€” used for the body section).
fn emit_host_table_section(
    section: &LayoutNode,
    grid_row: u32,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
    scrollable: bool,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);

    // The section's own part_name applies to the outer wrapper.
    let style = part_style_attr(section, part_styles);

    let mut out = String::new();

    if scrollable {
        writeln!(
            out,
            "{pad}<ScrollViewer Grid.Row=\"{grid_row}\" VerticalScrollBarVisibility=\"Auto\" HorizontalScrollBarVisibility=\"Disabled\">"
        )
        .unwrap();
        writeln!(out, "{pad2}<StackPanel Orientation=\"Vertical\"{style}>").unwrap();
        out.push_str(&emit_host_table_rows(
            &section.children,
            indent + 8,
            part_styles,
            ctx,
        )?);
        writeln!(out, "{pad2}</StackPanel>").unwrap();
        writeln!(out, "{pad}</ScrollViewer>").unwrap();
    } else {
        writeln!(
            out,
            "{pad}<StackPanel Grid.Row=\"{grid_row}\" Orientation=\"Vertical\"{style}>"
        )
        .unwrap();
        out.push_str(&emit_host_table_rows(
            &section.children,
            indent + 4,
            part_styles,
            ctx,
        )?);
        writeln!(out, "{pad}</StackPanel>").unwrap();
    }

    Ok(out)
}

/// Emit the rows of one section. Only `Row` is permitted as a direct
/// child of a section per UI29 Â§2.1; any other tag is an
/// `UnsupportedPrimitive`. Each `Row` lowers as if it were a moslayout
/// `Row` primitive (a `<StackPanel Orientation="Horizontal">`).
fn emit_host_table_rows(
    rows: &[LayoutNode],
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    for row in rows {
        match row.tag.as_str() {
            "Row" => {
                out.push_str(&emit_stack_panel(
                    row,
                    indent,
                    part_styles,
                    "Horizontal",
                    ctx,
                )?);
            }
            "For" => {
                // Allow a `For` inside a section so authors can iterate
                // over data rows: `HostTableBody { For (each: slot: rows, as: r) { Row { ... } } }`.
                out.push_str(&emit_for(row, indent, part_styles, ctx)?);
            }
            "If" => {
                // Allow conditional rows (e.g. show a row only when an
                // option is enabled).
                out.push_str(&emit_if(row, None, indent, part_styles, ctx)?);
            }
            other => {
                return Err(PipelineEmitError::UnsupportedPrimitive(format!(
                    "{other} as a direct child of a HostTable section â€” only Row, For, If permitted"
                )));
            }
        }
    }
    Ok(out)
}

// =====================================================================
// PR-5: Component reference resolution (UI29 Â§4.4)
// =====================================================================

/// Emit a `<{prefix}:{Tag} ... />` reference for a non-kernel tag.
///
/// Resolution: look the tag up in `ctx.registry`. If absent (no registry
/// or tag not registered), the error path picks one of two variants:
///
/// - When a registry IS present (even if empty) â†’ `UnknownComponent`
///   means "the host gave us a registry, the tag isn't in it". This is
///   the spec's intended error for "missing manifest dependency".
/// - When the registry is absent â†’ `UnsupportedPrimitive` for parity
///   with the pre-PR-5 shape (preserves the diagnostic for `--backend
///   xaml` invocations that don't use packages at all).
fn emit_component_reference(
    tag: &str,
    node: &LayoutNode,
    indent: usize,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let registry = match ctx.registry {
        Some(r) => r,
        None => return Err(PipelineEmitError::UnsupportedPrimitive(tag.to_string())),
    };

    let entry = match registry.lookup(tag) {
        Some(e) => e.clone(),
        None => return Err(PipelineEmitError::UnknownComponent(tag.to_string())),
    };

    // Record the xmlns prefix â†’ value mapping for the `<UserControl>`
    // root injection. BTreeMap-keyed for deterministic output ordering.
    ctx.used_xmlns
        .insert(entry.xmlns_prefix.clone(), entry.xmlns_value.clone());

    // Build per-prop attributes. PR-5 supports slot refs, string
    // literals, numbers, and keywords. EmitRef props (`onClick: emit:
    // X`) are deferred â€” they need a host-side handler-stub
    // generation that is out of scope for PR-5. A clear comment in the
    // emitted XAML flags any deferred emit-ref props rather than
    // silently dropping them.
    let pad = " ".repeat(indent);
    let mut attrs = String::new();
    let mut emit_ref_skipped: Vec<String> = Vec::new();

    for prop in &node.props {
        let attr_name = kebab_to_pascal_case(&prop.name);
        match &prop.value {
            LayoutPropValue::SlotRef(slot) => {
                let pascal = kebab_to_pascal_case(slot);
                if !is_safe_identifier(&pascal) {
                    return Err(PipelineEmitError::UnsafeSlotName(pascal));
                }
                attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {pascal}}}\""));
            }
            LayoutPropValue::String(s) => {
                attrs.push_str(&format!(" {attr_name}=\"{}\"", escape_xaml_attr(s)));
            }
            LayoutPropValue::Number(n) => {
                attrs.push_str(&format!(" {attr_name}=\"{n}\""));
            }
            LayoutPropValue::Keyword(k) => {
                if ctx.lookup_for_binding(k).is_some() || ctx.lookup_for_index(k).is_some() {
                    let pascal = kebab_to_pascal_case(k);
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {pascal}}}\""));
                } else {
                    attrs.push_str(&format!(" {attr_name}=\"{}\"", escape_xaml_attr(k)));
                }
            }
            LayoutPropValue::EmitRef(emit) => {
                emit_ref_skipped.push(format!("{}: emit: {}", prop.name, emit));
            }
            LayoutPropValue::Expr(src) => match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {path}}}\""));
                }
                ExprLowering::Helper(call) => {
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {call}}}\""));
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            },
        }
    }

    let prefix = &entry.xmlns_prefix;
    let mut out = format!("{pad}<{prefix}:{tag}{attrs}/>\n");
    if !emit_ref_skipped.is_empty() {
        // Surface the deferred props as a XAML comment so they're
        // visible to reviewers and the diff makes the deferral obvious.
        let list = emit_ref_skipped.join(", ");
        out = format!(
            "{pad}<!-- Deferred (PR-5+ work): emit-ref props on component reference {tag}: {list} -->\n{out}"
        );
    }
    Ok(out)
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp};
    use mosmodel_compiler::{EmitParam, ListInnerType, MosmodelComponent, SlotDecl, SlotType};
    use mosstyle_compiler::{PartStyle, StyleDef, StyleProp};

    // â”€â”€ helpers â”€â”€

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

    // â”€â”€ version â”€â”€

    #[test]
    fn version_is_0_1_0() {
        assert_eq!(crate::VERSION, "0.1.0");
    }

    // â”€â”€ kebab â†’ casing â”€â”€

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

    // â”€â”€ component name mismatch â”€â”€

    #[test]
    fn component_name_mismatch_errors() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Bar", box_root());
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(
            err,
            PipelineEmitError::ComponentNameMismatch { .. }
        ));
    }

    // â”€â”€ XAML root shape â”€â”€

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
        assert!(r
            .xaml
            .contains("xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\""));
        assert!(r
            .xaml
            .contains("xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\""));
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

    // â”€â”€ code-behind shape â”€â”€

    #[test]
    fn code_behind_has_partial_class_and_init_call() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r
            .code_behind
            .contains("public sealed partial class Foo : UserControl"));
        assert!(r.code_behind.contains("this.InitializeComponent();"));
        assert!(r
            .code_behind
            .contains("public event EventHandler<FooEvent>? Dispatch;"));
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
        assert!(r
            .code_behind
            .contains("DependencyProperty.Register(nameof(Name), typeof(string), typeof(Foo)"));
        assert!(r.code_behind.contains("public double Count"));
        assert!(r
            .code_behind
            .contains("DependencyProperty.Register(nameof(Count), typeof(double), typeof(Foo)"));
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

    // â”€â”€ event union â”€â”€

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
                    vec![
                        param("row", EmitPayloadType::Number),
                        param("col", EmitPayloadType::Number),
                    ],
                ),
                emit("onEditCommit", vec![param("value", EmitPayloadType::Text)]),
                emit("onCancel", vec![]),
            ],
        );
        let l = layout_with_root("Grid", box_root());
        let r = compile(&c, &l, &empty_style("Grid"));
        assert!(
            r.events
                .contains("public abstract string MosaicName { get; }"),
            "got:\n{}",
            r.events
        );
        assert!(
            r.events.contains("public virtual System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicPayload"),
            "got:\n{}",
            r.events
        );
        assert!(
            r.events.contains("public System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicEnvelope"),
            "got:\n{}",
            r.events
        );
        assert!(
            r.events
                .contains("public sealed record Navigate(double Row, double Col) : GridEvent"),
            "got:\n{}",
            r.events
        );
        assert!(r
            .events
            .contains("public override string MosaicName => \"onNavigate\";"));
        assert!(r.events.contains("[\"row\"] = Row"));
        assert!(r.events.contains("[\"col\"] = Col"));
        assert!(r
            .events
            .contains("public sealed record EditCommit(string Value) : GridEvent"));
        assert!(r
            .events
            .contains("public override string MosaicName => \"onEditCommit\";"));
        assert!(r.events.contains("[\"value\"] = Value"));
        assert!(r
            .events
            .contains("public sealed record Cancel() : GridEvent"));
        assert!(r
            .events
            .contains("public override string MosaicName => \"onCancel\";"));
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

    // â”€â”€ primitive lowering: Box / containers â”€â”€

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
        // <Grid> in XAML is the z-axis container (matches UI29 Â§2.1 `Stack`).
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
        let row_pos = r
            .xaml
            .find("<StackPanel Orientation=\"Horizontal\"")
            .unwrap();
        let col_pos = r.xaml.find("<StackPanel Orientation=\"Vertical\"").unwrap();
        let txt_pos = r.xaml.find("<TextBlock").unwrap();
        assert!(row_pos < col_pos && col_pos < txt_pos);
    }

    // â”€â”€ primitive lowering: Text / Image / Spacer / Divider / Icon â”€â”€

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
        assert!(
            r.xaml.contains("<TextBlock Text=\"Hello\""),
            "got:\n{}",
            r.xaml
        );
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
        let c = component(
            "Foo",
            vec![slot("avatar-url", SlotType::Image, true)],
            vec![],
        );
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

    /// X5 Path A: `Icon (glyph: "spinner")` lowers to
    /// `<ProgressRing IsActive="True"/>` instead of the would-be
    /// `<FontIcon Glyph="spinner"/>` â€” Segoe Fluent has no glyph
    /// literally named `spinner`, and the toolkit's `Spinner`
    /// component wants the animated ring anyway.
    #[test]
    fn x5_icon_with_glyph_spinner_lowers_to_progress_ring() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Icon".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("spinner".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<ProgressRing IsActive=\"True\""),
            "expected ProgressRing lowering for `spinner`, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("<FontIcon"),
            "FontIcon must NOT appear for the semantic `spinner` lowering, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("Glyph=\"spinner\""),
            "literal Glyph=\"spinner\" must NOT survive â€” it's the bug, got:\n{}",
            r.xaml
        );
    }

    /// X5 scope: non-semantic glyph names still lower to
    /// `<FontIcon Glyph="..."/>`.  `spinner` is special; `Save`,
    /// `Refresh`, hex codepoints stay on the FontIcon path.
    #[test]
    fn x5_icon_with_non_semantic_glyph_still_emits_fonticon() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Icon".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("Save".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<FontIcon"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Glyph=\"Save\""), "got:\n{}", r.xaml);
    }

    /// X5 scope: slot-bound glyphs (`{x:Bind GlyphProp}`) stay on
    /// the FontIcon path even if the runtime value happens to be
    /// `"spinner"` â€” the lowering decision is static, by the
    /// layout's literal string, so a slot-bound glyph never enters
    /// the semantic table.  Future cycles could push the check to
    /// runtime via a binding converter, but PR-1 keeps it static.
    #[test]
    fn x5_icon_with_slot_bound_glyph_stays_on_fonticon_path() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Icon".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::SlotRef("glyph-name".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<FontIcon"), "got:\n{}", r.xaml);
        assert!(
            r.xaml.contains("Glyph=\"{x:Bind GlyphName}\""),
            "expected x:Bind passthrough, got:\n{}",
            r.xaml
        );
    }

    // â”€â”€ unsupported primitives surface clearly â”€â”€

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
        assert!(
            r.xaml.contains("<TextBox x:Name=\"HostInput_1\""),
            "got:\n{}",
            r.xaml
        );
    }

    /// PR-4 lowers HostTable. An empty HostTable (no section sub-tags)
    /// emits an empty `<Grid/>`. The PR-1 version of this test expected
    /// an `UnsupportedPrimitive` error; we now verify the empty-Grid
    /// lowering instead.
    #[test]
    fn host_table_empty_lowers_to_empty_grid_in_pr4() {
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
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<Grid></Grid>"), "got:\n{}", r.xaml);
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // PR-4: HostTable + section sub-tags tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn host_table_node(part: Option<&str>, sections: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "HostTable".to_string(),
            part_name: part.map(String::from),
            props: Vec::new(),
            children: sections,
        }
    }

    fn section_node(tag: &str, rows: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props: Vec::new(),
            children: rows,
        }
    }

    fn row_with_text_cells(parts: &[&str]) -> LayoutNode {
        LayoutNode {
            tag: "Row".to_string(),
            part_name: None,
            props: Vec::new(),
            children: parts
                .iter()
                .map(|s| LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String(s.to_string()),
                    }],
                    children: Vec::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn host_table_head_only_emits_grid_with_auto_row_for_head() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableHead",
                    vec![row_with_text_cells(&["A", "B"])],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<Grid>"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("<Grid.RowDefinitions>"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("<RowDefinition Height=\"Auto\"/>"));
        // Head section is a StackPanel at Grid.Row="0"
        assert!(
            r.xaml
                .contains("<StackPanel Grid.Row=\"0\" Orientation=\"Vertical\""),
            "got:\n{}",
            r.xaml
        );
        // Both header cells appear.
        assert!(r.xaml.contains("Text=\"A\""));
        assert!(r.xaml.contains("Text=\"B\""));
    }

    #[test]
    fn host_table_head_plus_body_emits_two_row_definitions_auto_and_star() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![
                    section_node("HostTableHead", vec![row_with_text_cells(&["H1"])]),
                    section_node("HostTableBody", vec![row_with_text_cells(&["B1"])]),
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // Two RowDefinitions: Auto + *.
        assert!(r.xaml.contains("<RowDefinition Height=\"Auto\"/>"));
        assert!(r.xaml.contains("<RowDefinition Height=\"*\"/>"));
    }

    #[test]
    fn host_table_body_wraps_in_scrollviewer() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableBody",
                    vec![row_with_text_cells(&["X"])],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("<ScrollViewer Grid.Row=\"0\" VerticalScrollBarVisibility=\"Auto\""),
            "got:\n{}",
            r.xaml
        );
        // Body StackPanel goes inside the ScrollViewer.
        assert!(r.xaml.contains("<StackPanel Orientation=\"Vertical\""));
        // The body's row contains its cells.
        assert!(r.xaml.contains("Text=\"X\""));
    }

    #[test]
    fn host_table_foot_emits_auto_row_no_scrollviewer() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableFoot",
                    vec![row_with_text_cells(&["F"])],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<RowDefinition Height=\"Auto\"/>"));
        // Foot section: StackPanel directly at Grid.Row="0" (no ScrollViewer wrapper).
        assert!(
            r.xaml
                .contains("<StackPanel Grid.Row=\"0\" Orientation=\"Vertical\""),
            "got:\n{}",
            r.xaml
        );
        // No ScrollViewer for the foot section.
        assert!(!r.xaml.contains("<ScrollViewer"), "got:\n{}", r.xaml);
    }

    #[test]
    fn host_table_full_quad_assigns_grid_rows_in_source_order() {
        // ColGroup is recognised but silently ignored in PR-4. The
        // Head, Body, Foot get Grid.Row 0, 1, 2.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![
                    section_node("HostTableColGroup", vec![]),
                    section_node("HostTableHead", vec![row_with_text_cells(&["H"])]),
                    section_node("HostTableBody", vec![row_with_text_cells(&["B"])]),
                    section_node("HostTableFoot", vec![row_with_text_cells(&["F"])]),
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // Three RowDefinitions: Auto, *, Auto.
        let auto_count = r.xaml.matches("<RowDefinition Height=\"Auto\"/>").count();
        let star_count = r.xaml.matches("<RowDefinition Height=\"*\"/>").count();
        assert_eq!(auto_count, 2);
        assert_eq!(star_count, 1);
        // Grid.Row assignments.
        assert!(r
            .xaml
            .contains("<StackPanel Grid.Row=\"0\" Orientation=\"Vertical\""));
        assert!(r
            .xaml
            .contains("<ScrollViewer Grid.Row=\"1\" VerticalScrollBarVisibility"));
        assert!(r
            .xaml
            .contains("<StackPanel Grid.Row=\"2\" Orientation=\"Vertical\""));
    }

    #[test]
    fn host_table_duplicate_section_errors() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![
                    section_node("HostTableBody", vec![]),
                    section_node("HostTableBody", vec![]),
                ],
            ),
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::DuplicateTableSection(ref t) if t == "HostTableBody"),
            "got: {err:?}"
        );
    }

    #[test]
    fn host_table_section_unknown_child_errors() {
        // A non-section child as a direct child of HostTable should
        // produce a clear UnsupportedPrimitive.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![LayoutNode {
                    tag: "Box".to_string(),
                    part_name: None,
                    props: Vec::new(),
                    children: Vec::new(),
                }],
            ),
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedPrimitive(_)));
    }

    #[test]
    fn host_table_section_with_non_row_child_errors() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableBody",
                    vec![LayoutNode {
                        tag: "Box".to_string(),
                        part_name: None,
                        props: Vec::new(),
                        children: Vec::new(),
                    }],
                )],
            ),
        );
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedPrimitive(_)));
    }

    #[test]
    fn host_table_section_accepts_for_iterating_rows() {
        // A `For` directly inside a section is allowed so authors can
        // iterate over data rows from a slot.
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "Grid",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableBody",
                    vec![for_node(
                        LayoutPropValue::SlotRef("rows".to_string()),
                        "row",
                        None,
                        vec![row_with_text_cells(&["x"])],
                    )],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        assert!(r
            .xaml
            .contains("<ItemsRepeater ItemsSource=\"{x:Bind GridRowVmRows}\""));
        assert!(r.code_behind.contains("var source = Rows;"));
        // The For-generated RowVm should be in for_view_models.
        assert!(!r.for_view_models.is_empty());
    }

    #[test]
    fn host_table_section_orphan_at_top_level_errors() {
        // A `HostTableHead` outside a HostTable is wrong nesting.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", section_node("HostTableHead", vec![]));
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(
            matches!(err, PipelineEmitError::UnsupportedPrimitive(ref t) if t.contains("outside HostTable")),
            "got: {err:?}"
        );
    }

    #[test]
    fn host_table_with_part_name_applies_style() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                Some("sheet"),
                vec![section_node(
                    "HostTableHead",
                    vec![row_with_text_cells(&["H"])],
                )],
            ),
        );
        let s = StyleDef {
            component_name: "Foo".to_string(),
            parts: vec![PartStyle {
                name: "sheet".to_string(),
                base: vec![StyleProp {
                    name: "background".to_string(),
                    value: "#1e1e1e".to_string(),
                }],
                states: Vec::new(),
            }],
        };
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("<Grid Background=\"#1e1e1e\""),
            "got:\n{}",
            r.xaml
        );
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // UI31 â€” HostTable a11y gate + RTL contract (XAML backend)
    //
    // Mirrors the React (#4143), HTML (#4156), WebComponent (#4162),
    // Flutter (#4166), Qt (#4185), and SwiftUI (#4194) precedents:
    //
    // - **A11y gate**: the XAML lowering must continue to emit a
    //   structural <Grid> with <Grid.RowDefinitions> per section â€”
    //   WinUI/UIA tooling sees that as a coherent table region, NOT
    //   a flat StackPanel where row associations are lost.
    // - **RTL gate**: when `dir:` is authored, the <Grid> carries
    //   `FlowDirection="RightToLeft"` (or LeftToRight, or a slot
    //   binding). Allow-list is `ltr|rtl|auto`; unknown keywords
    //   drop silently.
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Helper: build a `HostTable` LayoutDef carrying a `dir:` prop
    /// and a minimal HostTableBody so the table is non-empty (lets us
    /// exercise the assembled-Grid emission path, not just the empty
    /// short-circuit).
    fn host_table_with_dir(value: LayoutPropValue) -> LayoutDef {
        let table = LayoutNode {
            tag: "HostTable".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "dir".to_string(),
                value,
            }],
            children: vec![section_node(
                "HostTableBody",
                vec![row_with_text_cells(&["x"])],
            )],
        };
        layout_with_root("Foo", table)
    }

    /// UI31 Â§3.1 a11y gate â€” `HostTable` MUST continue to lower to
    /// a structural `<Grid>` with `<Grid.RowDefinitions>`. A
    /// regression to a flat `<StackPanel>` would lose the
    /// row-association semantics WinUI's automation peer derives
    /// from `Grid.Row="..."` attached properties.
    #[test]
    fn ui31_a11y_host_table_uses_structural_grid_with_row_definitions() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableBody",
                    vec![row_with_text_cells(&["a"])],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<Grid"),
            "HostTable must lower to <Grid>, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<Grid.RowDefinitions>"),
            "must include <Grid.RowDefinitions>, got:\n{}",
            r.xaml
        );
    }

    /// UI31 Â§3.2 RTL contract â€” `dir: rtl` keyword emits
    /// `FlowDirection="RightToLeft"` on the Grid. WinUI's
    /// `FrameworkElement.FlowDirection` cascades to all descendants,
    /// flipping column ordering inside the grid rows.
    #[test]
    fn ui31_rtl_host_table_dir_rtl_keyword_emits_flow_direction_right_to_left() {
        let c = component("Foo", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("rtl".to_string()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("FlowDirection=\"RightToLeft\""),
            "expected FlowDirection=\"RightToLeft\", got:\n{}",
            r.xaml
        );
    }

    /// `dir: ltr` keyword emits the explicit-LeftToRight form.
    /// Useful for tables that should stay LTR inside an ambient-RTL
    /// page (e.g. number-heavy spreadsheets).
    #[test]
    fn ui31_rtl_host_table_dir_ltr_keyword_emits_flow_direction_left_to_right() {
        let c = component("Foo", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("ltr".to_string()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("FlowDirection=\"LeftToRight\""),
            "expected FlowDirection=\"LeftToRight\", got:\n{}",
            r.xaml
        );
    }

    /// `dir: auto` keyword is the spec-mandated "let the host
    /// decide". WinUI has no `Auto` enum for FlowDirection â€” the
    /// right behaviour is to NOT emit the attribute so any
    /// ancestor's FlowDirection (typically the `Page`'s, set from
    /// CultureInfo) flows through.
    #[test]
    fn ui31_rtl_host_table_dir_auto_keyword_does_not_emit_attribute() {
        let c = component("Foo", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword("auto".to_string()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            !r.xaml.contains("FlowDirection"),
            "auto must NOT emit FlowDirection, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<Grid"),
            "bare <Grid> should still render, got:\n{}",
            r.xaml
        );
    }

    /// `dir: slot: layout-direction` interpolates the bound slot
    /// (Pascal-cased to `LayoutDirection`) into the `{x:Bind ...}`
    /// expression. The slot is expected to evaluate to a
    /// `FlowDirection`. Slot name goes through
    /// `kebab_to_pascal_case` + `is_safe_identifier` so it can't
    /// smuggle malicious XAML through the binding path.
    #[test]
    fn ui31_rtl_host_table_dir_slot_ref_interpolates_pascal_case_x_bind() {
        let c = component(
            "Foo",
            vec![SlotDecl {
                name: "layout-direction".to_string(),
                r#type: SlotType::Text,
                required: true,
                default: None,
            }],
            vec![],
        );
        let l = host_table_with_dir(LayoutPropValue::SlotRef("layout-direction".to_string()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("FlowDirection=\"{x:Bind LayoutDirection}\""),
            "expected FlowDirection=\"{{x:Bind LayoutDirection}}\", got:\n{}",
            r.xaml
        );
    }

    /// Unknown `dir:` keywords (anything outside the `ltr|rtl|auto`
    /// allow-list) MUST drop silently. This is the security gate:
    /// an attacker-controlled keyword cannot inject XAML because
    /// it never reaches the format string. Test payload
    /// `"RightToLeft\" Tag=\"pwn\""` is shaped to break out of the
    /// attribute-value quoting if naively interpolated.
    #[test]
    fn ui31_rtl_host_table_unknown_dir_keyword_drops_silently() {
        let c = component("Foo", vec![], vec![]);
        let l = host_table_with_dir(LayoutPropValue::Keyword(
            "RightToLeft\" Tag=\"pwn\"".to_string(),
        ));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            !r.xaml.contains("Tag=\"pwn\""),
            "unknown keyword payload must not appear, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("FlowDirection"),
            "unknown keyword must NOT emit FlowDirection, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<Grid"),
            "bare <Grid> should still render, got:\n{}",
            r.xaml
        );
    }

    /// Regression guard â€” `HostTable` with no `dir:` prop emits no
    /// `FlowDirection` attribute. A future refactor that always-
    /// emits would break authors who rely on the Page-level
    /// CultureInfo cascade.
    #[test]
    fn ui31_rtl_host_table_without_dir_prop_emits_no_flow_direction() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_table_node(
                None,
                vec![section_node(
                    "HostTableBody",
                    vec![row_with_text_cells(&["x"])],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            !r.xaml.contains("FlowDirection"),
            "no FlowDirection expected when dir absent, got:\n{}",
            r.xaml
        );
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // PR-5: ComponentRegistry / component-reference resolution
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn component_ref_node(tag: &str, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: tag.to_string(),
            part_name: None,
            props,
            children: Vec::new(),
        }
    }

    fn compile_with_registry(
        c: &MosmodelComponent,
        l: &LayoutDef,
        s: &StyleDef,
        reg: &ComponentRegistry,
    ) -> XamlEmitResult {
        from_pipeline(c, l, s, Some(reg), &opts()).expect("emit ok")
    }

    #[test]
    fn registry_register_and_lookup_round_trip() {
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let entry = reg.lookup("Grid").expect("registered");
        assert_eq!(entry.xmlns_prefix, "grid");
        assert_eq!(entry.xmlns_value, "using:Mosaic.Package.Grid");
        assert_eq!(entry.package_name, "mosaic-pkg-grid");
    }

    #[test]
    fn registry_lookup_misses_return_none() {
        let reg = ComponentRegistry::new();
        assert!(reg.lookup("Missing").is_none());
        assert!(reg.is_empty());
    }

    #[test]
    fn component_reference_with_no_registry_falls_back_to_unsupported() {
        // Pre-PR-5 behaviour: no registry â†’ non-kernel tags surface as
        // UnsupportedPrimitive (preserves the old diagnostic so demos
        // not using packages still get a clear error).
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", component_ref_node("Whatever", Vec::new()));
        let s = empty_style("Foo");
        let err = from_pipeline(&c, &l, &s, None, &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnsupportedPrimitive(ref t) if t == "Whatever"));
    }

    #[test]
    fn component_reference_with_empty_registry_returns_unknown() {
        // With an explicit (but empty) registry, missing-component
        // becomes UnknownComponent â€” the spec's intended error for
        // "missing manifest dependency".
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", component_ref_node("Whatever", Vec::new()));
        let s = empty_style("Foo");
        let reg = ComponentRegistry::new();
        let err = from_pipeline(&c, &l, &s, Some(&reg), &opts()).unwrap_err();
        assert!(matches!(err, PipelineEmitError::UnknownComponent(ref t) if t == "Whatever"));
    }

    #[test]
    fn component_reference_lowers_to_prefixed_xaml_tag() {
        let c = component("Demo", vec![], vec![]);
        let l = layout_with_root("Demo", component_ref_node("Grid", Vec::new()));
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        assert!(r.xaml.contains("<grid:Grid/>"), "got:\n{}", r.xaml);
    }

    #[test]
    fn component_reference_emits_xmlns_declaration_on_usercontrol() {
        let c = component("Demo", vec![], vec![]);
        let l = layout_with_root("Demo", component_ref_node("Grid", Vec::new()));
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        // xmlns declaration on the open UserControl tag.
        assert!(
            r.xaml.contains("xmlns:grid=\"using:Mosaic.Package.Grid\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn component_reference_slot_ref_prop_emits_xbind_attribute() {
        let c = component(
            "Demo",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "Demo",
            component_ref_node(
                "Grid",
                vec![LayoutProp {
                    name: "rows".to_string(),
                    value: LayoutPropValue::SlotRef("rows".to_string()),
                }],
            ),
        );
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        assert!(
            r.xaml.contains("Rows=\"{x:Bind Rows}\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn component_reference_string_literal_prop_emits_literal_attribute() {
        let c = component("Demo", vec![], vec![]);
        let l = layout_with_root(
            "Demo",
            component_ref_node(
                "Grid",
                vec![LayoutProp {
                    name: "title".to_string(),
                    value: LayoutPropValue::String("My Grid".to_string()),
                }],
            ),
        );
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        assert!(r.xaml.contains("Title=\"My Grid\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn component_reference_emit_ref_prop_surfaces_as_deferred_comment() {
        // Emit-ref props on component references aren't yet wired
        // (host-side handler-stub generation is PR-5+ work). The
        // emitter surfaces a XAML comment listing the deferred props
        // so reviewers see the gap immediately.
        let c = component("Demo", vec![], vec![emit("onNavigate", vec![])]);
        let l = layout_with_root(
            "Demo",
            component_ref_node(
                "Grid",
                vec![LayoutProp {
                    name: "onNavigate".to_string(),
                    value: LayoutPropValue::EmitRef("onNavigate".to_string()),
                }],
            ),
        );
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        // Comment present.
        assert!(
            r.xaml.contains("<!-- Deferred (PR-5+ work)"),
            "got:\n{}",
            r.xaml
        );
        // Emit ref doesn't leak into the tag's attributes.
        assert!(
            !r.xaml.contains("OnNavigate=\""),
            "emit-ref should not appear as a direct attribute, got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn component_reference_multiple_packages_emit_distinct_xmlns() {
        let c = component("Demo", vec![], vec![]);
        let l = layout_with_root(
            "Demo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    component_ref_node("Grid", Vec::new()),
                    component_ref_node("FancyInput", Vec::new()),
                ],
            },
        );
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        reg.register(
            "FancyInput",
            "input",
            "using:Mosaic.Package.Input",
            "mosaic-pkg-input",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        // Both xmlns declarations land on the UserControl.
        assert!(
            r.xaml.contains("xmlns:grid=\"using:Mosaic.Package.Grid\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("xmlns:input=\"using:Mosaic.Package.Input\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn component_reference_dedupes_xmlns_for_same_package_used_twice() {
        // Two component references to the same package should NOT
        // produce two xmlns declarations on the root.
        let c = component(
            "Demo",
            vec![
                slot(
                    "rows-a",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
                slot(
                    "rows-b",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
            ],
            vec![],
        );
        let l = layout_with_root(
            "Demo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    component_ref_node(
                        "Grid",
                        vec![LayoutProp {
                            name: "rows".to_string(),
                            value: LayoutPropValue::SlotRef("rows-a".to_string()),
                        }],
                    ),
                    component_ref_node(
                        "Grid",
                        vec![LayoutProp {
                            name: "rows".to_string(),
                            value: LayoutPropValue::SlotRef("rows-b".to_string()),
                        }],
                    ),
                ],
            },
        );
        let mut reg = ComponentRegistry::new();
        reg.register(
            "Grid",
            "grid",
            "using:Mosaic.Package.Grid",
            "mosaic-pkg-grid",
        );
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        let count = r
            .xaml
            .matches("xmlns:grid=\"using:Mosaic.Package.Grid\"")
            .count();
        assert_eq!(count, 1, "expected dedup to one xmlns declaration");
    }

    #[test]
    fn registered_kernel_primitive_name_is_ignored_in_favour_of_kernel_emitter() {
        // If a registry happens to contain a name that ALSO matches a
        // kernel primitive, the kernel emitter wins. This protects
        // against accidental package shadowing of `Box`, `Text`, etc.
        let c = component("Demo", vec![], vec![]);
        let l = layout_with_root("Demo", box_root());
        let mut reg = ComponentRegistry::new();
        reg.register("Box", "evil", "using:Evil.Override", "evil-package");
        let r = compile_with_registry(&c, &l, &empty_style("Demo"), &reg);
        // The kernel <Border> emission is used, not the shadowing pkg.
        assert!(r.xaml.contains("<Border"), "got:\n{}", r.xaml);
        // No `evil:Box` reference.
        assert!(!r.xaml.contains("<evil:"));
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

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // PR-3: HostInput / HostButton / HostScroll tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

    // â”€â”€ HostInput â”€â”€

    #[test]
    fn host_input_with_part_name_uses_pascal_case_as_xname() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_input_node(Some("formula-field"), Vec::new()));
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
        assert!(r.xaml.contains("MaxLength=\"100\""), "got:\n{}", r.xaml);
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
        assert!(r.xaml.contains("IsReadOnly=\"True\""), "got:\n{}", r.xaml);
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
            r.code_behind
                .contains("private void FormulaField_TextChanged"),
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
            vec![emit("onCommit", vec![]), emit("onCancel", vec![])],
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

    // â”€â”€ HostButton â”€â”€

    #[test]
    fn host_button_lowers_to_button_with_xname() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_button_node(Some("submit"), Vec::new()));
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
        assert!(r.xaml.contains("Content=\"Submit\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn host_button_inside_indexed_for_dispatches_index_payload() {
        let c = component(
            "Foo",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit(
                "onSelect",
                vec![param("index", EmitPayloadType::Number)],
            )],
        );
        let l = layout_with_root(
            "Foo",
            for_node(
                LayoutPropValue::SlotRef("items".to_string()),
                "item",
                Some("i"),
                vec![host_button_node(
                    None,
                    vec![
                        LayoutProp {
                            name: "label".to_string(),
                            value: LayoutPropValue::Keyword("item".to_string()),
                        },
                        LayoutProp {
                            name: "onClick".to_string(),
                            value: LayoutPropValue::EmitRef("onSelect".to_string()),
                        },
                    ],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Content=\"{x:Bind Item}\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains(
                "new FooEvent.Select((sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is Foo_ItemVm row ? (double)row.Index : -1.0)"
            ),
            "got:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn host_button_inside_for_dispatches_text_item_payload() {
        let c = component(
            "Foo",
            vec![slot(
                "options",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit(
                "onChange",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = layout_with_root(
            "Foo",
            for_node(
                LayoutPropValue::SlotRef("options".to_string()),
                "option",
                Some("i"),
                vec![host_button_node(
                    None,
                    vec![
                        LayoutProp {
                            name: "label".to_string(),
                            value: LayoutPropValue::Keyword("option".to_string()),
                        },
                        LayoutProp {
                            name: "onClick".to_string(),
                            value: LayoutPropValue::EmitRef("onChange".to_string()),
                        },
                    ],
                )],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Content=\"{x:Bind Option}\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains(
                "new FooEvent.Change((sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is Foo_OptionVm row ? row.Option : string.Empty)"
            ),
            "got:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn host_button_disabled_slot_uses_not_helper() {
        let c = component("Foo", vec![slot("is-busy", SlotType::Bool, true)], vec![]);
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
        let c = component("Foo", vec![], vec![emit("onSubmit", vec![])]);
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

    // â”€â”€ HostScroll â”€â”€

    #[test]
    fn host_button_on_tap_alias_emits_dispatch_handler() {
        let c = component("Foo", vec![], vec![emit("onSubmit", vec![])]);
        let l = layout_with_root(
            "Foo",
            host_button_node(
                Some("submit"),
                vec![LayoutProp {
                    name: "onTap".to_string(),
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

    #[test]
    fn host_scroll_default_direction_is_vertical() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_scroll_node(None, Vec::new()));
        let r = compile(&c, &l, &empty_style("Foo"));
        // V=Auto, H=Disabled is the vertical default.
        assert!(
            r.xaml.contains(
                "VerticalScrollBarVisibility=\"Auto\" HorizontalScrollBarVisibility=\"Disabled\""
            ),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_scroll_horizontal_swaps_visibilities() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_scroll_node(Some("horizontal"), Vec::new()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains(
                "VerticalScrollBarVisibility=\"Disabled\" HorizontalScrollBarVisibility=\"Auto\""
            ),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_scroll_both_directions_both_auto() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_scroll_node(Some("both"), Vec::new()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains(
                "VerticalScrollBarVisibility=\"Auto\" HorizontalScrollBarVisibility=\"Auto\""
            ),
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

    // â”€â”€ Multi-Host counter â”€â”€

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

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // PR-2: For / If / Else / ExprLowerer tests
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn for_node(
        each: LayoutPropValue,
        as_name: &str,
        index: Option<&str>,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        let mut props = vec![
            LayoutProp {
                name: "each".to_string(),
                value: each,
            },
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
            props: vec![LayoutProp {
                name: "when".to_string(),
                value: when,
            }],
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

    // â”€â”€ For lowering â”€â”€

    #[test]
    fn for_with_slot_ref_lowers_to_items_repeater_with_data_template() {
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
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
        // ItemsRepeater bound to the generated row-VM projection.
        assert!(
            r.xaml
                .contains("<ItemsRepeater ItemsSource=\"{x:Bind GridRowVmRows}\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind
                .contains("public IReadOnlyList<Grid_RowVm> GridRowVmRows"),
            "got:\n{}",
            r.code_behind
        );
        assert!(r.code_behind.contains("var source = Rows;"));
        // DataTemplate with the generated RowVm typed DataContext.
        assert!(
            r.xaml
                .contains("<DataTemplate x:DataType=\"local:Grid_RowVm\">"),
            "got:\n{}",
            r.xaml
        );
        // Inner Text binds to the for-bound name.
        assert!(r.xaml.contains("Text=\"{x:Bind Row}\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn for_generates_row_vm_record() {
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
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
        assert!(vm
            .source
            .contains("public sealed record Grid_RowVm(string Row);"));
    }

    #[test]
    fn for_with_index_adds_index_field_to_row_vm() {
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
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
            vm.source
                .contains("public sealed record Grid_RowVm(string Row, int Index);"),
            "got:\n{}",
            vm.source
        );
    }

    #[test]
    fn for_with_numeric_list_uses_double_element_type() {
        let c = component(
            "Stats",
            vec![slot(
                "values",
                SlotType::List(Box::new(ListInnerType::Number)),
                true,
            )],
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
        // class. The emitter must register only one â€” the assembly step
        // in from_pipeline depends on uniqueness.
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
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

    // â”€â”€ If / Else lowering â”€â”€

    #[test]
    fn if_with_slot_ref_lowers_to_contentcontrol_with_visibility() {
        let c = component("Foo", vec![slot("editable", SlotType::Bool, true)], vec![]);
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
        let c = component("Foo", vec![slot("editable", SlotType::Bool, true)], vec![]);
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
        assert!(r
            .xaml
            .contains("<local:BoolToVisibilityConverter x:Key=\"BoolToVisibilityConverter\"/>"));
        // Only one occurrence â€” converter is shared.
        let count = r.xaml.matches("BoolToVisibilityConverter x:Key").count();
        assert_eq!(count, 1, "expected exactly one converter resource entry");
    }

    #[test]
    fn if_without_else_does_not_emit_else_wrapper() {
        let c = component("Foo", vec![slot("editable", SlotType::Bool, true)], vec![]);
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
        let c = component("Foo", vec![slot("editable", SlotType::Bool, true)], vec![]);
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

    // â”€â”€ ExprLowerer â”€â”€

    #[test]
    fn expr_lowerer_bare_slot_ref_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("slot: editable", &mut ctx);
        assert!(matches!(r, ExprLowering::Bindable(ref p) if p == "Editable"));
    }

    #[test]
    fn expr_lowerer_dotted_member_access_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("slot: theme.dark.bg", &mut ctx);
        assert!(
            matches!(r, ExprLowering::Bindable(ref p) if p == "Theme.Dark.Bg"),
            "got: bindable shape mismatch"
        );
    }

    #[test]
    fn expr_lowerer_boolean_literal_is_bindable() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("true", &mut ctx);
        assert!(matches!(r, ExprLowering::Bindable(ref p) if p == "True"));
    }

    #[test]
    fn expr_lowerer_indexer_becomes_helper_call() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
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
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("slot: edit-row == 0", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers.len(), 1);
        assert_eq!(ctx.helpers[0].return_type, "bool");
    }

    #[test]
    fn expr_lowerer_logical_and_becomes_helper() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers[0].return_type, "bool");
        assert!(ctx.helpers[0].body.contains(" && "));
    }

    #[test]
    fn expr_lowerer_unary_not_becomes_helper() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let r = lower_expr_for_xbind("!slot: editable", &mut ctx);
        assert!(matches!(r, ExprLowering::Helper(_)));
        assert_eq!(ctx.helpers[0].return_type, "bool");
    }

    #[test]
    fn expr_lowerer_identical_expressions_dedupe_to_one_helper() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        let _ = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        let _ = lower_expr_for_xbind("slot: a && slot: b", &mut ctx);
        assert_eq!(ctx.helpers.len(), 1, "expected dedup");
    }

    #[test]
    fn expr_lowerer_for_bound_name_lowers_as_parameter() {
        let mut ctx = EmitContext::new("Foo", &[], &[]);
        // Simulate a For binding being in scope.
        ctx.for_scope.push(ForBinding {
            as_name: "row".to_string(),
            index_name: Some("r".to_string()),
            element_type: "string".to_string(),
            vm_class: "Foo_RowVm".to_string(),
            projection_property: None,
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

    // â”€â”€ End-to-end: For + If together â”€â”€

    #[test]
    fn for_body_can_contain_if_with_for_bound_name_via_expr() {
        // For (each: rows, as: row) { If (when: row.editable) { Text(...) } Else { Text(...) } }
        let c = component(
            "Grid",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
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

    // â”€â”€ part-style application â”€â”€

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

    // â”€â”€ unused-flag placeholders â”€â”€

    /// `EmitOptions::emit_project = false` (default) â†’ `project` is
    /// `None`, no host shell emitted.
    #[test]
    fn project_field_is_none_when_emit_project_false() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root());
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.project.is_none());
    }

    /// Fix B1: `EmitOptions::emit_project = true` â†’ `project` is
    /// populated with the full WinUI 3 host shell (csproj + App +
    /// MainWindow + manifest + build.ps1 + README).
    #[test]
    fn project_field_populated_when_emit_project_true() {
        let c = component("Foo", vec![slot("greeting", SlotType::Text, true)], vec![]);
        let l = layout_with_root("Foo", box_root());
        let s = empty_style("Foo");
        let mut o = opts();
        o.emit_project = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let p = r.project.as_ref().expect("project populated");
        // csproj has the WindowsAppSDK reference + unpackaged WinUI build switches.
        assert!(p.csproj.contains("Microsoft.WindowsAppSDK"));
        assert!(p.csproj.contains("AppxGeneratePriEnabled>false"));
        assert!(p.csproj.contains("EnableCoreMrtTooling>false"));
        assert!(p.csproj.contains("UseRidGraph>true"));
        assert!(p.csproj.contains("FlattenNativeRuntimeDlls"));
        assert!(p.csproj.contains("CopyMosaicNativeHostLibraries"));
        assert!(p.csproj.contains("$(MSBuildProjectDirectory)\\*.dll"));
        // App.xaml.cs references MainWindow.
        assert!(p.app_xaml_cs.contains("new MainWindow()"));
        // MainWindow.xaml.cs has the dispatch stub.
        assert!(p.main_window_cs.contains("OnComponentDispatch"));
        // MainWindow.xaml.cs can optionally delegate props/events to an
        // app-provided MosaicHost without requiring one to compile.
        assert!(p.main_window_cs.contains("TryApplyMosaicHostProps"));
        assert!(p.main_window_cs.contains("CoerceMosaicHostResult"));
        assert!(p.main_window_cs.contains("FindMosaicHostMethod"));
        assert!(p
            .main_window_cs
            .contains("private async void OnComponentDispatch"));
        assert!(p.main_window_cs.contains("await TryHandleMosaicHostEvent"));
        assert!(p.main_window_cs.contains("TryHandleMosaicHostIntent"));
        assert!(p.main_window_cs.contains("UnwrapMosaicHostResultAsync"));
        assert!(p.main_window_cs.contains("HandleHostIntent"));
        assert!(p.main_window_cs.contains("Mosaic.Generated.MosaicHost"));
        // MainWindow constructor pre-populates the Greeting slot stub.
        assert!(p.main_window_cs.contains("Greeting"));
        // build.ps1 passes -p:Platform=x64.
        assert!(p.build_script.contains("-p:Platform=x64"));
        assert!(p.build_script.contains("Get-Command dotnet"));
        assert!(p.build_script.contains("dotnet\\dotnet.exe"));
        assert!(p.build_script.contains("exit 127"));
        assert!(p.build_script.contains("$buildExitCode = $LASTEXITCODE"));
        assert!(p.build_script.contains("exit $buildExitCode"));
        // README documents the framework-dependent runtime requirement.
        assert!(p.readme.contains("Microsoft.WindowsAppRuntime.1.7"));
        // app.manifest declares DPI awareness.
        assert!(p.package_manifest.contains("PerMonitorV2"));
    }

    #[test]
    fn project_dispatch_match_uses_keyword_safe_payload_patterns() {
        let c = component(
            "Foo",
            vec![],
            vec![emit(
                "onToggle",
                vec![param("checked", EmitPayloadType::Bool)],
            )],
        );
        let l = layout_with_root("Foo", box_root());
        let s = empty_style("Foo");
        let mut o = opts();
        o.emit_project = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let p = r.project.as_ref().expect("project populated");
        assert!(
            p.main_window_cs
                .contains("case FooEvent.Toggle(var payload0) c:"),
            "got:\n{}",
            p.main_window_cs
        );
        assert!(
            !p.main_window_cs
                .contains("case FooEvent.Toggle(checked) c:"),
            "payload keyword must not be emitted as a pattern variable, got:\n{}",
            p.main_window_cs
        );
    }

    /// Fix B1: for a UserControl-rooted component, the MainWindow
    /// hosts the component directly in its Grid.
    #[test]
    fn project_main_window_hosts_user_control_directly() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", box_root()); // Box â†’ UserControl root
        let s = empty_style("Foo");
        let mut o = opts();
        o.emit_project = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let p = r.project.as_ref().unwrap();
        // The component appears as `<gen:Foo ... />` in the Grid.
        assert!(
            p.main_window_xaml.contains("<gen:Foo"),
            "got:\n{}",
            p.main_window_xaml
        );
    }

    /// Fix B1: for a HostDialog-rooted component, MainWindow has a
    /// Button + ShowAsync glue, not a direct embed.
    #[test]
    fn project_main_window_for_dialog_has_button_and_show_async() {
        let c = component("Foo", vec![], vec![]);
        let dialog_root = LayoutNode {
            tag: "HostDialog".to_string(),
            part_name: None,
            props: Vec::new(),
            children: Vec::new(),
        };
        let l = layout_with_root("Foo", dialog_root);
        let s = empty_style("Foo");
        let mut o = opts();
        o.emit_project = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let p = r.project.as_ref().unwrap();
        assert!(
            p.main_window_xaml.contains("Open the dialog"),
            "got:\n{}",
            p.main_window_xaml
        );
        assert!(
            p.main_window_cs.contains("ShowAsync"),
            "got:\n{}",
            p.main_window_cs
        );
        // Fix D1: use the button's XamlRoot.
        assert!(
            p.main_window_cs
                .contains("(sender as FrameworkElement)?.XamlRoot"),
            "got:\n{}",
            p.main_window_cs
        );
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

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // U29-1-K-xaml: HostDialog tests (UI29-1 Â§3.6)
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn host_dialog_node(
        part: Option<&str>,
        props: Vec<LayoutProp>,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            tag: "HostDialog".to_string(),
            part_name: part.map(String::from),
            props,
            children,
        }
    }

    #[test]
    fn host_dialog_empty_emits_contentdialog() {
        // Test 1 + Test 8: A bare HostDialog with no props lowers to
        // <ContentDialog> (the modal default) â€” and is recognised, i.e.
        // does not return UnsupportedPrimitive.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_dialog_node(None, Vec::new(), Vec::new()));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<ContentDialog"),
            "expected <ContentDialog, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("</ContentDialog>"),
            "expected closing tag, got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_dialog_modal_true_uses_contentdialog() {
        // Test 2: explicit `modal: true` keyword also lowers to
        // ContentDialog (same as the default).
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "modal".to_string(),
                    value: LayoutPropValue::Keyword("true".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<ContentDialog"), "got:\n{}", r.xaml);
        assert!(!r.xaml.contains("<Flyout"), "got:\n{}", r.xaml);
    }

    /// `modal: false` switches the *nested* HostDialog emission to a
    /// `<Flyout>` (popover form per spec Â§3.6). At the moslayout root,
    /// `modal:` is honored but the XAML root remains `<ContentDialog>`
    /// because Flyout cannot be a XAML root (it's an anchored
    /// popover). Updated for Fix A1: HostDialog-at-root â†’ ContentDialog
    /// root regardless of modal.
    #[test]
    fn nested_host_dialog_modal_false_uses_flyout() {
        let c = component("Foo", vec![], vec![]);
        // Nest the HostDialog inside a Box so we exercise the nested
        // emission path, not the root-hoisting one.
        let nested = host_dialog_node(
            None,
            vec![LayoutProp {
                name: "modal".to_string(),
                value: LayoutPropValue::Keyword("false".to_string()),
            }],
            Vec::new(),
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![nested],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<Flyout"), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("</Flyout>"), "got:\n{}", r.xaml);
    }

    /// Fix A1: HostDialog at the layout root hoists to a
    /// `<ContentDialog>` XAML root regardless of the `modal:` flag â€”
    /// Flyout cannot be a XAML root. The behavior `modal:` was meant
    /// to control surfaces at runtime (e.g. via
    /// IsLightDismissEnabled) and is documented as future work.
    #[test]
    fn host_dialog_at_root_modal_false_still_uses_contentdialog_root() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "modal".to_string(),
                    value: LayoutPropValue::Keyword("false".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<ContentDialog"),
            "expected ContentDialog root, got:\n{}",
            r.xaml
        );
        // Should NOT emit a wrapping UserControl.
        assert!(!r.xaml.contains("<UserControl"), "got:\n{}", r.xaml);
    }

    /// Fix A3: `title: slot: t` becomes `Title="{x:Bind ..., Mode=OneWay}"`,
    /// consistent with every other emitter (which uses `{x:Bind}`).
    /// Fix A4: when the component's root is ContentDialog, the slot
    /// `t` does NOT collide with `Title` (only a slot literally named
    /// `title` would). The slot's PascalCased name `T` is used.
    #[test]
    fn host_dialog_title_slot_emits_xbind_oneway_after_a3() {
        let c = component("Foo", vec![slot("t", SlotType::Text, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "title".to_string(),
                    value: LayoutPropValue::SlotRef("t".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("Title=\"{x:Bind T, Mode=OneWay}\""),
            "got:\n{}",
            r.xaml
        );
        // Make sure no {Binding} leaked through.
        assert!(!r.xaml.contains("{Binding"), "got:\n{}", r.xaml);
    }

    /// Fix A4: a slot literally named `title` on a ContentDialog-rooted
    /// component must be aliased to `DialogTitle` (so the DP doesn't
    /// shadow `ContentDialog.Title`). Both the DP declaration in the
    /// code-behind and the `{x:Bind}` path in the XAML resolve to the
    /// alias.
    #[test]
    fn host_dialog_title_slot_named_title_aliases_to_dialog_title() {
        let c = component("Foo", vec![slot("title", SlotType::Text, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "title".to_string(),
                    value: LayoutPropValue::SlotRef("title".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // XAML uses the aliased path on the outer ContentDialog.
        assert!(
            r.xaml
                .contains("Title=\"{x:Bind DialogTitle, Mode=OneWay}\""),
            "got:\n{}",
            r.xaml
        );
        // Code-behind declares DialogTitle, not Title.
        assert!(
            r.code_behind.contains("public string DialogTitle"),
            "got:\n{}",
            r.code_behind
        );
        assert!(
            !r.code_behind.contains("public string Title"),
            "Title would shadow ContentDialog.Title; got:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn host_dialog_with_children_renders_them_inside() {
        // Test 5: Children of HostDialog land inside the element body.
        // Verifies the emit_xaml_children walk runs at indent + 4.
        let c = component("Foo", vec![], vec![]);
        let child_text = LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String("Hello".to_string()),
            }],
            children: Vec::new(),
        };
        let l = layout_with_root("Foo", host_dialog_node(None, Vec::new(), vec![child_text]));
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(r.xaml.contains("<ContentDialog"), "got:\n{}", r.xaml);
        // The Text child renders as a <TextBlock Text="Hello"/> inside
        // the dialog body â€” substring check both for the literal text
        // and the order (TextBlock appears before the closing tag).
        let close = r.xaml.find("</ContentDialog>").expect("closing tag");
        let body_substr = &r.xaml[..close];
        assert!(
            body_substr.contains("<TextBlock Text=\"Hello\""),
            "expected TextBlock child inside ContentDialog body, got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_dialog_on_close_emits_handler_and_dispatch() {
        // Test 6: `onClose: emit: onCloseMe` emits Closed="..." on the
        // XAML element AND a private handler method in the code-behind
        // that invokes Dispatch with the right event case.
        let c = component("Foo", vec![], vec![emit("onCloseMe", Vec::new())]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "onClose".to_string(),
                    value: LayoutPropValue::EmitRef("onCloseMe".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // The XAML attribute references the generated handler name
        // (`OnHostDialogClose_<n>` per the spec's stub convention).
        assert!(
            r.xaml.contains("Closed=\"OnHostDialogClose_"),
            "got:\n{}",
            r.xaml
        );
        // The code-behind has the matching private void handler.
        assert!(
            r.code_behind.contains("private void OnHostDialogClose_"),
            "got:\n{}",
            r.code_behind
        );
        // The handler body dispatches the CloseMe case (the `on`
        // prefix gets stripped per strip_on_prefix â†’ PascalCase).
        assert!(
            r.code_behind.contains("new FooEvent.CloseMe()"),
            "got:\n{}",
            r.code_behind
        );
    }

    /// Fix A2: `open: slot: x` previously emitted a
    /// `mos:Dialog.IsOpen="{Binding X}"` attribute referencing an
    /// undeclared `mos:` namespace. That broke XAML parsing at
    /// runtime. The fix drops the attribute entirely â€” the host code-
    /// behind is the only mechanism for show/hide, and the comment
    /// stub documents that contract clearly.
    #[test]
    fn host_dialog_open_slot_emits_comment_stub_only_after_a2() {
        let c = component("Foo", vec![slot("show", SlotType::Bool, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "open".to_string(),
                    value: LayoutPropValue::SlotRef("show".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // Fix A2: no `mos:Dialog.IsOpen` attribute.
        assert!(
            !r.xaml.contains("mos:Dialog.IsOpen"),
            "mos:Dialog.IsOpen should NOT be emitted (undeclared namespace), got:\n{}",
            r.xaml
        );
        // But the comment stub still documents the host's
        // ShowAsync()/Hide() responsibility.
        assert!(
            r.xaml.contains("<!-- HostDialog "),
            "expected a code-behind stub comment, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("ShowAsync()"),
            "expected stub to mention ShowAsync(), got:\n{}",
            r.xaml
        );
        // The slot path appears in the comment (so authors can grep
        // for it).
        assert!(
            r.xaml.contains("'Show'"),
            "expected comment to reference the bound DP, got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_dialog_is_recognised_not_unknown_primitive() {
        // Test 8 (explicit): HostDialog must NOT surface as
        // UnsupportedPrimitive or any other error â€” that was the whole
        // point of UI29-1 Â§3.6. Construct a moderately-decorated
        // HostDialog and assert from_pipeline returns Ok.
        let c = component(
            "Foo",
            vec![
                slot("open-state", SlotType::Bool, true),
                slot("dialog-title", SlotType::Text, true),
            ],
            vec![emit("onClose", Vec::new())],
        );
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                Some("dialog-shell"),
                vec![
                    LayoutProp {
                        name: "open".to_string(),
                        value: LayoutPropValue::SlotRef("open-state".to_string()),
                    },
                    LayoutProp {
                        name: "modal".to_string(),
                        value: LayoutPropValue::Keyword("true".to_string()),
                    },
                    LayoutProp {
                        name: "title".to_string(),
                        value: LayoutPropValue::SlotRef("dialog-title".to_string()),
                    },
                    LayoutProp {
                        name: "onClose".to_string(),
                        value: LayoutPropValue::EmitRef("onClose".to_string()),
                    },
                ],
                Vec::new(),
            ),
        );
        let s = empty_style("Foo");
        // The cardinal assertion: the emitter SUCCEEDS for HostDialog.
        let result = from_pipeline(&c, &l, &s, None, &opts());
        assert!(
            result.is_ok(),
            "HostDialog should not error, got: {:?}",
            result
        );
    }

    #[test]
    fn host_dialog_dismiss_on_backdrop_false_emits_comment() {
        // Bonus test: dismiss-on-backdrop: false is documented as
        // not-cleanly-bindable on WinUI 3; the emitter should surface
        // a comment rather than silently drop or crash.
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            host_dialog_node(
                None,
                vec![LayoutProp {
                    name: "dismiss-on-backdrop".to_string(),
                    value: LayoutPropValue::Keyword("false".to_string()),
                }],
                Vec::new(),
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("dismiss-on-backdrop"),
            "expected stub comment mentioning dismiss-on-backdrop, got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("<ContentDialog"), "got:\n{}", r.xaml);
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // UI29-2 â€” HostCheckbox + HostRadio
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Helper: a one-component layout def rooted at a `HostCheckbox`.
    /// The HostCheckbox itself is wrapped in a `Box` root so the XAML
    /// root-shape selector treats it as a normal in-flow widget (the
    /// HostDialog-special-cased "root is dialog â†’ ContentDialog" rule
    /// doesn't fire).
    fn checkbox_in_box(props: Vec<LayoutProp>) -> LayoutDef {
        layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostCheckbox".to_string(),
                    part_name: None,
                    props,
                    children: Vec::new(),
                }],
            },
        )
    }

    /// Helper mirror for HostRadio.
    fn radio_in_box(props: Vec<LayoutProp>) -> LayoutDef {
        layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostRadio".to_string(),
                    part_name: None,
                    props,
                    children: Vec::new(),
                }],
            },
        )
    }

    /// UI29-2 XAML test 1 â€” bare HostCheckbox emits a `<CheckBox>`
    /// self-closing element with only the auto-assigned x:Name.
    #[test]
    fn host_checkbox_empty_emits_checkbox_with_xname() {
        let c = component("X", vec![], vec![]);
        let l = checkbox_in_box(vec![]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("<CheckBox x:Name="),
            "expected `<CheckBox x:Name=...>`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 2 â€” `checked: slot: c` emits
    /// `IsChecked="{x:Bind C, Mode=OneWay}"` (PascalCased slot, OneWay
    /// binding mirroring HostInput/HostButton's slot-binding form).
    #[test]
    fn host_checkbox_checked_slot_emits_xbind_is_checked() {
        let c = component("X", vec![slot("is-checked", SlotType::Bool, true)], vec![]);
        let l = checkbox_in_box(vec![LayoutProp {
            name: "checked".to_string(),
            value: LayoutPropValue::SlotRef("is-checked".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml
                .contains("IsChecked=\"{x:Bind IsChecked, Mode=OneWay}\""),
            "expected `IsChecked=\"{{x:Bind IsChecked, Mode=OneWay}}\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 3 â€” `label: "Agree"` flows into the
    /// `Content="..."` attribute (XAML's analog of children for
    /// content controls).
    #[test]
    fn host_checkbox_string_label_emits_content_literal() {
        let c = component("X", vec![], vec![]);
        let l = checkbox_in_box(vec![LayoutProp {
            name: "label".to_string(),
            value: LayoutPropValue::String("Agree".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("Content=\"Agree\""),
            "expected `Content=\"Agree\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 4 â€” `disabled: slot: d` reuses HostButton's
    /// `Not(bool)` helper to flip polarity into the XAML-native
    /// `IsEnabled` property.
    #[test]
    fn host_checkbox_disabled_slot_uses_not_helper_for_is_enabled() {
        let c = component("X", vec![slot("locked", SlotType::Bool, true)], vec![]);
        let l = checkbox_in_box(vec![LayoutProp {
            name: "disabled".to_string(),
            value: LayoutPropValue::SlotRef("locked".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("IsEnabled=\"{x:Bind Not(Locked)}\""),
            "expected `IsEnabled=\"{{x:Bind Not(Locked)}}\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 5 â€” `onToggle: emit: onChange` wires BOTH the
    /// `Checked` and `Unchecked` events to code-behind handlers that
    /// dispatch with the matching `checked: bool` payload (true for
    /// Checked, false for Unchecked). Matches UI29-2 Â§2.2's kernel-
    /// canonical onToggle(checked: bool) signature.
    #[test]
    fn host_checkbox_on_toggle_emits_checked_and_unchecked_handler_pair() {
        let c = component(
            "X",
            vec![],
            vec![EmitDecl {
                name: "onChange".to_string(),
                params: vec![EmitParam {
                    name: "checked".to_string(),
                    r#type: EmitPayloadType::Bool,
                }],
            }],
        );
        let l = checkbox_in_box(vec![LayoutProp {
            name: "onToggle".to_string(),
            value: LayoutPropValue::EmitRef("onChange".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("Checked=\""),
            "expected `Checked=...` attribute, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("Unchecked=\""),
            "expected `Unchecked=...` attribute, got:\n{}",
            r.xaml
        );
        // Both handlers should appear in the code-behind. The Checked
        // handler dispatches true, the Unchecked handler dispatches
        // false â€” matching the kernel-canonical checked: bool payload.
        assert!(
            r.code_behind.contains("XEvent.Change(true)"),
            "expected `XEvent.Change(true)` in code-behind, got:\n{}",
            r.code_behind
        );
        assert!(
            r.code_behind.contains("XEvent.Change(false)"),
            "expected `XEvent.Change(false)` in code-behind, got:\n{}",
            r.code_behind
        );
    }

    /// UI29-2 XAML test 6 â€” `indeterminate: true` (or any slot ref)
    /// adds `IsThreeState="True"` so the visual tri-state mode is on.
    /// The actual `IsChecked = null` toggle is the host's job via the
    /// bound slot â€” WinUI doesn't have a "show as indeterminate"
    /// attribute, only the tri-state-enabled flag.
    #[test]
    fn host_checkbox_indeterminate_keyword_enables_three_state() {
        let c = component("X", vec![], vec![]);
        let l = checkbox_in_box(vec![LayoutProp {
            name: "indeterminate".to_string(),
            value: LayoutPropValue::Keyword("true".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("IsThreeState=\"True\""),
            "expected `IsThreeState=\"True\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 7 â€” bare HostRadio emits a `<RadioButton>`
    /// self-closing element with only x:Name.
    #[test]
    fn host_radio_empty_emits_radio_button_with_xname() {
        let c = component("X", vec![], vec![]);
        let l = radio_in_box(vec![]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("<RadioButton x:Name="),
            "expected `<RadioButton x:Name=...>`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 8 â€” `group: "flavor"` lowers to WinUI's native
    /// `GroupName="flavor"` attribute. WinUI auto-deselects siblings
    /// sharing GroupName when one IsChecked goes true â€” true radio-
    /// group behavior at the XAML level (matches UI29-2's design).
    #[test]
    fn host_radio_group_string_emits_group_name_attribute() {
        let c = component("X", vec![], vec![]);
        let l = radio_in_box(vec![LayoutProp {
            name: "group".to_string(),
            value: LayoutPropValue::String("flavor".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("GroupName=\"flavor\""),
            "expected `GroupName=\"flavor\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-2 XAML test 9 â€” `onSelect: emit: onPick` + `value:
    /// "vanilla"` wires ONLY the Checked event (Unchecked is silent â€”
    /// sibling-caused deselects don't fire onSelect, per UI29-2 Â§2.2).
    /// The code-behind dispatches XEvent.Pick("vanilla") via the
    /// generated C# string-literal payload.
    #[test]
    fn host_radio_on_select_with_string_value_dispatches_literal() {
        let c = component(
            "X",
            vec![],
            vec![EmitDecl {
                name: "onPick".to_string(),
                params: vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Text,
                }],
            }],
        );
        let l = radio_in_box(vec![
            LayoutProp {
                name: "value".to_string(),
                value: LayoutPropValue::String("vanilla".to_string()),
            },
            LayoutProp {
                name: "onSelect".to_string(),
                value: LayoutPropValue::EmitRef("onPick".to_string()),
            },
        ]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("Checked=\""),
            "expected Checked= attribute, got:\n{}",
            r.xaml
        );
        // Onunchecked NOT wired â€” sibling deselects are silent.
        assert!(
            !r.xaml.contains("Unchecked=\""),
            "Unchecked must NOT be wired for HostRadio onSelect, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("XEvent.Pick(\"vanilla\")"),
            "expected `XEvent.Pick(\"vanilla\")` C# dispatch, got:\n{}",
            r.code_behind
        );
    }

    /// UI29-2 XAML test 10 â€” `value: slot: v` flows the camelCased
    /// slot identifier into the dispatch as `this.<Pascal>` so the
    /// runtime value drives the payload.
    #[test]
    fn host_radio_on_select_with_slot_value_dispatches_property_ref() {
        let c = component(
            "X",
            vec![slot("radio-value", SlotType::Text, true)],
            vec![EmitDecl {
                name: "onPick".to_string(),
                params: vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Text,
                }],
            }],
        );
        let l = radio_in_box(vec![
            LayoutProp {
                name: "value".to_string(),
                value: LayoutPropValue::SlotRef("radio-value".to_string()),
            },
            LayoutProp {
                name: "onSelect".to_string(),
                value: LayoutPropValue::EmitRef("onPick".to_string()),
            },
        ]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.code_behind.contains("XEvent.Pick(this.RadioValue)"),
            "expected `XEvent.Pick(this.RadioValue)` runtime dispatch, got:\n{}",
            r.code_behind
        );
    }

    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // UI29-4 â€” HostLink + HostTooltip + HostNumberInput
    // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn link_in_box(props: Vec<LayoutProp>) -> LayoutDef {
        layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostLink".to_string(),
                    part_name: None,
                    props,
                    children: Vec::new(),
                }],
            },
        )
    }

    fn number_input_in_box(props: Vec<LayoutProp>) -> LayoutDef {
        layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostNumberInput".to_string(),
                    part_name: None,
                    props,
                    children: Vec::new(),
                }],
            },
        )
    }

    /// UI29-4 XAML test 1 â€” bare `HostLink href + label` lowers to
    /// `<HyperlinkButton NavigateUri="..." Content="..."/>`.
    #[test]
    fn host_link_string_href_and_label_emits_hyperlink_button() {
        let c = component("X", vec![], vec![]);
        let l = link_in_box(vec![
            LayoutProp {
                name: "href".to_string(),
                value: LayoutPropValue::String("https://example.com".to_string()),
            },
            LayoutProp {
                name: "label".to_string(),
                value: LayoutPropValue::String("Click me".to_string()),
            },
        ]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("<HyperlinkButton x:Name="),
            "expected `<HyperlinkButton x:Name=...>`, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("NavigateUri=\"https://example.com\""),
            "expected `NavigateUri=\"https://example.com\"`, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("Content=\"Click me\""),
            "expected `Content=\"Click me\"`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-4 XAML test 2 â€” `external: false` + `onActivate` swaps
    /// to a Button with a Click handler that dispatches the named
    /// emit with the href in the payload (in-app routing path).
    #[test]
    fn host_link_external_false_with_on_activate_emits_button_with_click() {
        let c = component(
            "X",
            vec![],
            vec![EmitDecl {
                name: "onNavigate".to_string(),
                params: vec![EmitParam {
                    name: "href".to_string(),
                    r#type: EmitPayloadType::Text,
                }],
            }],
        );
        let l = link_in_box(vec![
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
        ]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("<Button x:Name="),
            "expected Button (not HyperlinkButton) for external=false, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("Click=\""),
            "expected Click handler attribute, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("XEvent.Navigate(\"/about\")"),
            "expected dispatch with href in payload, got:\n{}",
            r.code_behind
        );
    }

    /// UI29-4 XAML test 3 â€” a link inside an indexed `For` dispatches
    /// the row index and binds the row item as its label.
    #[test]
    fn host_link_inside_indexed_for_dispatches_index_payload() {
        let c = component(
            "Nav",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit(
                "onSelect",
                vec![param("index", EmitPayloadType::Number)],
            )],
        );
        let l = layout_with_root(
            "Nav",
            for_node(
                LayoutPropValue::SlotRef("items".to_string()),
                "item",
                Some("i"),
                vec![LayoutNode {
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
            ),
        );
        let r = compile(&c, &l, &empty_style("Nav"));
        assert!(
            r.xaml.contains("Content=\"{x:Bind Item}\""),
            "expected HostLink label to bind For item, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains(
                "new NavEvent.Select((sender as Microsoft.UI.Xaml.FrameworkElement)?.DataContext is Nav_ItemVm row ? (double)row.Index : -1.0)"
            ),
            "expected HostLink to dispatch For index payload, got:\n{}",
            r.code_behind
        );
    }

    /// UI29-4 XAML test 4 â€” `HostTooltip` wraps its child in a
    /// `Border` with the `ToolTipService.ToolTip` attached property.
    #[test]
    fn host_tooltip_wraps_child_in_border_with_tooltip_service() {
        let c = component("X", vec![], vec![]);
        let l = layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostTooltip".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "text".to_string(),
                        value: LayoutPropValue::String("Click to submit".to_string()),
                    }],
                    children: vec![LayoutNode {
                        tag: "HostButton".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "label".to_string(),
                            value: LayoutPropValue::String("Submit".to_string()),
                        }],
                        children: Vec::new(),
                    }],
                }],
            },
        );
        let r = compile(&c, &l, &empty_style("X"));
        let out = &r.xaml;
        assert!(
            out.contains("<Border ToolTipService.ToolTip=\"Click to submit\""),
            "expected Border with ToolTipService.ToolTip, got:\n{out}"
        );
        assert!(
            out.contains("</Border>"),
            "expected closing </Border>, got:\n{out}"
        );
    }

    /// UI29-4 XAML test 4 â€” bare `HostNumberInput` lowers to a
    /// `<NumberBox x:Name="..."/>` self-closing element.
    #[test]
    fn host_number_input_empty_emits_numberbox() {
        let c = component("X", vec![], vec![]);
        let l = number_input_in_box(vec![]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("<NumberBox x:Name="),
            "expected `<NumberBox x:Name=...>`, got:\n{}",
            r.xaml
        );
    }

    /// UI29-4 XAML test 5 â€” `min`/`max`/`step` numeric literals map
    /// to WinUI's `Minimum`/`Maximum`/`SmallChange` NumberBox
    /// properties.
    #[test]
    fn host_number_input_min_max_step_map_to_winui_numberbox_props() {
        let c = component("X", vec![], vec![]);
        let l = number_input_in_box(vec![
            LayoutProp {
                name: "min".to_string(),
                value: LayoutPropValue::Number(0.0),
            },
            LayoutProp {
                name: "max".to_string(),
                value: LayoutPropValue::Number(100.0),
            },
            LayoutProp {
                name: "step".to_string(),
                value: LayoutPropValue::Number(5.0),
            },
        ]);
        let r = compile(&c, &l, &empty_style("X"));
        let out = &r.xaml;
        assert!(
            out.contains("Minimum=\"0\""),
            "expected Minimum=0, got:\n{out}"
        );
        assert!(
            out.contains("Maximum=\"100\""),
            "expected Maximum=100, got:\n{out}"
        );
        assert!(
            out.contains("SmallChange=\"5\""),
            "expected SmallChange=5, got:\n{out}"
        );
    }

    /// UI29-4 XAML test 6 â€” `onChange: emit: onSet` registers a
    /// `ValueChanged` handler in the code-behind that dispatches
    /// `XEvent.Set(args.NewValue)` â€” WinUI's standard NumberBox
    /// event-arg shape.
    #[test]
    fn host_number_input_on_change_emits_value_changed_handler() {
        let c = component(
            "X",
            vec![],
            vec![EmitDecl {
                name: "onSet".to_string(),
                params: vec![EmitParam {
                    name: "value".to_string(),
                    r#type: EmitPayloadType::Number,
                }],
            }],
        );
        let l = number_input_in_box(vec![LayoutProp {
            name: "onChange".to_string(),
            value: LayoutPropValue::EmitRef("onSet".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml.contains("ValueChanged=\""),
            "expected ValueChanged attribute, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("XEvent.Set(args.NewValue)"),
            "expected dispatch with args.NewValue payload, got:\n{}",
            r.code_behind
        );
    }

    // â”€â”€ #4548 toolkit-demo emitter-gap regressions â”€â”€

    /// Helper: build a Box with a part name carrying a style on
    /// every interesting CSS property.
    fn styled_box_with_text_child(part: &str) -> LayoutNode {
        LayoutNode {
            tag: "Box".to_string(),
            part_name: Some(part.to_string()),
            props: Vec::new(),
            children: vec![LayoutNode {
                tag: "Text".to_string(),
                part_name: Some(format!("{part}-text")),
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::String("hello".to_string()),
                }],
                children: Vec::new(),
            }],
        }
    }

    fn style_for_box(part: &str, props: Vec<(&str, &str)>) -> StyleDef {
        StyleDef {
            component_name: "Foo".to_string(),
            parts: vec![PartStyle {
                name: part.to_string(),
                base: props
                    .into_iter()
                    .map(|(k, v)| StyleProp {
                        name: k.to_string(),
                        value: v.to_string(),
                    })
                    .collect(),
                states: Vec::new(),
            }],
        }
    }

    /// X1: `border-radius` lowers to WinUI's `CornerRadius`, not
    /// the made-up `BorderRadius` attribute (which doesn't exist
    /// in WinUI 3 and causes the XAML markup compiler to fail).
    #[test]
    fn border_radius_lowers_to_corner_radius() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", styled_box_with_text_child("frame"));
        let s = style_for_box("frame", vec![("border-radius", "4")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("CornerRadius=\"4\""),
            "expected CornerRadius=\"4\" in output, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("BorderRadius=\""),
            "BorderRadius is not a WinUI 3 property; got:\n{}",
            r.xaml
        );
    }

    /// X2: when the pascal-cased part name collides with the
    /// enclosing component class name (e.g. component `Button`
    /// and part `button`), the x:Name gets an `Element` suffix to
    /// avoid C# CS0542 ("member names cannot be the same as
    /// their enclosing type").
    #[test]
    fn x_name_avoids_component_class_name_collision() {
        let c = component("Button", vec![], vec![emit("onClick", vec![])]);
        // Part name "button" pascal-cases to "Button" â€” same as the
        // component class. The emitter must rename to ButtonElement.
        let l = layout_with_root(
            "Button",
            host_button_node(
                Some("button"),
                vec![LayoutProp {
                    name: "onClick".to_string(),
                    value: LayoutPropValue::EmitRef("onClick".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Button"));
        assert!(
            r.xaml.contains("x:Name=\"ButtonElement\""),
            "expected x:Name=\"ButtonElement\" (collision with class), got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("x:Name=\"Button\""),
            "raw x:Name=\"Button\" would collide with class Button, got:\n{}",
            r.xaml
        );
        // The handler stem is derived from x_name; both XAML and code-
        // behind must use the renamed identifier consistently.
        assert!(
            r.xaml.contains("Click=\"ButtonElement_Click\""),
            "handler stem must follow renamed x_name, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("private void ButtonElement_Click"),
            "code-behind handler must match renamed x_name, got:\n{}",
            r.code_behind
        );
    }

    /// X2 negative: when the pascal-cased part name does NOT
    /// collide with the component name, the original identifier
    /// is preserved. (Catches accidental over-suffixing.)
    #[test]
    fn x_name_unchanged_when_no_collision() {
        let c = component("Card", vec![], vec![emit("onClick", vec![])]);
        let l = layout_with_root(
            "Card",
            host_button_node(
                Some("submit"),
                vec![LayoutProp {
                    name: "onClick".to_string(),
                    value: LayoutPropValue::EmitRef("onClick".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Card"));
        assert!(
            r.xaml.contains("x:Name=\"Submit\""),
            "expected unchanged x:Name=\"Submit\", got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("SubmitElement"),
            "non-colliding name must not be suffixed, got:\n{}",
            r.xaml
        );
    }

    /// X3: `<Border>` doesn't accept Foreground / FontSize /
    /// FontWeight / FontFamily. The emitter splits the style props:
    /// container-paint stays on the Border, text-style attrs go
    /// into a scoped Style TargetType="TextBlock" resource that
    /// cascades to TextBlock descendants via WinUI implicit-style
    /// resolution.
    #[test]
    fn box_partitions_style_between_border_and_textblock_resource() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", styled_box_with_text_child("alert"));
        let s = style_for_box(
            "alert",
            vec![
                ("padding", "12"),
                ("background", "#cff4fc"),
                ("color", "#055160"),
                ("border-width", "1"),
                ("border-color", "#b6effb"),
                ("font-size", "14"),
                ("font-weight", "500"),
                ("border-radius", "4"),
            ],
        );
        let r = compile(&c, &l, &s);

        // Border keeps the container-paint props.
        assert!(
            r.xaml.contains("Background=\"#cff4fc\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("BorderBrush=\"#b6effb\""),
            "got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("BorderThickness=\"1\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("CornerRadius=\"4\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Padding=\"12\""), "got:\n{}", r.xaml);

        // Text-style props moved into a scoped Style resource.
        assert!(
            r.xaml.contains("<Border.Resources>"),
            "expected Border.Resources with TextBlock style, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<Style TargetType=\"TextBlock\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<Setter Property=\"Foreground\" Value=\"#055160\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<Setter Property=\"FontSize\" Value=\"14\"/>"),
            "got:\n{}",
            r.xaml
        );
        // X5: numeric CSS font-weight `500` â†’ WinUI `Medium` constant
        // (the bare `500` is not a valid WinUI `<Setter>` value).
        assert!(
            r.xaml
                .contains("<Setter Property=\"FontWeight\" Value=\"Medium\"/>"),
            "got:\n{}",
            r.xaml
        );

        // Crucially the Border opening tag must NOT carry text-style
        // attrs directly (WinUI rejects them).
        let border_open_line = r
            .xaml
            .lines()
            .find(|l| l.contains("<Border ") || l.contains("<Border\t"))
            .unwrap_or("");
        assert!(
            !border_open_line.contains("Foreground="),
            "Border tag must not carry Foreground, line: {}",
            border_open_line
        );
        assert!(
            !border_open_line.contains("FontSize="),
            "Border tag must not carry FontSize, line: {}",
            border_open_line
        );
        assert!(
            !border_open_line.contains("FontWeight="),
            "Border tag must not carry FontWeight, line: {}",
            border_open_line
        );
    }

    /// X6: Lattice/flex layout props must lower only where WinUI has a
    /// native equivalent. `gap` becomes StackPanel.Spacing for Row/Column,
    /// `max-width` becomes MaxWidth, and flex-only props are dropped instead
    /// of leaking into a fake TextBlock style setter.
    #[test]
    fn row_lattice_layout_style_lowers_to_native_stackpanel_attrs() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: Some("toolbar".to_string()),
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("Title".to_string()),
                    }],
                    children: Vec::new(),
                }],
            },
        );
        let s = style_for_box(
            "toolbar",
            vec![
                ("align-items", "center"),
                ("background", "#0f172a"),
                ("color", "#f8fafc"),
                ("flex-wrap", "wrap"),
                ("gap", "16"),
                ("justify-content", "space-between"),
                ("max-width", "980px"),
                ("padding", "12"),
            ],
        );
        let r = compile(&c, &l, &s);

        assert!(
            r.xaml.contains(
                "<StackPanel Orientation=\"Horizontal\" Spacing=\"16\" MaxWidth=\"980\">"
            ),
            "expected Row gap/max-width to land on StackPanel, got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("Background=\"#0f172a\""),
            "got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Padding=\"12\""), "got:\n{}", r.xaml);
        assert!(
            r.xaml
                .contains("<Setter Property=\"Foreground\" Value=\"#f8fafc\"/>"),
            "got:\n{}",
            r.xaml
        );
        for invalid in [
            "Property=\"Gap\"",
            "Property=\"AlignItems\"",
            "Property=\"FlexWrap\"",
            "Property=\"JustifyContent\"",
            "Value=\"980px\"",
        ] {
            assert!(
                !r.xaml.contains(invalid),
                "generated XAML must not contain {invalid}, got:\n{}",
                r.xaml
            );
        }
    }

    #[test]
    fn box_with_multiple_children_wraps_border_child_in_stackpanel() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("card".to_string()),
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("One".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("Two".to_string()),
                        }],
                        children: Vec::new(),
                    },
                ],
            },
        );
        let s = style_for_box("card", vec![("background", "#111827"), ("padding", "8")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<Border Background=\"#111827\" Padding=\"8\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<StackPanel Orientation=\"Vertical\">"),
            "multi-child Border content must be wrapped, got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Text=\"One\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Text=\"Two\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn if_branch_with_multiple_children_wraps_contentcontrol_child() {
        let c = component("Foo", vec![slot("visible", SlotType::Bool, true)], vec![]);
        let text = |value: &str| LayoutNode {
            tag: "Text".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".to_string(),
                value: LayoutPropValue::String(value.to_string()),
            }],
            children: Vec::new(),
        };
        let l = layout_with_root(
            "Foo",
            if_node(
                LayoutPropValue::SlotRef("visible".to_string()),
                vec![text("One"), text("Two")],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("<ContentControl Visibility=\"{x:Bind Visible"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<StackPanel Orientation=\"Vertical\">"),
            "ContentControl must have one child wrapper, got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Text=\"One\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Text=\"Two\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn for_template_with_if_else_wraps_datatemplate_child() {
        let c = component(
            "Foo",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            for_node(
                LayoutPropValue::SlotRef("items".to_string()),
                "item",
                None,
                vec![
                    if_node(LayoutPropValue::Expr("item == \"a\"".to_string()), vec![]),
                    else_node(vec![]),
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<DataTemplate x:DataType=\"local:Foo_ItemVm\">\n")
                && r.xaml.contains(
                    "<DataTemplate x:DataType=\"local:Foo_ItemVm\">\n                <StackPanel Orientation=\"Vertical\">"
                ),
            "DataTemplate must wrap multiple lowered children, got:\n{}",
            r.xaml
        );
        assert_eq!(r.xaml.matches("<ContentControl").count(), 2);
    }

    #[test]
    fn for_template_index_selected_predicate_lowers_to_row_vm_property() {
        let c = component(
            "Foo",
            vec![
                slot("items", SlotType::List(Box::new(ListInnerType::Text)), true),
                slot("selected-index", SlotType::Number, true),
            ],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            for_node(
                LayoutPropValue::SlotRef("items".to_string()),
                "item",
                Some("i"),
                vec![
                    if_node(
                        LayoutPropValue::Expr("i == selectedIndex".to_string()),
                        vec![],
                    ),
                    else_node(vec![]),
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml
                .contains("<ItemsRepeater ItemsSource=\"{x:Bind FooItemVmRows}\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("Visibility=\"{x:Bind IsSelected, Converter="),
            "template predicate must bind row-local IsSelected, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("Expr_"),
            "template predicate must not call page helper from DataTemplate, got:\n{}",
            r.xaml
        );
        let vm = r
            .for_view_models
            .iter()
            .find(|file| file.filename == "Foo_ItemVm.cs")
            .expect("row vm");
        assert!(
            vm.source.contains(
                "public sealed record Foo_ItemVm(string Item, int Index, bool IsSelected);"
            ),
            "got:\n{}",
            vm.source
        );
        assert!(
            r.code_behind
                .contains("rows.Add(new Foo_ItemVm(source[i], i, i == SelectedIndex));"),
            "got:\n{}",
            r.code_behind
        );
    }

    /// X3 negative: when the part has no text-style props, no
    /// `<Border.Resources>` block is emitted.
    #[test]
    fn box_without_text_style_emits_no_resources_block() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", styled_box_with_text_child("frame"));
        let s = style_for_box("frame", vec![("padding", "8"), ("background", "#ffffff")]);
        let r = compile(&c, &l, &s);
        assert!(
            !r.xaml.contains("Border.Resources"),
            "no text-style props should mean no Resources block, got:\n{}",
            r.xaml
        );
    }

    // â”€â”€ X4: color-value normalization for WinUI 3 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// X4: `background: "transparent"` in `.msl` must emit
    /// `Background="Transparent"` (PascalCase) â€” WinUI 3's markup
    /// compiler rejects the lowercase form.  Caught by the toolkit
    /// Alert demo's close-button background.
    #[test]
    fn x4_color_value_transparent_is_pascalcased() {
        let props = vec![StyleProp {
            name: "background".to_string(),
            value: "transparent".to_string(),
        }];
        let frag = build_style_fragment(&props);
        assert!(
            frag.contains("Background=\"Transparent\""),
            "expected PascalCased Transparent, got:\n{frag}"
        );
        assert!(
            !frag.contains("Background=\"transparent\""),
            "lowercase transparent must NOT survive normalization, got:\n{frag}"
        );
    }

    /// X4 negative: hex literals (`#â€¦`) pass through verbatim.
    /// PascalCasing a hex value would be wrong (`#abc` isn't a name).
    #[test]
    fn x4_color_value_hex_passes_through_unchanged() {
        let props = vec![
            StyleProp {
                name: "background".to_string(),
                value: "#cff4fc".to_string(),
            },
            StyleProp {
                name: "color".to_string(),
                value: "#055160".to_string(),
            },
        ];
        let frag = build_style_fragment(&props);
        assert!(frag.contains("Background=\"#cff4fc\""), "got:\n{frag}");
        assert!(frag.contains("Foreground=\"#055160\""), "got:\n{frag}");
    }

    /// X4/X5 scope: hex color setters and unit-free lengths pass
    /// through untouched, while `font-weight: normal` is now PascalCased
    /// to the WinUI `FontWeights.Normal` constant (X5) â€” the lowercase
    /// CSS keyword is NOT a valid WinUI `<Setter>` value.
    #[test]
    fn x4_non_color_setters_pass_through_unchanged() {
        let props = vec![
            StyleProp {
                name: "font-size".to_string(),
                value: "12".to_string(),
            },
            StyleProp {
                name: "font-weight".to_string(),
                value: "normal".to_string(),
            },
            StyleProp {
                name: "padding".to_string(),
                value: "6".to_string(),
            },
        ];
        let frag = build_style_fragment(&props);
        assert!(frag.contains("FontSize=\"12\""), "got:\n{frag}");
        // X5: `normal` â†’ WinUI `Normal` (the lowercase form is invalid).
        assert!(frag.contains("FontWeight=\"Normal\""), "got:\n{frag}");
        assert!(!frag.contains("FontWeight=\"normal\""), "got:\n{frag}");
        assert!(frag.contains("Padding=\"6\""), "got:\n{frag}");
    }

    #[test]
    fn side_specific_padding_lowers_to_xaml_thickness() {
        let bottom = build_style_fragment(&[StyleProp {
            name: "padding-bottom".to_string(),
            value: "14px".to_string(),
        }]);
        assert!(bottom.contains("Padding=\"0,0,0,14\""), "got:\n{bottom}");
        assert!(
            !bottom.contains("PaddingBottom"),
            "PaddingBottom is not a WinUI property, got:\n{bottom}"
        );

        let top = build_style_fragment(&[StyleProp {
            name: "padding-top".to_string(),
            value: "16".to_string(),
        }]);
        assert!(top.contains("Padding=\"0,16,0,0\""), "got:\n{top}");
    }

    #[test]
    fn styled_column_wraps_stack_panel_in_border() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: Some("shell".to_string()),
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("hello".to_string()),
                    }],
                    children: Vec::new(),
                }],
            },
        );
        let s = style_for_box("shell", vec![("background", "#101827"), ("padding", "24")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<Border Background=\"#101827\" Padding=\"24\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<StackPanel Orientation=\"Vertical\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml
                .contains("<StackPanel Orientation=\"Vertical\" Background="),
            "StackPanel must not carry Border/Control style attrs, got:\n{}",
            r.xaml
        );
    }

    /// X4: already-PascalCased color names (`Transparent`,
    /// `DarkGray`) pass through.  Authors who write XAML-native
    /// names in their `.msl` aren't double-cased.
    #[test]
    fn x4_pascalcased_color_value_passes_through_unchanged() {
        let props = vec![StyleProp {
            name: "background".to_string(),
            value: "Transparent".to_string(),
        }];
        let frag = build_style_fragment(&props);
        assert!(frag.contains("Background=\"Transparent\""), "got:\n{frag}");
    }

    /// X1 round-trip: `parse_style_fragment` reverses
    /// `build_style_fragment`'s output.
    #[test]
    fn parse_style_fragment_round_trips_build_style_fragment() {
        let props = vec![
            StyleProp {
                name: "padding".to_string(),
                value: "8".to_string(),
            },
            StyleProp {
                name: "background".to_string(),
                value: "#ffffff".to_string(),
            },
            StyleProp {
                name: "color".to_string(),
                value: "#212529".to_string(),
            },
            StyleProp {
                name: "border-radius".to_string(),
                value: "4".to_string(),
            },
        ];
        let joined = build_style_fragment(&props);
        let parsed = parse_style_fragment(&joined);
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0], ("Padding".to_string(), "8".to_string()));
        assert_eq!(parsed[1], ("Background".to_string(), "#ffffff".to_string()));
        assert_eq!(parsed[2], ("Foreground".to_string(), "#212529".to_string()));
        assert_eq!(parsed[3], ("CornerRadius".to_string(), "4".to_string()));
    }

    // â”€â”€ X5: WinUI value translation (Group A) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// X5: CSS `px` units are stripped from every length setter so the
    /// WinUI `Double` / `Thickness` parser accepts the value. A literal
    /// `12px` FontSize breaks the markup compiler.
    #[test]
    fn x5_px_units_stripped_from_length_setters() {
        let props = vec![
            StyleProp {
                name: "font-size".to_string(),
                value: "12px".to_string(),
            },
            StyleProp {
                name: "height".to_string(),
                value: "22px".to_string(),
            },
            StyleProp {
                name: "padding".to_string(),
                value: "2px".to_string(),
            },
            StyleProp {
                name: "border-width".to_string(),
                value: "0,0,0,1px".to_string(),
            },
        ];
        let frag = build_style_fragment(&props);
        assert!(frag.contains("FontSize=\"12\""), "got:\n{frag}");
        assert!(frag.contains("Height=\"22\""), "got:\n{frag}");
        assert!(frag.contains("Padding=\"2\""), "got:\n{frag}");
        // Thickness comma-shape is preserved while px is stripped.
        assert!(frag.contains("BorderThickness=\"0,0,0,1\""), "got:\n{frag}");
        // No `px` survives anywhere.
        assert!(!frag.contains("px\""), "no px should survive, got:\n{frag}");
    }

    /// X5: CSS-only properties with no WinUI analog are dropped, not
    /// emitted as invalid attributes / `<Setter>`s.
    #[test]
    fn x5_css_only_properties_are_dropped() {
        let props = vec![
            StyleProp {
                name: "border-collapse".to_string(),
                value: "collapse".to_string(),
            },
            StyleProp {
                name: "border-style".to_string(),
                value: "solid".to_string(),
            },
            StyleProp {
                name: "outline".to_string(),
                value: "1px solid #007acc".to_string(),
            },
            StyleProp {
                name: "text-decoration".to_string(),
                value: "underline".to_string(),
            },
            StyleProp {
                name: "box-shadow".to_string(),
                value: "0 1px 2px #000".to_string(),
            },
            // A real one alongside, to prove only the CSS-only ones drop.
            StyleProp {
                name: "background".to_string(),
                value: "#1e1e1e".to_string(),
            },
        ];
        let frag = build_style_fragment(&props);
        assert!(!frag.contains("BorderCollapse"), "got:\n{frag}");
        assert!(!frag.contains("BorderStyle"), "got:\n{frag}");
        assert!(!frag.contains("Outline"), "got:\n{frag}");
        assert!(!frag.contains("TextDecoration"), "got:\n{frag}");
        assert!(!frag.contains("BoxShadow"), "got:\n{frag}");
        assert!(frag.contains("Background=\"#1e1e1e\""), "got:\n{frag}");
    }

    /// X5: `width: 100%` is dropped â€” WinUI's `Width` is an absolute
    /// `Double`, not a percentage. The layout container sizes instead.
    #[test]
    fn x5_percentage_width_is_dropped() {
        let props = vec![StyleProp {
            name: "width".to_string(),
            value: "100%".to_string(),
        }];
        let frag = build_style_fragment(&props);
        assert!(
            !frag.contains("Width"),
            "percentage Width must drop, got:\n{frag}"
        );
        assert!(!frag.contains("100%"), "got:\n{frag}");
    }

    /// X5: `text-align` â†’ WinUI `TextAlignment` with a PascalCase value.
    /// The old output emitted `TextAlign="center"` â€” wrong on both the
    /// property name (no such property) and the value (lowercase).
    #[test]
    fn x5_text_align_maps_to_textalignment_pascalcase() {
        let center = build_style_fragment(&[StyleProp {
            name: "text-align".to_string(),
            value: "center".to_string(),
        }]);
        assert!(
            center.contains("TextAlignment=\"Center\""),
            "got:\n{center}"
        );
        assert!(!center.contains("TextAlign=\""), "got:\n{center}");
        assert!(!center.contains("\"center\""), "got:\n{center}");

        let right = build_style_fragment(&[StyleProp {
            name: "text-align".to_string(),
            value: "right".to_string(),
        }]);
        assert!(right.contains("TextAlignment=\"Right\""), "got:\n{right}");

        let left = build_style_fragment(&[StyleProp {
            name: "text-align".to_string(),
            value: "left".to_string(),
        }]);
        assert!(left.contains("TextAlignment=\"Left\""), "got:\n{left}");
    }

    /// X5: `font-weight` keyword and numeric forms map to WinUI
    /// `FontWeights` named constants (PascalCase).
    #[test]
    fn x5_font_weight_maps_to_named_constant() {
        let cases = [
            ("normal", "Normal"),
            ("bold", "Bold"),
            ("600", "SemiBold"),
            ("semibold", "SemiBold"),
            ("500", "Medium"),
            ("medium", "Medium"),
        ];
        for (input, expected) in cases {
            let frag = build_style_fragment(&[StyleProp {
                name: "font-weight".to_string(),
                value: input.to_string(),
            }]);
            assert!(
                frag.contains(&format!("FontWeight=\"{expected}\"")),
                "font-weight {input:?} should map to {expected:?}, got:\n{frag}"
            );
        }
    }

    /// X5: a `{x:Bind â€¦}` binding value must pass through unmangled â€”
    /// it is never px-stripped or case-mangled.
    #[test]
    fn x5_binding_value_passes_through_unmangled() {
        // FontSize is a length setter; a binding must not be px-touched.
        assert_eq!(
            translate_xaml_value("FontSize", "{x:Bind CellFontSize}"),
            Some("{x:Bind CellFontSize}".to_string())
        );
        // TextAlignment binding must not be PascalCase-mangled.
        assert_eq!(
            translate_xaml_value("TextAlignment", "{x:Bind Align}"),
            Some("{x:Bind Align}".to_string())
        );
    }

    /// X5 unit: `strip_px_units` preserves the Thickness separator
    /// (comma vs space) while removing each `px`.
    #[test]
    fn x5_strip_px_units_preserves_thickness_shape() {
        assert_eq!(strip_px_units("12px"), "12");
        assert_eq!(strip_px_units("0,0,0,1px"), "0,0,0,1");
        assert_eq!(strip_px_units("8px 4px"), "8 4");
        assert_eq!(strip_px_units("12"), "12");
    }

    // â”€â”€ Group B / Group C: the nested cell loop â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Build a Grid-shaped layout: an outer `For (each: slot:
    /// viewport-rows, as: row, index: r)` whose body is an inner
    /// `For (each: row, as: v, index: c)` rendering one styled cell
    /// containing `Text (content: v)`. Mirrors mosaic-pkg-grid's
    /// resolved body shape (UI29 Â§3.4 nested For).
    fn grid_nested_for_root() -> LayoutNode {
        let inner_for = LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props: vec![
                LayoutProp {
                    name: "each".to_string(),
                    value: LayoutPropValue::Keyword("row".to_string()),
                },
                LayoutProp {
                    name: "as".to_string(),
                    value: LayoutPropValue::Keyword("v".to_string()),
                },
                LayoutProp {
                    name: "index".to_string(),
                    value: LayoutPropValue::Keyword("c".to_string()),
                },
            ],
            children: vec![LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("cell".to_string()),
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Text".to_string(),
                    part_name: None,
                    props: vec![LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::Keyword("v".to_string()),
                    }],
                    children: Vec::new(),
                }],
            }],
        };
        LayoutNode {
            tag: "For".to_string(),
            part_name: None,
            props: vec![
                LayoutProp {
                    name: "each".to_string(),
                    value: LayoutPropValue::SlotRef("viewport-rows".to_string()),
                },
                LayoutProp {
                    name: "as".to_string(),
                    value: LayoutPropValue::Keyword("row".to_string()),
                },
                LayoutProp {
                    name: "index".to_string(),
                    value: LayoutPropValue::Keyword("r".to_string()),
                },
            ],
            children: vec![inner_for],
        }
    }

    fn grid_component() -> MosmodelComponent {
        component(
            "Grid",
            vec![slot(
                "viewport-rows",
                SlotType::List(Box::new(ListInnerType::List(Box::new(ListInnerType::Text)))),
                true,
            )],
            vec![],
        )
    }

    /// GROUP B: the inner value VM (`Grid_VVm`) must type its value
    /// field as `string`, NOT `IReadOnlyList<string>`. The outer row VM
    /// keeps `IReadOnlyList<string> Row`. Binding a `string` Text to a
    /// list field would block `dotnet build`.
    #[test]
    fn group_b_inner_value_vm_field_is_string_not_list() {
        let c = grid_component();
        let l = layout_with_root("Grid", grid_nested_for_root());
        let r = compile(&c, &l, &empty_style("Grid"));

        let vvm = r
            .for_view_models
            .iter()
            .find(|f| f.filename.contains("Grid_VVm"))
            .map(|f| f.source.as_str())
            .unwrap_or("");
        assert!(
            vvm.contains("string V"),
            "inner value VM must type V as `string`, got:\n{vvm}"
        );
        assert!(
            !vvm.contains("IReadOnlyList<string> V"),
            "inner value VM must NOT type V as a list, got:\n{vvm}"
        );

        // The outer row VM keeps the list type.
        let rowvm = r
            .for_view_models
            .iter()
            .find(|f| f.filename.contains("Grid_RowVm"))
            .map(|f| f.source.as_str())
            .unwrap_or("");
        assert!(
            rowvm.contains("IReadOnlyList<string> Row"),
            "outer row VM must keep the list type, got:\n{rowvm}"
        );
    }

    /// GROUP C: the per-column cell loop's value VM carries a
    /// `double Width` field and the generated cell element binds
    /// `Width="{x:Bind Width}"`.
    #[test]
    fn group_c_value_vm_carries_width_and_cell_binds_it() {
        let c = grid_component();
        let l = layout_with_root("Grid", grid_nested_for_root());
        let r = compile(&c, &l, &empty_style("Grid"));

        let vvm = r
            .for_view_models
            .iter()
            .find(|f| f.filename.contains("Grid_VVm"))
            .map(|f| f.source.as_str())
            .unwrap_or("");
        assert!(
            vvm.contains("double Width"),
            "value VM must carry a `double Width` field, got:\n{vvm}"
        );
        // The <remarks> tells the Windows dev how to populate it.
        assert!(
            vvm.contains("<remarks>") && vvm.contains("ColumnWidths"),
            "value VM must document how the host populates Width, got:\n{vvm}"
        );

        // The cell element binds the width.
        assert!(
            r.xaml.contains("Width=\"{x:Bind Width}\""),
            "cell element must bind Width, got:\n{}",
            r.xaml
        );

        // The OUTER row VM must NOT carry a Width (only the per-column
        // cell loop does).
        let rowvm = r
            .for_view_models
            .iter()
            .find(|f| f.filename.contains("Grid_RowVm"))
            .map(|f| f.source.as_str())
            .unwrap_or("");
        assert!(
            !rowvm.contains("double Width"),
            "outer row VM must NOT carry Width, got:\n{rowvm}"
        );
    }

    /// GROUP A end-to-end on the Grid shape: the cell's `text-align:
    /// right` style lands as a valid `TextAlignment="Right"` Setter and
    /// the px-laden `padding`/`height` are stripped â€” proving the
    /// translation runs through the full pipeline, not just the unit.
    #[test]
    fn group_a_cell_style_is_valid_winui() {
        let c = grid_component();
        let l = layout_with_root("Grid", grid_nested_for_root());
        let s = StyleDef {
            component_name: "Grid".to_string(),
            parts: vec![PartStyle {
                name: "cell".to_string(),
                base: vec![
                    StyleProp {
                        name: "padding".to_string(),
                        value: "2px".to_string(),
                    },
                    StyleProp {
                        name: "height".to_string(),
                        value: "22px".to_string(),
                    },
                    StyleProp {
                        name: "border-style".to_string(),
                        value: "solid".to_string(),
                    },
                    StyleProp {
                        name: "text-align".to_string(),
                        value: "right".to_string(),
                    },
                ],
                states: Vec::new(),
            }],
        };
        let r = compile(&c, &l, &s);
        assert!(r.xaml.contains("Padding=\"2\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Height=\"22\""), "got:\n{}", r.xaml);
        assert!(
            r.xaml
                .contains("<Setter Property=\"TextAlignment\" Value=\"Right\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(!r.xaml.contains("BorderStyle"), "got:\n{}", r.xaml);
        assert!(
            !r.xaml.contains("px\""),
            "no px should survive, got:\n{}",
            r.xaml
        );
    }
}
