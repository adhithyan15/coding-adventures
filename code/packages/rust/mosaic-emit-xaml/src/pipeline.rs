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
use mosstyle_compiler::{StyleDef, StyleProp, StyleTransition};

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

    /// Generated C# helper sources used by component resources, including
    /// visibility and native-focus value converters.
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
    /// `global.json` — keeps WinUI's markup compiler on the .NET 9 SDK
    /// family targeted by the generated project, even when a newer SDK is
    /// installed globally.
    pub global_json: String,
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
    /// `README.md` for the emitted project â€” describes prerequisites,
    /// bundled Windows App SDK deployment, the build command, and the
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

    /// Require Mosaic's standard Rust application runtime in the generated
    /// project shell. The strict shell loads the runtime before activation,
    /// applies runtime props before the component is shown, and omits both
    /// the reflection-host and deterministic sample-prop fallbacks. Default
    /// `false` keeps preview and compatibility output unchanged.
    pub require_runtime: bool,

    /// Top-level C# namespace for emitted types. Default
    /// `"Mosaic.Generated"`.
    pub namespace: String,

    /// Windows App SDK version to pin in the emitted `.csproj` (only used
    /// when `emit_project` is on). Default `"1.8.260710003"` â€” a known-
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
            require_runtime: false,
            namespace: "Mosaic.Generated".to_string(),
            windows_app_sdk: "1.8.260710003".to_string(),
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

    /// #12038: a literal `HostLink.href` has no scheme, or a scheme
    /// outside the `http`/`https`/`mailto` allowlist. `NavigateUri` is
    /// handed to the OS shell launcher, so a `file:`/UNC/custom-protocol
    /// target would launch rather than open as a web link. Rejected at
    /// compile time rather than escaped, since XML-escaping the value
    /// does nothing to make an unsafe scheme safe.
    UnsafeUriScheme(String),
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
            PipelineEmitError::UnsafeUriScheme(href) => write!(
                f,
                "HostLink href {href:?} has no scheme, or a scheme outside the allowed \
                 set (http, https, mailto) -- NavigateUri would hand it to the OS shell \
                 launcher. Use an allowed scheme, or `external: false` for in-app routing."
            ),
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
    if ctx.needs_focus_state_converter {
        if_helpers.push(EmittedFile {
            filename: "FocusStateToBoolConverter.cs".to_string(),
            source: emit_focus_state_to_bool_converter_source(&options.namespace),
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
    /// carry an extra `double Width` field so the enclosing generated
    /// projection can thread the matching authored column width onto every
    /// cell, and the generated cell element binds `Width="{x:Bind Width}"`.
    has_width: bool,
    /// True when a template-local predicate such as `i == selectedIndex`
    /// is lowered into a row-local boolean instead of a page helper call.
    has_is_selected: bool,
    /// Computed expression values exposed as ordinary properties to WinUI's
    /// typed DataTemplate binding compiler.
    helper_bindings: Vec<RowVmHelperBinding>,
    /// Lexical values copied from enclosing For rows so nested templates keep
    /// the same MIL expression scope after WinUI changes DataContext.
    captures: Vec<RowVmCapture>,
    /// Row projections owned by this VM for nested For blocks.
    nested_projections: Vec<RowProjection>,
}

#[derive(Debug, Clone)]
struct RowVmHelperBinding {
    property_name: String,
    return_type: String,
    owner_call: String,
}

#[derive(Debug, Clone)]
struct RowVmCapture {
    property_name: String,
    property_type: String,
}

/// A code-behind property that projects a slot list into generated row VMs.
#[derive(Debug, Clone)]
struct RowProjection {
    property_name: String,
    source_path: String,
    dependency_paths: Vec<String>,
    vm_class: String,
    has_index: bool,
    has_width: bool,
    width_source_path: Option<String>,
    selected_index_path: Option<String>,
    owner_expr: String,
    capture_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeTableRole {
    Header,
    Body,
}

#[derive(Debug, Clone)]
struct NativeTableEmission {
    role: NativeTableRole,
    header_helper: String,
    cell_name_helper: String,
    for_depth: usize,
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
    /// Per-alias allocation count for generated row-VM names. Package-expanded
    /// applications can contain unrelated loops that reuse a short alias such
    /// as `row`; only the first keeps the historical unsuffixed name.
    for_alias_counts: std::collections::HashMap<String, u32>,
    /// Helper methods to emit into the code-behind. Deduplicated by
    /// method name so two identical expressions in the same component
    /// produce only one helper.
    helpers: Vec<HelperMethod>,
    /// Tracks whether any `If` has been emitted. When `true`, the
    /// emitter writes a `BoolToVisibilityConverter` resource into the
    /// `<UserControl.Resources>` block.
    needs_bool_to_vis: bool,
    /// Tracks whether a focus-capable Host control consumes UI15's built-in
    /// `state focused`. The emitter writes a `FocusStateToBoolConverter`
    /// resource and ships its C# helper alongside the component triple.
    needs_focus_state_converter: bool,
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
    /// Property-scoped WinUI visual-state groups collected from native
    /// Host* controls that opt into MSL states through `state-when-*`.
    ///
    /// The groups are emitted on the generated root after the layout walk,
    /// where they can target each control's deterministic `x:Name`.
    visual_state_groups: Vec<XamlVisualStateGroup>,
    /// Visual-state groups collected for each active `For` DataTemplate,
    /// innermost template last. DataTemplate namescopes are isolated from the
    /// component root and from enclosing templates, so each template needs
    /// its own VisualStateManager attachment point.
    template_visual_state_groups: Vec<Vec<XamlVisualStateGroup>>,
    /// Monotonic suffix used to keep generated VisualState names unique in
    /// the generated XAML.
    visual_state_counter: u32,
    /// Canonical HostTable lowering currently being emitted. XAML needs a
    /// little context across the nested header/row/cell `For` templates so it
    /// can wrap the generated content in controls whose automation peers
    /// implement Table/Grid and TableItem/GridItem.
    native_table: Option<NativeTableEmission>,
    /// Whether this component emitted at least one canonical native table and
    /// therefore needs the component-scoped automation control classes in its
    /// code-behind.
    needs_native_table_support: bool,
    native_table_counter: u32,
    /// Whether this component emitted UI35 drag primitives and therefore needs
    /// the component-scoped WinUI drag controls/runtime in its code-behind.
    needs_native_drag_support: bool,
    /// Whether this component emitted HostSlider and therefore needs the
    /// component-scoped native Slider subclass that owns user-input and commit
    /// lifecycle tracking.
    needs_native_slider_support: bool,
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
            for_alias_counts: std::collections::HashMap::new(),
            helpers: Vec::new(),
            needs_bool_to_vis: false,
            needs_focus_state_converter: false,
            row_vms: Vec::new(),
            row_projections: Vec::new(),
            host_handlers: Vec::new(),
            host_counter: 0,
            registry: None,
            used_xmlns: std::collections::BTreeMap::new(),
            slot_aliases: std::collections::HashMap::new(),
            root_extra_attrs: None,
            visual_state_groups: Vec::new(),
            template_visual_state_groups: Vec::new(),
            visual_state_counter: 0,
            native_table: None,
            needs_native_table_support: false,
            native_table_counter: 0,
            needs_native_drag_support: false,
            needs_native_slider_support: false,
        }
    }

    /// PascalCased slot name (PR-1 default), unless the slot collides
    /// with a property on the chosen base class â€” in which case the
    /// alias from `slot_aliases` wins. `{x:Bind}` paths route through
    /// this so a slot named `title` on a ContentDialog-rooted
    /// component resolves to `DialogTitle`, not the shadowed
    /// `Title`. Fix A4.
    fn slot_property_name(&self, slot_name: &str) -> String {
        if let Some(alias) = self.slot_aliases.get(slot_name) {
            return alias.clone();
        }
        kebab_to_pascal_case(slot_name)
    }

    fn slot_xbind_path(&self, slot_name: &str) -> String {
        let property = self.slot_property_name(slot_name);
        if self.for_scope.is_empty() {
            property
        } else {
            format!("Owner.{property}")
        }
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

    fn scoped_name_xbind_path(&self, name: &str) -> String {
        if self.lookup_for_binding(name).is_some() {
            return kebab_to_pascal_case(name);
        }
        if let Some(position) = self
            .for_scope
            .iter()
            .rposition(|binding| binding.index_name.as_deref() == Some(name))
        {
            return if position + 1 == self.for_scope.len() {
                "Index".to_string()
            } else {
                kebab_to_pascal_case(name)
            };
        }
        self.slot_xbind_path(name)
    }

    fn allocate_for_vm_class(&mut self, as_name: &str) -> String {
        let count = self
            .for_alias_counts
            .entry(as_name.to_string())
            .or_default();
        *count += 1;
        let suffix = if *count == 1 {
            String::new()
        } else {
            count.to_string()
        };
        format!(
            "{}_{}{}Vm",
            self.component_name,
            kebab_to_pascal_case(as_name),
            suffix
        )
    }

    /// Add a helper method (or skip if a method by the same name already
    /// exists â€” assumed to be identical because helper names are a
    /// deterministic function of the expression they came from).
    fn add_helper(&mut self, helper: HelperMethod) {
        if !self.helpers.iter().any(|h| h.name == helper.name) {
            self.helpers.push(helper);
        }
    }

    fn next_visual_state_id(&mut self) -> u32 {
        self.visual_state_counter += 1;
        self.visual_state_counter
    }

    fn add_visual_state_group(&mut self, group: XamlVisualStateGroup) {
        if let Some(template_groups) = self.template_visual_state_groups.last_mut() {
            template_groups.push(group);
        } else {
            self.visual_state_groups.push(group);
        }
    }
}

// =====================================================================
// Part-style map (mosstyle â†’ flat property fragments)
// =====================================================================

#[derive(Debug, Clone)]
struct PartStateStyle {
    props: Vec<StyleProp>,
    transitions: Vec<StyleTransition>,
}

/// A part-style entry keeps the already-lowered base attribute fragment
/// together with the structured MSL state and transition data. Base-only
/// consumers remain a cheap string lookup; native controls can additionally
/// lower `state-when-*` predicates and ButtonBase pointer hover to WinUI
/// VisualStates.
#[derive(Debug, Clone)]
struct PartStyleEntry {
    base_fragment: String,
    transitions: Vec<StyleTransition>,
    states: std::collections::HashMap<String, PartStateStyle>,
    /// `flex-grow` / `align-items` / `justify-content` have no 1:1 XAML
    /// setter, so `css_property_to_xaml_setter` maps them to `None` and
    /// they never survive into `base_fragment`. The `Row`/`Column`→`Grid`
    /// lowering (see `mosaic-emit-xaml.md` §3.1) reads them here instead,
    /// straight off the raw mosstyle props.
    flex: FlexHints,
}

/// Layout hints a `Row`/`Column`'s `<Grid>` lowering needs but that have no
/// direct XAML attribute. Populated once per part in `build_part_style_map`
/// from the raw mosstyle props (`part.base`), independent of the
/// CSS-property→XAML-setter table `build_style_fragment` uses.
///
/// `flex_grow` / `main_axis_full` are read off a *child*'s own part (do I
/// get a star-sized cell?). `align_items` / `justify_content` are read off
/// the *container*'s own part (how do I place my children?). Both live on
/// the same struct because both come from the same raw-prop scan; only the
/// caller decides which fields are meaningful for its role.
#[derive(Debug, Clone, Default)]
struct FlexHints {
    /// `true` when this part's own style carries a non-zero `flex-grow`.
    /// Only `flex-grow: 1` is authored anywhere in the repo today, so this
    /// is a boolean ("does this child grow") rather than a weight; add a
    /// numeric weight if a real fractional/multi-value case ever appears.
    flex_grow: bool,
    /// `true` when this part's own style sets `width: 100%` (read by a
    /// `Row`'s child) or `height: 100%` (read by a `Column`'s child) — the
    /// main-axis case, flexbox's own "claim the remaining space", treated
    /// identically to `flex_grow`. The cross-axis case needs no flag: it's
    /// already handled by `stretch_alignment_for`'s `HorizontalAlignment`/
    /// `VerticalAlignment` = `Stretch`, which composes fine with `<Grid>`'s
    /// own default child-stretch behavior.
    width_full: bool,
    height_full: bool,
    /// Container-only: this part's own `align-items` value, when it's one
    /// this backend maps (today: only `center` is authored anywhere).
    align_items: Option<String>,
    /// Container-only: this part's own `justify-content` value, when it's
    /// one this backend maps (today: only `space-between` is authored
    /// anywhere).
    justify_content: Option<String>,
}

/// Which axis a flex `<Grid>` lowering runs along. `Row` children flow
/// left→right (one `ColumnDefinition` per slot, `Grid.Column` attached
/// property); `Column` children flow top→bottom (one `RowDefinition` per
/// slot, `Grid.Row`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlexAxis {
    Row,
    Column,
}

impl FlexAxis {
    /// The attached property that positions a child along this axis.
    fn grid_position_property(self) -> &'static str {
        match self {
            FlexAxis::Row => "Grid.Column",
            FlexAxis::Column => "Grid.Row",
        }
    }
    /// The `<Grid.*Definitions>` wrapper tag for this axis.
    fn definitions_tag(self) -> &'static str {
        match self {
            FlexAxis::Row => "Grid.ColumnDefinitions",
            FlexAxis::Column => "Grid.RowDefinitions",
        }
    }
    /// The individual `<*Definition>` tag for this axis.
    fn definition_tag(self) -> &'static str {
        match self {
            FlexAxis::Row => "ColumnDefinition",
            FlexAxis::Column => "RowDefinition",
        }
    }
    /// The size attribute name on a definition element (`Width` for a
    /// `ColumnDefinition`, `Height` for a `RowDefinition`).
    fn definition_size_attr(self) -> &'static str {
        match self {
            FlexAxis::Row => "Width",
            FlexAxis::Column => "Height",
        }
    }
    /// `Grid.ColumnSpacing` / `Grid.RowSpacing` — added to `Grid` in
    /// Windows App SDK 1.3+ (this spec pins 1.5, `mosaic-emit-xaml.md`
    /// §10), the `<Grid>` equivalent of the `<StackPanel>`-only `Spacing`
    /// that `gap` used to map to.
    fn spacing_attr(self) -> &'static str {
        match self {
            FlexAxis::Row => "ColumnSpacing",
            FlexAxis::Column => "RowSpacing",
        }
    }
    /// The cross-axis alignment property `align-items` sets on children —
    /// vertical for a `Row` (its cross axis), horizontal for a `Column`.
    fn cross_align_property(self) -> &'static str {
        match self {
            FlexAxis::Row => "VerticalAlignment",
            FlexAxis::Column => "HorizontalAlignment",
        }
    }
    /// Does this part's own `FlexHints` say a child on this axis should
    /// get a star-sized cell (either `flex-grow` or a main-axis 100%)?
    fn child_grows(self, hints: &FlexHints) -> bool {
        hints.flex_grow
            || match self {
                FlexAxis::Row => hints.width_full,
                FlexAxis::Column => hints.height_full,
            }
    }
}

/// Scan a part's raw mosstyle props for the flex hints that have no XAML
/// setter (`build_style_fragment` already drops all four for that reason —
/// see its `css_property_to_xaml_setter` call). Deliberately narrow: only
/// the exact values authored anywhere in the repo today are recognised: `
/// flex-grow: 1`, `align-items: center`, `justify-content: space-between`.
/// Anything else is left `None`/`false` — same "don't guess, defer" policy
/// as the rest of this emitter (see `mosaic-emit-xaml.md` §3.1 for what's
/// intentionally out of scope).
fn extract_flex_hints(props: &[StyleProp]) -> FlexHints {
    let mut hints = FlexHints::default();
    for p in props {
        let value = p.value.trim();
        match p.name.as_str() {
            "flex-grow" => {
                hints.flex_grow = value.parse::<f64>().map(|n| n != 0.0).unwrap_or(false);
            }
            "width" if value == "100%" => hints.width_full = true,
            "height" if value == "100%" => hints.height_full = true,
            "align-items" if value == "center" => hints.align_items = Some(value.to_string()),
            "justify-content" if value == "space-between" => {
                hints.justify_content = Some(value.to_string());
            }
            _ => {}
        }
    }
    hints
}

type PartStyleMap = std::collections::HashMap<String, PartStyleEntry>;

#[derive(Debug, Clone)]
struct XamlVisualState {
    name: String,
    trigger_value: String,
    value: String,
    transition: Option<StyleTransition>,
}

#[derive(Debug, Clone)]
struct PendingXamlVisualState {
    trigger_value: String,
    value: String,
    transition: Option<StyleTransition>,
}

#[derive(Debug, Clone)]
struct XamlVisualStateGroup {
    normal_name: String,
    target_name: String,
    property: String,
    base_transition: Option<StyleTransition>,
    states: Vec<XamlVisualState>,
}

/// Walk the `StyleDef` and produce a flat `part_name -> css_fragment`
/// map. The fragment is a comma-separated `key: "value"` list ready to
/// embed in a XAML setter chain.
fn build_part_style_map(style: &StyleDef) -> PartStyleMap {
    let mut out = PartStyleMap::with_capacity(style.parts.len());
    for part in &style.parts {
        let base_fragment = build_style_fragment(&part.base);
        let flex = extract_flex_hints(&part.base);
        let states = part
            .states
            .iter()
            .map(|state| {
                (
                    state.state.clone(),
                    PartStateStyle {
                        props: state.props.clone(),
                        transitions: state.transitions.clone(),
                    },
                )
            })
            .collect();
        let has_flex_hints = flex.flex_grow
            || flex.width_full
            || flex.height_full
            || flex.align_items.is_some()
            || flex.justify_content.is_some();
        if !base_fragment.is_empty()
            || !part.transitions.is_empty()
            || !part.states.is_empty()
            || has_flex_hints
        {
            out.insert(
                part.name.clone(),
                PartStyleEntry {
                    base_fragment,
                    transitions: part.transitions.clone(),
                    states,
                    flex,
                },
            );
        }
    }
    out
}

fn has_explicit_state_when(node: &LayoutNode, state_name: &str) -> bool {
    let prop_name = format!("state-when-{state_name}");
    node.props.iter().any(|prop| prop.name == prop_name)
}

fn button_base_supports_automatic_hover(xaml_tag: &str) -> bool {
    matches!(
        xaml_tag,
        "Button" | "CheckBox" | "RadioButton" | "HyperlinkButton"
    )
}

fn button_base_supports_automatic_press(xaml_tag: &str) -> bool {
    matches!(
        xaml_tag,
        "Button" | "CheckBox" | "RadioButton" | "HyperlinkButton"
    )
}

fn control_supports_automatic_focus(xaml_tag: &str) -> bool {
    matches!(
        xaml_tag,
        "TextBox" | "NumberBox" | "Button" | "CheckBox" | "RadioButton" | "HyperlinkButton"
    )
}

/// Register MSL state overrides for one native WinUI control.
///
/// Each property gets its own VisualStateGroup. This is deliberate:
/// `VisualTransition.GeneratedDuration` applies to every changed property in
/// a group, while MSL transitions are property-scoped. Isolating properties
/// preserves that contract and lets a background fade and opacity change use
/// different durations or easing curves.
///
fn register_host_visual_states(
    node: &LayoutNode,
    xaml_tag: &str,
    target_name: &str,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) {
    let Some(part_name) = node.part_name.as_deref() else {
        return;
    };
    let Some(part) = part_styles.get(part_name) else {
        return;
    };

    // XAML gives the first active trigger precedence. React and SwiftUI give
    // the last declared `state-when-*` layer precedence, so reverse declaration
    // order before materialising the VisualStates.
    let mut state_layers = Vec::new();
    for prop in node.props.iter().rev() {
        let Some(state_name) = prop.name.strip_prefix("state-when-") else {
            continue;
        };
        let Some(state_style) = part.states.get(state_name) else {
            continue;
        };
        let Some(trigger_value) = lower_state_trigger_value(&prop.value, ctx) else {
            continue;
        };
        state_layers.push((state_name, trigger_value, state_style));
    }
    if button_base_supports_automatic_press(xaml_tag) && !has_explicit_state_when(node, "pressed") {
        if let Some(press_style) = part.states.get("pressed") {
            state_layers.push((
                "pressed",
                format!("{{Binding IsPressed, ElementName={target_name}}}"),
                press_style,
            ));
        }
    }
    if control_supports_automatic_focus(xaml_tag) && !has_explicit_state_when(node, "focused") {
        if let Some(focus_style) = part.states.get("focused") {
            ctx.needs_focus_state_converter = true;
            state_layers.push((
                "focused",
                format!(
                    "{{Binding FocusState, ElementName={target_name}, Converter={{StaticResource FocusStateToBoolConverter}}}}"
                ),
                focus_style,
            ));
        }
    }
    if button_base_supports_automatic_hover(xaml_tag) && !has_explicit_state_when(node, "hover") {
        if let Some(hover_style) = part.states.get("hover") {
            state_layers.push((
                "hover",
                format!("{{Binding IsPointerOver, ElementName={target_name}}}"),
                hover_style,
            ));
        }
    }

    let mut by_property: std::collections::BTreeMap<String, Vec<PendingXamlVisualState>> =
        std::collections::BTreeMap::new();
    for (_state_name, trigger_value, state_style) in state_layers {
        for prop in &state_style.props {
            let Some(property) = css_property_to_xaml_setter(&prop.name) else {
                continue;
            };
            if !host_control_supports_state_property(xaml_tag, &property) {
                continue;
            }
            let Some(value) = translate_xaml_value(&property, &prop.value) else {
                continue;
            };
            let transition = transition_for_xaml_property(&state_style.transitions, &property);
            by_property
                .entry(property)
                .or_default()
                .push(PendingXamlVisualState {
                    trigger_value: trigger_value.clone(),
                    value,
                    transition: transition.cloned(),
                });
        }
    }

    for (property, states) in by_property {
        let group_id = ctx.next_visual_state_id();
        let normal_name = format!("MosaicState{group_id}Normal");
        let states = states
            .into_iter()
            .enumerate()
            .map(|(state_index, pending)| XamlVisualState {
                name: format!("MosaicState{group_id}State{state_index}"),
                trigger_value: pending.trigger_value,
                value: pending.value,
                transition: pending.transition,
            })
            .collect();
        ctx.add_visual_state_group(XamlVisualStateGroup {
            normal_name,
            target_name: target_name.to_string(),
            property: property.clone(),
            base_transition: transition_for_xaml_property(&part.transitions, &property).cloned(),
            states,
        });
    }
}

fn lower_state_trigger_value(value: &LayoutPropValue, ctx: &mut EmitContext<'_>) -> Option<String> {
    if !ctx.for_scope.is_empty() {
        return match value {
            LayoutPropValue::Keyword(k) if k == "true" => Some("True".to_string()),
            LayoutPropValue::Keyword(k) if k == "false" => Some("False".to_string()),
            LayoutPropValue::Expr(src) => {
                let path = if let Some(path) = try_lower_for_template_predicate(src, ctx) {
                    path
                } else {
                    let binding = ctx.for_scope.last()?;
                    let element_root = kebab_to_pascal_case(&binding.as_name);
                    let index_root = binding.index_name.as_deref().map(kebab_to_pascal_case);
                    let tokens = tokenise_expr(src).ok()?;
                    if tokens.iter().any(|token| {
                        matches!(
                            token,
                            ExprTok::EqEq
                                | ExprTok::NotEq
                                | ExprTok::Lt
                                | ExprTok::Le
                                | ExprTok::Gt
                                | ExprTok::Ge
                                | ExprTok::AndAnd
                                | ExprTok::OrOr
                                | ExprTok::Not
                                | ExprTok::LBracket
                                | ExprTok::RBracket
                        )
                    }) {
                        // Page-level expression helpers are not in a
                        // DataTemplate's typed x:Bind scope. Reject shapes
                        // that would require one instead of generating markup
                        // that compiles against the wrong namescope.
                        return None;
                    }
                    match lower_expr_for_xbind(src, ctx) {
                        ExprLowering::Bindable(path)
                            if path == element_root
                                || path.starts_with(&format!("{element_root}.")) =>
                        {
                            path
                        }
                        ExprLowering::Bindable(path)
                            if index_root.as_deref() == Some(path.as_str()) =>
                        {
                            "Index".to_string()
                        }
                        ExprLowering::Bindable(path)
                            if matches!(path.as_str(), "True" | "False") =>
                        {
                            path
                        }
                        ExprLowering::Bindable(_)
                        | ExprLowering::Helper(_)
                        | ExprLowering::Unsupported(_) => return None,
                    }
                };
                Some(format!("{{x:Bind {path}, Mode=OneWay}}"))
            }
            // Component slots live on the generated page, not on the row VM
            // that is the DataTemplate's x:DataType. Cross-scope slot
            // predicates therefore stay omitted unless they are projected by
            // `try_lower_for_template_predicate` (for example
            // `index == selectedIndex`).
            LayoutPropValue::SlotRef(_) | LayoutPropValue::String(_) => None,
            _ => None,
        };
    }

    match value {
        LayoutPropValue::SlotRef(slot) => Some(format!(
            "{{x:Bind {}, Mode=OneWay}}",
            ctx.slot_xbind_path(slot)
        )),
        LayoutPropValue::Keyword(k) if k == "true" => Some("True".to_string()),
        LayoutPropValue::Keyword(k) if k == "false" => Some("False".to_string()),
        LayoutPropValue::Expr(src) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) | ExprLowering::Helper(path) => {
                Some(format!("{{x:Bind {path}, Mode=OneWay}}"))
            }
            ExprLowering::Unsupported(_) => None,
        },
        _ => None,
    }
}

fn transition_for_xaml_property<'a>(
    transitions: &'a [StyleTransition],
    property: &str,
) -> Option<&'a StyleTransition> {
    transitions.iter().rev().find(|transition| {
        css_property_to_xaml_setter(&transition.property).as_deref() == Some(property)
    })
}

/// Conservative intersection shared by WinUI Control subclasses emitted for
/// Mosaic Host primitives. Properties outside this set are skipped instead of
/// generating a Setter that XamlCompiler rejects for one control type.
fn host_control_supports_state_property(xaml_tag: &str, property: &str) -> bool {
    matches!(
        property,
        "Background"
            | "Foreground"
            | "FontFamily"
            | "FontSize"
            | "FontWeight"
            | "Padding"
            | "Margin"
            | "Width"
            | "Height"
            | "MaxWidth"
            | "MaxHeight"
            | "MinWidth"
            | "MinHeight"
            | "BorderThickness"
            | "BorderBrush"
            | "CornerRadius"
            | "Opacity"
            | "HorizontalAlignment"
            | "VerticalAlignment"
    ) || (property == "TextAlignment" && matches!(xaml_tag, "TextBox" | "NumberBox"))
}

fn xaml_transition_duration(duration: &str) -> Option<String> {
    let duration = duration.trim();
    let seconds = if let Some(milliseconds) = duration.strip_suffix("ms") {
        milliseconds.trim().parse::<f64>().ok()? / 1000.0
    } else {
        duration.strip_suffix('s')?.trim().parse::<f64>().ok()?
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let mut rendered = format!("{seconds:.6}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    Some(format!("0:0:{rendered}"))
}

fn emit_xaml_easing(transition: &StyleTransition, indent: usize) -> String {
    let easing = transition.easing.trim();
    if easing == "linear" {
        return String::new();
    }
    let (element, mode) = match easing {
        "ease-in" => ("QuadraticEase", "EaseIn"),
        "ease-out" => ("QuadraticEase", "EaseOut"),
        "ease" | "ease-in-out" => ("QuadraticEase", "EaseInOut"),
        // WinUI's XAML EasingFunctionBase has no arbitrary cubic-bezier
        // implementation. CubicEase is the closest native generated-
        // transition curve; exact control points require the Composition API.
        value if value.starts_with("cubic-bezier(") => ("CubicEase", "EaseInOut"),
        _ => return String::new(),
    };
    let pad = " ".repeat(indent);
    format!(
        "{pad}<VisualTransition.GeneratedEasingFunction>\n\
         {pad}    <{element} EasingMode=\"{mode}\"/>\n\
         {pad}</VisualTransition.GeneratedEasingFunction>\n"
    )
}

fn emit_visual_transition(transition: &StyleTransition, to: Option<&str>, indent: usize) -> String {
    let Some(duration) = xaml_transition_duration(&transition.duration) else {
        return String::new();
    };
    let pad = " ".repeat(indent);
    let to_attr = to.map(|name| format!(" To=\"{name}\"")).unwrap_or_default();
    let easing = emit_xaml_easing(transition, indent + 4);
    if easing.is_empty() {
        return format!("{pad}<VisualTransition{to_attr} GeneratedDuration=\"{duration}\"/>\n");
    }
    format!(
        "{pad}<VisualTransition{to_attr} GeneratedDuration=\"{duration}\">\n\
         {easing}\
         {pad}</VisualTransition>\n"
    )
}

fn emit_visual_state_groups(groups: &[XamlVisualStateGroup], indent: usize) -> String {
    if groups.is_empty() {
        return String::new();
    }
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let pad3 = " ".repeat(indent + 8);
    let pad4 = " ".repeat(indent + 12);
    let mut out = String::new();
    writeln!(out, "{pad}<VisualStateManager.VisualStateGroups>").unwrap();
    for group in groups {
        writeln!(out, "{pad2}<VisualStateGroup>").unwrap();
        let has_transitions =
            group.base_transition.is_some() || group.states.iter().any(|s| s.transition.is_some());
        if has_transitions {
            writeln!(out, "{pad3}<VisualStateGroup.Transitions>").unwrap();
            if let Some(base_transition) = &group.base_transition {
                out.push_str(&emit_visual_transition(base_transition, None, indent + 12));
            }
            for state in &group.states {
                if let Some(transition) = &state.transition {
                    out.push_str(&emit_visual_transition(
                        transition,
                        Some(&state.name),
                        indent + 12,
                    ));
                }
            }
            writeln!(out, "{pad3}</VisualStateGroup.Transitions>").unwrap();
        }
        writeln!(out, "{pad3}<VisualState x:Name=\"{}\"/>", group.normal_name).unwrap();
        for state in &group.states {
            writeln!(out, "{pad3}<VisualState x:Name=\"{}\">", state.name).unwrap();
            writeln!(out, "{pad4}<VisualState.StateTriggers>").unwrap();
            writeln!(
                out,
                "{pad4}    <StateTrigger IsActive=\"{}\"/>",
                escape_xaml_attr(&state.trigger_value)
            )
            .unwrap();
            writeln!(out, "{pad4}</VisualState.StateTriggers>").unwrap();
            writeln!(out, "{pad4}<VisualState.Setters>").unwrap();
            let target = xaml_visual_state_target(&group.target_name, &group.property);
            writeln!(
                out,
                "{pad4}    <Setter Target=\"{}\" Value=\"{}\"/>",
                target,
                escape_xaml_attr(&state.value)
            )
            .unwrap();
            writeln!(out, "{pad4}</VisualState.Setters>").unwrap();
            writeln!(out, "{pad3}</VisualState>").unwrap();
        }
        writeln!(out, "{pad2}</VisualStateGroup>").unwrap();
    }
    writeln!(out, "{pad}</VisualStateManager.VisualStateGroups>").unwrap();
    out
}

/// Brush-valued control properties must target the brush's Color dependency
/// property for WinUI to generate an interpolating color animation. Replacing
/// the whole Brush object would make the state change correctly but discretely.
fn xaml_visual_state_target(target_name: &str, property: &str) -> String {
    if matches!(property, "Background" | "Foreground" | "BorderBrush") {
        format!("{target_name}.(Control.{property}).(SolidColorBrush.Color)")
    } else {
        format!("{target_name}.{property}")
    }
}

/// A property `build_style_fragment_with_drops` could not lower to any
/// XAML output at all — the raw ingredients `dropped_style_properties`
/// (issue #12022) turns into a public, part-tagged `DroppedStyleProperty`.
struct RawStyleDrop {
    name: String,
    value: String,
    reason: &'static str,
}

fn build_style_fragment(props: &[mosstyle_compiler::StyleProp]) -> String {
    build_style_fragment_with_drops(props).0
}

/// Same lowering as `build_style_fragment`, but also returns every property
/// that produced no XAML output at all, with why. Two call sites in this
/// function are genuine drops (see inline comments); a successful-but-
/// approximate translation (`stretch_alignment_for`) is NOT a drop.
fn build_style_fragment_with_drops(
    props: &[mosstyle_compiler::StyleProp],
) -> (String, Vec<RawStyleDrop>) {
    let mut parts: Vec<(String, String)> = Vec::with_capacity(props.len());
    let mut drops: Vec<RawStyleDrop> = Vec::new();
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
            None => {
                drops.push(RawStyleDrop {
                    name: p.name.clone(),
                    value: p.value.clone(),
                    reason: unsupported_property_reason(&p.name),
                });
                continue;
            }
        };

        // `width: 100%` / `height: 100%` express "fill the cross axis", and
        // XAML says that with an alignment, not a length. Translating the
        // value alone cannot do this — WinUI's Width/Height are absolute
        // Doubles — so the percentage was previously dropped entirely and the
        // element fell back to sizing itself to its content. That is a large
        // part of why generated apps hug the top-left of an otherwise empty
        // window instead of filling it.
        //
        // Only 100% maps this cleanly. Any other percentage needs
        // proportional sizing (a Grid with star columns), which is a
        // different and much larger change — those keep being dropped rather
        // than guessed at.
        if let Some(alignment_key) = stretch_alignment_for(&key, &p.value) {
            upsert_style_attr(&mut parts, alignment_key.to_string(), "Stretch".to_string());
            continue;
        }

        // X5: translate the *value* into the form the WinUI 3 markup
        // compiler accepts. `translate_xaml_value` may return `None`
        // when the whole property must be dropped (e.g. a percentage
        // `Width="100%"` â€” WinUI's `Width` is a `Double`, not a
        // percentage). `{x:Bind â€¦}` / `{Binding â€¦}` markup extensions
        // pass through untouched (never px-stripped or case-mangled).
        let value = match translate_xaml_value(&key, &p.value) {
            Some(v) => v,
            None => {
                drops.push(RawStyleDrop {
                    name: p.name.clone(),
                    value: p.value.clone(),
                    reason: "value could not be translated into a form the WinUI XAML markup compiler accepts (e.g. a non-100% percentage, or an unsupported CSS unit)",
                });
                continue;
            }
        };
        upsert_style_attr(&mut parts, key, value);
    }
    // X8/#12025: `escape_xaml_attr` does real XML attribute escaping
    // (`&`/`"`/`<`/`>`), not the C-string-style backslash escaping this
    // used to do. That old scheme protected nothing: `parse_style_fragment`
    // (below) stripped the backslashes back out before any downstream
    // consumer ever saw the value, so a value containing `"` reached a
    // real XAML attribute completely unescaped — able to terminate the
    // attribute and inject markup. Fixing it here, once, at the single
    // production write path for `PartStyleEntry.base_fragment`, means
    // every consumer (five `parse_style_fragment` callers, plus
    // `part_style_attr`'s whole-fragment raw splice used by 17 more sites)
    // is correct with no further changes: escaped values can no longer
    // contain a literal `"`, which is also exactly what keeps this
    // fragment's own `Key="Value"` delimiter structure parseable.
    let fragment = parts
        .into_iter()
        .map(|(key, value)| {
            let escaped = escape_xaml_attr(&value);
            format!("{key}=\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" ");
    (fragment, drops)
}

/// One mosstyle property, on one part, that XAML lowering could not express
/// at all — no attribute, no `<Setter>`, nothing. Public so
/// `mosaic-package-artifact-builder`'s degradation analyzer (issue #12022)
/// can surface these in `mosaic-degradations.json` instead of them vanishing
/// silently, the same way `host_table_has_native_semantics` and friends let
/// it ask about capability-level gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedStyleProperty {
    pub part: String,
    pub name: String,
    pub value: String,
    pub reason: String,
}

/// Every property, across every part of `style`, that XAML lowering drops
/// with no expressible output — see `mosaic-emit-xaml.md` §3.1.
///
/// Deliberately excludes `align-items`/`justify-content`/`flex-grow` when
/// their value is one `FlexHints` (PR #12980) actually consumes through its
/// own side channel outside `build_style_fragment` — those are NOT drops,
/// even though `build_style_fragment_with_drops` sees them as such in
/// isolation (it has no visibility into the separate flex-lowering path).
/// `flex-grow` is excluded unconditionally: it's fully boolean-handled
/// today (grows or doesn't), so there is nothing a reader would recognise
/// as "lost" by reporting it.
pub fn dropped_style_properties(style: &mosstyle_compiler::StyleDef) -> Vec<DroppedStyleProperty> {
    let mut out = Vec::new();
    for part in &style.parts {
        let (_, raw_drops) = build_style_fragment_with_drops(&part.base);
        if raw_drops.is_empty() {
            continue;
        }
        for drop in raw_drops {
            let consumed_via_flex_hints = match drop.name.as_str() {
                "flex-grow" => true,
                "align-items" => drop.value.trim() == "center",
                "justify-content" => drop.value.trim() == "space-between",
                _ => false,
            };
            if consumed_via_flex_hints {
                continue;
            }
            out.push(DroppedStyleProperty {
                part: part.name.clone(),
                name: drop.name,
                value: drop.value,
                reason: drop.reason.to_string(),
            });
        }
    }
    out
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

/// Map a full-width/full-height declaration onto the XAML alignment that
/// expresses it, or `None` when this is not that case.
///
/// `width: 100%` is a *sizing* property in CSS but an *alignment* in XAML:
/// WinUI's `Width` is an absolute `Double` with no percentage form, and the
/// way an element fills its parent's cross axis is
/// `HorizontalAlignment="Stretch"`.
///
/// Deliberately narrow. Percentages other than 100% need proportional
/// sizing — a `Grid` with star columns — which is a much larger change;
/// those still drop rather than being approximated.
fn stretch_alignment_for(key: &str, value: &str) -> Option<&'static str> {
    let v = value.trim();
    match (key, v) {
        // Percentage of the parent.
        ("Width", "100%") => Some("HorizontalAlignment"),
        ("Height", "100%") => Some("VerticalAlignment"),

        // Viewport units. `min-height: 100vh` is how a web shell says "fill
        // the window", and it was reaching XAML verbatim as
        // `MinHeight="100vh"` — not a Double, so the shell never got a
        // height. In a desktop app the window IS the viewport, so filling it
        // is the same stretch.
        ("Height" | "MinHeight" | "MaxHeight", "100vh") => Some("VerticalAlignment"),
        ("Width" | "MinWidth" | "MaxWidth", "100vw") => Some("HorizontalAlignment"),
        _ => None,
    }
}

/// Reject a length carrying a CSS unit XAML cannot parse.
///
/// The length path strips `px` and rejects `%`, but every other CSS unit fell
/// straight through into the emitted attribute — `MinHeight="100vh"` shipped
/// this way. WinUI lengths are `Double`, so such a value is not merely wrong,
/// it is unparseable, and the failure is silent at build time.
///
/// Anything ending in an alphabetic suffix that is not `px` is refused here.
/// Callers drop the property, which is the honest outcome: better an element
/// sized by its parent than an attribute the runtime cannot read.
fn has_unsupported_length_unit(value: &str) -> bool {
    let v = value.trim();
    let Some(first_alpha) = v.find(|c: char| c.is_ascii_alphabetic()) else {
        return false;
    };
    // A leading-alpha value is a keyword (`Auto`, `Stretch`), not a length.
    if first_alpha == 0 {
        return false;
    }
    !v[first_alpha..].eq_ignore_ascii_case("px")
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

    // Color setters: hand off to the X4 PascalCasing pass. It returns
    // `None` for CSS-only color forms that have no XAML literal
    // (`currentColor`), which drops the property rather than emitting a
    // value the XAML *runtime* rejects with E_XAMLPARSEFAILED. Note the
    // markup compiler does not validate brush literals, so an
    // unconvertible color builds cleanly and crashes on launch.
    if is_color_setter(key) {
        return normalize_xaml_color_value(raw);
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
        // Any other CSS unit (`vh`, `vw`, `em`, `rem`, `ch`, …) is equally
        // unparseable as a Double, and previously fell straight through into
        // the attribute. `MinHeight="100vh"` shipped that way, which is how
        // the generated shell lost its full-window height.
        if has_unsupported_length_unit(trimmed) {
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
fn normalize_xaml_color_value(s: &str) -> Option<String> {
    let trimmed = s.trim();
    // Defence in depth. #12025 fixed the general case — the base-style-
    // fragment path now applies `escape_xaml_attr` at the point the
    // fragment is built (`build_style_fragment_with_drops`), same as the
    // `<Setter>` path already did — so this guard is no longer the only
    // thing standing between a hostile colour token and markup injection.
    // Kept anyway: no legitimate colour literal contains `"`, `<`, `>`, or
    // `&`, so refusing them here costs nothing, and a value this
    // implausible is more usefully rejected outright than silently
    // encoded into something that still isn't a real colour.
    if trimmed.contains(['"', '<', '>', '&']) {
        return None;
    }
    if trimmed.starts_with('#') {
        return Some(s.to_string());
    }
    // `rgb()` / `rgba()` are CSS *functions*, not XAML literals. Convert
    // them to the `#AARRGGBB` form XAML understands rather than passing
    // the call syntax through (which parses at build time and throws
    // at runtime).
    if let Some(hex) = css_rgb_function_to_xaml_hex(trimmed) {
        return Some(hex);
    }
    // `currentColor` is a CSS *cascade* keyword — "whatever the
    // inherited text color is". XAML has no equivalent: a Brush must
    // resolve to an actual color, and there is no ambient inherited
    // value to read at parse time. Drop it and let the element keep its
    // theme default; inventing a color here would silently diverge from
    // the authored design.
    if trimmed.eq_ignore_ascii_case("currentcolor") {
        return None;
    }
    // `{x:Bind â€¦}` / `{Binding â€¦}` markup extensions or any string with
    // braces â€” keep verbatim.  These aren't color literals.
    if trimmed.starts_with('{') {
        return Some(s.to_string());
    }
    // Already PascalCased (or starts with an uppercase letter)?  Treat
    // as XAML-native and pass through.
    let first = trimmed.chars().next();
    if matches!(first, Some(c) if c.is_ascii_uppercase()) {
        return Some(s.to_string());
    }
    // All-lowercase identifier â€” PascalCase it.  `transparent` â†’
    // `Transparent`, `red` â†’ `Red`, etc.  We don't gate on a known
    // CSS-color whitelist: the markup compiler will reject anything
    // that isn't a real named color, and over-pascalCasing is the
    // failure mode we want (it just shifts which compiler complains).
    if trimmed.chars().all(|c| c.is_ascii_lowercase()) {
        let mut chars = trimmed.chars();
        match chars.next() {
            Some(c) => return Some(c.to_ascii_uppercase().to_string() + chars.as_str()),
            None => return Some(s.to_string()),
        }
    }
    Some(s.to_string())
}

/// Convert a CSS `rgb()` / `rgba()` function call into the `#RRGGBB` /
/// `#AARRGGBB` literal XAML expects.
///
/// XAML brushes accept only hex literals and named colors, so the CSS
/// call syntax has to be evaluated at emit time. The alpha channel is
/// CSS's `0.0..=1.0` fraction (commonly written with a leading dot, as
/// in `rgba(20,17,13,.28)`) and becomes XAML's leading `AA` byte.
///
/// Returns `None` for anything that isn't a well-formed 3- or 4-argument
/// `rgb`/`rgba` call with integer channels in `0..=255`, so malformed
/// input falls through to the caller's other branches rather than
/// producing a bogus color.
fn css_rgb_function_to_xaml_hex(s: &str) -> Option<String> {
    let lower = s.trim().to_ascii_lowercase();
    let inner = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))?
        .strip_suffix(')')?;

    // `splitn(5, ..)` caps the allocation: a hostile value like
    // `rgb(` + 100k commas + `)` would otherwise materialize one slice per
    // field before the arity check rejects it. Commas are legal in mosstyle
    // token values, so this input is reachable from a third-party package.
    // Five is enough to still distinguish "4 fields" from "too many".
    let parts: Vec<&str> = inner.splitn(5, ',').map(str::trim).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }

    let mut channels = [0u8; 3];
    for (slot, raw) in channels.iter_mut().zip(parts.iter()) {
        *slot = raw.parse::<u16>().ok().filter(|v| *v <= 255)? as u8;
    }

    let alpha = match parts.get(3) {
        None => 255u8,
        Some(raw) => {
            let fraction = raw.parse::<f64>().ok()?;
            if !(0.0..=1.0).contains(&fraction) {
                return None;
            }
            // Round rather than truncate so `.28` lands on 71 (0x47),
            // matching how browsers rasterize the same value.
            (fraction * 255.0).round() as u8
        }
    };

    Some(format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        alpha, channels[0], channels[1], channels[2]
    ))
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
        "opacity" => Some("Opacity".to_string()),
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
        //   align-items     — no 1:1 XAML setter (it becomes per-child
        //                     Horizontal/VerticalAlignment, not one
        //                     attribute on the container). `emit_flex_grid`
        //                     reads it straight off the raw StyleProp list
        //                     via `extract_flex_hints`/`FlexHints`, not
        //                     through this table — see mosaic-emit-xaml.md
        //                     §3.1. Still correctly `None` here.
        //   justify-content — same story: becomes spacer `ColumnDefinition`/
        //                     `RowDefinition`s, not a container attribute.
        //                     See `FlexHints`/§3.1.
        //   flex-wrap       — WinUI has no built-in WrapPanel in WinUI 3.
        //                     Genuinely unhandled (tracked separately).
        "align-items" | "border-collapse" | "border-style" | "box-shadow" | "flex-wrap"
        | "justify-content" | "outline" | "text-decoration" => None,
        // Unknown properties must not be PascalCased into fake WinUI
        // setters. That path let Lattice/web layout names such as
        // `gap` and `flex-wrap` leak into TextBlock styles and made
        // XamlCompiler.exe fail with code 1 and no useful diagnostic.
        _ => None,
    }
}

/// A short, specific reason a property named `name` has no XAML setter —
/// used by `dropped_style_properties` (issue #12022) to explain *why* a
/// property was dropped, not just that it was. Mirrors the prose already in
/// `css_property_to_xaml_setter`'s doc comment for the known cases.
///
/// `align-items` / `justify-content` / `flex-grow` are deliberately absent
/// from this function's callers' concern in the common case — they have no
/// XAML setter either, but PR #12980 consumes them through a side channel
/// (`FlexHints`) the caller checks separately before ever reaching here.
/// This function only supplies the *text*; it doesn't decide who's exempt.
fn unsupported_property_reason(name: &str) -> &'static str {
    match name {
        "border-collapse" => {
            "WinUI has no table model; gridlines are drawn by per-cell BorderThickness"
        }
        "border-style" => "WinUI borders are always solid; there is no dashed/dotted BorderStyle property",
        "outline" => "no WinUI equivalent (focus visuals use the FocusVisual* attached properties)",
        "text-decoration" => {
            "TextBlock's TextDecorations property has a different value shape; not wired yet"
        }
        "box-shadow" => "WinUI shadows use <ThemeShadow> / translation Z, not a CSS-shaped property",
        "flex-wrap" => "WinUI 3 has no built-in WrapPanel",
        "align-items" => {
            "this value isn't one FlexHints recognises (only \"center\" is); no per-child alignment was applied"
        }
        "justify-content" => {
            "this value isn't one FlexHints recognises (only \"space-between\" is); no distribution was applied"
        }
        _ => "no WinUI XAML setter is mapped for this property",
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
            | "Opacity"
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
            | "Opacity"
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
    if ctx.visual_state_groups.is_empty() {
        out.push_str(&body);
    } else {
        // WinUI only evaluates declarative StateTriggers automatically when
        // VisualStateGroups are attached to the root's first visual child.
        // A transparent Grid provides that stable attachment point without
        // changing the layout contract.
        writeln!(out, "    <Grid>").unwrap();
        out.push_str(&emit_visual_state_groups(&ctx.visual_state_groups, 8));
        out.push_str(&indent_xaml_fragment(&body, 4));
        writeln!(out, "    </Grid>").unwrap();
    }

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

    // After walking, declare any generated converter resources exactly once.
    // We splice them in after the open root tag.
    if ctx.needs_bool_to_vis || ctx.needs_focus_state_converter {
        let resources_tag = match shape {
            RootShape::UserControl => "UserControl.Resources",
            RootShape::ContentDialog => "ContentDialog.Resources",
        };
        let resources = emit_converter_resource_block(
            4,
            resources_tag,
            ctx.needs_bool_to_vis,
            ctx.needs_focus_state_converter,
        );
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

fn indent_xaml_fragment(fragment: &str, extra_spaces: usize) -> String {
    let pad = " ".repeat(extra_spaces);
    let mut out = String::with_capacity(fragment.len() + extra_spaces * 4);
    for line in fragment.split_inclusive('\n') {
        if line.trim().is_empty() {
            out.push_str(line);
        } else {
            out.push_str(&pad);
            out.push_str(line);
        }
    }
    out
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
        "Row" => emit_flex_grid(node, indent, part_styles, FlexAxis::Row, ctx),
        "Column" => emit_flex_grid(node, indent, part_styles, FlexAxis::Column, ctx),
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
        "If" => emit_if(node, None, indent, part_styles, ctx, None),
        // A standalone `Else` (no preceding `If`) is a moslayout-level
        // validation error per UI29 Â§3.2; we treat it as
        // UnsupportedPrimitive here for the second line of defence in
        // case validation was bypassed.
        "Else" => Err(PipelineEmitError::UnsupportedPrimitive(
            "Else without preceding If".to_string(),
        )),

        // PR-3: Host* primitives (single-element host-native controls).
        // UI25 `Input` remains the multiline text primitive used by the
        // Notes package. WinUI TextBox already implements its complete
        // property surface, so it shares HostInput's native lowering.
        "Input" | "HostInput" => emit_host_input(node, indent, part_styles, ctx),
        "HostButton" => emit_host_button(node, indent, part_styles, ctx),
        "HostSurface" => emit_host_surface(node, indent, part_styles, ctx),

        // UI29-2 â€” `HostCheckbox` lowers to WinUI/WPF `<CheckBox>` and
        // `HostRadio` lowers to `<RadioButton>`. Both controls share
        // the `IsChecked` / `IsEnabled` / `Content` property surface
        // with `<Button>`, plus their own checked-state events.
        "HostCheckbox" => emit_host_checkbox(node, indent, part_styles, ctx),
        "HostRadio" => emit_host_radio(node, indent, part_styles, ctx),
        "HostSlider" => emit_host_slider(node, indent, part_styles, ctx),

        // UI29-4 â€” HostLink lowers to a `<HyperlinkButton NavigateUri=
        // "..." Content="...">` (WinUI 3's first-class clickable
        // hyperlink). HostTooltip uses the `ToolTipService.ToolTip`
        // attached property on the wrapped child. HostNumberInput
        // uses `<NumberBox>` (WinUI 3 numeric input with built-in Â±
        // stepper).
        // UI35 — native WinUI pointer/touch drag-and-drop plus an accessible
        // keyboard path. Both paths converge on the target control's Accept
        // method so proposal payloads and accepted outcomes cannot drift.
        "HostDraggable" => emit_host_draggable(node, indent, part_styles, ctx),
        "HostDropTarget" => emit_host_drop_target(node, indent, part_styles, ctx),

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
            out.push_str(&emit_if(child, else_node, indent, part_styles, ctx, None)?);
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
        if let Some(entry) = part_styles.get(part) {
            // Each fragment is space-separated `Key="Value"` pairs ready
            // to splice straight into the opening tag.
            if !entry.base_fragment.is_empty() {
                return format!(" {}", entry.base_fragment);
            }
        }
    }
    String::new()
}

/// ContentControl descendants such as Button align their content through
/// `HorizontalContentAlignment`; unlike TextBox/TextBlock they do not expose a
/// `TextAlignment` dependency property. Keep shared MSL `text-align` semantics
/// while emitting the native property accepted by WinUI's markup compiler.
fn content_control_style_attr(node: &LayoutNode, part_styles: &PartStyleMap) -> String {
    let Some(fragment) = node
        .part_name
        .as_deref()
        .and_then(|part| part_styles.get(part))
        .map(|entry| entry.base_fragment.as_str())
    else {
        return String::new();
    };
    parse_style_fragment(fragment)
        .into_iter()
        .map(|(setter, value)| {
            let setter = if setter == "TextAlignment" {
                "HorizontalContentAlignment"
            } else {
                setter.as_str()
            };
            format!(" {setter}=\"{value}\"")
        })
        .collect()
}

/// Drag wrappers are ContentControls, so container-only `Spacing` must live on
/// the neutral StackPanel used when the primitive has multiple direct children.
/// Emitting it on ContentControl is rejected by the WinUI markup compiler.
fn drag_control_style_attr(
    node: &LayoutNode,
    part_styles: &PartStyleMap,
) -> (String, Option<String>) {
    let Some(fragment) = node
        .part_name
        .as_deref()
        .and_then(|part| part_styles.get(part))
        .map(|entry| entry.base_fragment.as_str())
    else {
        return (String::new(), None);
    };
    let mut attrs = String::new();
    let mut spacing = None;
    for (setter, value) in parse_style_fragment(fragment) {
        if setter == "Spacing" {
            spacing = Some(value);
            continue;
        }
        let setter = if setter == "TextAlignment" {
            "HorizontalContentAlignment"
        } else {
            setter.as_str()
        };
        write!(attrs, " {setter}=\"{value}\"").unwrap();
    }
    (attrs, spacing)
}

fn emit_drag_content_children(
    children: &[LayoutNode],
    indent: usize,
    spacing: Option<&str>,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    if emitted_content_child_count(children) <= 1 {
        return emit_xaml_children(children, indent, part_styles, ctx);
    }

    let pad = " ".repeat(indent);
    let spacing = spacing
        .map(|value| format!(" Spacing=\"{value}\""))
        .unwrap_or_default();
    let mut out = String::new();
    writeln!(out, "{pad}<StackPanel Orientation=\"Vertical\"{spacing}>").unwrap();
    out.push_str(&emit_xaml_children(children, indent + 4, part_styles, ctx)?);
    writeln!(out, "{pad}</StackPanel>").unwrap();
    Ok(out)
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
                      // Read the value up to the next '"'. #12025: this
                      // used to special-case `\"` as an escaped quote,
                      // matching `build_style_fragment_with_drops`'s old
                      // C-string-style escaping — that escaping is gone
                      // now (replaced with real `escape_xaml_attr` XML
                      // escaping at the point the fragment is built), so
                      // a value can no longer contain a literal `"` at
                      // all: it always arrives here already encoded as
                      // `&quot;`. No backslash handling is needed or
                      // correct to keep.
        let mut value = String::new();
        while let Some(&c) = chars.peek() {
            if c == '"' {
                chars.next();
                break;
            }
            value.push(c);
            chars.next();
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
        Some(entry) => entry.base_fragment.as_str(),
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
        Some(entry) => entry.base_fragment.as_str(),
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

/// `<Grid>` equivalent of `partition_stack_panel_style`. Same three-way
/// split (wrapper `<Border>` attrs / the panel's own attrs / text-style
/// setters for descendant `TextBlock`s) and the same "valid directly on
/// the panel" bucket (`is_stack_panel_style_attr` — `Margin`/`Width`/
/// `Height`/Min·Max/alignments/`Opacity` are all valid on `<Grid>` too;
/// `<Grid>` has no `Padding` any more than `<StackPanel>` does, so those
/// still route to the wrapping `<Border>`). The one difference: `gap`
/// lowers to the `StackPanel`-only `Spacing` setter, which `<Grid>`
/// doesn't have — rename it to `axis.spacing_attr()`
/// (`ColumnSpacing`/`RowSpacing`) here.
fn partition_flex_grid_style(
    part: Option<&str>,
    part_styles: &PartStyleMap,
    axis: FlexAxis,
) -> (String, String, Vec<(String, String)>) {
    let frag = match part.and_then(|p| part_styles.get(p)) {
        Some(entry) => entry.base_fragment.as_str(),
        None => return (String::new(), String::new(), Vec::new()),
    };
    let mut wrapper = String::new();
    let mut grid = String::new();
    let mut text = Vec::new();
    for (setter, value) in parse_style_fragment(frag) {
        if setter == "Spacing" {
            grid.push(' ');
            grid.push_str(axis.spacing_attr());
            grid.push_str("=\"");
            grid.push_str(&value);
            grid.push('"');
        } else if is_container_style_attr(&setter) && !is_stack_panel_style_attr(&setter) {
            wrapper.push(' ');
            wrapper.push_str(&setter);
            wrapper.push_str("=\"");
            wrapper.push_str(&value);
            wrapper.push('"');
        } else if is_stack_panel_style_attr(&setter) {
            grid.push(' ');
            grid.push_str(&setter);
            grid.push_str("=\"");
            grid.push_str(&value);
            grid.push('"');
        } else if is_text_style_attr(&setter) {
            text.push((setter, value));
        }
    }
    (wrapper, grid, text)
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

/// Mount a host-supplied `UIElement` node slot inside a styled native
/// composition boundary. The Border preserves shared MSL sizing/background
/// while ContentPresenter owns the actual WinUI child supplied by the host.
fn emit_host_surface(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);
    let (container_attrs, _) = partition_box_style(node.part_name.as_deref(), part_styles);
    let content = match find_prop_value(node, "content") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            format!(
                " Content=\"{{x:Bind {}, Mode=OneWay}}\"",
                ctx.slot_xbind_path(slot)
            )
        }
        _ => String::new(),
    };
    Ok(format!(
        "{pad}<Border{container_attrs}>\n{inner_pad}<ContentPresenter{content}/>\n{pad}</Border>\n"
    ))
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

/// One `Grid.Column`/`Grid.Row` slot inside a flex `<Grid>`. An `If`/`Else`
/// pair occupies exactly one slot (both branches share one grid index —
/// see `mosaic-emit-xaml.md` §3.1); a `justify-content: space-between`
/// spacer is a slot with no source node at all.
enum FlexSlot<'a> {
    Node {
        node: &'a LayoutNode,
        grows: bool,
    },
    If {
        if_node: &'a LayoutNode,
        else_node: Option<&'a LayoutNode>,
        grows: bool,
    },
    Spacer,
}

/// Does `node`'s own part style request a star-sized cell on `axis`
/// (`flex-grow`, or the main-axis `width`/`height: 100%` case — see
/// `FlexAxis::child_grows`)?
fn flex_child_grows(node: &LayoutNode, part_styles: &PartStyleMap, axis: FlexAxis) -> bool {
    node.part_name
        .as_deref()
        .and_then(|p| part_styles.get(p))
        .is_some_and(|entry| axis.child_grows(&entry.flex))
}

/// `Row`/`Column [name] { children }` → `<Grid>` with one
/// `ColumnDefinition`/`RowDefinition` per child slot and a matching
/// `Grid.Column`/`Grid.Row` attached property on each child — the flex
/// lowering documented in `mosaic-emit-xaml.md` §3.1. Replaces the old
/// `<StackPanel>` lowering (`emit_stack_panel`, kept for the unrelated
/// `HostTable` row-section emitter, which isn't a flex container).
fn emit_flex_grid(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    axis: FlexAxis,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);
    let (container_attrs, grid_attrs, text_setters) =
        partition_flex_grid_style(node.part_name.as_deref(), part_styles, axis);
    let wrapped = !container_attrs.is_empty() || !text_setters.is_empty();
    let mut out = if !wrapped {
        format!("{pad}<Grid{grid_attrs}>\n")
    } else {
        let mut wrapped_out = format!("{pad}<Border{container_attrs}>\n");
        emit_text_style_resources(&mut wrapped_out, "Border", indent + 4, &text_setters);
        writeln!(wrapped_out, "{inner_pad}<Grid{grid_attrs}>").unwrap();
        wrapped_out
    };
    let content_indent = if wrapped { indent + 8 } else { indent + 4 };
    let content_pad = " ".repeat(content_indent);

    let container_flex = node
        .part_name
        .as_deref()
        .and_then(|p| part_styles.get(p))
        .map(|entry| entry.flex.clone())
        .unwrap_or_default();
    let space_between = container_flex.justify_content.as_deref() == Some("space-between");
    let cross_align = container_flex
        .align_items
        .as_deref()
        .filter(|v| *v == "center")
        .map(|_| format!("{}=\"Center\"", axis.cross_align_property()));

    // -- Build the ordered slot list. An `If` looks ahead for a paired
    //    `Else` exactly like `emit_xaml_children` does — both branches
    //    are one logical slot even though `emit_if` emits two sibling
    //    `<ContentControl>`s for them. --
    let mut slots: Vec<FlexSlot<'_>> = Vec::with_capacity(node.children.len());
    let mut i = 0;
    while i < node.children.len() {
        let child = &node.children[i];
        if child.tag == "Else" {
            return Err(PipelineEmitError::UnsupportedPrimitive(
                "Else without preceding If".to_string(),
            ));
        }
        if space_between && !slots.is_empty() {
            slots.push(FlexSlot::Spacer);
        }
        if child.tag == "If" {
            let else_node = node.children.get(i + 1).filter(|n| n.tag == "Else");
            let grows = flex_child_grows(child, part_styles, axis);
            slots.push(FlexSlot::If {
                if_node: child,
                else_node,
                grows,
            });
            i += if else_node.is_some() { 2 } else { 1 };
        } else {
            let grows = flex_child_grows(child, part_styles, axis);
            slots.push(FlexSlot::Node { node: child, grows });
            i += 1;
        }
    }

    // -- Definitions: one per slot, `"*"` for a growing child or a
    //    space-between spacer, `Auto` otherwise. --
    if !slots.is_empty() {
        writeln!(out, "{content_pad}<{}>", axis.definitions_tag()).unwrap();
        let def_pad = " ".repeat(content_indent + 4);
        for slot in &slots {
            let grows = matches!(
                slot,
                FlexSlot::Spacer
                    | FlexSlot::Node { grows: true, .. }
                    | FlexSlot::If { grows: true, .. }
            );
            let size = if grows { "*" } else { "Auto" };
            writeln!(
                out,
                "{def_pad}<{} {}=\"{size}\"/>",
                axis.definition_tag(),
                axis.definition_size_attr()
            )
            .unwrap();
        }
        writeln!(out, "{content_pad}</{}>", axis.definitions_tag()).unwrap();
    }

    // -- Children, each positioned by the same-index attached property. --
    for (idx, slot) in slots.iter().enumerate() {
        let position_attr = format!("{}=\"{idx}\"", axis.grid_position_property());
        let combined_attr = match &cross_align {
            Some(a) => format!("{position_attr} {a}"),
            None => position_attr,
        };
        match slot {
            FlexSlot::Spacer => {
                let spacer = format!("{content_pad}<Rectangle Width=\"Auto\" Height=\"Auto\"/>\n");
                out.push_str(&inject_attr_into_first_element(&spacer, &combined_attr));
            }
            FlexSlot::If {
                if_node,
                else_node,
                ..
            } => {
                out.push_str(&emit_if(
                    if_node,
                    *else_node,
                    content_indent,
                    part_styles,
                    ctx,
                    Some(&combined_attr),
                )?);
            }
            FlexSlot::Node { node: child, .. } => {
                let body = emit_xaml_node(child, content_indent, part_styles, ctx)?;
                out.push_str(&inject_attr_into_first_element(&body, &combined_attr));
            }
        }
    }

    if !wrapped {
        writeln!(out, "{pad}</Grid>").unwrap();
    } else {
        writeln!(out, "{inner_pad}</Grid>").unwrap();
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
    let inner_pad = " ".repeat(indent + 4);
    let (container_style, text_setters) =
        partition_box_style(node.part_name.as_deref(), part_styles);
    let text_style = text_setters
        .into_iter()
        .map(|(setter, value)| format!(" {setter}=\"{value}\""))
        .collect::<String>();

    let mut accessibility_attrs = String::new();
    match find_prop_value(node, "a11y-label") {
        Some(LayoutPropValue::String(label)) => {
            write!(
                accessibility_attrs,
                " AutomationProperties.Name=\"{}\"",
                escape_xaml_attr(label)
            )
            .unwrap();
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let property = ctx.slot_property_name(slot);
            if !is_safe_identifier(&property) {
                return Err(PipelineEmitError::UnsafeSlotName(property));
            }
            // A slot-backed accessible name tracks the component's live
            // prop value, so it must be OneWay. `x:Bind` defaults to
            // OneTime, which would freeze the announced name at whatever
            // the slot held when the template was first realized.
            write!(
                accessibility_attrs,
                " AutomationProperties.Name=\"{{x:Bind {}, Mode=OneWay}}\"",
                ctx.slot_xbind_path(slot)
            )
            .unwrap();
        }
        Some(LayoutPropValue::Expr(src)) => {
            // #12126: same OneWay reasoning as the SlotRef arm above — an
            // expression can read live component slots via a generated
            // helper, so a name computed once at template-realize time
            // would go stale.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    write!(
                        accessibility_attrs,
                        " AutomationProperties.Name=\"{{x:Bind {path}, Mode=OneWay}}\""
                    )
                    .unwrap();
                }
                ExprLowering::Helper(call) => {
                    write!(
                        accessibility_attrs,
                        " AutomationProperties.Name=\"{{x:Bind {call}, Mode=OneWay}}\""
                    )
                    .unwrap();
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
        }
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
    }
    match find_prop_keyword(node, "a11y-role") {
        Some("heading") => accessibility_attrs
            .push_str(" AutomationProperties.HeadingLevel=\"Level2\""),
        Some("none") => accessibility_attrs
            .push_str(" AutomationProperties.AccessibilityView=\"Raw\""),
        _ => {}
    }
    if matches!(find_prop_keyword(node, "a11y-hidden"), Some("true"))
        && !matches!(find_prop_keyword(node, "a11y-role"), Some("none"))
    {
        accessibility_attrs.push_str(" AutomationProperties.AccessibilityView=\"Raw\"");
    }

    let text_attr = match find_prop_value(node, "content") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let property = ctx.slot_property_name(slot);
            if !is_safe_identifier(&property) {
                return Err(PipelineEmitError::UnsafeSlotName(property));
            }
            // Slot-backed text is the component's live prop value and
            // must be OneWay. `x:Bind`'s default is OneTime, which
            // renders the value once and never again — the defect that
            // froze every label in the generated TaskApp.
            format!(
                " Text=\"{{x:Bind {}, Mode=OneWay}}\"",
                ctx.slot_xbind_path(slot)
            )
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
                format!(" Text=\"{{x:Bind {pascal}, Mode=OneWay}}\"")
            } else {
                let escaped = escape_xaml_attr(k);
                format!(" Text=\"{escaped}\"")
            }
        }
        Some(LayoutPropValue::Number(n)) => format!(" Text=\"{n}\""),
        Some(LayoutPropValue::Expr(src)) => {
            // PR-2: route through ExprLowerer.
            // An expression can read component slots (directly or via a
            // generated helper), so it must be OneWay for the same
            // reason as the SlotRef arm above. Row-VM-only expressions
            // are re-evaluated when the item is rebuilt regardless, so
            // OneWay is never wrong here — only occasionally redundant.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    format!(" Text=\"{{x:Bind {path}, Mode=OneWay}}\"")
                }
                ExprLowering::Helper(call) => {
                    format!(" Text=\"{{x:Bind {call}, Mode=OneWay}}\"")
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
        }
        Some(LayoutPropValue::EmitRef(_)) | None => String::new(),
    };

    if container_style.is_empty() {
        Ok(format!(
            "{pad}<TextBlock{text_attr}{accessibility_attrs}{text_style}/>\n"
        ))
    } else {
        Ok(format!(
            "{pad}<Border{container_style}>\n{inner_pad}<TextBlock{text_attr}{accessibility_attrs}{text_style}/>\n{pad}</Border>\n"
        ))
    }
}

/// `Image [name] (source: slot: foo)` â†’ `<Image Source="{x:Bind Foo}"/>`.
fn emit_image(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);

    let source_attr = match find_prop_value(node, "source").or_else(|| find_prop_value(node, "src"))
    {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let property = ctx.slot_property_name(slot);
            if !is_safe_identifier(&property) {
                return Err(PipelineEmitError::UnsafeSlotName(property));
            }
            format!(" Source=\"{{x:Bind {}, Mode=OneWay}}\"", ctx.slot_xbind_path(slot))
        }
        Some(LayoutPropValue::String(s)) => format!(" Source=\"{}\"", escape_xaml_attr(s)),
        Some(LayoutPropValue::Expr(src)) => {
            // #12126: same OneWay reasoning as the SlotRef arm above. The
            // helper/path this lowers to yields a `string`; XAML's x:Bind
            // compiler applies its documented implicit `string ->
            // ImageSource` conversion for compiled bindings, the same
            // conversion a literal `Source="images/foo.png"` relies on.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    format!(" Source=\"{{x:Bind {path}, Mode=OneWay}}\"")
                }
                ExprLowering::Helper(call) => {
                    format!(" Source=\"{{x:Bind {call}, Mode=OneWay}}\"")
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
        }
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => String::new(),
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
    ctx: &mut EmitContext<'_>,
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
            let pascal = ctx.slot_xbind_path(slot);
            format!(" Glyph=\"{{x:Bind {pascal}, Mode=OneWay}}\"")
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
    if ctx.needs_native_table_support
        || ctx.needs_native_drag_support
        || ctx.needs_native_slider_support
    {
        writeln!(out, "using Microsoft.UI.Xaml.Automation;").unwrap();
        writeln!(out, "using Microsoft.UI.Xaml.Automation.Peers;").unwrap();
        writeln!(out, "using Microsoft.UI.Xaml.Input;").unwrap();
        writeln!(out, "using Microsoft.UI.Xaml.Media;").unwrap();
        writeln!(out, "using Windows.System;").unwrap();
    }
    if ctx.needs_native_table_support {
        writeln!(out, "using Microsoft.UI.Xaml.Automation.Provider;").unwrap();
    }
    if ctx.needs_native_drag_support {
        writeln!(out, "using System.Runtime.CompilerServices;").unwrap();
        writeln!(out, "using Windows.ApplicationModel.DataTransfer;").unwrap();
    }
    writeln!(out, "using System;").unwrap();
    writeln!(out, "using System.Collections.Generic;").unwrap();
    if !ctx.row_projections.is_empty() {
        writeln!(out, "using System.ComponentModel;").unwrap();
    }
    writeln!(out).unwrap();
    writeln!(out, "namespace {ns};").unwrap();
    writeln!(out).unwrap();

    let property_change_interface = if ctx.row_projections.is_empty() {
        ""
    } else {
        ", INotifyPropertyChanged"
    };
    writeln!(
        out,
        "public sealed partial class {name} : {base_class}{property_change_interface}"
    )
    .unwrap();
    writeln!(out, "{{").unwrap();

    // Constructor: `InitializeComponent()`. The XAML compiler generates
    // the actual `InitializeComponent` method at build time from the
    // matching `.xaml` file.
    writeln!(out, "    public {name}()").unwrap();
    writeln!(out, "    {{").unwrap();
    writeln!(out, "        this.InitializeComponent();").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    if !ctx.row_projections.is_empty() {
        writeln!(
            out,
            "    public event PropertyChangedEventHandler? PropertyChanged;"
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "    private void NotifyRowProjectionChanged(string propertyName)"
        )
        .unwrap();
        writeln!(out, "    {{").unwrap();
        writeln!(
            out,
            "        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));"
        )
        .unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
    }

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
    // logical). Row-VM expression properties call these methods from a
    // separate generated type, so assembly-local visibility is required.
    for helper in &ctx.helpers {
        let params: Vec<String> = helper
            .parameters
            .iter()
            .map(|(n, t)| format!("{t} {n}"))
            .collect();
        writeln!(
            out,
            "    internal {} {}({}) => {};",
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
    if ctx.needs_native_table_support {
        out.push_str(&emit_native_table_support_source(name));
    }
    if ctx.needs_native_drag_support {
        out.push_str(&emit_native_drag_support_source(name));
    }
    if ctx.needs_native_slider_support {
        out.push_str(&emit_native_slider_support_source(name));
    }
    Ok(out)
}

fn emit_native_table_support_source(component: &str) -> String {
    r#"

/// <summary>
/// Visual HostTable container whose peer exposes native UIA Table and Grid
/// patterns without replacing the authored Mosaic cell subtree.
/// </summary>
public sealed class __COMPONENT__MosaicTable : Grid
{
    public int RowCount
    {
        get => (int)GetValue(RowCountProperty);
        set => SetValue(RowCountProperty, value);
    }
    public static readonly DependencyProperty RowCountProperty =
        DependencyProperty.Register(nameof(RowCount), typeof(int), typeof(__COMPONENT__MosaicTable), new PropertyMetadata(0));

    public int ColumnCount
    {
        get => (int)GetValue(ColumnCountProperty);
        set => SetValue(ColumnCountProperty, value);
    }
    public static readonly DependencyProperty ColumnCountProperty =
        DependencyProperty.Register(nameof(ColumnCount), typeof(int), typeof(__COMPONENT__MosaicTable), new PropertyMetadata(0));

    protected override AutomationPeer OnCreateAutomationPeer() =>
        new __COMPONENT__MosaicTableAutomationPeer(this);

    internal __COMPONENT__MosaicTableCell? FindCell(int row, int column)
    {
        var cells = new List<__COMPONENT__MosaicTableCell>();
        CollectDescendants(this, cells);
        foreach (var cell in cells)
        {
            if (cell.Row == row && cell.Column == column) return cell;
        }
        return null;
    }

    internal IReadOnlyList<__COMPONENT__MosaicTableHeaderCell> ColumnHeaders()
    {
        var headers = new List<__COMPONENT__MosaicTableHeaderCell>();
        CollectDescendants(this, headers);
        headers.Sort((left, right) => left.Column.CompareTo(right.Column));
        return headers;
    }

    private static void CollectDescendants<T>(DependencyObject root, List<T> matches)
        where T : DependencyObject
    {
        var count = VisualTreeHelper.GetChildrenCount(root);
        for (var index = 0; index < count; index++)
        {
            var child = VisualTreeHelper.GetChild(root, index);
            if (child is T match) matches.Add(match);
            CollectDescendants(child, matches);
        }
    }
}

public sealed class __COMPONENT__MosaicTableHeaderCell : ContentControl
{
    public __COMPONENT__MosaicTableHeaderCell() => IsTabStop = false;

    public int Column
    {
        get => (int)GetValue(ColumnProperty);
        set => SetValue(ColumnProperty, value);
    }
    public static readonly DependencyProperty ColumnProperty =
        DependencyProperty.Register(nameof(Column), typeof(int), typeof(__COMPONENT__MosaicTableHeaderCell), new PropertyMetadata(0));

    public string Header
    {
        get => (string)GetValue(HeaderProperty);
        set => SetValue(HeaderProperty, value);
    }
    public static readonly DependencyProperty HeaderProperty =
        DependencyProperty.Register(nameof(Header), typeof(string), typeof(__COMPONENT__MosaicTableHeaderCell), new PropertyMetadata(string.Empty));

    protected override AutomationPeer OnCreateAutomationPeer() =>
        new __COMPONENT__MosaicTableHeaderCellAutomationPeer(this);
}

public sealed class __COMPONENT__MosaicTableCell : ContentControl
{
    public __COMPONENT__MosaicTableCell() => IsTabStop = true;

    public int Row
    {
        get => (int)GetValue(RowProperty);
        set => SetValue(RowProperty, value);
    }
    public static readonly DependencyProperty RowProperty =
        DependencyProperty.Register(nameof(Row), typeof(int), typeof(__COMPONENT__MosaicTableCell), new PropertyMetadata(0));

    public int Column
    {
        get => (int)GetValue(ColumnProperty);
        set => SetValue(ColumnProperty, value);
    }
    public static readonly DependencyProperty ColumnProperty =
        DependencyProperty.Register(nameof(Column), typeof(int), typeof(__COMPONENT__MosaicTableCell), new PropertyMetadata(0));

    public string Header
    {
        get => (string)GetValue(HeaderProperty);
        set => SetValue(HeaderProperty, value);
    }
    public static readonly DependencyProperty HeaderProperty =
        DependencyProperty.Register(nameof(Header), typeof(string), typeof(__COMPONENT__MosaicTableCell), new PropertyMetadata(string.Empty));

    public object? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }
    public static readonly DependencyProperty ValueProperty =
        DependencyProperty.Register(nameof(Value), typeof(object), typeof(__COMPONENT__MosaicTableCell), new PropertyMetadata(null));

    internal __COMPONENT__MosaicTable? FindTable()
    {
        DependencyObject? current = this;
        while ((current = VisualTreeHelper.GetParent(current)) is not null)
        {
            if (current is __COMPONENT__MosaicTable table) return table;
        }
        return null;
    }

    protected override void OnKeyDown(KeyRoutedEventArgs e)
    {
        base.OnKeyDown(e);
        if (e.Handled || FocusManager.GetFocusedElement(XamlRoot) != this) return;

        var nextRow = Row;
        var nextColumn = Column;
        switch (e.Key)
        {
            case VirtualKey.Left: nextColumn--; break;
            case VirtualKey.Right: nextColumn++; break;
            case VirtualKey.Up: nextRow--; break;
            case VirtualKey.Down: nextRow++; break;
            default: return;
        }

        var table = FindTable();
        var target = table?.FindCell(nextRow, nextColumn);
        if (target is not null && target.Focus(FocusState.Keyboard)) e.Handled = true;
    }

    protected override AutomationPeer OnCreateAutomationPeer() =>
        new __COMPONENT__MosaicTableCellAutomationPeer(this);
}

internal sealed class __COMPONENT__MosaicTableAutomationPeer : FrameworkElementAutomationPeer, IGridProvider, ITableProvider
{
    internal __COMPONENT__MosaicTableAutomationPeer(__COMPONENT__MosaicTable owner) : base(owner) { }
    private __COMPONENT__MosaicTable Table => (__COMPONENT__MosaicTable)Owner;

    protected override object GetPatternCore(PatternInterface patternInterface) =>
        patternInterface is PatternInterface.Grid or PatternInterface.Table
            ? this
            : base.GetPatternCore(patternInterface);
    protected override AutomationControlType GetAutomationControlTypeCore() => AutomationControlType.Table;
    protected override string GetClassNameCore() => nameof(__COMPONENT__MosaicTable);

    public int RowCount => Math.Max(0, Table.RowCount);
    public int ColumnCount => Math.Max(0, Table.ColumnCount);
    public RowOrColumnMajor RowOrColumnMajor => RowOrColumnMajor.RowMajor;

    public IRawElementProviderSimple GetItem(int row, int column)
    {
        var cell = Table.FindCell(row, column);
        return cell is null ? null! : ProviderFor(cell);
    }

    public IRawElementProviderSimple[] GetColumnHeaders()
    {
        var headers = Table.ColumnHeaders();
        var providers = new IRawElementProviderSimple[headers.Count];
        for (var index = 0; index < headers.Count; index++) providers[index] = ProviderFor(headers[index]);
        return providers;
    }

    public IRawElementProviderSimple[] GetRowHeaders() => Array.Empty<IRawElementProviderSimple>();

    private IRawElementProviderSimple ProviderFor(UIElement element)
    {
        var peer = FrameworkElementAutomationPeer.CreatePeerForElement(element);
        return peer is null ? null! : ProviderFromPeer(peer);
    }
}

internal sealed class __COMPONENT__MosaicTableHeaderCellAutomationPeer : FrameworkElementAutomationPeer
{
    internal __COMPONENT__MosaicTableHeaderCellAutomationPeer(__COMPONENT__MosaicTableHeaderCell owner) : base(owner) { }
    protected override AutomationControlType GetAutomationControlTypeCore() => AutomationControlType.HeaderItem;
    protected override string GetClassNameCore() => nameof(__COMPONENT__MosaicTableHeaderCell);
    protected override string GetNameCore()
    {
        var name = base.GetNameCore();
        return string.IsNullOrEmpty(name) ? ((__COMPONENT__MosaicTableHeaderCell)Owner).Header : name;
    }
}

internal sealed class __COMPONENT__MosaicTableCellAutomationPeer : FrameworkElementAutomationPeer, IGridItemProvider, ITableItemProvider
{
    internal __COMPONENT__MosaicTableCellAutomationPeer(__COMPONENT__MosaicTableCell owner) : base(owner) { }
    private __COMPONENT__MosaicTableCell Cell => (__COMPONENT__MosaicTableCell)Owner;

    protected override object GetPatternCore(PatternInterface patternInterface) =>
        patternInterface is PatternInterface.GridItem or PatternInterface.TableItem
            ? this
            : base.GetPatternCore(patternInterface);
    protected override AutomationControlType GetAutomationControlTypeCore() => AutomationControlType.DataItem;
    protected override string GetClassNameCore() => nameof(__COMPONENT__MosaicTableCell);
    protected override string GetNameCore()
    {
        var name = base.GetNameCore();
        return string.IsNullOrEmpty(name)
            ? $"{Cell.Header}, row {Cell.Row + 1}: {Convert.ToString(Cell.Value) ?? string.Empty}"
            : name;
    }

    public int Row => Math.Max(0, Cell.Row);
    public int Column => Math.Max(0, Cell.Column);
    public int RowSpan => 1;
    public int ColumnSpan => 1;
    public IRawElementProviderSimple ContainingGrid
    {
        get
        {
            var table = Cell.FindTable();
            if (table is null) return null!;
            var peer = FrameworkElementAutomationPeer.CreatePeerForElement(table);
            return peer is null ? null! : ProviderFromPeer(peer);
        }
    }

    public IRawElementProviderSimple[] GetColumnHeaderItems()
    {
        var table = Cell.FindTable();
        if (table is null) return Array.Empty<IRawElementProviderSimple>();
        foreach (var header in table.ColumnHeaders())
        {
            if (header.Column != Cell.Column) continue;
            var peer = FrameworkElementAutomationPeer.CreatePeerForElement(header);
            return peer is null
                ? Array.Empty<IRawElementProviderSimple>()
                : new[] { ProviderFromPeer(peer) };
        }
        return Array.Empty<IRawElementProviderSimple>();
    }

    public IRawElementProviderSimple[] GetRowHeaderItems() => Array.Empty<IRawElementProviderSimple>();
}
"#
    .replace("__COMPONENT__", component)
}

fn emit_native_drag_support_source(component: &str) -> String {
    r#"

public sealed class __COMPONENT__MosaicDragSourceEventArgs : EventArgs
{
    internal __COMPONENT__MosaicDragSourceEventArgs(string key, string kind)
    {
        Key = key;
        Kind = kind;
    }
    public string Key { get; }
    public string Kind { get; }
}

public sealed class __COMPONENT__MosaicDragEndEventArgs : EventArgs
{
    internal __COMPONENT__MosaicDragEndEventArgs(string key, string kind, bool dropped)
    {
        Key = key;
        Kind = kind;
        Dropped = dropped;
    }
    public string Key { get; }
    public string Kind { get; }
    public bool Dropped { get; }
}

public sealed class __COMPONENT__MosaicDropEventArgs : EventArgs
{
    internal __COMPONENT__MosaicDropEventArgs(string key, string kind, string targetKey, string position)
    {
        Key = key;
        Kind = kind;
        TargetKey = targetKey;
        Position = position;
    }
    public string Key { get; }
    public string Kind { get; }
    public string TargetKey { get; }
    public string Position { get; }
}

public sealed class __COMPONENT__MosaicDragSource : ContentControl
{
    public __COMPONENT__MosaicDragSource()
    {
        CanDrag = true;
        IsTabStop = true;
        DragStarting += OnDragStarting;
        DropCompleted += OnDropCompleted;
        KeyDown += OnKeyDown;
    }

    public string DragKey
    {
        get => (string)GetValue(DragKeyProperty);
        set => SetValue(DragKeyProperty, value);
    }
    public static readonly DependencyProperty DragKeyProperty =
        DependencyProperty.Register(nameof(DragKey), typeof(string), typeof(__COMPONENT__MosaicDragSource), new PropertyMetadata(string.Empty));

    public string DragKind
    {
        get => (string)GetValue(DragKindProperty);
        set => SetValue(DragKindProperty, value);
    }
    public static readonly DependencyProperty DragKindProperty =
        DependencyProperty.Register(nameof(DragKind), typeof(string), typeof(__COMPONENT__MosaicDragSource), new PropertyMetadata(string.Empty));

    public string DragLabel
    {
        get => (string)GetValue(DragLabelProperty);
        set => SetValue(DragLabelProperty, value);
    }
    public static readonly DependencyProperty DragLabelProperty =
        DependencyProperty.Register(nameof(DragLabel), typeof(string), typeof(__COMPONENT__MosaicDragSource), new PropertyMetadata(string.Empty));

    public bool DragDisabled
    {
        get => (bool)GetValue(DragDisabledProperty);
        set => SetValue(DragDisabledProperty, value);
    }
    public static readonly DependencyProperty DragDisabledProperty =
        DependencyProperty.Register(
            nameof(DragDisabled),
            typeof(bool),
            typeof(__COMPONENT__MosaicDragSource),
            new PropertyMetadata(false, OnDragDisabledChanged));

    public event EventHandler<__COMPONENT__MosaicDragSourceEventArgs>? MosaicDragStarted;
    public event EventHandler<__COMPONENT__MosaicDragEndEventArgs>? MosaicDragEnded;

    internal __COMPONENT__MosaicDragScope Scope => __COMPONENT__MosaicDragRuntime.ScopeFor(this);

    internal void RaiseStarted() =>
        MosaicDragStarted?.Invoke(this, new __COMPONENT__MosaicDragSourceEventArgs(DragKey, DragKind));

    internal void RaiseEnded(bool dropped) =>
        MosaicDragEnded?.Invoke(this, new __COMPONENT__MosaicDragEndEventArgs(DragKey, DragKind, dropped));

    private static void OnDragDisabledChanged(DependencyObject sender, DependencyPropertyChangedEventArgs args)
    {
        var source = (__COMPONENT__MosaicDragSource)sender;
        var disabled = args.NewValue is true;
        source.CanDrag = !disabled;
        source.IsTabStop = !disabled;
    }

    private void OnDragStarting(UIElement sender, DragStartingEventArgs args)
    {
        if (DragDisabled)
        {
            args.Cancel = true;
            return;
        }
        Scope.BeginPointer(this);
        args.Data.RequestedOperation = DataPackageOperation.Move;
        args.Data.SetData(__COMPONENT__MosaicDragRuntime.Format, DragKey);
    }

    private void OnDropCompleted(UIElement sender, DropCompletedEventArgs args) =>
        Scope.FinishPointer(this);

    private void OnKeyDown(object sender, KeyRoutedEventArgs args)
    {
        bool handled;
        switch (args.Key)
        {
            case VirtualKey.Escape:
                handled = Scope.Cancel();
                break;
            case VirtualKey.Space:
            case VirtualKey.Enter:
                handled = !DragDisabled && Scope.ToggleKeyboard(this);
                break;
            case VirtualKey.Down:
                handled = Scope.StepKeyboard(1);
                break;
            case VirtualKey.Up:
                handled = Scope.StepKeyboard(-1);
                break;
            case VirtualKey.Right:
                handled = Scope.StepKeyboard(FlowDirection == FlowDirection.RightToLeft ? -1 : 1);
                break;
            case VirtualKey.Left:
                handled = Scope.StepKeyboard(FlowDirection == FlowDirection.RightToLeft ? 1 : -1);
                break;
            default:
                handled = false;
                break;
        }
        if (handled) args.Handled = true;
    }

    protected override AutomationPeer OnCreateAutomationPeer() =>
        new __COMPONENT__MosaicDragSourceAutomationPeer(this);
}

public sealed class __COMPONENT__MosaicDropTarget : ContentControl
{
    private __COMPONENT__MosaicDragScope? _scope;
    private string _pointerPosition = "into";

    public __COMPONENT__MosaicDropTarget()
    {
        AllowDrop = true;
        IsTabStop = false;
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
        DragEnter += OnDragEnter;
        DragLeave += OnDragLeave;
        DragOver += OnDragOver;
        Drop += OnDrop;
    }

    public string DropKey
    {
        get => (string)GetValue(DropKeyProperty);
        set => SetValue(DropKeyProperty, value);
    }
    public static readonly DependencyProperty DropKeyProperty =
        DependencyProperty.Register(nameof(DropKey), typeof(string), typeof(__COMPONENT__MosaicDropTarget), new PropertyMetadata(string.Empty));

    public object? Accepts
    {
        get => GetValue(AcceptsProperty);
        set => SetValue(AcceptsProperty, value);
    }
    public static readonly DependencyProperty AcceptsProperty =
        DependencyProperty.Register(nameof(Accepts), typeof(object), typeof(__COMPONENT__MosaicDropTarget), new PropertyMetadata(null));

    public bool DropDisabled
    {
        get => (bool)GetValue(DropDisabledProperty);
        set => SetValue(DropDisabledProperty, value);
    }
    public static readonly DependencyProperty DropDisabledProperty =
        DependencyProperty.Register(nameof(DropDisabled), typeof(bool), typeof(__COMPONENT__MosaicDropTarget), new PropertyMetadata(false));

    public event EventHandler<__COMPONENT__MosaicDragSourceEventArgs>? MosaicDragEntered;
    public event EventHandler<__COMPONENT__MosaicDragSourceEventArgs>? MosaicDragLeft;
    public event EventHandler<__COMPONENT__MosaicDropEventArgs>? MosaicDropHovered;
    public event EventHandler<__COMPONENT__MosaicDropEventArgs>? MosaicDropped;

    internal __COMPONENT__MosaicDragScope Scope =>
        _scope ??= __COMPONENT__MosaicDragRuntime.ScopeFor(this);

    internal bool AcceptsSource(__COMPONENT__MosaicDragSource source)
    {
        if (DropDisabled || !ReferenceEquals(source.Scope, Scope)) return false;
        if (Accepts is null) return true;
        if (Accepts is IEnumerable<string> kinds)
        {
            foreach (var kind in kinds)
            {
                if (string.Equals(kind, source.DragKind, StringComparison.Ordinal)) return true;
            }
        }
        return false;
    }

    internal void Enter(__COMPONENT__MosaicDragSource source)
    {
        if (AcceptsSource(source))
            MosaicDragEntered?.Invoke(this, new __COMPONENT__MosaicDragSourceEventArgs(source.DragKey, source.DragKind));
    }

    internal void Leave(__COMPONENT__MosaicDragSource source)
    {
        if (ReferenceEquals(source.Scope, Scope))
            MosaicDragLeft?.Invoke(this, new __COMPONENT__MosaicDragSourceEventArgs(source.DragKey, source.DragKind));
    }

    internal void Hover(__COMPONENT__MosaicDragSource source, string position)
    {
        if (!AcceptsSource(source)) return;
        _pointerPosition = position;
        MosaicDropHovered?.Invoke(
            this,
            new __COMPONENT__MosaicDropEventArgs(source.DragKey, source.DragKind, DropKey, position));
    }

    internal bool Accept(__COMPONENT__MosaicDragSource source, string position, bool keyboard)
    {
        if (!AcceptsSource(source)) return false;
        var acceptedPosition = keyboard ? "into" : position;
        Scope.MarkAccepted(source);
        MosaicDropped?.Invoke(
            this,
            new __COMPONENT__MosaicDropEventArgs(source.DragKey, source.DragKind, DropKey, acceptedPosition));
        Scope.Announce(this, $"Dropped {source.DragLabel} on {DropKey}.");
        return true;
    }

    private void OnLoaded(object sender, RoutedEventArgs args) => Scope.Register(this);

    private void OnUnloaded(object sender, RoutedEventArgs args)
    {
        _scope?.Unregister(this);
        _scope = null;
    }

    private bool TryGetPointerSource(DragEventArgs args, out __COMPONENT__MosaicDragSource source)
    {
        source = null!;
        if (!args.DataView.Contains(__COMPONENT__MosaicDragRuntime.Format)) return false;
        var active = Scope.ActivePointerSource;
        if (active is null || !AcceptsSource(active)) return false;
        source = active;
        return true;
    }

    private string PositionFor(DragEventArgs args)
    {
        if (ActualHeight <= 0) return "into";
        var ratio = args.GetPosition(this).Y / ActualHeight;
        return ratio < (1.0 / 3.0) ? "before" : ratio > (2.0 / 3.0) ? "after" : "into";
    }

    private void OnDragEnter(object sender, DragEventArgs args)
    {
        if (!TryGetPointerSource(args, out var source)) return;
        args.AcceptedOperation = DataPackageOperation.Move;
        args.Handled = true;
        Enter(source);
    }

    private void OnDragLeave(object sender, DragEventArgs args)
    {
        var source = Scope.ActivePointerSource;
        if (source is not null) Leave(source);
    }

    private void OnDragOver(object sender, DragEventArgs args)
    {
        if (!TryGetPointerSource(args, out var source)) return;
        args.AcceptedOperation = DataPackageOperation.Move;
        args.Handled = true;
        Hover(source, PositionFor(args));
    }

    private void OnDrop(object sender, DragEventArgs args)
    {
        if (!TryGetPointerSource(args, out var source)) return;
        var position = PositionFor(args);
        if (!Accept(source, position, keyboard: false)) return;
        args.AcceptedOperation = DataPackageOperation.Move;
        args.Handled = true;
    }

    protected override AutomationPeer OnCreateAutomationPeer() =>
        new __COMPONENT__MosaicDropTargetAutomationPeer(this);
}

internal sealed class __COMPONENT__MosaicDragScope
{
    private readonly List<__COMPONENT__MosaicDropTarget> _targets = new();
    private __COMPONENT__MosaicDragSource? _activeSource;
    private __COMPONENT__MosaicDropTarget? _keyboardTarget;
    private bool _keyboard;
    private bool _acceptedPointer;

    internal __COMPONENT__MosaicDragSource? ActivePointerSource =>
        _keyboard ? null : _activeSource;

    internal void Register(__COMPONENT__MosaicDropTarget target)
    {
        if (!_targets.Contains(target)) _targets.Add(target);
    }

    internal void Unregister(__COMPONENT__MosaicDropTarget target)
    {
        _targets.Remove(target);
        if (ReferenceEquals(_keyboardTarget, target)) _keyboardTarget = null;
    }

    internal void BeginPointer(__COMPONENT__MosaicDragSource source)
    {
        if (_activeSource is not null) Cancel();
        _activeSource = source;
        _keyboard = false;
        _acceptedPointer = false;
        source.RaiseStarted();
        Announce(source, $"Grabbed {source.DragLabel}.");
    }

    internal void FinishPointer(__COMPONENT__MosaicDragSource source)
    {
        if (!ReferenceEquals(_activeSource, source) || _keyboard) return;
        var dropped = _acceptedPointer;
        Clear();
        if (!dropped) Announce(source, "Cancelled drag.");
        source.RaiseEnded(dropped);
    }

    internal bool ToggleKeyboard(__COMPONENT__MosaicDragSource source)
    {
        if (_activeSource is null)
        {
            _activeSource = source;
            _keyboard = true;
            _acceptedPointer = false;
            _keyboardTarget = null;
            source.RaiseStarted();
            Announce(source, $"Grabbed {source.DragLabel}. Use arrow keys to choose a target, then press Space or Enter to drop.");
            return true;
        }
        if (!ReferenceEquals(_activeSource, source) || !_keyboard) return false;
        if (_keyboardTarget is null || !_keyboardTarget.Accept(source, "into", keyboard: true))
            return Cancel();
        Clear();
        source.RaiseEnded(true);
        return true;
    }

    internal bool StepKeyboard(int delta)
    {
        var source = _activeSource;
        if (source is null || !_keyboard) return false;
        var eligible = new List<__COMPONENT__MosaicDropTarget>();
        foreach (var target in _targets)
        {
            if (target.AcceptsSource(source)) eligible.Add(target);
        }
        if (eligible.Count == 0)
        {
            Announce(source, "No available drop targets.");
            return true;
        }
        var current = _keyboardTarget is null ? -1 : eligible.IndexOf(_keyboardTarget);
        var nextIndex = (current + delta) % eligible.Count;
        if (nextIndex < 0) nextIndex += eligible.Count;
        var next = eligible[nextIndex];
        if (!ReferenceEquals(_keyboardTarget, next))
        {
            _keyboardTarget?.Leave(source);
            next.Enter(source);
        }
        next.Hover(source, "into");
        _keyboardTarget = next;
        Announce(next, $"Move to {next.DropKey}, position {nextIndex + 1} of {eligible.Count}.");
        return true;
    }

    internal bool Cancel()
    {
        var source = _activeSource;
        if (source is null) return false;
        if (_keyboard) _keyboardTarget?.Leave(source);
        Clear();
        Announce(source, "Cancelled drag.");
        source.RaiseEnded(false);
        return true;
    }

    internal void MarkAccepted(__COMPONENT__MosaicDragSource source)
    {
        if (ReferenceEquals(_activeSource, source) && !_keyboard) _acceptedPointer = true;
    }

    internal void Announce(FrameworkElement anchor, string message)
    {
        AutomationProperties.SetHelpText(anchor, message);
        var peer = FrameworkElementAutomationPeer.CreatePeerForElement(anchor);
        peer?.RaiseNotificationEvent(
            AutomationNotificationKind.ActionCompleted,
            AutomationNotificationProcessing.MostRecent,
            message,
            "MosaicDrag");
    }

    private void Clear()
    {
        _activeSource = null;
        _keyboardTarget = null;
        _keyboard = false;
        _acceptedPointer = false;
    }
}

internal static class __COMPONENT__MosaicDragRuntime
{
    internal const string Format = "application/x-mosaic-drag-v1";
    private static readonly ConditionalWeakTable<__COMPONENT__, __COMPONENT__MosaicDragScope> Scopes = new();

    internal static __COMPONENT__MosaicDragScope ScopeFor(DependencyObject element)
    {
        DependencyObject? current = element;
        while (current is not null)
        {
            if (current is __COMPONENT__ owner)
                return Scopes.GetValue(owner, _ => new __COMPONENT__MosaicDragScope());
            current = VisualTreeHelper.GetParent(current);
        }
        throw new InvalidOperationException("Mosaic drag primitives require a component drag scope.");
    }
}

internal sealed class __COMPONENT__MosaicDragSourceAutomationPeer : FrameworkElementAutomationPeer
{
    internal __COMPONENT__MosaicDragSourceAutomationPeer(__COMPONENT__MosaicDragSource owner) : base(owner) { }
    protected override AutomationControlType GetAutomationControlTypeCore() => AutomationControlType.Button;
    protected override string GetClassNameCore() => nameof(__COMPONENT__MosaicDragSource);
    protected override string GetNameCore()
    {
        var name = base.GetNameCore();
        return string.IsNullOrEmpty(name) ? ((__COMPONENT__MosaicDragSource)Owner).DragLabel : name;
    }
}

internal sealed class __COMPONENT__MosaicDropTargetAutomationPeer : FrameworkElementAutomationPeer
{
    internal __COMPONENT__MosaicDropTargetAutomationPeer(__COMPONENT__MosaicDropTarget owner) : base(owner) { }
    protected override AutomationControlType GetAutomationControlTypeCore() => AutomationControlType.Group;
    protected override string GetClassNameCore() => nameof(__COMPONENT__MosaicDropTarget);
    protected override string GetNameCore()
    {
        var name = base.GetNameCore();
        return string.IsNullOrEmpty(name) ? ((__COMPONENT__MosaicDropTarget)Owner).DropKey : name;
    }
}
"#
    .replace("__COMPONENT__", component)
}

fn emit_native_slider_support_source(component: &str) -> String {
    r#"

public sealed class __COMPONENT__MosaicSliderValueEventArgs : EventArgs
{
    internal __COMPONENT__MosaicSliderValueEventArgs(double newValue) => NewValue = newValue;
    public double NewValue { get; }
}

/// <summary>
/// Native WinUI Slider with Mosaic's change-versus-commit event lifecycle.
/// The inherited automation peer continues to expose the RangeValue pattern.
/// </summary>
public sealed class __COMPONENT__MosaicSlider : Slider
{
    private bool _pointerActive;
    private bool _keyboardActive;
    private bool _dirty;

    public __COMPONENT__MosaicSlider()
    {
        ValueChanged += OnMosaicValueChanged;
        LostFocus += OnMosaicLostFocus;
        Loaded += (_, _) => ConfigureStep();
        RegisterPropertyChangedCallback(MinimumProperty, (_, _) => ConfigureStep());
        RegisterPropertyChangedCallback(MaximumProperty, (_, _) => ConfigureStep());
        AddHandler(PointerPressedEvent, new PointerEventHandler(OnMosaicPointerPressed), true);
        AddHandler(PointerReleasedEvent, new PointerEventHandler(OnMosaicPointerReleased), true);
        AddHandler(PointerCaptureLostEvent, new PointerEventHandler(OnMosaicPointerCaptureLost), true);
    }

    public double MosaicStep
    {
        get => (double)GetValue(MosaicStepProperty);
        set => SetValue(MosaicStepProperty, value);
    }

    public static readonly DependencyProperty MosaicStepProperty =
        DependencyProperty.Register(
            nameof(MosaicStep),
            typeof(double),
            typeof(__COMPONENT__MosaicSlider),
            new PropertyMetadata(1.0, OnMosaicStepChanged));

    public event EventHandler<__COMPONENT__MosaicSliderValueEventArgs>? MosaicValueChanged;
    public event EventHandler<__COMPONENT__MosaicSliderValueEventArgs>? MosaicValueCommitted;

    private static void OnMosaicStepChanged(
        DependencyObject sender,
        DependencyPropertyChangedEventArgs args) =>
        ((__COMPONENT__MosaicSlider)sender).ConfigureStep();

    private void ConfigureStep()
    {
        var span = Math.Abs(Maximum - Minimum);
        if (MosaicStep > 0.0)
        {
            StepFrequency = MosaicStep;
            SmallChange = MosaicStep;
            return;
        }

        // WinUI Slider always snaps to a positive StepFrequency. One million
        // native stops across the current range is comfortably below a physical
        // display pixel, preserving effectively continuous pointer/touch input.
        StepFrequency = Math.Max(span / 1_000_000.0, 0.000000001);
        SmallChange = Math.Max(span / 100.0, StepFrequency);
    }

    private void OnMosaicValueChanged(
        object sender,
        Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs args)
    {
        if (!IsLoaded) return;
        if (!_pointerActive && !_keyboardActive && FocusState == FocusState.Unfocused) return;
        _dirty = true;
        MosaicValueChanged?.Invoke(this, new __COMPONENT__MosaicSliderValueEventArgs(args.NewValue));
    }

    private void OnMosaicPointerPressed(object sender, PointerRoutedEventArgs args) =>
        _pointerActive = true;

    private void OnMosaicPointerReleased(object sender, PointerRoutedEventArgs args)
    {
        if (!_pointerActive) return;
        _pointerActive = false;
        Commit(Value, force: true);
    }

    private void OnMosaicPointerCaptureLost(object sender, PointerRoutedEventArgs args)
    {
        if (!_pointerActive) return;
        _pointerActive = false;
        Commit(Value, force: true);
    }

    protected override void OnKeyDown(KeyRoutedEventArgs args)
    {
        if (IsAdjustmentKey(args.Key)) _keyboardActive = true;
        base.OnKeyDown(args);
    }

    protected override void OnKeyUp(KeyRoutedEventArgs args)
    {
        base.OnKeyUp(args);
        if (!IsAdjustmentKey(args.Key) || !_keyboardActive) return;
        _keyboardActive = false;
        Commit(Value, force: true);
    }

    private void OnMosaicLostFocus(object sender, RoutedEventArgs args)
    {
        _pointerActive = false;
        _keyboardActive = false;
        Commit(Value, force: false);
    }

    private void Commit(double value, bool force)
    {
        if (!force && !_dirty) return;
        _dirty = false;
        MosaicValueCommitted?.Invoke(this, new __COMPONENT__MosaicSliderValueEventArgs(value));
    }

    private static bool IsAdjustmentKey(VirtualKey key) => key is
        VirtualKey.Left or VirtualKey.Right or VirtualKey.Up or VirtualKey.Down or
        VirtualKey.Home or VirtualKey.End or VirtualKey.PageUp or VirtualKey.PageDown;
}
"#
    .replace("__COMPONENT__", component)
}

fn emit_dependency_property(
    slot: &SlotDecl,
    component: &str,
    ctx: &EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    // Fix A4: use the alias if this slot's PascalCased name would
    // collide with an inherited property on the chosen base class
    // (e.g. ContentDialog.Title).
    let pascal = ctx.slot_property_name(&slot.name);
    if !is_safe_identifier(&pascal) {
        return Err(PipelineEmitError::UnsafeSlotName(pascal));
    }
    let csharp_type = slot_type_to_csharp(&slot.r#type)?;
    let dependent_projections = row_projections_depending_on(ctx, &pascal);
    let changed_callback = format!("OnMosaic{pascal}RowProjectionInputChanged");

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
        "        DependencyProperty.Register(nameof({pascal}), typeof({csharp_type}), typeof({component}), new PropertyMetadata(default({csharp_type}){}));",
        if dependent_projections.is_empty() {
            String::new()
        } else {
            format!(", {changed_callback}")
        }
    )
    .unwrap();
    if !dependent_projections.is_empty() {
        writeln!(out).unwrap();
        writeln!(
            out,
            "    private static void {changed_callback}(DependencyObject d, DependencyPropertyChangedEventArgs _)"
        )
        .unwrap();
        writeln!(out, "    {{").unwrap();
        writeln!(out, "        var control = ({component})d;").unwrap();
        for property_name in dependent_projections {
            writeln!(
                out,
                "        control.NotifyRowProjectionChanged(nameof({property_name}));"
            )
            .unwrap();
        }
        writeln!(out, "    }}").unwrap();
    }
    Ok(out)
}

fn row_projections_depending_on<'a>(ctx: &'a EmitContext<'_>, slot_path: &str) -> Vec<&'a str> {
    let mut properties = Vec::new();
    for projection in &ctx.row_projections {
        let depends_on_slot = projection
            .dependency_paths
            .iter()
            .any(|dependency| dependency == slot_path)
            || projection.selected_index_path.as_deref() == Some(slot_path);
        if depends_on_slot
            && !properties
                .iter()
                .any(|property| *property == projection.property_name)
        {
            properties.push(projection.property_name.as_str());
        }
    }
    properties
}

/// Translate a mosmodel slot type to its C# property type per spec Â§8.
fn emit_row_projection_property(projection: &RowProjection) -> String {
    let property_name = &projection.property_name;
    let source_path = &projection.source_path;
    let vm_class = &projection.vm_class;

    let mut args = vec![projection.owner_expr.clone(), "source[i]".to_string()];
    if projection.has_index {
        args.push("i".to_string());
    }
    if projection.has_width {
        args.push(
            projection
                .width_source_path
                .as_ref()
                .map(|path| format!("{path} is {{ }} widths && i < widths.Count ? widths[i] : 0"))
                .unwrap_or_else(|| "0".to_string()),
        );
    }
    if let Some(selected_index_path) = &projection.selected_index_path {
        args.push(format!("i == {selected_index_path}"));
    }
    args.extend(projection.capture_args.iter().cloned());
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
    let is_nested = !ctx.for_scope.is_empty();

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
                let pascal = ctx.slot_property_name(slot);
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
    let vm_class = ctx.allocate_for_vm_class(as_name);
    let element_property = kebab_to_pascal_case(as_name);
    let has_index = index_name.is_some();
    let capture_specs = row_vm_capture_specs(&ctx.for_scope, &element_property, has_index);
    let captures = capture_specs
        .iter()
        .map(|(capture, _)| capture.clone())
        .collect();
    let capture_args = capture_specs
        .into_iter()
        .map(|(_, argument)| argument)
        .collect::<Vec<_>>();
    let width_component_source = is_cell_loop.then(|| {
        ctx.slot_types
            .keys()
            .filter(|slot| slot.ends_with("column-widths"))
            .min()
            .map(|slot| ctx.slot_property_name(slot))
    });
    let width_component_source = width_component_source.flatten();

    let vm = RowVm {
        class_name: vm_class.clone(),
        element_property: element_property.clone(),
        element_type: element_type.clone(),
        has_index,
        // GROUP C: only the per-column cell loop's VM carries `Width`.
        has_width: is_cell_loop,
        has_is_selected: false,
        helper_bindings: Vec::new(),
        captures,
        nested_projections: Vec::new(),
    };
    if !ctx.row_vms.iter().any(|v| v.class_name == vm.class_name) {
        ctx.row_vms.push(vm);
    }

    let projection_source = if is_nested {
        Some(
            slot_backed_items_source
                .as_ref()
                .map(|source| format!("Owner.{source}"))
                .unwrap_or_else(|| items_path.clone()),
        )
    } else {
        slot_backed_items_source.clone()
    };
    let projection_property = projection_source.as_ref().map(|source| {
        let prop = format!("{}Rows", vm_class.replace('_', ""));
        let projection = RowProjection {
            property_name: prop.clone(),
            source_path: source.clone(),
            dependency_paths: vec![source.clone()],
            vm_class: vm_class.clone(),
            has_index,
            has_width: is_cell_loop,
            width_source_path: width_component_source.as_ref().map(|path| {
                if is_nested {
                    format!("Owner.{path}")
                } else {
                    path.clone()
                }
            }),
            selected_index_path: None,
            owner_expr: if is_nested { "Owner" } else { "this" }.to_string(),
            capture_args: capture_args.clone(),
        };
        if is_nested {
            let parent_vm_class = &ctx.for_scope.last().expect("nested For parent").vm_class;
            let parent_vm = ctx
                .row_vms
                .iter_mut()
                .find(|vm| vm.class_name == *parent_vm_class)
                .expect("nested For parent row VM");
            if !parent_vm
                .nested_projections
                .iter()
                .any(|existing| existing.property_name == prop)
            {
                parent_vm.nested_projections.push(projection);
            }
            if let (Some(component_source), Some(top_projection_name)) = (
                slot_backed_items_source.as_ref(),
                ctx.for_scope
                    .first()
                    .and_then(|binding| binding.projection_property.as_ref()),
            ) {
                if let Some(top_projection) = ctx
                    .row_projections
                    .iter_mut()
                    .find(|existing| existing.property_name == *top_projection_name)
                {
                    if !top_projection
                        .dependency_paths
                        .iter()
                        .any(|dependency| dependency == component_source)
                    {
                        top_projection
                            .dependency_paths
                            .push(component_source.clone());
                    }
                }
            }
            if let (Some(width_source), Some(top_projection_name)) = (
                width_component_source.as_ref(),
                ctx.for_scope
                    .first()
                    .and_then(|binding| binding.projection_property.as_ref()),
            ) {
                if let Some(top_projection) = ctx
                    .row_projections
                    .iter_mut()
                    .find(|existing| existing.property_name == *top_projection_name)
                {
                    if !top_projection
                        .dependency_paths
                        .iter()
                        .any(|dependency| dependency == width_source)
                    {
                        top_projection.dependency_paths.push(width_source.clone());
                    }
                }
            }
        } else if !ctx
            .row_projections
            .iter()
            .any(|existing| existing.property_name == prop)
        {
            ctx.row_projections.push(projection);
        }
        prop
    });

    // -- 4. Push the binding and a namescope-local visual-state collector,
    //       walk the body, then pop both. --
    ctx.for_scope.push(ForBinding {
        as_name: as_name.to_string(),
        index_name: index_name.map(String::from),
        element_type,
        vm_class: vm_class.clone(),
        projection_property: if is_nested {
            None
        } else {
            projection_property.clone()
        },
    });
    let native_table_entry = ctx.native_table.as_ref().map(|table| {
        (
            table.role,
            table.header_helper.clone(),
            table.cell_name_helper.clone(),
            table.for_depth,
        )
    });
    if let Some(table) = ctx.native_table.as_mut() {
        table.for_depth += 1;
    }
    if let Some((NativeTableRole::Body, header_helper, cell_name_helper, _)) =
        native_table_entry.as_ref()
    {
        if is_cell_loop {
            let row_index_property = ctx
                .for_scope
                .iter()
                .rev()
                .nth(1)
                .and_then(|binding| binding.index_name.as_deref())
                .map(kebab_to_pascal_case)
                .unwrap_or_else(|| "Index".to_string());
            if let Some(vm) = ctx.row_vms.iter_mut().find(|vm| vm.class_name == vm_class) {
                vm.helper_bindings.push(RowVmHelperBinding {
                    property_name: "MosaicTableHeader".to_string(),
                    return_type: "string".to_string(),
                    owner_call: format!("Owner.{header_helper}(Index)"),
                });
                vm.helper_bindings.push(RowVmHelperBinding {
                    property_name: "MosaicTableName".to_string(),
                    return_type: "string".to_string(),
                    owner_call: format!(
                        "Owner.{cell_name_helper}(MosaicTableHeader, {element_property}, {row_index_property})"
                    ),
                });
            }
        }
    }
    ctx.template_visual_state_groups.push(Vec::new());
    let body_result =
        emit_xaml_single_content_children(&node.children, indent + 12, part_styles, ctx);
    let template_visual_state_groups = ctx
        .template_visual_state_groups
        .pop()
        .expect("For template visual-state collector");
    if let Some(table) = ctx.native_table.as_mut() {
        table.for_depth = table.for_depth.saturating_sub(1);
    }
    ctx.for_scope.pop();
    let mut body = body_result?;

    // GROUP C: bind the fixed per-column width onto the rendered cell.
    // The cell element is the first opening tag of this loop's body â€”
    // either a kernel `<Border â€¦>` (when Cell.mll resolved inline) or
    // a component reference `<grid:Cell â€¦>` (both are FrameworkElements
    // and so have a `Width` property). Inject `Width="{x:Bind Width}"`
    // into that opening tag so the column renders at the colgroup's
    // fixed pixel width regardless of cell content.
    if is_cell_loop {
        body = inject_attr_into_first_element(&body, "Width=\"{x:Bind Width, Mode=OneWay}\"");
    }

    if !template_visual_state_groups.is_empty() {
        let pad = " ".repeat(indent + 12);
        let mut wrapped = String::new();
        writeln!(wrapped, "{pad}<Grid>").unwrap();
        wrapped.push_str(&emit_visual_state_groups(
            &template_visual_state_groups,
            indent + 16,
        ));
        wrapped.push_str(&indent_xaml_fragment(&body, 4));
        writeln!(wrapped, "{pad}</Grid>").unwrap();
        body = wrapped;
    }

    if let Some((role, _, _, entry_depth)) = native_table_entry {
        let wrapper = match role {
            NativeTableRole::Header if entry_depth == 0 => Some(format!(
                "local:{}MosaicTableHeaderCell Column=\"{{x:Bind Index, Mode=OneWay}}\" Header=\"{{x:Bind {element_property}, Mode=OneWay}}\" AutomationProperties.Name=\"{{x:Bind {element_property}, Mode=OneWay}}\"",
                ctx.component_name
            )),
            NativeTableRole::Body if is_cell_loop => {
                let row_index_property = ctx
                    .for_scope
                    .last()
                    .and_then(|binding| binding.index_name.as_deref())
                    .map(kebab_to_pascal_case)
                    .unwrap_or_else(|| "Index".to_string());
                Some(format!(
                    "local:{}MosaicTableCell Row=\"{{x:Bind {row_index_property}, Mode=OneWay}}\" Column=\"{{x:Bind Index, Mode=OneWay}}\" Header=\"{{x:Bind MosaicTableHeader, Mode=OneWay}}\" Value=\"{{x:Bind {element_property}, Mode=OneWay}}\" AutomationProperties.Name=\"{{x:Bind MosaicTableName, Mode=OneWay}}\"",
                    ctx.component_name
                ))
            }
            _ => None,
        };
        if let Some(wrapper) = wrapper {
            let wrapper_pad = " ".repeat(indent + 12);
            let mut wrapped = String::new();
            writeln!(wrapped, "{wrapper_pad}<{wrapper}>").unwrap();
            wrapped.push_str(&indent_xaml_fragment(&body, 4));
            writeln!(
                wrapped,
                "{wrapper_pad}</{}>",
                wrapper.split_whitespace().next().unwrap()
            )
            .unwrap();
            body = wrapped;
        }
    }

    // -- 5. Assemble the XAML --
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let pad3 = " ".repeat(indent + 8);
    let style = part_style_attr(node, part_styles);
    let items_source = projection_property.as_deref().unwrap_or(&items_path);
    // Always OneWay. This was previously conditional on there being a
    // projection property, which meant a repeater bound directly to a slot
    // got the x:Bind default — OneTime — and so never re-rendered when the
    // list changed. The distinction was never load-bearing: both forms read
    // a collection that the host reassigns wholesale on every prop update.
    let mut out = String::new();
    writeln!(
        out,
        "{pad}<ItemsRepeater ItemsSource=\"{{x:Bind {items_source}, Mode=OneWay}}\"{style}>"
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

fn row_vm_capture_specs(
    scope: &[ForBinding],
    own_element_property: &str,
    own_has_index: bool,
) -> Vec<(RowVmCapture, String)> {
    let mut candidates = Vec::new();
    for (position, binding) in scope.iter().enumerate() {
        let element_property = kebab_to_pascal_case(&binding.as_name);
        candidates.push((
            RowVmCapture {
                property_name: element_property.clone(),
                property_type: binding.element_type.clone(),
            },
            element_property,
        ));
        if let Some(index_name) = &binding.index_name {
            let index_property = kebab_to_pascal_case(index_name);
            let argument = if position + 1 == scope.len() {
                "Index".to_string()
            } else {
                index_property.clone()
            };
            candidates.push((
                RowVmCapture {
                    property_name: index_property,
                    property_type: "int".to_string(),
                },
                argument,
            ));
        }
    }

    let own_index_collision = own_has_index.then_some("Index");
    candidates
        .iter()
        .enumerate()
        .filter(|(position, (capture, _))| {
            capture.property_name != own_element_property
                && own_index_collision != Some(capture.property_name.as_str())
                && !candidates[position + 1..]
                    .iter()
                    .any(|(later, _)| later.property_name == capture.property_name)
        })
        .map(|(_, candidate)| candidate.clone())
        .collect()
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
    // Extra attribute(s) (e.g. `Grid.Column="2" VerticalAlignment="Center"`)
    // spliced onto BOTH branches' `<ContentControl>` tags. Used by
    // `emit_flex_grid` (mosaic-emit-xaml.md §3.1): an If/Else pair is one
    // logical grid slot, and both mutually-exclusive branches must carry
    // the same attached property to land in that slot.
    extra_attr: Option<&str>,
) -> Result<String, PipelineEmitError> {
    let extra = extra_attr.map(|a| format!(" {a}")).unwrap_or_default();
    // -- 1. Lower the `when:` expression --
    let when_path = match find_prop_value(if_node, "when") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let property = ctx.slot_property_name(slot);
            if !is_safe_identifier(&property) {
                return Err(PipelineEmitError::UnsafeSlotName(property));
            }
            ctx.slot_xbind_path(slot)
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
        "{pad}<ContentControl Visibility=\"{{x:Bind {when_path}, Converter={{StaticResource BoolToVisibilityConverter}}, Mode=OneWay}}\"{extra}>"
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
            "{pad}<ContentControl Visibility=\"{{x:Bind {when_path}, Converter={{StaticResource BoolToVisibilityConverter}}, ConverterParameter=invert, Mode=OneWay}}\"{extra}>"
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

/// The generated converter resources required by this component. Added
/// exactly once beneath the root resources tag.
fn emit_converter_resource_block(
    indent: usize,
    resources_tag: &str,
    needs_bool_to_vis: bool,
    needs_focus_state: bool,
) -> String {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let mut out = String::new();
    writeln!(out, "{pad}<{resources_tag}>").unwrap();
    if needs_bool_to_vis {
        writeln!(
            out,
            "{pad2}<local:BoolToVisibilityConverter x:Key=\"BoolToVisibilityConverter\"/>"
        )
        .unwrap();
    }
    if needs_focus_state {
        writeln!(
            out,
            "{pad2}<local:FocusStateToBoolConverter x:Key=\"FocusStateToBoolConverter\"/>"
        )
        .unwrap();
    }
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
                 var b = value switch\n        \
                 {{\n            \
                     null => false,\n            \
                     bool x => x,\n            \
                     double number => number != 0,\n            \
                     float number => number != 0,\n            \
                     int number => number != 0,\n            \
                     string text => text.Length != 0,\n            \
                     System.Collections.ICollection collection => collection.Count != 0,\n            \
                     _ => true,\n        \
                 }};\n        \
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

/// C# source for the converter that activates UI15's built-in `focused`
/// state from WinUI's native `Control.FocusState` enum.
fn emit_focus_state_to_bool_converter_source(namespace: &str) -> String {
    format!(
        "// Auto-generated by mosaic-emit-xaml. Do not edit.\n\
         //\n\
         // Native WinUI focus state → Mosaic focused-state activation.\n\
         using System;\n\
         using Microsoft.UI.Xaml;\n\
         using Microsoft.UI.Xaml.Data;\n\
         \n\
         namespace {namespace};\n\
         \n\
         public sealed class FocusStateToBoolConverter : IValueConverter\n\
         {{\n    \
             public object Convert(object value, Type targetType, object parameter, string language)\n    \
             {{\n        \
                 if (value is FocusState state) return state != FocusState.Unfocused;\n        \
                 return DependencyProperty.UnsetValue;\n    \
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
fn emit_row_vm_source(component: &str, vm: &RowVm, options: &EmitOptions) -> String {
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
    let capture_fields = vm
        .captures
        .iter()
        .map(|capture| format!(", {} {}", capture.property_type, capture.property_name))
        .collect::<String>();
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
        writeln!(
            out,
            "/// <remarks>\n\
             /// GROUP C â€” fixed per-column widths. This VM carries a `Width`\n\
             /// (double) the cell element binds via `Width=\"{{x:Bind Width, Mode=OneWay}}\"`.\n\
             /// The enclosing generated row projection zips each cell index\n\
             /// with the component's authored `column-widths` slot.\n\
             /// </remarks>"
        )
        .unwrap();
    }
    write!(
        out,
        "public sealed record {class_name}({component} Owner, {element_type} {element_property}{index_field}{width_field}{selected_field}{capture_fields})"
    )
    .unwrap();
    if vm.helper_bindings.is_empty() && vm.nested_projections.is_empty() {
        writeln!(out, ";").unwrap();
    } else {
        writeln!(out).unwrap();
        writeln!(out, "{{").unwrap();
        for binding in &vm.helper_bindings {
            writeln!(
                out,
                "    public {} {} => {};",
                binding.return_type, binding.property_name, binding.owner_call
            )
            .unwrap();
        }
        for projection in &vm.nested_projections {
            writeln!(out).unwrap();
            out.push_str(&emit_row_projection_property(projection));
        }
        writeln!(out, "}}").unwrap();
    }
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
        global_json: emit_global_json(),
        csproj: emit_csproj(name, options),
        app_xaml: emit_app_xaml(options),
        app_xaml_cs: emit_app_xaml_cs(options),
        main_window_xaml: emit_main_window_xaml(name, options, shape),
        main_window_cs: emit_main_window_cs(name, slots, emits, options, shape),
        package_manifest: emit_app_manifest(name),
        build_script: emit_build_script(name),
        readme: emit_project_readme(name, shape, options.require_runtime),
    }
}

fn emit_global_json() -> String {
    "{\n  \"sdk\": {\n    \"version\": \"9.0.100\",\n    \"rollForward\": \"latestFeature\",\n    \"allowPrerelease\": false\n  }\n}\n"
        .to_string()
}

fn emit_csproj(_name: &str, options: &EmitOptions) -> String {
    let ns = &options.namespace;
    let sdk_ver = if options.windows_app_sdk.is_empty() {
        "1.8.260710003"
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
             <EnableCoreMrtTooling>true</EnableCoreMrtTooling>\n\
             <Nullable>enable</Nullable>\n\
             <ImplicitUsings>enable</ImplicitUsings>\n\
             <LangVersion>latest</LangVersion>\n\
             <!-- Bundle the pinned Windows App SDK with the host so users do not\n\
                  need a separately registered Windows App Runtime. The .NET\n\
                  runtime remains framework-dependent. -->\n\
             <WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>\n\
             <SelfContained>false</SelfContained>\n\
             <!-- WindowsAppSDK uses the legacy `win10-*` RIDs that .NET 8+\n\
                  removed from the default graph. UseRidGraph=true restores\n\
                  support for them. -->\n\
             <UseRidGraph>true</UseRidGraph>\n\
             <!-- MSIX packaging targets stay OFF above (EnableMsixTooling):\n\
                  they need the VS-shipped Microsoft.Build.AppxPackage.* tasks,\n\
                  absent on SDK-only machines — see hello-dialog-xaml\n\
                  ISSUES.md C1.\n\
             \n\
                  PRI generation is a SEPARATE subsystem and must stay ON. The\n\
                  original C1 mitigation disabled both together, which was too\n\
                  broad: makepri.exe ships inside the Microsoft.Windows.SDK.BuildTools\n\
                  package referenced below, so PRI needs no Visual Studio.\n\
                  Without the app PRI, WinUI cannot resolve\n\
                  ms-appx:///Microsoft.UI.Xaml/Themes/themeresources.xaml and the\n\
                  app dies at startup with E_XAMLPARSEFAILED (0x802B000A) — it\n\
                  builds cleanly and then cannot launch. -->\n\
             <AppxGeneratePriEnabled>true</AppxGeneratePriEnabled>\n\
             <EnableDefaultPriItems>true</EnableDefaultPriItems>\n\
           </PropertyGroup>\n\
         \n\
           <ItemGroup>\n\
             <PackageReference Include=\"Microsoft.WindowsAppSDK\" Version=\"{sdk_ver}\" />\n\
             <PackageReference Include=\"Microsoft.Windows.SDK.BuildTools\" Version=\"10.0.26100.4654\" />\n\
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
                     Title=\"{name} — Mosaic → XAML demo\">\n    \
                     <Grid>\n        \
                         <Grid.RowDefinitions>\n            \
                             <RowDefinition Height=\"*\"/>\n            \
                             <RowDefinition Height=\"Auto\"/>\n        \
                         </Grid.RowDefinitions>\n        \
                         <TextBlock Grid.Row=\"0\" Margin=\"40\" FontSize=\"18\" TextWrapping=\"Wrap\"\n                   \
                                    Text=\"Mosaic-authored {name} dialog. Click the button to open it.\"/>\n        \
                         <TextBlock Grid.Row=\"1\" Margin=\"40,0,40,20\" x:Name=\"StatusText\" Foreground=\"#888\"\n                   \
                                    Text=\"Status: waiting for dispatch…\"/>\n        \
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
                     Title=\"{name} — Mosaic → XAML demo\">\n    \
                     <Grid>\n        \
                         <Grid.RowDefinitions>\n            \
                             <RowDefinition Height=\"*\"/>\n            \
                             <RowDefinition Height=\"Auto\"/>\n        \
                         </Grid.RowDefinitions>\n        \
                         <gen:{name} Grid.Row=\"0\" x:Name=\"Component\"/>\n        \
                         <TextBlock Grid.Row=\"1\" Margin=\"20\" x:Name=\"StatusText\" Foreground=\"#888\"\n                   \
                                    Text=\"Status: waiting for dispatch…\"/>\n    \
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
    if options.require_runtime {
        return emit_runtime_required_main_window_cs(name, slots, options, shape);
    }

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
                         this.Component.Dispatch += OnComponentDispatch;\n        \
                         TryRunMosaicHostInteractionAcceptance(this.Component);\n    \
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

fn emit_runtime_required_main_window_cs(
    name: &str,
    slots: &[SlotDecl],
    options: &EmitOptions,
    shape: RootShape,
) -> String {
    let ns = &options.namespace;
    let required_props = build_required_prop_names(slots);
    match shape {
        RootShape::ContentDialog => format!(
            "// Auto-generated by mosaic-emit-xaml in native-complete emit-project mode.\n\
             using Microsoft.UI.Xaml;\n\
             using Microsoft.UI.Xaml.Controls;\n\
             \n\
             namespace {ns};\n\
             \n\
             public sealed partial class MainWindow : Window\n\
             {{\n    \
                 private static readonly string[] RequiredProps = {required_props};\n\
             \n    \
                 public MainWindow()\n    \
                 {{\n        \
                     MosaicRuntimeHost.LoadRequired();\n        \
                     this.InitializeComponent();\n    \
                 }}\n\
             \n    \
                 private async void OnOpenButtonClick(object sender, RoutedEventArgs e)\n    \
                 {{\n        \
                     var xamlRoot = (sender as FrameworkElement)?.XamlRoot\n            \
                         ?? throw new System.InvalidOperationException(\"No XamlRoot on click sender\");\n        \
                     var dialog = new {name}();\n        \
                     MosaicRuntimeHost.ApplyRequiredProps(dialog, RequiredProps);\n        \
                     dialog.XamlRoot = xamlRoot;\n        \
                     dialog.Dispatch += OnComponentDispatch;\n        \
                     await dialog.ShowAsync();\n    \
                 }}\n\
             \n    \
                 private async void OnComponentDispatch(object? sender, {name}Event mosaicEvent)\n    \
                 {{\n        \
                     var component = sender as {name}\n            \
                         ?? throw new System.InvalidOperationException(\"Mosaic event sender was not {name}\");\n        \
                     var result = await MosaicRuntimeHost.HandleRequiredEvent(\n            \
                         component, mosaicEvent, RequiredProps);\n        \
                     this.StatusText.Text = result.Status;\n    \
                 }}\n\
             }}\n"
        ),
        RootShape::UserControl => format!(
            "// Auto-generated by mosaic-emit-xaml in native-complete emit-project mode.\n\
             using Microsoft.UI.Xaml;\n\
             \n\
             namespace {ns};\n\
             \n\
             public sealed partial class MainWindow : Window\n\
             {{\n    \
                 private static readonly string[] RequiredProps = {required_props};\n\
             \n    \
                 public MainWindow()\n    \
                 {{\n        \
                     MosaicRuntimeHost.LoadRequired();\n        \
                     this.InitializeComponent();\n        \
                     MosaicRuntimeHost.ApplyRequiredProps(this.Component, RequiredProps);\n        \
                     this.StatusText.Text = \"Status: Mosaic runtime props loaded\";\n        \
                     this.Component.Dispatch += OnComponentDispatch;\n    \
                 }}\n\
             \n    \
                 private async void OnComponentDispatch(object? sender, {name}Event mosaicEvent)\n    \
                 {{\n        \
                     var result = await MosaicRuntimeHost.HandleRequiredEvent(\n            \
                         this.Component, mosaicEvent, RequiredProps);\n        \
                     this.StatusText.Text = result.Status;\n    \
                 }}\n\
             }}\n"
        ),
    }
}

fn build_required_prop_names(slots: &[SlotDecl]) -> String {
    let required = slots
        .iter()
        .filter(|slot| slot.required && slot.default.is_none())
        .map(|slot| format!("\"{}\"", escape_csharp_string(&slot.name)))
        .collect::<Vec<_>>();
    if required.is_empty() {
        "System.Array.Empty<string>()".to_string()
    } else {
        format!("new[] {{ {} }}", required.join(", "))
    }
}

fn build_optional_host_helpers(name: &str, namespace: &str) -> String {
    let host_type = escape_csharp_string(&format!("{namespace}.MosaicHost"));
    let runtime_type = escape_csharp_string(&format!("{namespace}.MosaicRuntimeHost"));
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
         private void TryRunMosaicHostInteractionAcceptance({name} component)\n    \
         {{\n        \
             var method = FindMosaicHostMethod(\n            \
                 \"RunInteractionAcceptance\",\n            \
                 typeof(Window),\n            \
                 typeof({name}));\n        \
             if (method is null) {{ return; }}\n        \
             try\n        \
             {{\n            \
                 method.Invoke(null, new object[] {{ this, component }});\n        \
             }}\n        \
             catch (System.Exception ex)\n        \
             {{\n            \
                 System.Diagnostics.Debug.WriteLine(\n                \
                     $\"Mosaic host interaction acceptance failed: {{ex}}\");\n        \
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
             var runtimeType = System.Type.GetType(\"{runtime_type}\");\n        \
             var available = runtimeType?.GetProperty(\n            \
                 \"IsAvailable\",\n            \
                 System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)?.GetValue(null);\n        \
             if (available is true) {{ return runtimeType; }}\n        \
             return System.Type.GetType(\"{host_type}\");\n    \
         }}\n\
         \n    \
         private static System.Reflection.MethodInfo? FindMosaicHostMethod(string methodName, params System.Type[] parameterTypes)\n    \
         {{\n        \
             var hostType = FindMosaicHostType();\n        \
             if (hostType is null) {{ return null; }}\n        \
             foreach (var method in hostType.GetMethods(\n            \
                 System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static))\n        \
             {{\n            \
                 if (method.Name != methodName) {{ continue; }}\n            \
                 var parameters = method.GetParameters();\n            \
                 if (parameters.Length != parameterTypes.Length) {{ continue; }}\n            \
                 var matches = true;\n            \
                 for (var index = 0; index < parameters.Length; index++)\n            \
                 {{\n                \
                     if (!parameters[index].ParameterType.IsAssignableFrom(parameterTypes[index]))\n                \
                     {{\n                    \
                         matches = false;\n                    \
                         break;\n                \
                     }}\n            \
                 }}\n            \
                 if (matches) {{ return method; }}\n        \
             }}\n        \
             return null;\n    \
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
         # The pinned Windows App SDK is bundled with the executable; no separate\n\
         # Windows App Runtime install is required.\n\
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
         # Resolve the SDK from the generated global.json. The dotnet SDK\n\
         # resolver starts at the process working directory, not the project\n\
         # argument's directory.\n\
         Push-Location $PSScriptRoot\n\
         try {{\n    \
             & $dotnet build (Split-Path -Leaf $proj) -c Debug -p:Platform=x64 --nologo\n    \
             $buildExitCode = $LASTEXITCODE\n\
         }} finally {{\n    \
             Pop-Location\n\
         }}\n\
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

fn emit_project_readme(name: &str, shape: RootShape, require_runtime: bool) -> String {
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
    let runtime_blurb = if require_runtime {
        "## Rust application runtime\n\n\
         This `native-complete` shell requires Mosaic's standard Rust application\n\
         runtime. Set `MOSAIC_APP_LIBRARY` to the application DLL, or package\n\
         `mosaic_app.dll` beside the executable. The runtime is loaded before the\n\
         window activates, required MIL props are validated before the component is\n\
         shown, and all Mosaic events return to Rust. There is no reflection host or\n\
         sample-prop fallback.\n\n"
    } else {
        "## Host integration\n\n\
         The permissive shell can use Mosaic's standard Rust runtime, an app-owned\n\
         reflection host, or deterministic sample props for previews. Set\n\
         `MOSAIC_APP_LIBRARY` to the Rust application DLL when using the standard\n\
         runtime. Use\n\
         `--profile native-complete` to require Rust and remove both fallbacks.\n\n"
    };
    let logic_blurb = if require_runtime {
        "- **`MainWindow.xaml.cs`** is generated runtime wiring. It loads Rust,\n\
           validates initial props, and sends component events back to the engine;\n\
           do not replace it with app-specific state logic.\n"
    } else {
        "- **`MainWindow.xaml.cs`** has stub slot values for the component and a\n\
           stub `OnComponentDispatch` handler. Replace the stub values with your\n\
           real data, and fill in the body of each match arm with the logic that\n\
           should run when each Mosaic emit fires.\n"
    };
    let main_window_edit = if require_runtime {
        "No"
    } else {
        "**Yes** — your host"
    };
    format!(
        "# {name} â€” WinUI 3 host project\n\
         \n\
         Auto-generated by `mosaic-compile --backend xaml --emit-project`.\n\
         \n\
         {shape_blurb}\n\
         {runtime_blurb}\
         ## Prerequisites\n\
         \n\
         1. **.NET 9.0 SDK** â€” `dotnet --list-sdks` should list one matching `9.0.*`.\n\
         2. No separate Windows App Runtime install is required; the pinned\n\
            Windows App SDK is bundled with the generated host.\n\
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
         The build emits a .NET framework-dependent .exe with the pinned Windows\n\
         App SDK bundled beside it at\n\
         `bin\\x64\\Debug\\net9.0-windows10.0.19041.0\\win-x64\\{name}.exe`. The native\n\
         WindowsAppRuntime DLLs are auto-flattened next to the .exe by an\n\
         MSBuild post-build target (see the project's `.csproj`).\n\
         \n\
         ## Where to add business logic\n\
         \n\
         {logic_blurb}\
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
         | `MainWindow.xaml(.cs)` | --emit-project | {main_window_edit} |\n\
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
            ctx.slot_property_name(right)
        }
        [ExprTok::Name(left), ExprTok::EqEq, ExprTok::Name(right)]
            if ctx.lookup_for_index(right).is_some() =>
        {
            kebab_to_pascal_case(left)
        }
        [ExprTok::SlotPrefix, ExprTok::Name(left), ExprTok::EqEq, ExprTok::Name(right)]
            if ctx.lookup_for_index(right).is_some() =>
        {
            ctx.slot_property_name(left)
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
    let inside_template = !ctx.for_scope.is_empty();
    let mut p = ExprParser::new(&tokens, ctx, src);
    let lowered = match p.parse_or() {
        Ok(lowering) => {
            if p.is_done() {
                lowering
            } else {
                ExprLowering::Unsupported(format!("expression {src:?} has trailing tokens"))
            }
        }
        Err(e) => ExprLowering::Unsupported(e),
    };
    match lowered {
        ExprLowering::Helper(call) if inside_template => {
            register_template_helper_binding(&call, ctx)
                .map(ExprLowering::Bindable)
                .unwrap_or(ExprLowering::Helper(call))
        }
        other => other,
    }
}

/// WinUI's typed DataTemplate compiler accepts normal row properties but does
/// not accept function bindings rooted through another property
/// (`Owner.Helper(...)`). Expose the helper result on the generated row VM and
/// let that C# property delegate to the owning component.
fn register_template_helper_binding(call: &str, ctx: &mut EmitContext<'_>) -> Option<String> {
    let helper_name = call.split('(').next()?;
    let helper = ctx
        .helpers
        .iter()
        .find(|helper| helper.name == helper_name)?
        .clone();
    let arguments = helper
        .parameters
        .iter()
        .map(|(parameter, _)| template_helper_argument(parameter, ctx))
        .collect::<Vec<_>>()
        .join(", ");
    let owner_call = format!("Owner.{}({arguments})", helper.name);
    register_row_vm_computed_binding(ctx, helper.name.clone(), helper.return_type, owner_call)?;
    Some(helper.name)
}

fn register_row_vm_computed_binding(
    ctx: &mut EmitContext<'_>,
    property_name: String,
    return_type: String,
    owner_call: String,
) -> Option<()> {
    let vm_class = ctx.for_scope.last()?.vm_class.clone();
    let vm = ctx
        .row_vms
        .iter_mut()
        .find(|vm| vm.class_name == vm_class)?;
    if !vm
        .helper_bindings
        .iter()
        .any(|binding| binding.property_name == property_name)
    {
        vm.helper_bindings.push(RowVmHelperBinding {
            property_name,
            return_type,
            owner_call,
        });
    }
    Some(())
}

fn disabled_slot_xbind_path(slot: &str, ctx: &mut EmitContext<'_>) -> String {
    let property = ctx.slot_property_name(slot);
    if ctx.for_scope.is_empty() {
        ctx.add_helper(HelperMethod {
            name: "Not".to_string(),
            parameters: vec![("b".to_string(), "bool".to_string())],
            return_type: "bool".to_string(),
            body: "!b".to_string(),
        });
        format!("Not({property})")
    } else {
        let binding_property = format!("Not{property}");
        let _ = register_row_vm_computed_binding(
            ctx,
            binding_property.clone(),
            "bool".to_string(),
            format!("!Owner.{property}"),
        );
        binding_property
    }
}

fn template_helper_argument(parameter: &str, ctx: &EmitContext<'_>) -> String {
    for (position, binding) in ctx.for_scope.iter().enumerate().rev() {
        if kebab_to_pascal_case(&binding.as_name) == parameter {
            return kebab_to_pascal_case(&binding.as_name);
        }
        if binding
            .index_name
            .as_deref()
            .is_some_and(|name| kebab_to_pascal_case(name) == parameter)
        {
            return if position + 1 == ctx.for_scope.len() {
                "Index".to_string()
            } else {
                parameter.to_string()
            };
        }
    }
    parameter.to_string()
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
                Ok(self.ctx.slot_xbind_path(&name))
            }
            ExprTok::Name(n) => {
                self.pos += 1;
                Ok(self.ctx.scoped_name_xbind_path(&n))
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
    register_host_visual_states(node, "TextBox", &x_name, part_styles, ctx);

    // -- Build the attribute set --
    let mut attrs = String::new();
    if let Some(part_name) = &node.part_name {
        attrs.push_str(&format!(
            " AutomationProperties.AutomationId=\"{}\"",
            escape_xaml_attr(part_name)
        ));
    }

    // value: slot/string/expr â†’ Text binding
    match find_prop_value(node, "value") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" Text=\"{{x:Bind {pascal}, Mode=TwoWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Text=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Expr(src)) => {
            // #12126: unlike the SlotRef arm above, this is OneWay, not
            // TwoWay. `Mode=TwoWay` needs a settable target for user
            // edits to write back to; an indexer expression like
            // `row[1]` lowers to a C# *method call*
            // (`ExprLowering::Helper`), which is not an lvalue, so
            // `x:Bind Expr_xxx(Row), Mode=TwoWay` would not compile.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    attrs.push_str(&format!(" Text=\"{{x:Bind {path}, Mode=OneWay}}\""));
                }
                ExprLowering::Helper(call) => {
                    attrs.push_str(&format!(" Text=\"{{x:Bind {call}, Mode=OneWay}}\""));
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
        }
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
    }

    // read-only: slot/keyword
    match find_prop_value(node, "read-only") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" IsReadOnly=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::Keyword(k)) if k == "true" => {
            attrs.push_str(" IsReadOnly=\"True\"");
        }
        Some(LayoutPropValue::Keyword(k)) if k == "false" => {
            attrs.push_str(" IsReadOnly=\"False\"");
        }
        _ => {}
    }

    // placeholder: literal/slot/expr
    match find_prop_value(node, "placeholder") {
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" PlaceholderText=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            // #12126: this arm was missing entirely — a slot-valued
            // placeholder silently emitted no `PlaceholderText` at all.
            // OneWay, matching HostNumberInput's sibling `placeholder`
            // handling: a placeholder isn't a two-way-bound value.
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(
                " PlaceholderText=\"{{x:Bind {pascal}, Mode=OneWay}}\""
            ));
        }
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(
                    " PlaceholderText=\"{{x:Bind {path}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(
                    " PlaceholderText=\"{{x:Bind {call}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
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
        let args = host_input_event_args(ctx, emit_name, "tb.Text")?;
        let body = format!(
            "    private void {handler}(object sender, Microsoft.UI.Xaml.Controls.TextChangedEventArgs e)\n    {{\n        if (sender is Microsoft.UI.Xaml.Controls.TextBox tb)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case_pascal}({args}));\n        }}\n    }}"
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
            "    private void {handler}(object sender, Microsoft.UI.Xaml.Input.KeyRoutedEventArgs e)\n    {{\n        if (sender is not Microsoft.UI.Xaml.Controls.TextBox tb) return;\n"
        ));
        let component = ctx.component_name;
        if let Some(emit) = &commit {
            let case = kebab_to_pascal_case(&strip_on_prefix(emit));
            let args = host_input_event_args(ctx, emit, "tb.Text")?;
            body.push_str(&format!(
                "        if (e.Key == Windows.System.VirtualKey.Enter)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case}({args}));\n        }}\n"
            ));
        }
        if let Some(emit) = &cancel {
            let case = kebab_to_pascal_case(&strip_on_prefix(emit));
            let args = host_input_event_args(ctx, emit, "tb.Text")?;
            body.push_str(&format!(
                "        if (e.Key == Windows.System.VirtualKey.Escape)\n        {{\n            Dispatch?.Invoke(this, new {component}Event.{case}({args}));\n        }}\n"
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

/// Construct the native text input callback arguments required by an authored
/// emit. A HostInput owns one textual callback value, so it can satisfy void or
/// single-value text/number/bool events without app-authored platform glue.
fn host_input_event_args(
    ctx: &EmitContext<'_>,
    emit_name: &str,
    value_expr: &str,
) -> Result<String, PipelineEmitError> {
    let Some(payloads) = ctx.emit_payloads.get(emit_name) else {
        // Direct emitter callers historically omitted the interface declaration
        // and received the text value. Keep that narrow compatibility fallback;
        // package compilation always supplies the canonical emit schema.
        return Ok(value_expr.to_string());
    };
    match payloads.as_slice() {
        [] => Ok(String::new()),
        [(_, ty)] if ty == "string" => Ok(value_expr.to_string()),
        [(_, ty)] if ty == "double" => Ok(format!(
            "double.TryParse({value_expr}, out var mosaicNumber) ? mosaicNumber : 0.0"
        )),
        [(_, ty)] if ty == "bool" => Ok(format!(
            "bool.TryParse({value_expr}, out var mosaicBool) && mosaicBool"
        )),
        _ => Err(PipelineEmitError::UnsupportedExpression(format!(
            "HostInput callback {emit_name:?} must emit zero or one text, number, or bool payload"
        ))),
    }
}

/// `HostButton` â†’ `<Button>` per spec Â§4.2.
fn emit_host_button(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = content_control_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostButton", ctx);
    register_host_visual_states(node, "Button", &x_name, part_styles, ctx);

    let mut attrs = String::new();
    if let Some(part_name) = &node.part_name {
        attrs.push_str(&format!(
            " AutomationProperties.AutomationId=\"{}\"",
            escape_xaml_attr(part_name)
        ));
    }

    // label: slot/string
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            if ctx.lookup_for_binding(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
            } else if ctx.lookup_for_index(k).is_some() {
                attrs.push_str(" Content=\"{x:Bind Index, Mode=OneWay}\"");
            } else {
                attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
            }
        }
        // An expression label — `label: ( row[1] )` is the common shape.
        //
        // This arm was missing, and the catch-all below swallowed it, so a
        // button whose label came from a row expression emitted NO Content
        // attribute at all. That is why every task row, project-rail row and
        // notes row rendered a blank button: not a binding-mode problem, the
        // attribute simply never existed. The Text lowering already routes
        // expressions through the same helper successfully two elements away
        // in the same template.
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {path}, Mode=OneWay}}\""));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {call}, Mode=OneWay}}\""));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        _ => {}
    }

    // disabled: slot/keyword. Polarity flip handled via a generated
    // `Not(bool)` helper on the partial class.
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let path = disabled_slot_xbind_path(slot, ctx);
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind {path}, Mode=OneWay}}\""));
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

// =====================================================================
// UI35: HostDraggable / HostDropTarget
// =====================================================================

/// Both UI35 primitives lower to native WinUI drag/drop controls with the
/// shared pointer, keyboard, filtering, lifecycle, and accessibility runtime.
/// Native-completeness analysis calls this predicate so reporting stays in
/// lockstep with the emitter.
pub fn host_drag_drop_has_native_semantics(node: &LayoutNode) -> bool {
    matches!(node.tag.as_str(), "HostDraggable" | "HostDropTarget")
}

/// Lower a UI35 text prop to an attribute value. Expressions inside a `For`
/// reuse the normal row-VM projection path, so a value such as `card[2]`
/// becomes a typed row property rather than an invalid page-scoped binding.
fn drag_text_attr_value(
    node: &LayoutNode,
    prop: &str,
    default: &str,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    match find_prop_value(node, prop) {
        Some(LayoutPropValue::String(value)) => Ok(escape_xaml_attr(value)),
        Some(LayoutPropValue::SlotRef(slot)) => Ok(format!(
            "{{x:Bind {}, Mode=OneWay}}",
            ctx.slot_xbind_path(slot)
        )),
        Some(LayoutPropValue::Keyword(value)) if ctx.lookup_for_binding(value).is_some() => Ok(
            format!("{{x:Bind {}, Mode=OneWay}}", kebab_to_pascal_case(value)),
        ),
        Some(LayoutPropValue::Keyword(value)) if ctx.lookup_for_index(value).is_some() => {
            Ok("{x:Bind Index, Mode=OneWay}".to_string())
        }
        Some(LayoutPropValue::Keyword(value)) => Ok(escape_xaml_attr(value)),
        Some(LayoutPropValue::Expr(source)) => match lower_expr_for_xbind(source, ctx) {
            ExprLowering::Bindable(path) | ExprLowering::Helper(path) => {
                Ok(format!("{{x:Bind {path}, Mode=OneWay}}"))
            }
            ExprLowering::Unsupported(reason) => Err(PipelineEmitError::UnsupportedExpression(
                format!("{prop}: {reason}"),
            )),
        },
        _ => Ok(escape_xaml_attr(default)),
    }
}

fn drag_bool_attr_value(
    node: &LayoutNode,
    prop: &str,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    match find_prop_value(node, prop) {
        Some(LayoutPropValue::SlotRef(slot)) => Ok(format!(
            "{{x:Bind {}, Mode=OneWay}}",
            ctx.slot_xbind_path(slot)
        )),
        Some(LayoutPropValue::Keyword(value)) if value == "true" => Ok("True".to_string()),
        Some(LayoutPropValue::Keyword(value)) if value == "false" => Ok("False".to_string()),
        Some(LayoutPropValue::Expr(source)) => match lower_expr_for_xbind(source, ctx) {
            ExprLowering::Bindable(path) | ExprLowering::Helper(path) => {
                Ok(format!("{{x:Bind {path}, Mode=OneWay}}"))
            }
            ExprLowering::Unsupported(reason) => Err(PipelineEmitError::UnsupportedExpression(
                format!("{prop}: {reason}"),
            )),
        },
        _ => Ok("False".to_string()),
    }
}

fn drag_accepts_attr_value(
    node: &LayoutNode,
    ctx: &mut EmitContext<'_>,
) -> Result<Option<String>, PipelineEmitError> {
    match find_prop_value(node, "accepts") {
        None => Ok(None),
        Some(LayoutPropValue::SlotRef(slot)) => Ok(Some(format!(
            "{{x:Bind {}, Mode=OneWay}}",
            ctx.slot_xbind_path(slot)
        ))),
        Some(LayoutPropValue::Keyword(value)) if ctx.lookup_for_binding(value).is_some() => {
            Ok(Some(format!(
                "{{x:Bind {}, Mode=OneWay}}",
                kebab_to_pascal_case(value)
            )))
        }
        Some(LayoutPropValue::Expr(source)) => match lower_expr_for_xbind(source, ctx) {
            ExprLowering::Bindable(path) | ExprLowering::Helper(path) => {
                Ok(Some(format!("{{x:Bind {path}, Mode=OneWay}}")))
            }
            ExprLowering::Unsupported(reason) => Err(PipelineEmitError::UnsupportedExpression(
                format!("accepts: {reason}"),
            )),
        },
        _ => Err(PipelineEmitError::UnsupportedExpression(
            "HostDropTarget.accepts must bind to a list<text> value".to_string(),
        )),
    }
}

fn register_drag_event_handler(
    node: &LayoutNode,
    prop: &str,
    handler: String,
    args_type: &str,
    values: &[(&str, &str)],
    ctx: &mut EmitContext<'_>,
) -> Result<Option<String>, PipelineEmitError> {
    let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, prop) else {
        return Ok(None);
    };
    let component = ctx.component_name;
    let case = kebab_to_pascal_case(&strip_on_prefix(emit_name));
    let args = ctx
        .emit_payloads
        .get(emit_name)
        .map(|payloads| {
            payloads
                .iter()
                .map(|(name, ty)| {
                    values
                        .iter()
                        .find_map(|(field, expression)| (*field == name).then_some(*expression))
                        .map(str::to_string)
                        .unwrap_or_else(|| match ty.as_str() {
                            "string" => "string.Empty".to_string(),
                            "double" => "0.0".to_string(),
                            "bool" => "false".to_string(),
                            _ => "null!".to_string(),
                        })
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let ctor = if args.is_empty() {
        format!("new {component}Event.{case}()")
    } else {
        format!("new {component}Event.{case}({args})")
    };
    ctx.add_host_handler(HostHandler {
        name: handler.clone(),
        source: format!(
            "    private void {handler}(object? sender, {args_type} args)\n    {{\n        Dispatch?.Invoke(this, {ctor});\n    }}"
        ),
    });
    Ok(Some(handler))
}

fn emit_host_draggable(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let component = ctx.component_name;
    let x_name = host_x_name(node, "HostDraggable", ctx);
    let key = drag_text_attr_value(node, "drag-key", "", ctx)?;
    let kind = drag_text_attr_value(node, "drag-kind", "", ctx)?;
    let label = if find_prop_value(node, "drag-label").is_some() {
        drag_text_attr_value(node, "drag-label", "", ctx)?
    } else {
        key.clone()
    };
    let disabled = drag_bool_attr_value(node, "drag-disabled", ctx)?;
    let mut attrs = format!(
        " x:Name=\"{x_name}\" DragKey=\"{key}\" DragKind=\"{kind}\" DragLabel=\"{label}\" DragDisabled=\"{disabled}\" AutomationProperties.AutomationId=\"{}\" AutomationProperties.Name=\"{label}\" AutomationProperties.HelpText=\"Press Space or Enter to grab, use arrow keys to choose a target, then press Space or Enter to drop.\"",
        escape_xaml_attr(node.part_name.as_deref().unwrap_or("draggable"))
    );
    if let Some(handler) = register_drag_event_handler(
        node,
        "onDragStart",
        format!("{x_name}_MosaicDragStarted"),
        &format!("{component}MosaicDragSourceEventArgs"),
        &[("key", "args.Key"), ("kind", "args.Kind")],
        ctx,
    )? {
        attrs.push_str(&format!(" MosaicDragStarted=\"{handler}\""));
    }
    if let Some(handler) = register_drag_event_handler(
        node,
        "onDragEnd",
        format!("{x_name}_MosaicDragEnded"),
        &format!("{component}MosaicDragEndEventArgs"),
        &[
            ("key", "args.Key"),
            ("kind", "args.Kind"),
            ("dropped", "args.Dropped"),
        ],
        ctx,
    )? {
        attrs.push_str(&format!(" MosaicDragEnded=\"{handler}\""));
    }
    let (style, spacing) = drag_control_style_attr(node, part_styles);
    let children = emit_drag_content_children(
        &node.children,
        indent + 4,
        spacing.as_deref(),
        part_styles,
        ctx,
    )?;
    ctx.needs_native_drag_support = true;
    Ok(format!(
        "{pad}<local:{component}MosaicDragSource{attrs}{style}>\n{children}{pad}</local:{component}MosaicDragSource>\n"
    ))
}

fn emit_host_drop_target(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let component = ctx.component_name;
    let x_name = host_x_name(node, "HostDropTarget", ctx);
    let key = drag_text_attr_value(node, "drop-key", "", ctx)?;
    let disabled = drag_bool_attr_value(node, "drop-disabled", ctx)?;
    let mut attrs = format!(
        " x:Name=\"{x_name}\" DropKey=\"{key}\" DropDisabled=\"{disabled}\" AutomationProperties.AutomationId=\"{}\" AutomationProperties.Name=\"{key}\"",
        escape_xaml_attr(node.part_name.as_deref().unwrap_or("drop-target"))
    );
    if let Some(accepts) = drag_accepts_attr_value(node, ctx)? {
        attrs.push_str(&format!(" Accepts=\"{accepts}\""));
    }
    let source_values = [("key", "args.Key"), ("kind", "args.Kind")];
    let drop_values = [
        ("key", "args.Key"),
        ("kind", "args.Kind"),
        ("targetKey", "args.TargetKey"),
        ("position", "args.Position"),
    ];
    for (prop, event, suffix, args_type, values) in [
        (
            "onDragEnter",
            "MosaicDragEntered",
            "MosaicDragEntered",
            format!("{component}MosaicDragSourceEventArgs"),
            source_values.as_slice(),
        ),
        (
            "onDragLeave",
            "MosaicDragLeft",
            "MosaicDragLeft",
            format!("{component}MosaicDragSourceEventArgs"),
            source_values.as_slice(),
        ),
        (
            "onDropHover",
            "MosaicDropHovered",
            "MosaicDropHovered",
            format!("{component}MosaicDropEventArgs"),
            drop_values.as_slice(),
        ),
        (
            "onDrop",
            "MosaicDropped",
            "MosaicDropped",
            format!("{component}MosaicDropEventArgs"),
            drop_values.as_slice(),
        ),
    ] {
        if let Some(handler) = register_drag_event_handler(
            node,
            prop,
            format!("{x_name}_{suffix}"),
            &args_type,
            values,
            ctx,
        )? {
            attrs.push_str(&format!(" {event}=\"{handler}\""));
        }
    }
    let (style, spacing) = drag_control_style_attr(node, part_styles);
    let children = emit_drag_content_children(
        &node.children,
        indent + 4,
        spacing.as_deref(),
        part_styles,
        ctx,
    )?;
    ctx.needs_native_drag_support = true;
    Ok(format!(
        "{pad}<local:{component}MosaicDropTarget{attrs}{style}>\n{children}{pad}</local:{component}MosaicDropTarget>\n"
    ))
}

/// `HostCheckbox` â†’ WinUI / WPF `<CheckBox>` per UI29-2.
///
/// ## Property handling
///
/// | moslayout prop          | XAML                                                        |
/// |---|---|
/// | `checked: slot: c`      | `IsChecked="{x:Bind C, Mode=OneWay}"`                       |
/// | `checked: true/false`   | `IsChecked="True"` / `IsChecked="False"`                    |
/// | `disabled: slot: d`     | `IsEnabled="{x:Bind Not(D), Mode=OneWay}"` (shared helper)  |
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
        return Some(host_link_href_payload_expr(node, ctx));
    }
    if let Some(expr) = host_button_click_payload_expr(emit_name, ctx) {
        return Some(expr);
    }
    if param_type == "string" {
        return Some(host_link_href_payload_expr(node, ctx));
    }
    None
}

fn host_link_href_payload_expr(node: &LayoutNode, ctx: &EmitContext<'_>) -> String {
    match find_prop_value(node, "href") {
        Some(LayoutPropValue::String(s)) => {
            format!("\"{}\"", escape_csharp_string(s))
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_property_name(slot);
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
    register_host_visual_states(node, "CheckBox", &x_name, part_styles, ctx);

    let mut attrs = String::new();

    // Content (label).
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
        }
        // An expression label — `label: ( row[1] )` is the common shape.
        //
        // Same missing arm, same silent catch-all, and the same blank-control
        // symptom that #12045 fixed on HostButton: with no arm here the
        // checkbox emitted no Content attribute at all. See emit_host_button
        // for the full story.
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {path}, Mode=OneWay}}\""));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {call}, Mode=OneWay}}\""));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        _ => {}
    }

    // IsChecked from `checked:`.
    match find_prop_value(node, "checked") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
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
            let path = disabled_slot_xbind_path(slot, ctx);
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind {path}, Mode=OneWay}}\""));
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
    register_host_visual_states(node, "RadioButton", &x_name, part_styles, ctx);

    let mut attrs = String::new();

    // Content (label).
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            attrs.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
        }
        // An expression label — see emit_host_button and emit_host_checkbox.
        // A radio button with no Content renders as a bare dot with no text
        // beside it, which is worse than useless in a mutex group.
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {path}, Mode=OneWay}}\""));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(" Content=\"{{x:Bind {call}, Mode=OneWay}}\""));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        _ => {}
    }

    // GroupName from `group:` â€” native WinUI radio-mutex.
    match find_prop_value(node, "group") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" GroupName=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" GroupName=\"{}\"", escape_xaml_attr(s)));
        }
        _ => {}
    }

    // IsChecked from `checked:`.
    match find_prop_value(node, "checked") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
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
            let path = disabled_slot_xbind_path(slot, ctx);
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind {path}, Mode=OneWay}}\""));
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
                let pascal = ctx.slot_property_name(slot);
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

/// Lower `HostSlider` to a component-scoped subclass of WinUI's native
/// `Slider` control.
///
/// The generated subclass keeps WinUI's native range-value automation peer,
/// pointer/touch input, keyboard navigation, high-contrast rendering, and
/// platform theming. It adds only the portable Mosaic lifecycle split:
/// `MosaicValueChanged` fires during a user adjustment, while
/// `MosaicValueCommitted` fires once on pointer release/capture loss, key-up,
/// or blur. A zero Mosaic step selects effectively continuous pointer input;
/// positive steps preserve WinUI snapping exactly.
fn emit_host_slider(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let style = part_style_attr(node, part_styles);
    let x_name = host_x_name(node, "HostSlider", ctx);
    register_host_visual_states(node, "Slider", &x_name, part_styles, ctx);
    ctx.needs_native_slider_support = true;

    let mut attrs = String::new();
    if let Some(part_name) = &node.part_name {
        attrs.push_str(&format!(
            " AutomationProperties.AutomationId=\"{}\"",
            escape_xaml_attr(part_name)
        ));
    }
    match find_prop_value(node, "a11y-label") {
        Some(LayoutPropValue::String(label)) => {
            attrs.push_str(&format!(
                " AutomationProperties.Name=\"{}\"",
                escape_xaml_attr(label)
            ));
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            attrs.push_str(&format!(
                " AutomationProperties.Name=\"{{x:Bind {}, Mode=OneWay}}\"",
                ctx.slot_xbind_path(slot)
            ));
        }
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(
                    " AutomationProperties.Name=\"{{x:Bind {path}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(
                    " AutomationProperties.Name=\"{{x:Bind {call}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
    }

    for (prop, attr, default) in [
        ("value", "Value", "0"),
        ("min", "Minimum", "0"),
        ("max", "Maximum", "100"),
        ("step", "MosaicStep", "1"),
    ] {
        let value = host_slider_number_attr_value(node, prop, default, ctx)?;
        attrs.push_str(&format!(" {attr}=\"{value}\""));
    }

    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let path = disabled_slot_xbind_path(slot, ctx);
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind {path}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::Keyword(value)) if value == "true" => {
            attrs.push_str(" IsEnabled=\"False\"");
        }
        Some(LayoutPropValue::Keyword(value)) if value == "false" => {
            attrs.push_str(" IsEnabled=\"True\"");
        }
        _ => {}
    }

    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onChange") {
        let handler = format!("{x_name}_MosaicValueChanged");
        let event = host_slider_event_constructor(ctx, emit_name, "args.NewValue")?;
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: format!(
                "    private void {handler}(object? sender, {component}MosaicSliderValueEventArgs args)\n    {{\n        Dispatch?.Invoke(this, {event});\n    }}",
                component = ctx.component_name,
            ),
        });
        attrs.push_str(&format!(" MosaicValueChanged=\"{handler}\""));
    }

    if let Some(LayoutPropValue::EmitRef(emit_name)) = find_prop_value(node, "onCommit") {
        let handler = format!("{x_name}_MosaicValueCommitted");
        let event = host_slider_event_constructor(ctx, emit_name, "args.NewValue")?;
        ctx.add_host_handler(HostHandler {
            name: handler.clone(),
            source: format!(
                "    private void {handler}(object? sender, {component}MosaicSliderValueEventArgs args)\n    {{\n        Dispatch?.Invoke(this, {event});\n    }}",
                component = ctx.component_name,
            ),
        });
        attrs.push_str(&format!(" MosaicValueCommitted=\"{handler}\""));
    }

    let component = ctx.component_name;
    Ok(format!(
        "{pad}<local:{component}MosaicSlider x:Name=\"{x_name}\"{attrs}{style}/>\n"
    ))
}

fn host_slider_number_attr_value(
    node: &LayoutNode,
    prop: &str,
    default: &str,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    match find_prop_value(node, prop) {
        Some(LayoutPropValue::SlotRef(slot)) => Ok(format!(
            "{{x:Bind {}, Mode=OneWay}}",
            ctx.slot_xbind_path(slot)
        )),
        Some(LayoutPropValue::Number(value)) => Ok(value.to_string()),
        Some(LayoutPropValue::Expr(source)) => match lower_expr_for_xbind(source, ctx) {
            ExprLowering::Bindable(path) | ExprLowering::Helper(path) => {
                Ok(format!("{{x:Bind {path}, Mode=OneWay}}"))
            }
            ExprLowering::Unsupported(reason) => Err(PipelineEmitError::UnsupportedExpression(
                format!("HostSlider.{prop}: {reason}"),
            )),
        },
        _ => Ok(default.to_string()),
    }
}

fn host_slider_event_constructor(
    ctx: &EmitContext<'_>,
    emit_name: &str,
    value_expr: &str,
) -> Result<String, PipelineEmitError> {
    let case = kebab_to_pascal_case(&strip_on_prefix(emit_name));
    if !is_safe_identifier(&case) {
        return Err(PipelineEmitError::UnsafeEmitName(case));
    }
    let component = ctx.component_name;
    let Some(payloads) = ctx.emit_payloads.get(emit_name) else {
        return Ok(format!("new {component}Event.{case}({value_expr})"));
    };
    let args = match payloads.as_slice() {
        [] => String::new(),
        [(_, ty)] if ty == "double" => value_expr.to_string(),
        [(_, ty)] if ty == "string" => {
            format!("{value_expr}.ToString(System.Globalization.CultureInfo.InvariantCulture)")
        }
        [(_, ty)] if ty == "bool" => format!("{value_expr} != 0.0"),
        _ => {
            let reason = format!(
                "HostSlider callback {emit_name:?} must emit zero or one text, number, or bool payload"
            );
            return Err(PipelineEmitError::UnsupportedExpression(reason));
        }
    };
    Ok(format!("new {component}Event.{case}({args})"))
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

/// #12038: whether a `HostLink.href` value's scheme is on the allowlist
/// (`http`/`https`/`mailto`) used by `emit_host_link`'s `NavigateUri`
/// handling below. `NavigateUri` is handed to the OS shell launcher, so
/// a `file:`/UNC/custom-protocol target would launch rather than open as
/// a web link -- reject rather than escape, since XML-escaping does
/// nothing to make an unsafe scheme safe. Extracts the scheme per RFC
/// 3986 §3.1 (`scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`); no
/// new crate dependency, a small self-contained parse matching the rest
/// of the file's hand-rolled text handling (`escape_xaml_attr`,
/// `tokenise_expr`). A string with no colon-delimited scheme at all (a
/// relative reference) is also rejected -- an author who wants in-app
/// routing has `external: false` for exactly that.
fn has_allowed_uri_scheme(href: &str) -> bool {
    const ALLOWED: [&str; 3] = ["http", "https", "mailto"];
    let Some(colon) = href.find(':') else {
        return false;
    };
    let scheme = &href[..colon];
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        return false;
    }
    ALLOWED.iter().any(|s| s.eq_ignore_ascii_case(scheme))
}

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
    let xaml_tag = if external_false {
        "Button"
    } else {
        "HyperlinkButton"
    };
    register_host_visual_states(node, xaml_tag, &x_name, part_styles, ctx);
    let on_activate = match find_prop_value(node, "onActivate") {
        Some(LayoutPropValue::EmitRef(s)) => Some(s.as_str()),
        _ => None,
    };

    // Content (label) â€” shared between Button and HyperlinkButton.
    let mut content_attr = String::new();
    match find_prop_value(node, "label") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            content_attr.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::String(s)) => {
            content_attr.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::Keyword(k)) => {
            if ctx.lookup_for_binding(k).is_some() {
                let pascal = kebab_to_pascal_case(k);
                content_attr.push_str(&format!(" Content=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
            } else if ctx.lookup_for_index(k).is_some() {
                content_attr.push_str(" Content=\"{x:Bind Index, Mode=OneWay}\"");
            } else {
                content_attr.push_str(&format!(" Content=\"{}\"", escape_xaml_attr(k)));
            }
        }
        // An expression label — the third instance of the same gap (see
        // emit_host_button, emit_host_checkbox, emit_host_radio). Here the
        // catch-all below is not empty, it falls back to `href`, so the
        // symptom varied: with a string href the link rendered the raw URL
        // instead of its label, and with a slot/expression href it rendered
        // blank. Both are wrong; an explicit label must win.
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                content_attr.push_str(&format!(" Content=\"{{x:Bind {path}, Mode=OneWay}}\""));
            }
            ExprLowering::Helper(call) => {
                content_attr.push_str(&format!(" Content=\"{{x:Bind {call}, Mode=OneWay}}\""));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
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
        //
        // #12038: NavigateUri is handed to the OS shell launcher, so a
        // `file:`/UNC/custom-protocol target would launch rather than
        // open as a web link. Reject rather than escape -- XML-escaping
        // the value does nothing to make an unsafe scheme safe.
        let mut attrs = String::new();
        match find_prop_value(node, "href") {
            Some(LayoutPropValue::String(s)) => {
                // The scheme is known at compile time -- reject outright
                // rather than emit a NavigateUri that could launch
                // arbitrary local content. Confirmed against every
                // current `href` usage in the repo (all `"#"` + `external:
                // false`, which never reaches this branch) that this
                // cannot regress an existing package.
                if !has_allowed_uri_scheme(s) {
                    return Err(PipelineEmitError::UnsafeUriScheme(s.clone()));
                }
                attrs.push_str(&format!(" NavigateUri=\"{}\"", escape_xaml_attr(s)));
            }
            Some(LayoutPropValue::SlotRef(slot)) => {
                // The value is only known at runtime -- validate host-
                // side via a generated helper instead of trusting the
                // raw slot value. `SafeNavigateUri` returns `null` for a
                // disallowed/unparseable scheme, which WinUI treats as
                // "no navigation target": the button simply doesn't
                // navigate on click rather than launching one.
                let pascal = ctx.slot_xbind_path(slot);
                ctx.add_helper(HelperMethod {
                    name: "SafeNavigateUri".to_string(),
                    parameters: vec![("raw".to_string(), "string?".to_string())],
                    return_type: "Uri?".to_string(),
                    body: "Uri.TryCreate(raw, UriKind.Absolute, out var u) && \
                           (u.Scheme == \"http\" || u.Scheme == \"https\" || \
                           u.Scheme == \"mailto\") ? u : null"
                        .to_string(),
                });
                attrs.push_str(&format!(
                    " NavigateUri=\"{{x:Bind SafeNavigateUri({pascal}), Mode=OneWay}}\""
                ));
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
            let pascal = ctx.slot_xbind_path(slot);
            format!(" ToolTipService.ToolTip=\"{{x:Bind {pascal}, Mode=OneWay}}\"")
        }
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                format!(" ToolTipService.ToolTip=\"{{x:Bind {path}, Mode=OneWay}}\"")
            }
            ExprLowering::Helper(call) => {
                format!(" ToolTipService.ToolTip=\"{{x:Bind {call}, Mode=OneWay}}\"")
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => String::new(),
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
/// | `disabled`      | `IsEnabled="{x:Bind Not(D), Mode=OneWay}"` (via Not helper)  |
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
    register_host_visual_states(node, "NumberBox", &x_name, part_styles, ctx);

    let mut attrs = String::new();

    // value: slot ref TwoWay binding; numeric literal as a Value attr.
    match find_prop_value(node, "value") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" Value=\"{{x:Bind {pascal}, Mode=TwoWay}}\""));
        }
        Some(LayoutPropValue::Number(n)) => {
            attrs.push_str(&format!(" Value=\"{n}\""));
        }
        Some(LayoutPropValue::Expr(src)) => {
            // #12126: OneWay, not TwoWay (same reasoning as HostInput's
            // value fix above — a Helper-lowered indexer is a method
            // call, not an lvalue). `NumberBox.Value` is a C# `double`;
            // the generated helper for a row-cell indexer like
            // `row[1]` returns the cell's own element type, which is
            // `string` when the slot is `list<list<text>>` — the same
            // implicit-conversion story as `Image.src`'s `ImageSource`
            // target below, verified against a real `dotnet build`.
            match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    attrs.push_str(&format!(" Value=\"{{x:Bind {path}, Mode=OneWay}}\""));
                }
                ExprLowering::Helper(call) => {
                    attrs.push_str(&format!(" Value=\"{{x:Bind {call}, Mode=OneWay}}\""));
                }
                ExprLowering::Unsupported(reason) => {
                    return Err(PipelineEmitError::UnsupportedExpression(reason));
                }
            }
        }
        Some(LayoutPropValue::String(_))
        | Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
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

    // placeholder: string, slot, or expr.
    match find_prop_value(node, "placeholder") {
        Some(LayoutPropValue::String(s)) => {
            attrs.push_str(&format!(" PlaceholderText=\"{}\"", escape_xaml_attr(s)));
        }
        Some(LayoutPropValue::SlotRef(slot)) => {
            let pascal = ctx.slot_xbind_path(slot);
            attrs.push_str(&format!(" PlaceholderText=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
        }
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(
                    " PlaceholderText=\"{{x:Bind {path}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(
                    " PlaceholderText=\"{{x:Bind {call}, Mode=OneWay}}\""
                ));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
    }

    // disabled: slot polarity-flip (Not helper) or literal keyword.
    match find_prop_value(node, "disabled") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let path = disabled_slot_xbind_path(slot, ctx);
            attrs.push_str(&format!(" IsEnabled=\"{{x:Bind {path}, Mode=OneWay}}\""));
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

    // title: slot/string/expr â€” Fix A3 + A4.
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
        Some(LayoutPropValue::Expr(src)) => match lower_expr_for_xbind(src, ctx) {
            ExprLowering::Bindable(path) => {
                attrs.push_str(&format!(" Title=\"{{x:Bind {path}, Mode=OneWay}}\""));
            }
            ExprLowering::Helper(call) => {
                attrs.push_str(&format!(" Title=\"{{x:Bind {call}, Mode=OneWay}}\""));
            }
            ExprLowering::Unsupported(reason) => {
                return Err(PipelineEmitError::UnsupportedExpression(reason));
            }
        },
        Some(LayoutPropValue::Keyword(_))
        | Some(LayoutPropValue::Number(_))
        | Some(LayoutPropValue::EmitRef(_))
        | None => {}
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
// WinUI 3 has no core DataGrid. Canonical indexed dynamic tables keep
// the hand-rolled Grid/ItemsRepeater visual tree but gain component-scoped
// controls whose automation peers expose native Table/Grid semantics.
// Other HostTable shapes retain the structural Grid fallback.
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

/// Canonical dynamic UI31/Grid structure that the XAML backend can expose as
/// a real UI Automation table while retaining the authored cell subtree.
///
/// WinUI does not ship a core DataGrid, and the old Community Toolkit DataGrid
/// is no longer a current dependency. The canonical path therefore keeps the
/// existing Grid/ItemsRepeater visuals but wraps the table and its realized
/// cells in component-scoped controls whose automation peers implement the
/// Table/Grid and TableItem/GridItem provider contracts.
#[derive(Clone, Copy)]
struct XamlNativeTableShape<'a> {
    head: &'a LayoutNode,
    body: &'a LayoutNode,
    header_slot: &'a str,
    rows_slot: &'a str,
}

fn xaml_native_table_shape(host_table: &LayoutNode) -> Option<XamlNativeTableShape<'_>> {
    if host_table.children.iter().any(|child| {
        !matches!(
            child.tag.as_str(),
            "HostTableColGroup" | "HostTableHead" | "HostTableBody"
        )
    }) {
        return None;
    }

    let mut heads = host_table
        .children
        .iter()
        .filter(|child| child.tag == "HostTableHead");
    let head = heads.next()?;
    if heads.next().is_some() {
        return None;
    }
    let mut bodies = host_table
        .children
        .iter()
        .filter(|child| child.tag == "HostTableBody");
    let body = bodies.next()?;
    if bodies.next().is_some() {
        return None;
    }

    let [header_row] = head.children.as_slice() else {
        return None;
    };
    if header_row.tag != "Row" {
        return None;
    }
    let [header_cells] = header_row.children.as_slice() else {
        return None;
    };
    if header_cells.tag != "For" || header_cells.children.len() != 1 {
        return None;
    }

    let [body_rows] = body.children.as_slice() else {
        return None;
    };
    if body_rows.tag != "For" {
        return None;
    }
    let [body_row] = body_rows.children.as_slice() else {
        return None;
    };
    if body_row.tag != "Row" {
        return None;
    }
    let [body_cells] = body_row.children.as_slice() else {
        return None;
    };
    if body_cells.tag != "For" || body_cells.children.len() != 1 {
        return None;
    }

    let header_slot = match find_prop_value(header_cells, "each")? {
        LayoutPropValue::SlotRef(slot) => slot.as_str(),
        _ => return None,
    };
    let rows_slot = match find_prop_value(body_rows, "each")? {
        LayoutPropValue::SlotRef(slot) => slot.as_str(),
        _ => return None,
    };
    let row_alias = find_prop_keyword(body_rows, "as")?;
    let inner_collection = match find_prop_value(body_cells, "each")? {
        LayoutPropValue::Keyword(binding) => binding.as_str(),
        _ => return None,
    };
    if inner_collection != row_alias {
        return None;
    }

    // Stable zero-based indices are required by IGridProvider/IGridItemProvider.
    for loop_node in [header_cells, body_rows, body_cells] {
        let alias = find_prop_keyword(loop_node, "as")?;
        let index = find_prop_keyword(loop_node, "index")?;
        if !is_safe_identifier(&kebab_to_pascal_case(alias))
            || !is_safe_identifier(&kebab_to_pascal_case(index))
        {
            return None;
        }
    }

    Some(XamlNativeTableShape {
        head,
        body,
        header_slot,
        rows_slot,
    })
}

/// Returns whether a HostTable has the canonical dynamic structure lowered to
/// component-scoped WinUI controls with native Table/Grid automation patterns.
/// Capability analysis calls this same predicate so reporting cannot drift
/// from the emitter's actual accessible path.
pub fn host_table_has_native_semantics(host_table: &LayoutNode) -> bool {
    xaml_native_table_shape(host_table).is_some()
}

fn emit_native_host_table(
    node: &LayoutNode,
    shape: XamlNativeTableShape<'_>,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let pad2 = " ".repeat(indent + 4);
    let style = part_style_attr(node, part_styles);
    let component = ctx.component_name;
    let table_type = format!("{component}MosaicTable");
    let header_path = ctx.slot_property_name(shape.header_slot);
    let rows_path = ctx.slot_property_name(shape.rows_slot);
    let flow_direction_attr = match find_prop_value(node, "dir") {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let property = ctx.slot_property_name(slot);
            if is_safe_identifier(&property) {
                format!(
                    " FlowDirection=\"{{x:Bind {}, Mode=OneWay}}\"",
                    ctx.slot_xbind_path(slot)
                )
            } else {
                String::new()
            }
        }
        Some(LayoutPropValue::Keyword(keyword)) => match keyword.as_str() {
            "rtl" => " FlowDirection=\"RightToLeft\"".to_string(),
            "ltr" => " FlowDirection=\"LeftToRight\"".to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    };
    let table_name = node
        .part_name
        .as_deref()
        .map(|part| format!("{} table", part.replace('-', " ")))
        .unwrap_or_else(|| "Data table".to_string());

    ctx.native_table_counter += 1;
    let table_id = ctx.native_table_counter;
    let header_helper = format!("MosaicTableHeader{table_id}");
    let cell_name_helper = format!("MosaicTableCellName{table_id}");
    ctx.add_helper(HelperMethod {
        name: header_helper.clone(),
        parameters: vec![("column".to_string(), "int".to_string())],
        return_type: "string".to_string(),
        body: format!(
            "{header_path} is {{ }} headers && column >= 0 && column < headers.Count \
             ? Convert.ToString(headers[column]) ?? $\"Column {{column + 1}}\" \
             : $\"Column {{column + 1}}\""
        ),
    });
    ctx.add_helper(HelperMethod {
        name: cell_name_helper.clone(),
        parameters: vec![
            ("header".to_string(), "string".to_string()),
            ("value".to_string(), "object?".to_string()),
            ("row".to_string(), "int".to_string()),
        ],
        return_type: "string".to_string(),
        body: "$\"{header}, row {row + 1}: {Convert.ToString(value) ?? string.Empty}\"".to_string(),
    });
    ctx.needs_native_table_support = true;

    let previous_table = ctx.native_table.take();
    let mut out = String::new();
    writeln!(
        out,
        "{pad}<local:{table_type} RowCount=\"{{x:Bind {rows_path}.Count, Mode=OneWay}}\" ColumnCount=\"{{x:Bind {header_path}.Count, Mode=OneWay}}\" AutomationProperties.Name=\"{}\"{flow_direction_attr}{style}>",
        escape_xaml_attr(&table_name)
    )
    .unwrap();
    writeln!(out, "{pad2}<Grid.RowDefinitions>").unwrap();
    writeln!(out, "{pad2}    <RowDefinition Height=\"Auto\"/>").unwrap();
    writeln!(out, "{pad2}    <RowDefinition Height=\"*\"/>").unwrap();
    writeln!(out, "{pad2}</Grid.RowDefinitions>").unwrap();

    ctx.native_table = Some(NativeTableEmission {
        role: NativeTableRole::Header,
        header_helper: header_helper.clone(),
        cell_name_helper: cell_name_helper.clone(),
        for_depth: 0,
    });
    out.push_str(&emit_host_table_section(
        shape.head,
        0,
        indent + 4,
        part_styles,
        ctx,
        false,
    )?);

    ctx.native_table = Some(NativeTableEmission {
        role: NativeTableRole::Body,
        header_helper,
        cell_name_helper,
        for_depth: 0,
    });
    out.push_str(&emit_host_table_section(
        shape.body,
        1,
        indent + 4,
        part_styles,
        ctx,
        true,
    )?);
    ctx.native_table = previous_table;

    writeln!(out, "{pad}</local:{table_type}>").unwrap();
    Ok(out)
}

/// `HostTable [name] { section sub-tags... }` per spec Â§5.
fn emit_host_table(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    ctx: &mut EmitContext<'_>,
) -> Result<String, PipelineEmitError> {
    if let Some(shape) = xaml_native_table_shape(node) {
        return emit_native_host_table(node, shape, indent, part_styles, ctx);
    }

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
            let property = ctx.slot_property_name(slot);
            if is_safe_identifier(&property) {
                format!(
                    " FlowDirection=\"{{x:Bind {}, Mode=OneWay}}\"",
                    ctx.slot_xbind_path(slot)
                )
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
                out.push_str(&emit_if(row, None, indent, part_styles, ctx, None)?);
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
                let property = ctx.slot_property_name(slot);
                if !is_safe_identifier(&property) {
                    return Err(PipelineEmitError::UnsafeSlotName(property));
                }
                attrs.push_str(&format!(
                    " {attr_name}=\"{{x:Bind {}, Mode=OneWay}}\"",
                    ctx.slot_xbind_path(slot)
                ));
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
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {pascal}, Mode=OneWay}}\""));
                } else {
                    attrs.push_str(&format!(" {attr_name}=\"{}\"", escape_xaml_attr(k)));
                }
            }
            LayoutPropValue::EmitRef(emit) => {
                emit_ref_skipped.push(format!("{}: emit: {}", prop.name, emit));
            }
            LayoutPropValue::Expr(src) => match lower_expr_for_xbind(src, ctx) {
                ExprLowering::Bindable(path) => {
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {path}, Mode=OneWay}}\""));
                }
                ExprLowering::Helper(call) => {
                    attrs.push_str(&format!(" {attr_name}=\"{{x:Bind {call}, Mode=OneWay}}\""));
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
    use mosstyle_compiler::{PartStyle, StateStyle, StyleDef, StyleProp, StyleTransition};

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
        assert!(
            !r.code_behind.contains("INotifyPropertyChanged"),
            "components without generated row projections should keep the lean code-behind shape"
        );
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
    fn row_lowers_to_horizontal_grid() {
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
        // An empty Row has no children, hence no ColumnDefinitions — the
        // bare wrapper is enough to prove `Row` no longer lowers to
        // `StackPanel` (mosaic-emit-xaml.md §3.1).
        assert!(r.xaml.contains("<Grid>"), "got:\n{}", r.xaml);
        assert!(!r.xaml.contains("StackPanel"), "got:\n{}", r.xaml);
    }

    #[test]
    fn column_lowers_to_vertical_grid() {
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
        assert!(r.xaml.contains("<Grid>"), "got:\n{}", r.xaml);
        assert!(!r.xaml.contains("StackPanel"), "got:\n{}", r.xaml);
    }

    #[test]
    fn row_with_children_gets_one_column_definition_per_child() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("a".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("b".to_string()),
                        }],
                        children: Vec::new(),
                    },
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains("<Grid.ColumnDefinitions>"),
            "got:\n{}",
            r.xaml
        );
        assert_eq!(
            r.xaml
                .matches("<ColumnDefinition Width=\"Auto\"/>")
                .count(),
            2,
            "expected one Auto ColumnDefinition per child, got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Grid.Column=\"0\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Grid.Column=\"1\""), "got:\n{}", r.xaml);
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
        // The Text should be nested under the Column under the Row. Both
        // Row and Column lower to `<Grid ...>` (mosaic-emit-xaml.md §3.1)
        // — the inner one carries its own `Grid.Column="0"` slot index, so
        // match on the open tag rather than the exact bare `<Grid>` form.
        let row_pos = r.xaml.find("<Grid>").unwrap();
        let col_pos = r.xaml[row_pos + 1..].find("<Grid ").unwrap() + row_pos + 1;
        let txt_pos = r.xaml.find("<TextBlock").unwrap();
        assert!(
            row_pos < col_pos && col_pos < txt_pos,
            "got:\n{}",
            r.xaml
        );
    }

    // ── §3.1 flex hints: flex-grow / main-axis 100% / justify-content /
    //    If-Else grid-slot sharing ──

    /// A child whose own style carries `flex-grow: 1` gets a `"*"`
    /// definition instead of `Auto` — the one real weight authored
    /// anywhere in the repo today (mosaic-emit-xaml.md §3.1).
    #[test]
    fn flex_grow_child_gets_star_column() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: Some("fixed".to_string()),
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("fixed".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: Some("grows".to_string()),
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("grows".to_string()),
                        }],
                        children: Vec::new(),
                    },
                ],
            },
        );
        let mut s = style_for_box("fixed", vec![]);
        s.parts
            .push(style_for_box("grows", vec![("flex-grow", "1")]).parts.remove(0));
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains(
                "<ColumnDefinition Width=\"Auto\"/>\n            <ColumnDefinition Width=\"*\"/>"
            ),
            "expected the first child Auto and the flex-grow child star, got:\n{}",
            r.xaml
        );
    }

    /// `width: 100%` on a `Row`'s direct child is flexbox's own "claim the
    /// remaining main-axis space" — treated identically to `flex-grow: 1`
    /// (mosaic-emit-xaml.md §3.1). `height: 100%` is the `Column` analog.
    #[test]
    fn main_axis_width_full_gets_star_column_like_flex_grow() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "Box".to_string(),
                    part_name: Some("filler".to_string()),
                    props: Vec::new(),
                    children: Vec::new(),
                }],
            },
        );
        let s = style_for_box("filler", vec![("width", "100%")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("<ColumnDefinition Width=\"*\"/>"),
            "got:\n{}",
            r.xaml
        );
    }

    /// `justify-content: space-between` inserts a `"*"` spacer between
    /// each pair of children — N children get N−1 spacers, none leading
    /// or trailing (mosaic-emit-xaml.md §3.1).
    #[test]
    fn justify_content_space_between_inserts_star_spacers_between_children() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: Some("toolbar".to_string()),
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("a".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("b".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("c".to_string()),
                        }],
                        children: Vec::new(),
                    },
                ],
            },
        );
        let s = style_for_box("toolbar", vec![("justify-content", "space-between")]);
        let r = compile(&c, &l, &s);
        // 3 real children + 2 spacers = 5 definitions; the 2 spacer slots
        // (indices 1 and 3) are the star ones, real children stay Auto.
        assert_eq!(
            r.xaml.matches("<ColumnDefinition").count(),
            5,
            "got:\n{}",
            r.xaml
        );
        assert_eq!(
            r.xaml
                .matches("<ColumnDefinition Width=\"*\"/>")
                .count(),
            2,
            "got:\n{}",
            r.xaml
        );
        assert!(r.xaml.contains("Grid.Column=\"0\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Grid.Column=\"2\""), "got:\n{}", r.xaml);
        assert!(r.xaml.contains("Grid.Column=\"4\""), "got:\n{}", r.xaml);
    }

    /// An `If`/`Else` pair is ONE logical grid slot even though `emit_if`
    /// emits two sibling `<ContentControl>`s for it (§6.2) — both must
    /// carry the SAME `Grid.Column`/`Grid.Row` index (mosaic-emit-xaml.md
    /// §3.1), since only one is ever visible at a time.
    #[test]
    fn if_else_pair_shares_one_grid_index() {
        let c = component("Foo", vec![slot("editable", SlotType::Bool, true)], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("above".to_string()),
                        }],
                        children: Vec::new(),
                    },
                    if_node(
                        LayoutPropValue::SlotRef("editable".to_string()),
                        vec![LayoutNode {
                            tag: "Text".to_string(),
                            part_name: None,
                            props: vec![LayoutProp {
                                name: "content".to_string(),
                                value: LayoutPropValue::String("then".to_string()),
                            }],
                            children: Vec::new(),
                        }],
                    ),
                    else_node(vec![LayoutNode {
                        tag: "Text".to_string(),
                        part_name: None,
                        props: vec![LayoutProp {
                            name: "content".to_string(),
                            value: LayoutPropValue::String("else".to_string()),
                        }],
                        children: Vec::new(),
                    }]),
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        // 2 logical slots (the plain Text, then the If/Else pair) — NOT 3.
        assert_eq!(
            r.xaml.matches("<RowDefinition").count(),
            2,
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "Converter={StaticResource BoolToVisibilityConverter}, Mode=OneWay}\" Grid.Row=\"1\">"
            ),
            "then-branch should carry Grid.Row=\"1\", got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "ConverterParameter=invert, Mode=OneWay}\" Grid.Row=\"1\">"
            ),
            "else-branch should carry Grid.Row=\"1\" too, got:\n{}",
            r.xaml
        );
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
    fn text_accessibility_metadata_lowers_to_automation_properties() {
        let c = component(
            "Title",
            vec![slot("spoken-title", SlotType::Text, true)],
            vec![],
        );
        let l = layout_with_root(
            "Title",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![
                    LayoutProp {
                        name: "content".to_string(),
                        value: LayoutPropValue::String("Visible title".to_string()),
                    },
                    LayoutProp {
                        name: "a11y-label".to_string(),
                        value: LayoutPropValue::SlotRef("spoken-title".to_string()),
                    },
                    LayoutProp {
                        name: "a11y-role".to_string(),
                        value: LayoutPropValue::Keyword("heading".to_string()),
                    },
                ],
                children: Vec::new(),
            },
        );
        let out = compile(&c, &l, &empty_style("Title")).xaml;
        assert!(out.contains("AutomationProperties.Name=\"{x:Bind SpokenTitle, Mode=OneWay}\""));
        assert!(out.contains("AutomationProperties.HeadingLevel=\"Level2\""));

        let hidden = layout_with_root(
            "Hidden",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "a11y-role".to_string(),
                    value: LayoutPropValue::Keyword("none".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let hidden_out = compile(
            &component("Hidden", vec![], vec![]),
            &hidden,
            &empty_style("Hidden"),
        )
        .xaml;
        assert!(hidden_out.contains("AutomationProperties.AccessibilityView=\"Raw\""));
    }

    #[test]
    fn styled_text_moves_box_paint_to_border() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Text".to_string(),
                part_name: Some("pill".to_string()),
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::String("Ready".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let s = style_for_box(
            "pill",
            vec![
                ("background", "#22301f"),
                ("padding", "4"),
                ("border-radius", "20"),
                ("color", "#6fb489"),
                ("font-size", "12"),
            ],
        );
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<Border Background=\"#22301f\" Padding=\"4\" CornerRadius=\"20\">"),
            "box paint must live on a valid WinUI Border:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<TextBlock Text=\"Ready\" Foreground=\"#6fb489\" FontSize=\"12\"/>"),
            "typography must remain on the TextBlock:\n{}",
            r.xaml
        );
        assert!(!r.xaml.contains("<TextBlock Text=\"Ready\" Background="));
        assert!(!r.xaml.contains("<TextBlock Text=\"Ready\" CornerRadius="));
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
            r.xaml
                .contains("<TextBlock Text=\"{x:Bind Greeting, Mode=OneWay}\""),
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
        assert!(r.xaml.contains("{x:Bind DisplayName, Mode=OneWay}"));
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

    // ── issue #12025: base_fragment style values escape at every
    //    consumption path, not just literal Text content ──

    /// A hostile mosstyle value carrying all four XML-significant
    /// characters. Its escaped form must appear literally in the output;
    /// none of `"`/`<`/`>`/`&` may appear raw — a raw `"` in particular
    /// would terminate the attribute early and let the rest inject markup.
    const HOSTILE_STYLE_VALUE: &str = "foo\"bar<baz>&qux";
    const ESCAPED_HOSTILE_STYLE_VALUE: &str = "foo&quot;bar&lt;baz&gt;&amp;qux";

    fn assert_hostile_value_escaped_not_injected(xaml: &str) {
        assert!(
            xaml.contains(ESCAPED_HOSTILE_STYLE_VALUE),
            "expected the escaped value to appear literally, got:\n{xaml}"
        );
        assert!(
            !xaml.contains(HOSTILE_STYLE_VALUE),
            "the raw, unescaped hostile value must never appear, got:\n{xaml}"
        );
    }

    /// `partition_box_style` → `emit_container`'s `<Border>` path.
    #[test]
    fn box_style_escapes_hostile_value() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("card".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = style_for_box("card", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let r = compile(&c, &l, &s);
        assert_hostile_value_escaped_not_injected(&r.xaml);
    }

    /// `partition_flex_grid_style` (issue #12021's `<Grid>` lowering).
    #[test]
    fn row_style_escapes_hostile_value() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Row".to_string(),
                part_name: Some("toolbar".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = style_for_box("toolbar", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let r = compile(&c, &l, &s);
        assert_hostile_value_escaped_not_injected(&r.xaml);
    }

    /// `content_control_style_attr` (`HostButton`'s `<Button>` lowering).
    #[test]
    fn host_button_style_escapes_hostile_value() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "HostButton".to_string(),
                part_name: Some("submit".to_string()),
                props: vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::String("Submit".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let s = style_for_box("submit", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let r = compile(&c, &l, &s);
        assert_hostile_value_escaped_not_injected(&r.xaml);
    }

    /// `part_style_attr`'s whole-fragment raw splice — the path used by
    /// `Image`/`Spacer`/`Divider` and 14 other primitives that never call
    /// `parse_style_fragment` at all.
    #[test]
    fn image_style_escapes_hostile_value() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Image".to_string(),
                part_name: Some("thumb".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = style_for_box("thumb", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let r = compile(&c, &l, &s);
        assert_hostile_value_escaped_not_injected(&r.xaml);
    }

    /// `drag_control_style_attr` (`HostDraggable`/`HostDropTarget`'s
    /// `<ContentControl>` lowering) — direct unit test rather than full
    /// emission, since `HostDraggable` needs several required props
    /// (`drag-key`, etc.) unrelated to what this is testing.
    #[test]
    fn drag_control_style_attr_escapes_hostile_value() {
        let style = style_for_box("handle", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let part_styles = build_part_style_map(&style);
        let node = LayoutNode {
            tag: "HostDraggable".to_string(),
            part_name: Some("handle".to_string()),
            props: Vec::new(),
            children: Vec::new(),
        };
        let (attrs, _spacing) = drag_control_style_attr(&node, &part_styles);
        assert_hostile_value_escaped_not_injected(&attrs);
    }

    /// `partition_stack_panel_style` (still used by the `HostTable` row-
    /// section emitter, `emit_host_table_rows` — table rows aren't flex
    /// containers, so they didn't move to `partition_flex_grid_style` in
    /// #12021). Direct unit test for the same reason as the one above.
    #[test]
    fn partition_stack_panel_style_escapes_hostile_value() {
        let style = style_for_box("row", vec![("opacity", HOSTILE_STYLE_VALUE)]);
        let part_styles = build_part_style_map(&style);
        let (wrapper, stack, _text) = partition_stack_panel_style(Some("row"), &part_styles);
        assert_hostile_value_escaped_not_injected(&format!("{wrapper}{stack}"));
    }

    /// `parse_style_fragment` no longer needs (or has) backslash-escape
    /// handling — confirm a value containing a literal backslash round-
    /// trips as itself (backslash isn't XML-significant, so
    /// `escape_xaml_attr` passes it through unchanged).
    #[test]
    fn style_value_with_literal_backslash_round_trips_unchanged() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("path".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = style_for_box("path", vec![("background", "C:\\images\\bg.png")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("Background=\"C:\\images\\bg.png\""),
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
            r.code_behind.contains("internal string Expr_"),
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
        assert!(r.xaml.contains("<Image Source=\"{x:Bind AvatarUrl, Mode=OneWay}\""));
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
            r.xaml.contains("Glyph=\"{x:Bind GlyphName, Mode=OneWay}\""),
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

    #[test]
    fn drag_family_emits_native_pointer_keyboard_and_accessibility_runtime() {
        let text_param = |name: &str| param(name, EmitPayloadType::Text);
        let c = component(
            "Board",
            vec![
                slot("drag-key", SlotType::Text, true),
                slot("drag-kind", SlotType::Text, true),
                slot("drag-label", SlotType::Text, true),
                slot("drag-disabled", SlotType::Bool, true),
                slot(
                    "accepted-kinds",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
            ],
            vec![
                emit("onDragStart", vec![text_param("key"), text_param("kind")]),
                emit(
                    "onDragEnd",
                    vec![
                        text_param("key"),
                        text_param("kind"),
                        param("dropped", EmitPayloadType::Bool),
                    ],
                ),
                emit("onDragEnter", vec![text_param("key"), text_param("kind")]),
                emit("onDragLeave", vec![text_param("key"), text_param("kind")]),
                emit(
                    "onDropHover",
                    vec![
                        text_param("key"),
                        text_param("kind"),
                        text_param("targetKey"),
                        text_param("position"),
                    ],
                ),
                emit(
                    "onDrop",
                    vec![
                        text_param("key"),
                        text_param("kind"),
                        text_param("targetKey"),
                        text_param("position"),
                    ],
                ),
            ],
        );
        let prop = |name: &str, value: LayoutPropValue| LayoutProp {
            name: name.to_string(),
            value,
        };
        let draggable = LayoutNode {
            tag: "HostDraggable".to_string(),
            part_name: Some("card".to_string()),
            props: vec![
                prop("drag-key", LayoutPropValue::SlotRef("drag-key".to_string())),
                prop(
                    "drag-kind",
                    LayoutPropValue::SlotRef("drag-kind".to_string()),
                ),
                prop(
                    "drag-label",
                    LayoutPropValue::SlotRef("drag-label".to_string()),
                ),
                prop(
                    "drag-disabled",
                    LayoutPropValue::SlotRef("drag-disabled".to_string()),
                ),
                prop(
                    "onDragStart",
                    LayoutPropValue::EmitRef("onDragStart".to_string()),
                ),
                prop(
                    "onDragEnd",
                    LayoutPropValue::EmitRef("onDragEnd".to_string()),
                ),
            ],
            children: vec![
                row_with_text_cells(&["Card"]),
                row_with_text_cells(&["Details"]),
            ],
        };
        let target = LayoutNode {
            tag: "HostDropTarget".to_string(),
            part_name: Some("lane".to_string()),
            props: vec![
                prop("drop-key", LayoutPropValue::String("lane-a".to_string())),
                prop(
                    "accepts",
                    LayoutPropValue::SlotRef("accepted-kinds".to_string()),
                ),
                prop(
                    "onDragEnter",
                    LayoutPropValue::EmitRef("onDragEnter".to_string()),
                ),
                prop(
                    "onDragLeave",
                    LayoutPropValue::EmitRef("onDragLeave".to_string()),
                ),
                prop(
                    "onDropHover",
                    LayoutPropValue::EmitRef("onDropHover".to_string()),
                ),
                prop("onDrop", LayoutPropValue::EmitRef("onDrop".to_string())),
            ],
            children: vec![draggable],
        };

        let style = StyleDef {
            component_name: "Board".to_string(),
            parts: vec![PartStyle {
                name: "card".to_string(),
                base: vec![StyleProp {
                    name: "gap".to_string(),
                    value: "6".to_string(),
                }],
                transitions: Vec::new(),
                states: Vec::new(),
            }],
        };
        let result = compile(&c, &layout_with_root("Board", target), &style);
        for expected in [
            "<local:BoardMosaicDropTarget",
            "AllowDrop = true",
            "DragEnter += OnDragEnter",
            "DragOver += OnDragOver",
            "Drop += OnDrop",
            "<local:BoardMosaicDragSource",
            "CanDrag = true",
            "DragStarting += OnDragStarting",
            "DropCompleted += OnDropCompleted",
            "case VirtualKey.Space",
            "case VirtualKey.Escape",
            "AutomationNotificationKind.ActionCompleted",
            "ConditionalWeakTable<Board, BoardMosaicDragScope>",
            "_keyboardTarget.Accept(source, \"into\", keyboard: true)",
            "Accept(source, position, keyboard: false)",
            "args.Data.SetData(BoardMosaicDragRuntime.Format, DragKey)",
            "new BoardEvent.Drop(args.Key, args.Kind, args.TargetKey, args.Position)",
            "new BoardEvent.DragEnd(args.Key, args.Kind, args.Dropped)",
        ] {
            assert!(
                result.xaml.contains(expected) || result.code_behind.contains(expected),
                "missing {expected:?}\nXAML:\n{}\nC#:\n{}",
                result.xaml,
                result.code_behind
            );
        }
        assert!(result
            .xaml
            .contains("Accepts=\"{x:Bind AcceptedKinds, Mode=OneWay}\""));
        assert!(result
            .xaml
            .contains("DragDisabled=\"{x:Bind DragDisabled, Mode=OneWay}\""));
        assert!(result
            .xaml
            .contains("<StackPanel Orientation=\"Vertical\" Spacing=\"6\">"));
        let source_open = result
            .xaml
            .lines()
            .find(|line| line.contains("<local:BoardMosaicDragSource"))
            .expect("drag source opening tag");
        assert!(!source_open.contains("Spacing="));
    }

    #[test]
    fn non_drag_component_omits_drag_runtime() {
        let result = compile(
            &component("Plain", vec![], vec![]),
            &layout_with_root("Plain", box_root()),
            &empty_style("Plain"),
        );
        assert!(!result.code_behind.contains("MosaicDragRuntime"));
        assert!(!result.code_behind.contains("DataTransfer"));
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

    fn canonical_native_table_node() -> LayoutNode {
        host_table_node(
            Some("sheet"),
            vec![
                section_node(
                    "HostTableHead",
                    vec![LayoutNode {
                        tag: "Row".to_string(),
                        part_name: None,
                        props: Vec::new(),
                        children: vec![for_node(
                            LayoutPropValue::SlotRef("headers".to_string()),
                            "header",
                            Some("header-index"),
                            vec![row_with_text_cells(&["heading"])],
                        )],
                    }],
                ),
                section_node(
                    "HostTableBody",
                    vec![for_node(
                        LayoutPropValue::SlotRef("rows".to_string()),
                        "row",
                        Some("row-index"),
                        vec![LayoutNode {
                            tag: "Row".to_string(),
                            part_name: None,
                            props: Vec::new(),
                            children: vec![for_node(
                                LayoutPropValue::Keyword("row".to_string()),
                                "cell",
                                Some("column-index"),
                                vec![row_with_text_cells(&["value"])],
                            )],
                        }],
                    )],
                ),
            ],
        )
    }

    #[test]
    fn native_table_shape_and_emission_are_conservative() {
        let canonical = canonical_native_table_node();
        assert!(host_table_has_native_semantics(&canonical));

        let c = component(
            "Sheet",
            vec![
                slot(
                    "headers",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                ),
                slot(
                    "rows",
                    SlotType::List(Box::new(ListInnerType::List(Box::new(ListInnerType::Text)))),
                    true,
                ),
            ],
            vec![],
        );
        let r = compile(
            &c,
            &layout_with_root("Sheet", canonical.clone()),
            &empty_style("Sheet"),
        );
        assert!(r.xaml.contains("<local:SheetMosaicTable "));
        assert!(r.xaml.contains("<local:SheetMosaicTableHeaderCell "));
        assert!(r.xaml.contains("<local:SheetMosaicTableCell "));
        assert!(r.code_behind.contains("IGridProvider, ITableProvider"));
        assert!(r
            .code_behind
            .contains("IGridItemProvider, ITableItemProvider"));

        let mut with_foot = canonical.clone();
        with_foot
            .children
            .push(section_node("HostTableFoot", Vec::new()));
        assert!(!host_table_has_native_semantics(&with_foot));

        let mut missing_index = canonical.clone();
        missing_index.children[0].children[0].children[0]
            .props
            .retain(|prop| prop.name != "index");
        assert!(!host_table_has_native_semantics(&missing_index));

        let mut wrong_inner_collection = canonical;
        let inner = &mut wrong_inner_collection.children[1].children[0].children[0].children[0];
        inner
            .props
            .iter_mut()
            .find(|prop| prop.name == "each")
            .expect("inner each prop")
            .value = LayoutPropValue::Keyword("other-row".to_string());
        assert!(!host_table_has_native_semantics(&wrong_inner_collection));
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
            .contains("<ItemsRepeater ItemsSource=\"{x:Bind GridRowVmRows, Mode=OneWay}\""));
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
                transitions: vec![],
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
                .contains("FlowDirection=\"{x:Bind LayoutDirection, Mode=OneWay}\""),
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
            r.xaml.contains("Rows=\"{x:Bind Rows, Mode=OneWay}\""),
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

    fn legacy_input_node(part: Option<&str>, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: "Input".to_string(),
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
        assert!(
            r.xaml
                .contains("AutomationProperties.AutomationId=\"formula-field\""),
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
    fn legacy_multiline_input_uses_native_textbox_contract() {
        let c = component("Notes", vec![], vec![]);
        let l = layout_with_root(
            "Notes",
            legacy_input_node(
                Some("notes-body-input"),
                vec![
                    LayoutProp {
                        name: "placeholder".to_string(),
                        value: LayoutPropValue::String("Write something…".to_string()),
                    },
                    LayoutProp {
                        name: "multiline".to_string(),
                        value: LayoutPropValue::Keyword("true".to_string()),
                    },
                ],
            ),
        );
        let r = compile(&c, &l, &empty_style("Notes"));
        assert!(r.xaml.contains("<TextBox"), "got:\n{}", r.xaml);
        assert!(
            r.xaml
                .contains("AcceptsReturn=\"True\" TextWrapping=\"Wrap\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("AutomationProperties.AutomationId=\"notes-body-input\""),
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

    #[test]
    fn host_input_commit_supplies_required_text_payload() {
        let c = component(
            "Foo",
            vec![],
            vec![emit(
                "onCommit",
                vec![param("value", EmitPayloadType::Text)],
            )],
        );
        let l = layout_with_root(
            "Foo",
            host_input_node(
                Some("formula-field"),
                vec![LayoutProp {
                    name: "onCommit".to_string(),
                    value: LayoutPropValue::EmitRef("onCommit".to_string()),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.code_behind.contains("FooEvent.Commit(tb.Text)"),
            "HostInput commit must forward its controlled value:\n{}",
            r.code_behind
        );
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
        assert!(
            r.xaml
                .contains("AutomationProperties.AutomationId=\"submit\""),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_button_maps_text_alignment_to_content_alignment() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root("Foo", host_button_node(Some("submit"), Vec::new()));
        let s = style_for_box("submit", vec![("text-align", "left")]);
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("HorizontalContentAlignment=\"Left\""),
            "Button content alignment must use the WinUI ContentControl property:\n{}",
            r.xaml
        );
        assert!(!r.xaml.contains("TextAlignment=\"Left\""));
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
            r.xaml.contains("Content=\"{x:Bind ButtonLabel, Mode=OneWay}\""),
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
            r.xaml.contains("Content=\"{x:Bind Item, Mode=OneWay}\""),
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
            r.xaml.contains("Content=\"{x:Bind Option, Mode=OneWay}\""),
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
            r.xaml
                .contains("IsEnabled=\"{x:Bind Not(IsBusy), Mode=OneWay}\""),
            "got:\n{}",
            r.xaml
        );
        // The Not(bool) helper should be in the code-behind.
        assert!(
            r.code_behind.contains("internal bool Not(bool b) => !b;"),
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
                .contains("<ItemsRepeater ItemsSource=\"{x:Bind GridRowVmRows, Mode=OneWay}\""),
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
        assert!(r.xaml.contains("Text=\"{x:Bind Row, Mode=OneWay}\""), "got:\n{}", r.xaml);
    }

    #[test]
    fn for_template_routes_component_slots_through_row_owner() {
        let c = component(
            "Grid",
            vec![
                slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true),
                slot("title", SlotType::Text, true),
            ],
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
                        value: LayoutPropValue::SlotRef("title".to_string()),
                    }],
                    children: Vec::new(),
                }],
            ),
        );
        let r = compile(&c, &l, &empty_style("Grid"));
        assert!(
            r.xaml.contains("Text=\"{x:Bind Owner.Title, Mode=OneWay}\""),
            "component slots inside a typed template must route through Owner:\n{}",
            r.xaml
        );
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
            .contains("public sealed record Grid_RowVm(Grid Owner, string Row);"));
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
                .contains("public sealed record Grid_RowVm(Grid Owner, string Row, int Index);"),
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
                .contains("public sealed record Stats_VVm(Stats Owner, double V);"),
            "got:\n{}",
            r.for_view_models[0].source
        );
    }

    #[test]
    fn for_disambiguates_reused_aliases_within_one_component() {
        // Package composition can place unrelated loops with the same short
        // alias in one component. They need distinct VM and projection types
        // so each ItemsRepeater keeps its own source and row shape.
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
        assert_eq!(r.for_view_models.len(), 2);
        assert!(r
            .for_view_models
            .iter()
            .any(|vm| vm.filename == "Grid_RowVm.cs"));
        assert!(r
            .for_view_models
            .iter()
            .any(|vm| vm.filename == "Grid_Row2Vm.cs"));
        assert!(r.xaml.contains("GridRowVmRows"));
        assert!(r.xaml.contains("GridRow2VmRows"));
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
        // The `If` is the Column's one child slot, so it carries that
        // slot's `Grid.Row="0"` attached property (mosaic-emit-xaml.md
        // §3.1) alongside its own Visibility binding.
        assert!(
            r.xaml
                .contains("<ContentControl Visibility=\"{x:Bind Editable, Converter={StaticResource BoolToVisibilityConverter}, Mode=OneWay}\" Grid.Row=\"0\">"),
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
        let helper = r
            .if_helpers
            .iter()
            .find(|file| file.filename == "BoolToVisibilityConverter.cs")
            .expect("visibility converter helper");
        assert!(
            helper.source.contains("string text => text.Length != 0")
                && helper.source.contains("double number => number != 0")
                && helper
                    .source
                    .contains("ICollection collection => collection.Count != 0"),
            "visibility conversion must preserve Mosaic value truthiness:\n{}",
            helper.source
        );
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
            ExprLowering::Helper(call) => {
                // This low-level fixture has no registered RowVm, so the
                // helper remains a call. Full For lowering projects it onto
                // the generated RowVm; see the end-to-end test below.
                assert!(call.starts_with("Expr_"));
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
                transitions: vec![],
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

    /// Scan emitted XAML for any `{x:Bind ...}` that does not declare an
    /// explicit `Mode`.
    ///
    /// This is the guard that matters. `x:Bind` defaults to **OneTime**, so a
    /// binding emitted without a mode renders once and then never updates
    /// again. Because each emission site used to choose its own mode by hand,
    /// coverage drifted: `Text=` and `Content=` had `Mode=OneWay` while
    /// `Visibility=` did not, which silently froze every conditional surface
    /// in a generated app while the engine behind it worked perfectly.
    ///
    /// Returns the offending binding substrings so a failure names them.
    fn xbinds_missing_mode(xaml: &str) -> Vec<String> {
        let mut missing = Vec::new();
        let mut rest = xaml;
        while let Some(start) = rest.find("{x:Bind ") {
            // A binding may embed a nested markup extension (a Converter),
            // so match braces rather than scanning to the first '}'.
            let tail = &rest[start..];
            let mut depth = 0usize;
            let mut end = None;
            for (i, ch) in tail.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            let binding = &tail[..end];
            if !binding.contains("Mode=") {
                missing.push(binding.to_string());
            }
            rest = &tail[end..];
        }
        missing
    }

    /// Every `x:Bind` emission site in this file must declare its mode.
    ///
    /// This scans the emitter's own source rather than emitted output,
    /// deliberately. A fixture-based test only covers the sites its fixture
    /// happens to reach — the first version of this test exercised a Text and
    /// an If, both already correct, and passed while roughly twenty other
    /// sites still emitted mode-less bindings. Scanning the source is the
    /// only form that actually holds the invariant.
    #[test]
    fn every_xbind_emission_site_declares_its_mode() {
        let src = include_str!("pipeline.rs");
        let mut offenders = Vec::new();

        // Scan only the emitter, not this test module. Tests legitimately
        // pass mode-less `{x:Bind …}` strings as *input* (translate_xaml_value
        // cases) and assert on pre-existing output shapes; neither is an
        // emission site. Cutting at the module boundary is exact, where
        // string filters were not.
        let emitter_src = match src.find("#[cfg(test)]") {
            Some(i) => &src[..i],
            None => src,
        };

        for (n, line) in emitter_src.lines().enumerate() {
            let trimmed = line.trim();
            // Skip prose — only emission code counts.
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            // The format-string form the emitter uses: `{{x:Bind ...}}`.
            //
            // Walk `{{`/`}}` pairs rather than stopping at the first `}}` —
            // a binding may embed a nested markup extension, as the If
            // lowering does with `Converter={{StaticResource ...}}`, and
            // `Mode=` sits *after* that nested close.
            for (idx, _) in line.match_indices("{{x:Bind ") {
                let rest = &line[idx..];
                let bytes = rest.as_bytes();
                let mut depth = 0usize;
                let mut end = None;
                let mut i = 0usize;
                while i + 1 < bytes.len() {
                    if bytes[i] == b'{' && bytes[i + 1] == b'{' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'}' && bytes[i + 1] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i);
                            break;
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                // An unterminated binding means the format string wraps
                // across lines; the closing line carries the mode, so skip.
                let Some(end) = end else { continue };
                if !rest[..end].contains("Mode=") {
                    offenders.push(format!("line {}: {}", n + 1, trimmed));
                }
            }

            // Some sites emit a literal (non-format) string, so the braces
            // are single rather than doubled — `inject_attr_into_first_element`
            // is the one that does this today. Those need the same guard;
            // missing it is how the GROUP C `Width` binding stayed OneTime
            // while its own doc comment claimed otherwise.
            for (idx, _) in line.match_indices("{x:Bind ") {
                // Skip the doubled form already handled above.
                if idx > 0 && line.as_bytes()[idx - 1] == b'{' {
                    continue;
                }
                let rest = &line[idx..];
                let Some(end) = rest.find('}') else { continue };
                if !rest[..end].contains("Mode=") {
                    offenders.push(format!("line {}: {}", n + 1, trimmed));
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "these x:Bind emission sites declare no Mode, so WinUI defaults \
             them to OneTime and whatever they render freezes after the first \
             pass:\n{}",
            offenders.join("\n")
        );
    }

    /// `width: 100%` must become an alignment, not vanish.
    ///
    /// CSS treats full-width as a sizing property; XAML treats it as an
    /// alignment, because WinUI's `Width` is an absolute `Double` with no
    /// percentage form. Translating the value alone therefore could not
    /// express it, and the property was dropped outright — leaving the
    /// element to size itself to its content, which is a large part of why
    /// generated apps hug the top-left of an empty window.
    #[test]
    fn full_width_and_height_become_stretch_alignments() {
        assert_eq!(
            stretch_alignment_for("Width", "100%"),
            Some("HorizontalAlignment")
        );
        assert_eq!(
            stretch_alignment_for("Height", "100%"),
            Some("VerticalAlignment")
        );
        // Tolerate incidental whitespace from the stylesheet.
        assert_eq!(
            stretch_alignment_for("Width", " 100% "),
            Some("HorizontalAlignment")
        );

        // Other percentages need proportional (star) sizing, which is a
        // different change — they must NOT be silently approximated to
        // "fill the parent".
        assert_eq!(stretch_alignment_for("Width", "50%"), None);
        assert_eq!(stretch_alignment_for("Height", "33%"), None);
        // Absolute lengths are unaffected.
        assert_eq!(stretch_alignment_for("Width", "34"), None);
        // Only the two size setters map this way.
        assert_eq!(stretch_alignment_for("FontSize", "100%"), None);
        assert_eq!(stretch_alignment_for("Padding", "100%"), None);
    }

    /// `min-height: 100vh` is how a web shell says "fill the window", and it
    /// was reaching XAML verbatim as `MinHeight="100vh"` — not a Double, so
    /// the generated shell silently lost its full-window height. This is the
    /// single declaration the whole TaskApp shell relies on for its height.
    #[test]
    fn viewport_units_become_stretch_alignments() {
        assert_eq!(
            stretch_alignment_for("MinHeight", "100vh"),
            Some("VerticalAlignment")
        );
        assert_eq!(
            stretch_alignment_for("Height", "100vh"),
            Some("VerticalAlignment")
        );
        assert_eq!(
            stretch_alignment_for("MinWidth", "100vw"),
            Some("HorizontalAlignment")
        );
        // A viewport unit that is not the full viewport is proportional
        // sizing, not a stretch — it must not be approximated.
        assert_eq!(stretch_alignment_for("MinHeight", "50vh"), None);
    }

    /// CSS units XAML cannot parse must be refused, not emitted.
    ///
    /// WinUI lengths are `Double`. Before this, only `px` and `%` were
    /// handled and every other unit fell through into the attribute, so the
    /// failure was silent at build time and only visible as a mis-laid-out
    /// window.
    #[test]
    fn unsupported_length_units_are_refused() {
        for bad in ["100vh", "50vw", "2em", "1.5rem", "10ch", "3pt"] {
            assert!(
                has_unsupported_length_unit(bad),
                "{bad} should be refused as a XAML length"
            );
            assert_eq!(
                translate_xaml_value("MinHeight", bad),
                None,
                "{bad} must not reach the emitted attribute"
            );
        }
        // Plain numbers and px are fine.
        for ok in ["100", "12.5", "34px", " 8 "] {
            assert!(
                !has_unsupported_length_unit(ok),
                "{ok} should be accepted as a XAML length"
            );
        }
        // Keywords are not lengths and must not be mistaken for one.
        assert!(!has_unsupported_length_unit("Auto"));
        assert!(!has_unsupported_length_unit("Stretch"));
    }

    /// End-to-end: a part declaring `width: 100%` emits a stretch alignment
    /// rather than nothing at all.
    #[test]
    fn part_with_full_width_emits_stretch_not_nothing() {
        let c = component("Foo", vec![], vec![]);
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: Some("panel".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = StyleDef {
            component_name: "Foo".to_string(),
            parts: vec![PartStyle {
                name: "panel".to_string(),
                base: vec![StyleProp {
                    name: "width".to_string(),
                    value: "100%".to_string(),
                }],
                transitions: Vec::new(),
                states: Vec::new(),
            }],
        };
        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("HorizontalAlignment=\"Stretch\""),
            "expected a stretch alignment, got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("Width=\"100%\""),
            "must not emit a percentage as an absolute Width:\n{}",
            r.xaml
        );
    }

    /// A labelled control whose label is a row expression must emit `Content`.
    ///
    /// `label: ( row[1] )` lands on `LayoutPropValue::Expr`, and that arm was
    /// missing from the label match — the catch-all swallowed it, so the
    /// control emitted no `Content` attribute at all and rendered blank. It
    /// looked like a styling gap for weeks because an empty control is
    /// invisible rather than obviously broken; it affected every task row,
    /// project-rail row and notes row.
    ///
    /// The gap was first found and fixed on `HostButton`. The sibling label
    /// matches in the `HostCheckbox`, `HostRadio` and `HostLink` lowerings
    /// had the exact same shape and the exact same missing arm, so this
    /// guard covers all four content controls rather than just the one that
    /// was noticed first. The general assertion — that no emitted content
    /// control lacks `Content` — is the half that matters: it is what
    /// catches the next element to grow a label match without an `Expr` arm.
    ///
    /// `HostLink` differed only in its symptom: its catch-all is not empty,
    /// it falls back to `href`, so an expression label rendered the raw URL
    /// (string href) or nothing at all (slot/expression href) rather than
    /// the label the author asked for.
    #[test]
    fn labelled_hosts_with_row_expression_label_emit_content() {
        /// The XAML tags whose visible text comes from `Content`. A control
        /// of one of these types with no `Content` attribute renders blank.
        const CONTENT_CONTROLS: [&str; 4] =
            ["Button", "CheckBox", "RadioButton", "HyperlinkButton"];

        fn labelled(tag: &str) -> LayoutNode {
            LayoutNode {
                tag: tag.to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "label".to_string(),
                    value: LayoutPropValue::Expr("row[1]".to_string()),
                }],
                children: Vec::new(),
            }
        }

        let c = component(
            "Foo",
            vec![slot(
                "rows",
                // list<list<text>> — a list of rows, each a list of cells,
                // which is the shape task-rows actually has.
                SlotType::List(Box::new(ListInnerType::List(Box::new(
                    ListInnerType::Text,
                )))),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![for_node(
                    LayoutPropValue::SlotRef("rows".to_string()),
                    "row",
                    None,
                    vec![
                        labelled("HostButton"),
                        labelled("HostCheckbox"),
                        labelled("HostRadio"),
                        labelled("HostLink"),
                    ],
                )],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));

        // Every emitted content control must carry a Content attribute — a
        // control that renders empty is the failure mode this guards against.
        for line in r.xaml.lines() {
            let t = line.trim();
            for tag in CONTENT_CONTROLS {
                if t.starts_with(&format!("<{tag} ")) {
                    assert!(
                        t.contains("Content="),
                        "emitted a <{tag}> with no Content, which renders blank:\n{t}\n\nfull XAML:\n{}",
                        r.xaml
                    );
                }
            }
        }

        // ...and each of the three must specifically have lowered the row
        // expression to a binding, not to some empty placeholder.
        let bindings = r.xaml.matches("Content=\"{x:Bind").count();
        assert_eq!(
            bindings,
            CONTENT_CONTROLS.len(),
            "expected every row-expression label to lower to a binding, got {bindings}:\n{}",
            r.xaml
        );
    }

    /// #12126: seven more sites had the identical shape as the label
    /// matches above — a bare `_ => {}` catch-all silently swallowing
    /// `LayoutPropValue::Expr` — but for different attributes:
    /// `HostTooltip.text`, `HostInput.value`/`.placeholder`,
    /// `HostNumberInput.value`, `HostDialog.title`, `a11y-label` on
    /// `Text`/`HostSlider`, and `Image.src`. Each is fixed here by
    /// routing through the same `lower_expr_for_xbind` helper the label
    /// sites use, and each match is now exhaustive over every
    /// `LayoutPropValue` variant (no `_`), so a future 7th variant is a
    /// compile error at these sites instead of silent runtime blankness.
    #[test]
    fn row_expression_valued_props_bind_at_every_remaining_drop_site() {
        fn expr_prop(name: &str) -> LayoutProp {
            LayoutProp {
                name: name.to_string(),
                value: LayoutPropValue::Expr("row[1]".to_string()),
            }
        }

        let c = component(
            "Foo",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::List(Box::new(
                    ListInnerType::Text,
                )))),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![for_node(
                    LayoutPropValue::SlotRef("rows".to_string()),
                    "row",
                    None,
                    vec![
                        LayoutNode {
                            tag: "HostTooltip".to_string(),
                            part_name: None,
                            props: vec![expr_prop("text")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "HostInput".to_string(),
                            part_name: None,
                            props: vec![expr_prop("value"), expr_prop("placeholder")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "HostNumberInput".to_string(),
                            part_name: None,
                            props: vec![expr_prop("value")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "HostDialog".to_string(),
                            part_name: None,
                            props: vec![expr_prop("title")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "Text".to_string(),
                            part_name: None,
                            props: vec![expr_prop("a11y-label")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "HostSlider".to_string(),
                            part_name: None,
                            props: vec![expr_prop("a11y-label")],
                            children: Vec::new(),
                        },
                        LayoutNode {
                            tag: "Image".to_string(),
                            part_name: None,
                            props: vec![expr_prop("src")],
                            children: Vec::new(),
                        },
                    ],
                )],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));

        for (site, needle) in [
            ("HostTooltip.text", " ToolTipService.ToolTip=\"{x:Bind"),
            ("HostInput.value", " Text=\"{x:Bind"),
            ("HostInput.placeholder", " PlaceholderText=\"{x:Bind"),
            ("HostNumberInput.value", " Value=\"{x:Bind"),
            ("HostDialog.title", " Title=\"{x:Bind"),
            ("Image.src", " Source=\"{x:Bind"),
        ] {
            assert!(
                r.xaml.contains(needle),
                "{site}: expected a row-expression value to lower to an \
                 x:Bind, found none. full XAML:\n{}",
                r.xaml
            );
        }

        // Both `Text.a11y-label` and `HostSlider.a11y-label` share the
        // same target attribute — assert both fired, not just one.
        let a11y_bindings = r
            .xaml
            .matches(" AutomationProperties.Name=\"{x:Bind")
            .count();
        assert_eq!(
            a11y_bindings, 2,
            "expected both Text and HostSlider a11y-label to bind, got {a11y_bindings}:\n{}",
            r.xaml
        );
    }

    /// #12126 audit finding (not in the issue's own list, found while
    /// fixing the neighbouring `Expr` gap): `HostInput.placeholder` had
    /// no `SlotRef` arm at all — a slot-valued placeholder silently
    /// emitted no `PlaceholderText` attribute, the same invisible-drop
    /// failure mode as the `Expr` sites, just via a different variant.
    #[test]
    fn host_input_placeholder_binds_slot_ref() {
        let c = component(
            "Foo",
            vec![slot("hint", SlotType::Text, true)],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "HostInput".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "placeholder".to_string(),
                    value: LayoutPropValue::SlotRef("hint".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));
        assert!(
            r.xaml.contains(" PlaceholderText=\"{x:Bind Hint, Mode=OneWay}\""),
            "got:\n{}",
            r.xaml
        );
    }

    /// Companion to the source scan: emitted output for a representative
    /// layout carries no mode-less binding either.
    #[test]
    fn every_emitted_xbind_declares_its_mode() {
        fn text_bound_to(slot_name: &str) -> LayoutNode {
            LayoutNode {
                tag: "Text".to_string(),
                part_name: None,
                props: vec![LayoutProp {
                    name: "content".to_string(),
                    value: LayoutPropValue::SlotRef(slot_name.to_string()),
                }],
                children: Vec::new(),
            }
        }

        let c = component(
            "Foo",
            vec![
                slot("title", SlotType::Text, true),
                slot("editable", SlotType::Bool, true),
            ],
            vec![],
        );
        let l = layout_with_root(
            "Foo",
            LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![
                    // A plain slot-bound text binding.
                    text_bound_to("title"),
                    // A conditional — the site that was still OneTime and
                    // froze every view switch in the generated TaskApp.
                    if_node(
                        LayoutPropValue::SlotRef("editable".to_string()),
                        vec![text_bound_to("title")],
                    ),
                ],
            },
        );
        let r = compile(&c, &l, &empty_style("Foo"));

        let missing = xbinds_missing_mode(&r.xaml);
        assert!(
            missing.is_empty(),
            "these bindings default to OneTime and will freeze after first \
             render; give each an explicit Mode: {missing:#?}\n\nfull XAML:\n{}",
            r.xaml
        );
    }

    /// CSS `rgb()`/`rgba()` are function calls, not XAML literals. They
    /// must be evaluated at emit time into `#AARRGGBB`, because the XAML
    /// markup compiler does NOT validate brush literals — an
    /// unconverted value builds cleanly and then throws
    /// E_XAMLPARSEFAILED when the runtime loads the page.
    #[test]
    fn css_rgb_functions_convert_to_xaml_hex() {
        // Opaque 3-arg form gets a fully opaque alpha byte.
        assert_eq!(
            css_rgb_function_to_xaml_hex("rgb(20,17,13)").as_deref(),
            Some("#FF14110D")
        );
        // 4-arg form with CSS's leading-dot fraction: .28 * 255 = 71.4 â†’ 0x47.
        assert_eq!(
            css_rgb_function_to_xaml_hex("rgba(20,17,13,.28)").as_deref(),
            Some("#4714110D")
        );
        assert_eq!(
            css_rgb_function_to_xaml_hex("rgba(255, 255, 255, 1)").as_deref(),
            Some("#FFFFFFFF")
        );
        // Malformed / out-of-range input falls through rather than
        // producing a bogus color.
        assert_eq!(css_rgb_function_to_xaml_hex("rgb(300,0,0)"), None);
        assert_eq!(css_rgb_function_to_xaml_hex("rgba(1,2,3,9)"), None);
        assert_eq!(css_rgb_function_to_xaml_hex("#ffffff"), None);
    }

    /// `currentColor` is a CSS cascade keyword with no XAML equivalent,
    /// so the color normalizer drops the property instead of emitting a
    /// literal the runtime rejects.
    #[test]
    fn current_color_is_dropped_not_emitted() {
        assert_eq!(normalize_xaml_color_value("currentColor"), None);
        assert_eq!(normalize_xaml_color_value("currentcolor"), None);
        // Ordinary values still normalize as before.
        assert_eq!(
            normalize_xaml_color_value("transparent").as_deref(),
            Some("Transparent")
        );
        assert_eq!(
            normalize_xaml_color_value("#1e1e1e").as_deref(),
            Some("#1e1e1e")
        );
        assert_eq!(
            normalize_xaml_color_value("rgba(20,17,13,.28)").as_deref(),
            Some("#4714110D")
        );
    }

    /// A colour value carrying XML metacharacters is refused rather than
    /// echoed into an attribute. Several arms of the normalizer return the
    /// stylesheet value verbatim, and the base-style-fragment sink does not
    /// XML-escape, so a value like `x" Foo="bar` would otherwise close the
    /// attribute and inject markup. mosstyle token validation permits these
    /// characters, so a third-party package can supply them.
    #[test]
    fn color_values_with_xml_metacharacters_are_refused() {
        assert_eq!(normalize_xaml_color_value(r#"x" Foo="bar"#), None);
        assert_eq!(normalize_xaml_color_value("a<b"), None);
        assert_eq!(normalize_xaml_color_value("a>b"), None);
        assert_eq!(normalize_xaml_color_value("a&b"), None);
        // A quoted value (mosstyle STRING tokens retain their quotes) is
        // refused for the same reason.
        assert_eq!(normalize_xaml_color_value("\"red\""), None);
        // Legitimate literals are unaffected.
        assert_eq!(normalize_xaml_color_value("#1e1e1e").as_deref(), Some("#1e1e1e"));
        assert_eq!(normalize_xaml_color_value("Transparent").as_deref(), Some("Transparent"));
    }

    /// A hostile `rgb(` value with a huge field count must not allocate one
    /// slice per field before the arity check rejects it.
    #[test]
    fn rgb_parser_bounds_its_field_split() {
        let hostile = format!("rgb({})", ",".repeat(100_000));
        assert_eq!(css_rgb_function_to_xaml_hex(&hostile), None);
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
        assert!(p.global_json.contains("\"version\": \"9.0.100\""));
        assert!(p.global_json.contains("\"rollForward\": \"latestFeature\""));
        // csproj has the WindowsAppSDK reference + unpackaged WinUI build switches.
        assert!(p.csproj.contains("Microsoft.WindowsAppSDK"));
        assert!(p.csproj.contains("Version=\"1.8.260710003\""));
        assert!(p.csproj.contains("Version=\"10.0.26100.4654\""));
        assert!(p
            .csproj
            .contains("<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>"));
        // PRI generation must be ON — makepri.exe comes from the
        // Microsoft.Windows.SDK.BuildTools package reference, and
        // without an app PRI the built app cannot resolve WinUI's
        // themeresources and dies at startup. MSIX tooling stays off
        // separately (that is the part which needs Visual Studio).
        assert!(p.csproj.contains("AppxGeneratePriEnabled>true"));
        assert!(p.csproj.contains("EnableDefaultPriItems>true"));
        assert!(p.csproj.contains("EnableCoreMrtTooling>true"));
        assert!(p.csproj.contains("EnableMsixTooling>false"));
        assert!(p.csproj.contains("UseRidGraph>true"));
        assert!(p.csproj.contains("FlattenNativeRuntimeDlls"));
        assert!(p.csproj.contains("CopyMosaicNativeHostLibraries"));
        assert!(p.csproj.contains("$(MSBuildProjectDirectory)\\*.dll"));
        // The window title must be correctly encoded UTF-8.
        //
        // Every generated WinUI app displayed `TaskApp â€" Mosaic â†' XAML
        // demo` in its title bar: the em dash and arrow were double-encoded
        // in the emitter's own source literal, so the mojibake shipped to
        // every consumer. It survived because nothing asserted on the
        // title's bytes -- cosmetic, always visible, easy to stop noticing.
        assert!(
            p.main_window_xaml.contains('\u{2014}'),
            "title should carry a real em dash (U+2014), got:\n{}",
            p.main_window_xaml
        );
        assert!(
            p.main_window_xaml.contains('\u{2192}'),
            "title should carry a real rightwards arrow (U+2192), got:\n{}",
            p.main_window_xaml
        );
        // `Ã¢` is the giveaway: a UTF-8 em dash reinterpreted as Latin-1 and
        // re-encoded.
        assert!(
            !p.main_window_xaml.contains('\u{00e2}'),
            "title contains a double-encoded character, got:\n{}",
            p.main_window_xaml
        );

        // App.xaml.cs references MainWindow.
        assert!(p.app_xaml_cs.contains("new MainWindow()"));
        // MainWindow.xaml.cs has the dispatch stub.
        assert!(p.main_window_cs.contains("OnComponentDispatch"));
        // MainWindow.xaml.cs can optionally delegate props/events to an
        // app-provided MosaicHost without requiring one to compile.
        assert!(p.main_window_cs.contains("TryApplyMosaicHostProps"));
        assert!(p
            .main_window_cs
            .contains("TryRunMosaicHostInteractionAcceptance(this.Component)"));
        assert!(p.main_window_cs.contains("\"RunInteractionAcceptance\""));
        assert!(p.main_window_cs.contains("CoerceMosaicHostResult"));
        assert!(p.main_window_cs.contains("FindMosaicHostMethod"));
        assert!(p
            .main_window_cs
            .contains("private async void OnComponentDispatch"));
        assert!(p.main_window_cs.contains("await TryHandleMosaicHostEvent"));
        assert!(p.main_window_cs.contains("TryHandleMosaicHostIntent"));
        assert!(p.main_window_cs.contains("UnwrapMosaicHostResultAsync"));
        assert!(p.main_window_cs.contains("HandleHostIntent"));
        assert!(p
            .main_window_cs
            .contains("Mosaic.Generated.MosaicRuntimeHost"));
        assert!(p.main_window_cs.contains("\"IsAvailable\""));
        assert!(p
            .main_window_cs
            .contains("ParameterType.IsAssignableFrom(parameterTypes[index])"));
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
        // README documents that Windows App SDK is bundled with the host.
        assert!(p
            .readme
            .contains("No separate Windows App Runtime install is required"));
        // app.manifest declares DPI awareness.
        assert!(p.package_manifest.contains("PerMonitorV2"));
    }

    #[test]
    fn native_complete_project_requires_runtime_without_host_or_sample_fallbacks() {
        let c = component(
            "Foo",
            vec![
                slot("greeting", SlotType::Text, true),
                slot("subtitle", SlotType::Text, false),
            ],
            vec![emit("onToggle", vec![])],
        );
        let l = layout_with_root("Foo", box_root());
        let s = empty_style("Foo");
        let mut o = opts();
        o.emit_project = true;
        o.require_runtime = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let p = r.project.as_ref().expect("project populated");

        assert!(p
            .main_window_cs
            .contains("private static readonly string[] RequiredProps = new[] { \"greeting\" }"));
        assert!(p
            .main_window_cs
            .contains("MosaicRuntimeHost.LoadRequired()"));
        assert!(p
            .main_window_cs
            .contains("MosaicRuntimeHost.ApplyRequiredProps(this.Component, RequiredProps)"));
        assert!(p
            .main_window_cs
            .contains("await MosaicRuntimeHost.HandleRequiredEvent("));
        assert!(!p.main_window_cs.contains("FindMosaicHostMethod"));
        assert!(!p.main_window_cs.contains("Mosaic.Generated.MosaicHost"));
        assert!(!p.main_window_cs.contains("sample props loaded"));
        assert!(!p.main_window_cs.contains("Sample Greeting"));
        assert!(!p.main_window_cs.contains("TODO: business logic"));
        assert!(p
            .readme
            .contains("This `native-complete` shell requires Mosaic's standard Rust application"));
        assert!(p.readme.contains("There is no reflection host or"));
    }

    #[test]
    fn native_complete_dialog_applies_runtime_props_before_showing() {
        let c = component(
            "Foo",
            vec![slot("title", SlotType::Text, true)],
            vec![emit("onClose", vec![])],
        );
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
        o.require_runtime = true;
        let r = from_pipeline(&c, &l, &s, None, &o).unwrap();
        let source = &r
            .project
            .as_ref()
            .expect("project populated")
            .main_window_cs;

        let create = source.find("var dialog = new Foo()").unwrap();
        let apply = source
            .find("MosaicRuntimeHost.ApplyRequiredProps(dialog, RequiredProps)")
            .unwrap();
        let show = source.find("await dialog.ShowAsync()").unwrap();
        assert!(create < apply && apply < show);
        assert!(source.contains("new[] { \"title\" }"));
        assert!(source.contains("await MosaicRuntimeHost.HandleRequiredEvent("));
        assert!(!source.contains("TryApplyMosaicHostProps"));
        assert!(!source.contains("Sample Title"));
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
            r.xaml
                .contains("IsEnabled=\"{x:Bind Not(Locked), Mode=OneWay}\""),
            "expected a one-way IsEnabled binding for Locked, got:\n{}",
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

    fn slider_in_box(part_name: Option<&str>, props: Vec<LayoutProp>) -> LayoutDef {
        layout_with_root(
            "X",
            LayoutNode {
                tag: "Box".to_string(),
                part_name: None,
                props: Vec::new(),
                children: vec![LayoutNode {
                    tag: "HostSlider".to_string(),
                    part_name: part_name.map(str::to_string),
                    props,
                    children: Vec::new(),
                }],
            },
        )
    }

    #[test]
    fn host_slider_lowers_native_range_step_disabled_and_events() {
        let c = component(
            "X",
            vec![
                slot("volume", SlotType::Number, true),
                slot("locked", SlotType::Bool, true),
                slot("label", SlotType::Text, true),
            ],
            vec![
                emit("onChange", vec![param("value", EmitPayloadType::Number)]),
                emit("onCommit", vec![param("value", EmitPayloadType::Number)]),
            ],
        );
        let l = slider_in_box(
            Some("volume-slider"),
            vec![
                LayoutProp {
                    name: "value".to_string(),
                    value: LayoutPropValue::SlotRef("volume".to_string()),
                },
                LayoutProp {
                    name: "min".to_string(),
                    value: LayoutPropValue::Number(-10.0),
                },
                LayoutProp {
                    name: "max".to_string(),
                    value: LayoutPropValue::Number(10.0),
                },
                LayoutProp {
                    name: "step".to_string(),
                    value: LayoutPropValue::Number(2.5),
                },
                LayoutProp {
                    name: "disabled".to_string(),
                    value: LayoutPropValue::SlotRef("locked".to_string()),
                },
                LayoutProp {
                    name: "a11y-label".to_string(),
                    value: LayoutPropValue::SlotRef("label".to_string()),
                },
                LayoutProp {
                    name: "onChange".to_string(),
                    value: LayoutPropValue::EmitRef("onChange".to_string()),
                },
                LayoutProp {
                    name: "onCommit".to_string(),
                    value: LayoutPropValue::EmitRef("onCommit".to_string()),
                },
            ],
        );
        let r = compile(&c, &l, &empty_style("X"));

        assert!(
            r.xaml
                .contains("<local:XMosaicSlider x:Name=\"VolumeSlider\""),
            "expected component-scoped native Slider, got:\n{}",
            r.xaml
        );
        assert!(r
            .xaml
            .contains("AutomationProperties.AutomationId=\"volume-slider\""));
        assert!(r
            .xaml
            .contains("AutomationProperties.Name=\"{x:Bind Label, Mode=OneWay}\""));
        assert!(r.xaml.contains("Value=\"{x:Bind Volume, Mode=OneWay}\""));
        assert!(r.xaml.contains("Minimum=\"-10\""));
        assert!(r.xaml.contains("Maximum=\"10\""));
        assert!(r.xaml.contains("MosaicStep=\"2.5\""));
        assert!(r
            .xaml
            .contains("IsEnabled=\"{x:Bind Not(Locked), Mode=OneWay}\""));
        assert!(r
            .xaml
            .contains("MosaicValueChanged=\"VolumeSlider_MosaicValueChanged\""));
        assert!(r
            .xaml
            .contains("MosaicValueCommitted=\"VolumeSlider_MosaicValueCommitted\""));

        assert!(r
            .code_behind
            .contains("public sealed class XMosaicSlider : Slider"));
        assert!(r.code_behind.contains("new XEvent.Change(args.NewValue)"));
        assert!(r.code_behind.contains("new XEvent.Commit(args.NewValue)"));
        assert!(r.code_behind.contains("PointerCaptureLostEvent"));
        assert!(r.code_behind.contains("protected override void OnKeyUp"));
        assert!(r.code_behind.contains("FocusState == FocusState.Unfocused"));
    }

    #[test]
    fn host_slider_step_zero_is_effectively_continuous_and_callbacks_are_optional() {
        let c = component("X", vec![], vec![]);
        let l = slider_in_box(
            None,
            vec![
                LayoutProp {
                    name: "step".to_string(),
                    value: LayoutPropValue::Number(0.0),
                },
                LayoutProp {
                    name: "a11y-label".to_string(),
                    value: LayoutPropValue::String("Opacity".to_string()),
                },
            ],
        );
        let r = compile(&c, &l, &empty_style("X"));

        assert!(r.xaml.contains("MosaicStep=\"0\""));
        assert!(r.xaml.contains("AutomationProperties.Name=\"Opacity\""));
        assert!(!r.xaml.contains("MosaicValueChanged=\""));
        assert!(!r.xaml.contains("MosaicValueCommitted=\""));
        assert!(r
            .code_behind
            .contains("StepFrequency = Math.Max(span / 1_000_000.0"));
        assert!(r
            .code_behind
            .contains("SmallChange = Math.Max(span / 100.0, StepFrequency)"));
    }

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

    // ── issue #12038: HostLink.href scheme validation ──

    /// A literal `href` with no scheme, or a scheme outside the
    /// `http`/`https`/`mailto` allowlist, is rejected at compile time
    /// rather than emitted -- `NavigateUri` is handed to the OS shell
    /// launcher, so a `file:`/UNC/custom-protocol target would launch
    /// rather than open as a web link.
    #[test]
    fn host_link_disallowed_scheme_href_is_rejected() {
        for hostile in [
            "file:///etc/passwd",
            "ms-appx-web:///malicious.html",
            "\\\\attacker\\share\\payload.exe",
            "javascript:alert(1)",
            "no-scheme-at-all",
        ] {
            let c = component("X", vec![], vec![]);
            let l = link_in_box(vec![LayoutProp {
                name: "href".to_string(),
                value: LayoutPropValue::String(hostile.to_string()),
            }]);
            let err = from_pipeline(&c, &l, &empty_style("X"), None, &opts())
                .expect_err(&format!("expected {hostile:?} to be rejected"));
            assert!(
                matches!(err, PipelineEmitError::UnsafeUriScheme(ref h) if h == hostile),
                "expected UnsafeUriScheme({hostile:?}), got {err:?}"
            );
        }
    }

    /// Every allowed scheme -- including case-insensitively -- still
    /// emits `NavigateUri` unchanged. This is the regression guard that
    /// the fix for #12038 doesn't also reject legitimate links.
    #[test]
    fn host_link_allowed_scheme_hrefs_still_emit_navigate_uri() {
        for allowed in [
            "http://example.com",
            "https://example.com/path?q=1",
            "mailto:hello@example.com",
            "HTTPS://Example.com",
        ] {
            let c = component("X", vec![], vec![]);
            let l = link_in_box(vec![LayoutProp {
                name: "href".to_string(),
                value: LayoutPropValue::String(allowed.to_string()),
            }]);
            let r = compile(&c, &l, &empty_style("X"));
            assert!(
                r.xaml.contains(&format!("NavigateUri=\"{allowed}\"")),
                "expected NavigateUri={allowed:?} to survive unchanged, got:\n{}",
                r.xaml
            );
        }
    }

    /// A slot-valued `href` binds through the generated `SafeNavigateUri`
    /// helper rather than the raw slot value -- the runtime-bound case
    /// can't be checked at compile time, so it's validated host-side
    /// instead. The helper itself must also be emitted into the
    /// generated code-behind.
    #[test]
    fn host_link_slot_href_binds_through_safe_navigate_uri_helper() {
        let c = component("X", vec![slot("target-url", SlotType::Text, true)], vec![]);
        let l = link_in_box(vec![LayoutProp {
            name: "href".to_string(),
            value: LayoutPropValue::SlotRef("target-url".to_string()),
        }]);
        let r = compile(&c, &l, &empty_style("X"));
        assert!(
            r.xaml
                .contains(" NavigateUri=\"{x:Bind SafeNavigateUri(TargetUrl), Mode=OneWay}\""),
            "expected the slot href to bind through SafeNavigateUri, got:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains("SafeNavigateUri"),
            "expected the SafeNavigateUri helper to be emitted, got:\n{}",
            r.code_behind
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
            r.xaml.contains("Content=\"{x:Bind Item, Mode=OneWay}\""),
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
                transitions: vec![],
                states: Vec::new(),
            }],
        }
    }

    // ── issue #12022: dropped_style_properties ──

    /// A genuinely-unexpressible property (`box-shadow`) is reported, with
    /// a real reason, tagged with its part name.
    #[test]
    fn dropped_style_properties_reports_box_shadow() {
        let style = style_for_box("card", vec![("box-shadow", "0 1px 2px #000")]);
        let dropped = dropped_style_properties(&style);
        assert_eq!(dropped.len(), 1, "got: {dropped:?}");
        assert_eq!(dropped[0].part, "card");
        assert_eq!(dropped[0].name, "box-shadow");
        assert_eq!(dropped[0].value, "0 1px 2px #000");
        assert!(
            dropped[0].reason.contains("ThemeShadow"),
            "got: {}",
            dropped[0].reason
        );
    }

    /// `align-items: center` / `justify-content: space-between` are
    /// consumed by `FlexHints` (PR #12980) through a side channel outside
    /// `build_style_fragment` — reporting them as dropped would be a false
    /// positive.
    #[test]
    fn dropped_style_properties_excludes_recognised_flex_values() {
        let style = style_for_box(
            "toolbar",
            vec![
                ("align-items", "center"),
                ("justify-content", "space-between"),
                ("flex-grow", "1"),
            ],
        );
        assert!(
            dropped_style_properties(&style).is_empty(),
            "got: {:?}",
            dropped_style_properties(&style)
        );
    }

    /// An `align-items`/`justify-content` value `FlexHints` does NOT
    /// recognise is a genuine drop — no per-child alignment or
    /// distribution is applied for it anywhere.
    #[test]
    fn dropped_style_properties_reports_unrecognised_flex_values() {
        let style = style_for_box(
            "toolbar",
            vec![
                ("align-items", "flex-end"),
                ("justify-content", "center"),
            ],
        );
        let dropped = dropped_style_properties(&style);
        assert_eq!(dropped.len(), 2, "got: {dropped:?}");
        assert!(dropped.iter().any(|d| d.name == "align-items"));
        assert!(dropped.iter().any(|d| d.name == "justify-content"));
    }

    /// `flex-grow` is never reported regardless of value — it's fully
    /// boolean-handled today (grows or doesn't), so there's nothing a
    /// reader would recognise as lost.
    #[test]
    fn dropped_style_properties_never_reports_flex_grow() {
        for value in ["1", "0", "2.5", "not-a-number"] {
            let style = style_for_box("cell", vec![("flex-grow", value)]);
            assert!(
                dropped_style_properties(&style).is_empty(),
                "flex-grow: {value} should not be reported, got: {:?}",
                dropped_style_properties(&style)
            );
        }
    }

    /// `width: 100%` is fully handled via `stretch_alignment_for` (a
    /// successful translation, not a drop) — must never appear here.
    #[test]
    fn dropped_style_properties_excludes_full_width() {
        let style = style_for_box("filler", vec![("width", "100%")]);
        assert!(
            dropped_style_properties(&style).is_empty(),
            "got: {:?}",
            dropped_style_properties(&style)
        );
    }

    /// A non-100% percentage width is the issue's own motivating example —
    /// silently dropped today, must now be reported.
    #[test]
    fn dropped_style_properties_reports_partial_percentage_width() {
        let style = style_for_box("cell", vec![("width", "50%")]);
        let dropped = dropped_style_properties(&style);
        assert_eq!(dropped.len(), 1, "got: {dropped:?}");
        assert_eq!(dropped[0].name, "width");
        assert_eq!(dropped[0].value, "50%");
    }

    /// An unrecognised/typo'd property name falls through to the generic
    /// reason rather than being silently ignored.
    #[test]
    fn dropped_style_properties_reports_unknown_property_with_generic_reason() {
        let style = style_for_box("card", vec![("colr", "#fff")]);
        let dropped = dropped_style_properties(&style);
        assert_eq!(dropped.len(), 1, "got: {dropped:?}");
        assert_eq!(dropped[0].name, "colr");
        assert!(
            dropped[0].reason.contains("no WinUI XAML setter"),
            "got: {}",
            dropped[0].reason
        );
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

    /// X6/§3.1: Lattice/flex layout props must lower only where WinUI has a
    /// native equivalent. `gap` becomes `Grid.ColumnSpacing` for a `Row`
    /// (`RowSpacing` for a `Column`), `max-width` becomes `MaxWidth`,
    /// `align-items: center` becomes `VerticalAlignment="Center"` on the
    /// child (the Row's cross axis), and `flex-wrap` (genuinely
    /// unsupported — no WinUI 3 WrapPanel) is dropped instead of leaking
    /// into a fake TextBlock style setter. A single child under
    /// `justify-content: space-between` gets no spacer (space-between needs
    /// ≥2 children to have anything to distribute between).
    #[test]
    fn row_lattice_layout_style_lowers_to_native_grid_attrs() {
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
            r.xaml
                .contains("<Grid ColumnSpacing=\"16\" MaxWidth=\"980\">"),
            "expected Row gap/max-width to land on Grid, got:\n{}",
            r.xaml
        );
        // align-items: center → the child's cross-axis (vertical, for a
        // Row) alignment, injected alongside its Grid.Column index.
        assert!(
            r.xaml
                .contains("Text=\"Title\" Grid.Column=\"0\" VerticalAlignment=\"Center\""),
            "got:\n{}",
            r.xaml
        );
        // One child under space-between: no spacer definitions.
        assert_eq!(
            r.xaml.matches("<ColumnDefinition").count(),
            1,
            "got:\n{}",
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
            "StackPanel",
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
        assert!(
            r.xaml.contains("Visibility=\"{x:Bind Expr_"),
            "typed templates must bind a normal row-VM property:\n{}",
            r.xaml
        );
        let vm = r
            .for_view_models
            .iter()
            .find(|file| file.filename == "Foo_ItemVm.cs")
            .expect("row vm");
        assert!(
            vm.source.contains("public bool Expr_")
                && vm.source.contains("=> Owner.Expr_")
                && vm.source.contains("(Item);"),
            "row VM must delegate the computed expression to its owner:\n{}",
            vm.source
        );
        assert!(
            r.code_behind.contains("internal bool Expr_"),
            "row VM helper target must be assembly-visible:\n{}",
            r.code_behind
        );
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
                .contains("<ItemsRepeater ItemsSource=\"{x:Bind FooItemVmRows, Mode=OneWay}\""),
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
                "public sealed record Foo_ItemVm(Foo Owner, string Item, int Index, bool IsSelected);"
            ),
            "got:\n{}",
            vm.source
        );
        assert!(
            r.code_behind
                .contains("rows.Add(new Foo_ItemVm(this, source[i], i, i == SelectedIndex));"),
            "got:\n{}",
            r.code_behind
        );
        assert!(
            r.code_behind
                .contains("public sealed partial class Foo : UserControl, INotifyPropertyChanged"),
            "projected row properties must publish invalidation, got:\n{}",
            r.code_behind
        );
        assert!(
            r.code_behind.contains(
                "new PropertyMetadata(default(IReadOnlyList<string>), \
                 OnMosaicItemsRowProjectionInputChanged)"
            ),
            "source-list replacement must invalidate the projection, got:\n{}",
            r.code_behind
        );
        assert!(
            r.code_behind.contains(
                "new PropertyMetadata(default(double), \
                 OnMosaicSelectedIndexRowProjectionInputChanged)"
            ),
            "selection changes must invalidate row-local state, got:\n{}",
            r.code_behind
        );
        assert_eq!(
            r.code_behind
                .matches("NotifyRowProjectionChanged(nameof(FooItemVmRows));")
                .count(),
            2,
            "both projection inputs must notify the one-way ItemsSource binding"
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
    fn styled_column_wraps_grid_in_border() {
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
        assert!(r.xaml.contains("<Grid>"), "got:\n{}", r.xaml);
        assert!(
            !r.xaml.contains("<Grid Background="),
            "Grid must not carry Border/Control style attrs, got:\n{}",
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
            vec![
                slot(
                    "viewport-rows",
                    SlotType::List(Box::new(ListInnerType::List(Box::new(ListInnerType::Text)))),
                    true,
                ),
                slot(
                    "column-widths",
                    SlotType::List(Box::new(ListInnerType::Number)),
                    true,
                ),
            ],
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
        assert!(
            rowvm.contains("public IReadOnlyList<Grid_VVm> GridVVmRows")
                && rowvm.contains("var source = Row;")
                && rowvm.contains(
                    "new Grid_VVm(Owner, source[i], i, Owner.ColumnWidths is { } widths && i < widths.Count ? widths[i] : 0, Row, Index)"
                ),
            "outer row VM must own the nested typed projection, got:\n{rowvm}"
        );
        assert!(
            vvm.contains("IReadOnlyList<string> Row, int R"),
            "inner value VM must capture the outer row and authored index, got:\n{vvm}"
        );
        assert!(
            r.xaml
                .contains("ItemsSource=\"{x:Bind GridVVmRows, Mode=OneWay}\""),
            "nested repeater must bind its parent row-VM projection, got:\n{}",
            r.xaml
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
        // The <remarks> documents the generated width projection.
        assert!(
            vvm.contains("<remarks>") && vvm.contains("column-widths"),
            "value VM must document generated Width population, got:\n{vvm}"
        );

        // The cell element binds the width.
        assert!(
            r.xaml.contains("Width=\"{x:Bind Width, Mode=OneWay}\""),
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
                transitions: vec![],
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

    fn transition(property: &str, duration: &str, easing: &str) -> StyleTransition {
        StyleTransition {
            property: property.to_string(),
            duration: duration.to_string(),
            easing: easing.to_string(),
        }
    }

    fn styled_host_button(state_props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: "HostButton".to_string(),
            part_name: Some("button".to_string()),
            props: state_props,
            children: Vec::new(),
        }
    }

    #[test]
    fn transition_values_lower_to_winui_duration_and_easing() {
        assert_eq!(
            xaml_transition_duration("80ms").as_deref(),
            Some("0:0:0.08")
        );
        assert_eq!(xaml_transition_duration("1s").as_deref(), Some("0:0:1"));
        assert_eq!(
            xaml_transition_duration("150ms").as_deref(),
            Some("0:0:0.15")
        );
        assert!(xaml_transition_duration("fast").is_none());

        let ease_out = emit_xaml_easing(&transition("opacity", "150ms", "ease-out"), 0);
        assert!(ease_out.contains("<QuadraticEase EasingMode=\"EaseOut\"/>"));
        let linear = emit_xaml_easing(&transition("opacity", "150ms", "linear"), 0);
        assert!(linear.is_empty());
        let cubic = emit_xaml_easing(
            &transition("opacity", "150ms", "cubic-bezier(0.34, 1.56, 0.64, 1)"),
            0,
        );
        assert!(cubic.contains("<CubicEase EasingMode=\"EaseInOut\"/>"));
    }

    #[test]
    fn literal_state_predicate_lowers_to_literal_trigger() {
        let c = component("AlwaysSelected", vec![], vec![]);
        let l = layout_with_root(
            "AlwaysSelected",
            styled_host_button(vec![LayoutProp {
                name: "state-when-selected".to_string(),
                value: LayoutPropValue::Keyword("true".to_string()),
            }]),
        );
        let s = StyleDef {
            component_name: "AlwaysSelected".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "selected".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.8".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains("<StateTrigger IsActive=\"True\"/>"),
            "literal predicates must not become x:Bind paths:\n{}",
            r.xaml
        );
        assert!(!r.xaml.contains("x:Bind True"), "got:\n{}", r.xaml);
    }

    #[test]
    fn button_base_hover_state_uses_native_pointer_binding() {
        let c = component("HoverButton", vec![], vec![]);
        let l = layout_with_root("HoverButton", styled_host_button(vec![]));
        let s = StyleDef {
            component_name: "HoverButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "background".to_string(),
                    value: "#202020".to_string(),
                }],
                transitions: vec![transition("background", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "hover".to_string(),
                    props: vec![StyleProp {
                        name: "background".to_string(),
                        value: "#264f78".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains(
                "<StateTrigger IsActive=\"{Binding IsPointerOver, ElementName=Button}\"/>"
            ),
            "built-in hover must bind directly to native ButtonBase pointer state:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<Setter Target=\"Button.(Control.Background).(SolidColorBrush.Color)\" Value=\"#264f78\"/>"
            ),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<VisualTransition GeneratedDuration=\"0:0:0.08\">"),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn button_base_hover_is_local_to_each_for_template_instance() {
        let c = component(
            "HoverRows",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "HoverRows",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                None,
                vec![styled_host_button(vec![])],
            ),
        );
        let s = StyleDef {
            component_name: "HoverRows".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "0.8".to_string(),
                }],
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "hover".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "1".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains(
                "<DataTemplate x:DataType=\"local:HoverRows_RowVm\">\n                <Grid>\n                    <VisualStateManager.VisualStateGroups>"
            ),
            "hover groups must live in the repeated row namescope:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<StateTrigger IsActive=\"{Binding IsPointerOver, ElementName=Button}\"/>"
            ),
            "each template instance must bind to its own Button:\n{}",
            r.xaml
        );
    }

    #[test]
    fn explicit_hover_predicate_remains_author_controlled() {
        let c = component(
            "ManualHover",
            vec![slot("force-hover", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "ManualHover",
            styled_host_button(vec![LayoutProp {
                name: "state-when-hover".to_string(),
                value: LayoutPropValue::SlotRef("force-hover".to_string()),
            }]),
        );
        let s = StyleDef {
            component_name: "ManualHover".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "hover".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.8".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{x:Bind ForceHover, Mode=OneWay}\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("Binding IsPointerOver"),
            "explicit hover state must not install native pointer tracking:\n{}",
            r.xaml
        );
    }

    #[test]
    fn native_pressed_state_uses_button_base_is_pressed() {
        let c = component("PressedButton", vec![], vec![]);
        let l = layout_with_root("PressedButton", styled_host_button(vec![]));
        let s = StyleDef {
            component_name: "PressedButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "1".to_string(),
                }],
                transitions: vec![transition("opacity", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "pressed".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.7".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{Binding IsPressed, ElementName=Button}\"/>"),
            "pressed state must bind directly to ButtonBase.IsPressed:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<Setter Target=\"Button.Opacity\" Value=\"0.7\"/>"),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn explicit_pressed_predicate_remains_author_controlled() {
        let c = component(
            "ManualPress",
            vec![slot("force-press", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "ManualPress",
            styled_host_button(vec![LayoutProp {
                name: "state-when-pressed".to_string(),
                value: LayoutPropValue::SlotRef("force-press".to_string()),
            }]),
        );
        let s = StyleDef {
            component_name: "ManualPress".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "pressed".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.7".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{x:Bind ForcePress, Mode=OneWay}\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("Binding IsPressed"),
            "explicit press state must not install native tracking:\n{}",
            r.xaml
        );
    }

    #[test]
    fn native_focused_state_uses_focus_state_converter() {
        let c = component("FocusField", vec![], vec![]);
        let l = layout_with_root(
            "FocusField",
            LayoutNode {
                tag: "HostInput".to_string(),
                part_name: Some("field".to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        );
        let s = StyleDef {
            component_name: "FocusField".to_string(),
            parts: vec![PartStyle {
                name: "field".to_string(),
                base: vec![StyleProp {
                    name: "border-color".to_string(),
                    value: "#d0d0d0".to_string(),
                }],
                transitions: vec![transition("border-color", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "focused".to_string(),
                    props: vec![StyleProp {
                        name: "border-color".to_string(),
                        value: "#e0942a".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<local:FocusStateToBoolConverter x:Key=\"FocusStateToBoolConverter\"/>"),
            "native focus requires one generated converter resource:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<StateTrigger IsActive=\"{Binding FocusState, ElementName=Field, Converter={StaticResource FocusStateToBoolConverter}}\"/>"
            ),
            "focused state must bind to the control's native FocusState:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<Setter Target=\"Field.(Control.BorderBrush).(SolidColorBrush.Color)\" Value=\"#e0942a\"/>"
            ),
            "got:\n{}",
            r.xaml
        );
        let helper = r
            .if_helpers
            .iter()
            .find(|file| file.filename == "FocusStateToBoolConverter.cs")
            .expect("focus converter helper");
        assert!(
            helper.source.contains("state != FocusState.Unfocused"),
            "converter must include pointer, keyboard, and programmatic focus:\n{}",
            helper.source
        );
        assert!(
            helper.source.contains("DependencyProperty.UnsetValue"),
            "invalid converter inputs must not throw:\n{}",
            helper.source
        );
    }

    #[test]
    fn native_focused_state_is_local_to_each_for_template_instance() {
        let c = component(
            "FocusRows",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "FocusRows",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                None,
                vec![LayoutNode {
                    tag: "HostInput".to_string(),
                    part_name: Some("field".to_string()),
                    props: Vec::new(),
                    children: Vec::new(),
                }],
            ),
        );
        let s = StyleDef {
            component_name: "FocusRows".to_string(),
            parts: vec![PartStyle {
                name: "field".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "0.8".to_string(),
                }],
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "focused".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "1".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml.contains(
                "<DataTemplate x:DataType=\"local:FocusRows_RowVm\">\n                <Grid>\n                    <VisualStateManager.VisualStateGroups>"
            ),
            "focus groups must live in the repeated row namescope:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<StateTrigger IsActive=\"{Binding FocusState, ElementName=Field, Converter={StaticResource FocusStateToBoolConverter}}\"/>"
            ),
            "each template instance must bind to its own TextBox:\n{}",
            r.xaml
        );
    }

    #[test]
    fn explicit_focused_predicate_remains_author_controlled() {
        let c = component(
            "ManualFocus",
            vec![slot("force-focus", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "ManualFocus",
            LayoutNode {
                tag: "HostInput".to_string(),
                part_name: Some("field".to_string()),
                props: vec![LayoutProp {
                    name: "state-when-focused".to_string(),
                    value: LayoutPropValue::SlotRef("force-focus".to_string()),
                }],
                children: Vec::new(),
            },
        );
        let s = StyleDef {
            component_name: "ManualFocus".to_string(),
            parts: vec![PartStyle {
                name: "field".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "focused".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.8".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{x:Bind ForceFocus, Mode=OneWay}\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            !r.xaml.contains("FocusStateToBoolConverter"),
            "explicit focus state must not install native focus tracking:\n{}",
            r.xaml
        );
        assert!(
            !r.if_helpers
                .iter()
                .any(|file| file.filename == "FocusStateToBoolConverter.cs"),
            "explicit focus state must not ship an unused converter"
        );
    }

    #[test]
    fn native_focus_precedes_hover_when_both_are_active() {
        let c = component("FocusHoverButton", vec![], vec![]);
        let l = layout_with_root("FocusHoverButton", styled_host_button(vec![]));
        let s = StyleDef {
            component_name: "FocusHoverButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "0.7".to_string(),
                }],
                transitions: Vec::new(),
                states: vec![
                    StateStyle {
                        state: "hover".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "0.9".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                    StateStyle {
                        state: "focused".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "1".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                ],
            }],
        };

        let r = compile(&c, &l, &s);
        let focus = r.xaml.find("Binding FocusState").expect("focus trigger");
        let hover = r.xaml.find("Binding IsPointerOver").expect("hover trigger");
        assert!(
            focus < hover,
            "WinUI's first-active trigger precedence must match SwiftUI's focused-over-hover layering:\n{}",
            r.xaml
        );
    }

    #[test]
    fn native_pressed_precedes_focus_and_hover_when_all_are_active() {
        let c = component("PressFocusHoverButton", vec![], vec![]);
        let l = layout_with_root("PressFocusHoverButton", styled_host_button(vec![]));
        let s = StyleDef {
            component_name: "PressFocusHoverButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "0.6".to_string(),
                }],
                transitions: Vec::new(),
                states: vec![
                    StateStyle {
                        state: "hover".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "0.8".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                    StateStyle {
                        state: "focused".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "0.9".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                    StateStyle {
                        state: "pressed".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "1".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                ],
            }],
        };

        let r = compile(&c, &l, &s);
        let pressed = r.xaml.find("Binding IsPressed").expect("pressed trigger");
        let focus = r.xaml.find("Binding FocusState").expect("focus trigger");
        let hover = r.xaml.find("Binding IsPointerOver").expect("hover trigger");
        assert!(
            pressed < focus && focus < hover,
            "WinUI first-active precedence must be pressed, focused, hover:\n{}",
            r.xaml
        );
    }

    #[test]
    fn host_control_state_and_base_transition_lower_end_to_end() {
        let c = component(
            "AnimatedButton",
            vec![slot("selected", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "AnimatedButton",
            styled_host_button(vec![LayoutProp {
                name: "state-when-selected".to_string(),
                value: LayoutPropValue::SlotRef("selected".to_string()),
            }]),
        );
        let s = StyleDef {
            component_name: "AnimatedButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "background".to_string(),
                    value: "#202020".to_string(),
                }],
                transitions: vec![transition("background", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "selected".to_string(),
                    props: vec![StyleProp {
                        name: "background".to_string(),
                        value: "#264f78".to_string(),
                    }],
                    transitions: vec![],
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(
            r.xaml
                .contains("<Button x:Name=\"Button\" AutomationProperties.AutomationId=\"button\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("Background=\"#202020\""),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<VisualStateManager.VisualStateGroups>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("    <Grid>\n        <VisualStateManager.VisualStateGroups>"),
            "WinUI StateTriggers must live on the root's first visual child:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{x:Bind Selected, Mode=OneWay}\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<Setter Target=\"Button.(Control.Background).(SolidColorBrush.Color)\" Value=\"#264f78\"/>"
            ),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<VisualTransition GeneratedDuration=\"0:0:0.08\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("<QuadraticEase EasingMode=\"EaseOut\"/>"),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn state_local_transition_overrides_entry_and_base_handles_exit() {
        let c = component(
            "FadeButton",
            vec![slot("disabled", SlotType::Bool, true)],
            vec![],
        );
        let l = layout_with_root(
            "FadeButton",
            styled_host_button(vec![LayoutProp {
                name: "state-when-disabled".to_string(),
                value: LayoutPropValue::SlotRef("disabled".to_string()),
            }]),
        );
        let s = StyleDef {
            component_name: "FadeButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "1".to_string(),
                }],
                transitions: vec![transition("opacity", "150ms", "ease-out")],
                states: vec![StateStyle {
                    state: "disabled".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.4".to_string(),
                    }],
                    transitions: vec![transition("opacity", "300ms", "linear")],
                }],
            }],
        };

        let r = compile(&c, &l, &s);
        assert!(r.xaml.contains("Opacity=\"1\""), "got:\n{}", r.xaml);
        assert!(
            r.xaml
                .contains("<VisualTransition GeneratedDuration=\"0:0:0.15\">"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains("GeneratedDuration=\"0:0:0.3\"/>"),
            "got:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<Setter Target=\"Button.Opacity\" Value=\"0.4\"/>"),
            "got:\n{}",
            r.xaml
        );
    }

    #[test]
    fn last_declared_state_keeps_cross_backend_precedence() {
        let c = component(
            "PriorityButton",
            vec![
                slot("selected", SlotType::Bool, true),
                slot("disabled", SlotType::Bool, true),
            ],
            vec![],
        );
        let l = layout_with_root(
            "PriorityButton",
            styled_host_button(vec![
                LayoutProp {
                    name: "state-when-selected".to_string(),
                    value: LayoutPropValue::SlotRef("selected".to_string()),
                },
                LayoutProp {
                    name: "state-when-disabled".to_string(),
                    value: LayoutPropValue::SlotRef("disabled".to_string()),
                },
            ]),
        );
        let s = StyleDef {
            component_name: "PriorityButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![
                    StateStyle {
                        state: "selected".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "0.8".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                    StateStyle {
                        state: "disabled".to_string(),
                        props: vec![StyleProp {
                            name: "opacity".to_string(),
                            value: "0.4".to_string(),
                        }],
                        transitions: Vec::new(),
                    },
                ],
            }],
        };

        let r = compile(&c, &l, &s);
        let disabled = r
            .xaml
            .find("IsActive=\"{x:Bind Disabled")
            .expect("disabled trigger");
        let selected = r
            .xaml
            .find("IsActive=\"{x:Bind Selected")
            .expect("selected trigger");
        assert!(
            disabled < selected,
            "last state-when declaration must be first for WinUI trigger precedence:\n{}",
            r.xaml
        );
    }

    #[test]
    fn template_local_host_state_lowers_into_datatemplate_namescope() {
        let c = component(
            "AnimatedRow",
            vec![
                slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true),
                slot("selected-index", SlotType::Number, true),
            ],
            vec![],
        );
        let l = layout_with_root(
            "AnimatedRow",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                Some("r"),
                vec![styled_host_button(vec![LayoutProp {
                    name: "state-when-selected".to_string(),
                    value: LayoutPropValue::Expr("r == selectedIndex".to_string()),
                }])],
            ),
        );
        let style = StyleDef {
            component_name: "AnimatedRow".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![StyleProp {
                    name: "opacity".to_string(),
                    value: "1".to_string(),
                }],
                transitions: vec![transition("opacity", "120ms", "ease-out")],
                states: vec![StateStyle {
                    state: "selected".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.8".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &style);
        assert_eq!(
            r.xaml
                .matches("<VisualStateManager.VisualStateGroups>")
                .count(),
            1,
            "template state groups must not leak onto the component root:\n{}",
            r.xaml
        );
        assert!(
            r.xaml.contains(
                "<DataTemplate x:DataType=\"local:AnimatedRow_RowVm\">\n                <Grid>\n                    <VisualStateManager.VisualStateGroups>"
            ),
            "WinUI StateTriggers must live on the DataTemplate's first visual child:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<StateTrigger IsActive=\"{x:Bind IsSelected, Mode=OneWay}\"/>"),
            "template predicate must bind row-local projected state:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<Setter Target=\"Button.Opacity\" Value=\"0.8\"/>"),
            "the template-local group must target the row control:\n{}",
            r.xaml
        );
        assert!(
            r.xaml
                .contains("<VisualTransition GeneratedDuration=\"0:0:0.12\">"),
            "template-local transitions must preserve MSL timing:\n{}",
            r.xaml
        );
        assert!(
            r.code_behind.contains(
                "rows.Add(new AnimatedRow_RowVm(this, source[i], i, i == SelectedIndex));"
            ),
            "the selected predicate must be projected onto each row VM:\n{}",
            r.code_behind
        );
    }

    #[test]
    fn template_state_never_binds_page_helper_from_row_namescope() {
        let c = component(
            "AnimatedRow",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let l = layout_with_root(
            "AnimatedRow",
            for_node(
                LayoutPropValue::SlotRef("rows".to_string()),
                "row",
                Some("r"),
                vec![styled_host_button(vec![LayoutProp {
                    name: "state-when-selected".to_string(),
                    value: LayoutPropValue::Expr("r == 0".to_string()),
                }])],
            ),
        );
        let style = StyleDef {
            component_name: "AnimatedRow".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: Vec::new(),
                transitions: Vec::new(),
                states: vec![StateStyle {
                    state: "selected".to_string(),
                    props: vec![StyleProp {
                        name: "opacity".to_string(),
                        value: "0.8".to_string(),
                    }],
                    transitions: Vec::new(),
                }],
            }],
        };

        let r = compile(&c, &l, &style);
        assert!(
            !r.xaml.contains("<VisualStateManager.VisualStateGroups>"),
            "unsupported template predicates must be omitted instead of targeting the root:\n{}",
            r.xaml
        );
        assert!(
            !r.code_behind.contains("private bool Expr_"),
            "DataTemplate x:Bind cannot resolve page-level helper methods:\n{}",
            r.code_behind
        );
    }
}
