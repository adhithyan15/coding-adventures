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

use std::collections::HashMap;
use std::fmt::Write as _;

use moslayout_compiler::{LayoutDef, LayoutNode, LayoutPropValue};
use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotDefault, SlotType,
};
use mosstyle_compiler::{StyleDef, StyleProp, StyleTransition};

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

#[derive(Clone, Copy)]
struct ForPayloadScope<'a> {
    item: &'a str,
    index: Option<&'a str>,
}

/// Errors the SwiftUI pipeline emitter can return.
///
/// These mirror the React backend's variants verbatim so a generic CLI can
/// print them with the same code path. Each variant carries the offending
/// identifier so the caller can include it in user-visible diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineEmitError {
    /// The mosmodel component name and the moslayout component name disagree.
    ComponentNameMismatch { mosmodel: String, moslayout: String },
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
                "moslayout primitive '{t}' is not yet supported by the SwiftUI emitter"
            ),
        }
    }
}

impl std::error::Error for PipelineEmitError {}

// =====================================================================
// UI32-K-swiftui — `--emit-project` Swift Package Manager shell
//
// L7 of UI32. Mirrors L2 (React #4297), L3 (HTML #4309),
// L4 (WebComponent #4315), L5 (Flutter #4319), L6 (Qt #4325):
// EmitOptions / ProjectFiles / from_pipeline_with_options.
//
// When `--emit-project` is on, emits a SwiftPM-shaped scaffold
// alongside the component .swift. Author runs `swift run` to see
// the component on macOS (the v1 target — iOS/Android need
// xcrun + a separate iOS shell, deferred).
// =====================================================================

/// Options controlling the SwiftUI emitter's behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Also emit `Package.swift`, `Sources/App/App.swift`,
    /// `README.md` alongside the component `.swift`. Default `false`.
    pub emit_project: bool,

    /// Pinned `swift-tools-version` for `Package.swift`. UI32 spec
    /// §3.6.3 requires exact pinning. Default `"5.10"` — a
    /// known-good Swift 5.10 LTS that supports macOS 14 SwiftUI.
    pub pinned_swift_tools: String,

    /// Pinned macOS deployment target. Default `".v13"` — the
    /// lowest macOS that supports modern SwiftUI (`@Observable`,
    /// the v3 `Sheet` API, etc.). Lower targets miss APIs.
    pub pinned_macos_min: String,

    /// Pinned iOS deployment target. Default `".v16"` keeps the
    /// generated component compatible with emitted tooltip/help
    /// modifiers while still covering currently supported devices.
    pub pinned_ios_min: String,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            emit_project: false,
            pinned_swift_tools: "5.10".to_string(),
            pinned_macos_min: ".v13".to_string(),
            pinned_ios_min: ".v16".to_string(),
        }
    }
}

/// Project-shaped artifacts emitted when `EmitOptions::emit_project`
/// is on. Three files — enough for `swift run` to launch a SwiftUI
/// macOS app that mounts the component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFiles {
    /// `Package.swift` — pinned `swift-tools-version`, single
    /// executable target named `App` that depends on the component
    /// source.
    pub package_swift: String,
    /// `Sources/App/App.swift` — SwiftUI `@main App` + `WindowGroup`
    /// shell that mounts `{Component}View(...)` as the root view.
    /// Imports the component sibling-relative (via SwiftPM target
    /// dependency or `#include`-equivalent — for v1, the component
    /// .swift sits in the same `Sources/App/` directory).
    pub app_swift: String,
    /// `README.md` — prereqs (Swift 5.10+, Xcode CLT or full
    /// Xcode), `swift run` recipe, file map.
    pub readme: String,
}

/// Error shapes specific to the SwiftUI project-shell emission path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectShellError {
    /// The component name collides with a Swift reserved keyword.
    /// Per UI32 spec §3.6.2 SwiftUI row: Swift keywords (`Class`,
    /// `Protocol`, `Actor`, `Self`, etc.) must be backtick-quoted
    /// or avoided. We choose to reject them outright with a
    /// fail-loud error rather than emit backtick-quoted Swift
    /// (which compiles but is non-idiomatic).
    SwiftKeywordCollision(String),

    /// The component name is not a valid Swift identifier (e.g.
    /// contains a hyphen — the mosmodel `NAME` grammar regex
    /// permits hyphens, but Swift identifier syntax does not).
    /// Without this guard, a name like `Foo-Bar` would produce
    /// `Foo-BarView()` which `swift build` rejects with a
    /// confusing "expected ')'" diagnostic. Defense-in-depth
    /// flagged in L7's security review.
    InvalidSwiftIdentifier(String),
}

impl std::fmt::Display for ProjectShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectShellError::SwiftKeywordCollision(n) => write!(
                f,
                "component name '{n}' collides with a Swift reserved keyword; rename the component to avoid backtick-quoting"
            ),
            ProjectShellError::InvalidSwiftIdentifier(n) => write!(
                f,
                "component name '{n}' is not a valid Swift identifier (Swift identifier shape is [A-Za-z_][A-Za-z0-9_]*; the mosmodel grammar permits hyphens but Swift does not)"
            ),
        }
    }
}

impl std::error::Error for ProjectShellError {}

impl From<ProjectShellError> for PipelineEmitError {
    fn from(e: ProjectShellError) -> Self {
        PipelineEmitError::UnsafeSlotName(e.to_string())
    }
}

/// Extended pipeline result — carries the optional `ProjectFiles`
/// when `emit_project` is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResultWithProject {
    pub output: String,
    pub component_name: String,
    pub project: Option<ProjectFiles>,
}

/// Compile a three-file Mosaic pipeline triple to SwiftUI with
/// explicit emit options.
pub fn from_pipeline_with_options(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
    options: &EmitOptions,
) -> Result<PipelineEmitResultWithProject, PipelineEmitError> {
    let component = from_pipeline(interface, layout, style)?;

    let project = if options.emit_project {
        Some(build_swiftui_project_files(interface, options)?)
    } else {
        None
    };

    Ok(PipelineEmitResultWithProject {
        output: component.output,
        component_name: component.component_name,
        project,
    })
}

/// Build the three SwiftUI app-shell side files for a single
/// component.
///
/// UI32 spec §3.6.2 SwiftUI row contract: Swift keywords (`Class`,
/// `Protocol`, `Actor`, `Self`, `Any`, `Type`, etc.) must be
/// rejected to avoid backtick-quoting in identifier positions.
fn build_swiftui_project_files(
    interface: &MosmodelComponent,
    options: &EmitOptions,
) -> Result<ProjectFiles, ProjectShellError> {
    let name = &interface.component;
    if !is_safe_swift_identifier(name) {
        return Err(ProjectShellError::InvalidSwiftIdentifier(name.to_string()));
    }
    if is_swift_keyword(name) {
        return Err(ProjectShellError::SwiftKeywordCollision(name.to_string()));
    }

    Ok(ProjectFiles {
        package_swift: build_package_swift(options),
        app_swift: build_app_swift(name, &interface.slots),
        readme: build_swiftui_platform_readme(name),
    })
}

/// Swift reserved keyword reject-list — the subset that's plausibly
/// PascalCase (so we don't false-fire on lowercase keywords like
/// `if`, `let`, `var` that the upstream validator already rejects
/// at the first-char-must-be-uppercase check).
///
/// Sources: https://docs.swift.org/swift-book/documentation/the-swift-programming-language/lexicalstructure
/// — declaration keywords + type keywords that collide with the
/// PascalCase namespace. We err on the strict side; the upstream
/// validator already gates `[A-Z][A-Za-z0-9_]*` shape, so any
/// remaining keyword collisions are these PascalCase names.
const SWIFT_RESERVED_KEYWORDS: &[&str] = &[
    "Any",
    "AnyObject",
    "Class",
    "Protocol",
    "Actor",
    "Self",
    "Type",
    "Module",
    "Package",
    "Import",
    "Throws",
    "Rethrows",
    "True",
    "False",
];

fn is_swift_keyword(s: &str) -> bool {
    SWIFT_RESERVED_KEYWORDS.contains(&s)
}

const BANNER_SWIFT: &str = "// AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n// Fork the file (remove this banner) to customise.\n";
const BANNER_MD: &str = "<!-- AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit. -->\n<!-- Fork the file (remove this banner) to customise. -->\n";

fn build_package_swift(options: &EmitOptions) -> String {
    format!(
        "// swift-tools-version: {}\n{BANNER_SWIFT}import PackageDescription\n\nlet package = Package(\n  name: \"App\",\n  platforms: [.macOS({}), .iOS({})],\n  targets: [\n    .executableTarget(\n      name: \"App\",\n      path: \"Sources/App\"\n    ),\n  ]\n)\n",
        options.pinned_swift_tools, options.pinned_macos_min, options.pinned_ios_min,
    )
}

fn build_app_swift(component_name: &str, slots: &[SlotDecl]) -> String {
    // The Mosaic SwiftUI emitter produces a `View` struct named
    // `{component_name}View` (per pipeline.rs:120 doc comment), so
    // mount that here.
    let root_view = build_root_view_initializer(component_name, slots, "host.props", "host");
    let mut out = String::new();
    write!(
        out,
        "{BANNER_SWIFT}import Combine\nimport Foundation\nimport SwiftUI\n#if os(macOS)\nimport AppKit\n#elseif os(iOS)\nimport UIKit\n#endif\n\n"
    )
    .unwrap();
    writeln!(out, "@main").unwrap();
    writeln!(out, "struct MosaicApp: App {{").unwrap();
    writeln!(out, "  @StateObject private var host = MosaicHostState()").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "  var body: some Scene {{").unwrap();
    writeln!(out, "    WindowGroup(\"{component_name}\") {{").unwrap();
    writeln!(out, "      {root_view}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
    out.push_str(&build_mosaic_host_state(component_name));
    out
}

fn build_root_view_initializer(
    component_name: &str,
    slots: &[SlotDecl],
    props_expr: &str,
    host_expr: &str,
) -> String {
    let mut out = format!("{component_name}View(\n");
    for slot in slots {
        let field = to_camel_case_first_lower(&slot.name);
        let value = host_value_for_slot(slot, props_expr, host_expr);
        writeln!(out, "        {field}: {value},").unwrap();
    }
    out.push_str("        dispatch: { event in\n");
    out.push_str("          host.dispatch(event)\n");
    out.push_str("        }\n");
    out.push_str("      )");
    out
}

fn host_value_for_slot(slot: &SlotDecl, props_expr: &str, host_expr: &str) -> String {
    let key = escape_swift_string(&slot.name);
    let fallback = sample_value_for_slot(slot);
    match &slot.r#type {
        SlotType::Text | SlotType::Image | SlotType::Color => {
            format!("MosaicHostValue.string({props_expr}, \"{key}\", fallback: {fallback})")
        }
        SlotType::Number => {
            format!("MosaicHostValue.double({props_expr}, \"{key}\", fallback: {fallback})")
        }
        SlotType::Bool => {
            format!("MosaicHostValue.bool({props_expr}, \"{key}\", fallback: {fallback})")
        }
        SlotType::List(inner) => match inner.as_ref() {
            ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                format!("MosaicHostValue.stringList({props_expr}, \"{key}\", fallback: {fallback})")
            }
            ListInnerType::Number => {
                format!("MosaicHostValue.doubleList({props_expr}, \"{key}\", fallback: {fallback})")
            }
            ListInnerType::Bool => {
                format!("MosaicHostValue.boolList({props_expr}, \"{key}\", fallback: {fallback})")
            }
            ListInnerType::Node | ListInnerType::Component(_) | ListInnerType::List(_) => fallback,
        },
        SlotType::Node | SlotType::Component(_) => {
            format!("{host_expr}.node(named: \"{key}\")")
        }
    }
}

fn build_mosaic_host_state(component_name: &str) -> String {
    format!(
        "private final class MosaicHostState: ObservableObject {{\n  @Published var props: [String: Any] = [:]\n  @Published var lastHostIntent: [String: Any]? = nil\n  private let bridge: MosaicHostBridgeObject?\n\n  init() {{\n    self.bridge = MosaicHostBridge.load()\n    applyHostResponse(bridge?.applyProps() as? [String: Any])\n  }}\n\n  func node(named name: String) -> AnyView {{\n    guard let object = bridge?.node?(named: name as NSString) else {{\n      return AnyView(EmptyView())\n    }}\n#if os(macOS)\n    guard let view = object as? NSView else {{ return AnyView(EmptyView()) }}\n    return AnyView(MosaicHostPlatformView(view: view))\n#elseif os(iOS)\n    guard let view = object as? UIView else {{ return AnyView(EmptyView()) }}\n    return AnyView(MosaicHostPlatformView(view: view))\n#else\n    return AnyView(EmptyView())\n#endif\n  }}\n\n  func dispatch(_ event: {component_name}Event) {{\n    guard let bridge else {{\n      print(\"Mosaic dispatch: \\(event.mosaicEnvelope)\")\n      return\n    }}\n    applyHostResponse(bridge.handleEvent(event.mosaicEnvelope as NSDictionary, name: event.mosaicName as NSString) as? [String: Any])\n  }}\n\n  private func applyHostResponse(_ response: [String: Any]?) {{\n    guard let response else {{ return }}\n    if let intent = response[\"hostIntent\"] as? [String: Any] {{\n      self.lastHostIntent = intent\n    }}\n    if let next = response[\"props\"] as? [String: Any] {{\n      self.props = next\n      return\n    }}\n    if response[\"hostIntent\"] != nil || response[\"error\"] != nil {{\n      return\n    }}\n    self.props = response\n  }}\n}}\n\n@objc protocol MosaicHostBridgeObject {{\n  func applyProps() -> NSDictionary?\n  func handleEvent(_ envelope: NSDictionary, name: NSString) -> NSDictionary?\n  @objc optional func node(named name: NSString) -> NSObject?\n}}\n\n#if os(macOS)\nprivate struct MosaicHostPlatformView: NSViewRepresentable {{\n  let view: NSView\n  func makeNSView(context: Context) -> NSView {{ view }}\n  func updateNSView(_ nsView: NSView, context: Context) {{}}\n}}\n#elseif os(iOS)\nprivate struct MosaicHostPlatformView: UIViewRepresentable {{\n  let view: UIView\n  func makeUIView(context: Context) -> UIView {{ view }}\n  func updateUIView(_ uiView: UIView, context: Context) {{}}\n}}\n#endif\n\nprivate enum MosaicHostBridge {{\n  static func load() -> MosaicHostBridgeObject? {{\n    for className in [\"App.MosaicHost\", \"MosaicHost\"] {{\n      guard let hostType = NSClassFromString(className) as? NSObject.Type else {{\n        continue\n      }}\n      if let bridge = hostType.init() as? MosaicHostBridgeObject {{\n        return bridge\n      }}\n    }}\n    return nil\n  }}\n}}\n\nprivate enum MosaicHostValue {{\n  static func string(_ props: [String: Any], _ key: String, fallback: String) -> String {{\n    if let value = props[key] as? String {{ return value }}\n    if let value = props[key] {{ return String(describing: value) }}\n    return fallback\n  }}\n\n  static func double(_ props: [String: Any], _ key: String, fallback: Double) -> Double {{\n    if let value = props[key] as? Double {{ return value }}\n    if let value = props[key] as? NSNumber {{ return value.doubleValue }}\n    if let value = props[key] as? String, let parsed = Double(value) {{ return parsed }}\n    return fallback\n  }}\n\n  static func bool(_ props: [String: Any], _ key: String, fallback: Bool) -> Bool {{\n    if let value = props[key] as? Bool {{ return value }}\n    if let value = props[key] as? NSNumber {{ return value.boolValue }}\n    if let value = props[key] as? String, let parsed = Bool(value) {{ return parsed }}\n    return fallback\n  }}\n\n  static func stringList(_ props: [String: Any], _ key: String, fallback: [String]) -> [String] {{\n    if let value = props[key] as? [String] {{ return value }}\n    if let value = props[key] as? [Any] {{ return value.map {{ String(describing: $0) }} }}\n    return fallback\n  }}\n\n  static func doubleList(_ props: [String: Any], _ key: String, fallback: [Double]) -> [Double] {{\n    if let value = props[key] as? [Double] {{ return value }}\n    if let value = props[key] as? [NSNumber] {{ return value.map {{ $0.doubleValue }} }}\n    return fallback\n  }}\n\n  static func boolList(_ props: [String: Any], _ key: String, fallback: [Bool]) -> [Bool] {{\n    if let value = props[key] as? [Bool] {{ return value }}\n    if let value = props[key] as? [NSNumber] {{ return value.map {{ $0.boolValue }} }}\n    return fallback\n  }}\n}}\n"
    )
}

fn sample_value_for_slot(slot: &SlotDecl) -> String {
    match &slot.default {
        Some(SlotDefault::Text(value)) => format!("\"{}\"", escape_swift_string(value)),
        Some(SlotDefault::Number(value)) if value.is_finite() => value.to_string(),
        Some(SlotDefault::Number(_)) => "0".to_string(),
        Some(SlotDefault::Bool(value)) => value.to_string(),
        None => sample_value_for_slot_type(&slot.r#type, &slot.name),
    }
}

fn sample_value_for_slot_type(slot_type: &SlotType, slot_name: &str) -> String {
    match slot_type {
        SlotType::Text => format!("\"Sample {}\"", kebab_to_pascal_case_for_label(slot_name)),
        SlotType::Number => "0".to_string(),
        SlotType::Bool => "false".to_string(),
        SlotType::Image => "\"sample-image\"".to_string(),
        SlotType::Color => "\"#808080\"".to_string(),
        SlotType::Node | SlotType::Component(_) => "AnyView(EmptyView())".to_string(),
        SlotType::List(_) => "[]".to_string(),
    }
}

fn kebab_to_pascal_case_for_label(s: &str) -> String {
    let mut out = String::new();
    for part in s.split('-').filter(|part| !part.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    if out.is_empty() {
        "Value".to_string()
    } else {
        out
    }
}

fn build_swiftui_platform_readme(component_name: &str) -> String {
    format!(
        "{BANNER_MD}# {component_name} - SwiftUI macOS and iOS-ready shell\n\nAuto-generated by `mosaic-compile --backend swiftui --emit-project`.\n\n## Prerequisites\n\n- Swift 5.10+ (Xcode 15.3+ or the standalone Swift toolchain).\n- macOS 13 (Ventura) or later for `swift run`.\n- Xcode 15.3+ for adding the generated SwiftUI sources to an iOS 16+ host target.\n\n## Run on macOS\n\n```sh\nswift run\n```\n\nThis builds and launches a SwiftUI app with a single window hosting `{component_name}View(...)`. The first run downloads no dependencies; the package only depends on SwiftUI system frameworks.\n\n## Use from iOS\n\n`Package.swift` pins both `.macOS(.v13)` and `.iOS(.v16)`, and generated source guards macOS-only modifiers. Add the generated `Sources/App/{component_name}.swift`, `Sources/App/App.swift`, and any host adapter files to an iOS app target, or import this package from Xcode and mount `{component_name}View(...)` from your own iOS `App` shell.\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `{component_name}.swift` | The Mosaic-compiled SwiftUI component emitted flat by single-component CLI runs. |\n| `Sources/App/{component_name}.swift` | The nested component source written by `mosaic-compile pkg` so SwiftPM can compile immediately. |\n| `Package.swift` | SwiftPM manifest with pinned Swift tools, macOS, and iOS deployment targets. |\n| `Sources/App/App.swift` | `@main App` with `WindowGroup` mounting `{component_name}View(...)` with sample slot values and a dispatch closure. |\n| `README.md` | This file. |\n\n## Layout\n\n`mosaic-compile pkg --backend swiftui --emit-project` writes a ready-to-build SwiftPM layout. If you used the single-component CLI and only have the flat `{component_name}.swift`, copy that file into `Sources/App/` next to `App.swift` before running `swift run`.\n\n## Editing\n\nGenerated files carry an AUTO-GENERATED banner. Re-running `mosaic-compile --emit-project` will overwrite them. To customise the shell, remove the banner from a file and rename or relocate it; the next `--emit-project` run will recreate the original at its original name without touching your forked copy.\n"
    )
}

#[allow(dead_code)]
fn build_swiftui_readme(component_name: &str) -> String {
    format!(
        "{BANNER_MD}# {component_name} — SwiftUI macOS shell\n\nAuto-generated by `mosaic-compile --backend swiftui --emit-project`.\n\n## Prerequisites\n\n- Swift 5.10+ (Xcode 15.3+ or the standalone Swift toolchain).\n- macOS 13 (Ventura) or later target machine.\n\n## Run\n\n```sh\nswift run\n```\n\nThis builds and launches a SwiftUI app with a single window hosting `{component_name}View(...)`. The first run downloads no dependencies — the package only depends on the SwiftUI system framework.\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `{component_name}.swift` | The Mosaic-compiled SwiftUI component. Place inside `Sources/App/` for SwiftPM to pick it up. |\n| `Package.swift` | SwiftPM manifest. Pinned swift-tools-version per UI32 spec §3.6.3. |\n| `Sources/App/App.swift` | `@main App` with `WindowGroup` mounting `{component_name}View(...)` with sample slot values and a dispatch closure. |\n| `README.md` | This file. |\n\n## Layout\n\nFor SwiftPM to compile the component, move `{component_name}.swift` into `Sources/App/` (next to `App.swift`):\n\n```sh\nmkdir -p Sources/App\nmv {component_name}.swift Sources/App/\nswift run\n```\n\n## Editing\n\nEvery file except `{component_name}.swift` carries an AUTO-GENERATED banner. Re-running `mosaic-compile --emit-project` will overwrite them. To customise the shell, remove the banner from a file and rename or relocate it; the next `--emit-project` run will recreate the original at its original name without touching your forked copy.\n"
    )
}

// =====================================================================
// Part-style lowering — `.msl` `part` blocks → SwiftUI view modifiers.
//
// Covers BASE styles AND state-block lowering (`state hover {...}`,
// `state selected {...}`).  When a layout node carries one or more
// `state-when-<name>: ( expr )` props (UI28-1 / Task #35), each state
// block's overriding properties are folded into the SwiftUI modifier
// chain as nested ternaries.  Author surface mirrors the React
// emitter's `state-when-X` mechanism (`mosaic_emit_react::pipeline`
// lines 895–960).
//
// The map shape is identical to the React emitter's
// `build_part_style_map` (`HashMap<String, …>` keyed by part name OR a
// composite `{part}:{state}` key) so the two backends can share
// downstream tooling that walks "which parts have author-declared
// styles?".  React stores the per-part lowering as a pre-rendered
// TSX-style string; SwiftUI stores it as the raw `Vec<StyleProp>` so
// we can re-lower per node with the right indentation, since SwiftUI's
// modifier chain is indent-sensitive whereas TSX's `style={{ ... }}` is
// not.
// =====================================================================

/// Map from `part` name (or composite `{part}:{state}` key) to its
/// style props.
///
/// Built once in [`from_pipeline`] and threaded through the view-tree
/// walker.  The base-state entries are keyed by the part name verbatim
/// (e.g. `cell`, `header`, `row`).  State-block entries are keyed under
/// a composite `{part}:{state}` key — `cell:selected`, `cell:editing`,
/// etc — mirroring the React emitter's `build_part_style_map` shape so
/// downstream tooling that walks "which parts have author-declared
/// styles?" can share the same data structure across backends.
///
/// Empty when the `.msl` declares no parts — every lookup returns
/// `None` and emission proceeds unchanged.
#[derive(Debug)]
struct PartStyleEntry {
    props: Vec<StyleProp>,
    transitions: Vec<StyleTransition>,
}

type PartStyleMap = HashMap<String, PartStyleEntry>;

const HOVER_STATE_HELPER_SWIFT: &str = r#"private struct _MosaicHoverState<Content: View>: View {
    @State private var isHovered = false
    private let content: (Bool) -> Content

    init(@ViewBuilder content: @escaping (Bool) -> Content) {
        self.content = content
    }

    @ViewBuilder
    var body: some View {
#if os(macOS)
        content(isHovered)
            .onHover { isHovered = $0 }
#else
        content(false)
#endif
    }
}
"#;

const FOCUS_STATE_HELPER_SWIFT: &str = r#"private struct _MosaicFocusState<Content: View>: View {
    @FocusState private var isFocused: Bool
    private let content: (Bool) -> Content

    init(@ViewBuilder content: @escaping (Bool) -> Content) {
        self.content = content
    }

    var body: some View {
        content(isFocused)
            .focused($isFocused)
    }
}
"#;

const PRESS_STATE_HELPER_SWIFT: &str = r#"private struct _MosaicPressState<Content: View>: View {
    @GestureState private var isPressed = false
    private let content: (Bool) -> Content

    init(@ViewBuilder content: @escaping (Bool) -> Content) {
        self.content = content
    }

    var body: some View {
        content(isPressed)
            .simultaneousGesture(
                DragGesture(minimumDistance: 0)
                    .updating($isPressed) { _, state, _ in state = true }
            )
    }
}
"#;

// =====================================================================
// HostTable column-widths threading — TableContext
// =====================================================================

/// Per-`HostTable` discovered context for column-width threading.
///
/// SwiftUI has no direct analog of HTML's `<colgroup><col width>` — the
/// kernel's `HostTableColGroup` block carries per-column widths via a
/// slot which the .mll's `For ( each: slot: column-widths, as: w,
/// index: cw ) { Col [col] ( width: ( w ) ) }` shape lifts to a list.
/// To render those widths in SwiftUI we attach `.frame(width:
/// columnWidths[Int(<idx>)])` to each cell view, where `<idx>` is the
/// column index from the enclosing cell-emitting `For`.
///
/// `column_widths_slot` is the Swift identifier (post-camel-case) for
/// the column-widths array.  It is `None` when the HostTable has no
/// `HostTableColGroup`, when the ColGroup does not contain a `For`
/// whose body is a `Col`, or when the For's `each:` is not a `SlotRef`.
/// In any of those cases the emitter falls back to today's
/// auto-sized-cell behaviour and the visible diff is just
/// `HStack(spacing: 0)` (still applied because the HostTable itself
/// signals border-collapse semantics).
///
/// The context is set only at HostTable entry and threaded down
/// through `emit_view_tree`/`emit_children`/`container`/`emit_for_swift`
/// /`emit_if_swift` so deeply-nested Rows inside e.g. a
/// `For (each: viewport-rows)` body still see it.  Width threading
/// itself fires only for Fors in *cell-position* (a direct child of a
/// Row inside the HostTable) — gated by [`emit_table_cell`] and the
/// `width_thread` flag on `emit_for_swift`.
struct TableContext {
    /// Swift identifier (e.g. `columnWidths`) referring to the
    /// column-widths array.  `None` when no addressable column-widths
    /// slot was discovered; cells render unchanged in that case.
    column_widths_slot: Option<String>,
}

/// Scan a `HostTable` node's immediate children for a
/// `HostTableColGroup > For (each: slot: <name>) { Col ... }` shape and
/// extract the column-widths slot name (camel-cased).
///
/// This is a structural match — the For's `each:` must be a `SlotRef`
/// (not an Expr or Keyword), it must have an `as:` binding, the body
/// must contain a `Col` primitive (the standard col-group cell tag
/// from UI31 §3.2).  When the match fails, returns a [`TableContext`]
/// with `column_widths_slot: None` — cells then render at their
/// natural text-content width, and only the `HStack(spacing: 0)`
/// edit applies.
///
/// The returned identifier is the camelCased slot name passed through
/// [`is_safe_swift_identifier`]: a `column-widths` slot becomes
/// `columnWidths`.  Authors who name their slot something the
/// identifier check rejects (which today's NAME grammar cannot
/// produce, but the gate is defense-in-depth) silently fall back to
/// `None`.
fn extract_table_context(host_table: &LayoutNode) -> TableContext {
    for child in &host_table.children {
        if child.tag != "HostTableColGroup" {
            continue;
        }
        // The col-group's first For child is the column-widths iterator.
        // Authors may add comments / annotations between elements, but
        // structurally the For is what we need; we accept the first one
        // we find rather than insisting on it being the only child.
        for cg_child in &child.children {
            if cg_child.tag != "For" {
                continue;
            }
            // The For body must be a `Col`.  We don't require it to be
            // the ONLY child of the For (the .mll grammar guarantees a
            // single child today, but the check is permissive — any
            // `Col` in the body is enough).
            let has_col = cg_child.children.iter().any(|n| n.tag == "Col");
            if !has_col {
                continue;
            }
            // Extract the `each:` slot.  An Expr- or Keyword-valued
            // `each:` cannot be threaded as an indexed Swift array here
            // (the index lookup requires a Swift identifier), so we
            // skip those and return `None`.
            if let Some(slot) = find_slot_ref_prop(cg_child, "each") {
                let camel = to_camel_case_first_lower(slot);
                if is_safe_swift_identifier(&camel) {
                    return TableContext {
                        column_widths_slot: Some(camel),
                    };
                }
            }
        }
    }
    TableContext {
        column_widths_slot: None,
    }
}

/// Build a `part_name → style entry` map from a [`StyleDef`].
///
/// Mirrors `mosaic_emit_react::pipeline::build_part_style_map` in shape
/// but keeps props and structured transitions rather than a pre-rendered
/// string. SwiftUI modifier chains are indent-sensitive, and transitions
/// need the matching state predicate at each call site.
///
/// Two key shapes populate the map:
///   * Bare part name (`cell`) → that part's base props and transitions.
///   * Composite `{part}:{state}` (`cell:selected`) → that state
///     block's overriding props and entry transitions.
///
/// Entries with neither props nor transitions are skipped.
fn build_part_style_map(style: &StyleDef) -> PartStyleMap {
    let mut out = PartStyleMap::with_capacity(style.parts.len());
    for part in &style.parts {
        if !part.base.is_empty() || !part.transitions.is_empty() {
            out.insert(
                part.name.clone(),
                PartStyleEntry {
                    props: part.base.clone(),
                    transitions: part.transitions.clone(),
                },
            );
        }
        // State blocks (`state selected { ... }`) are surfaced under a
        // composite key `{part}:{state}` so [`collect_state_layers`]
        // can look up the matching state-when prop without having to
        // walk `style.parts` again.  Matches the React emitter's
        // shape (`build_part_style_map` in `mosaic-emit-react`).
        for state in &part.states {
            if !state.props.is_empty() || !state.transitions.is_empty() {
                let key = format!("{}:{}", part.name, state.state);
                out.insert(
                    key,
                    PartStyleEntry {
                        props: state.props.clone(),
                        transitions: state.transitions.clone(),
                    },
                );
            }
        }
    }
    out
}

fn has_explicit_state_when(node: &LayoutNode, state_name: &str) -> bool {
    let prop_name = format!("state-when-{state_name}");
    node.props.iter().any(|prop| prop.name == prop_name)
}

fn automatic_hover_style<'a>(
    node: &LayoutNode,
    part_styles: &'a PartStyleMap,
) -> Option<&'a PartStyleEntry> {
    let part_name = node.part_name.as_deref()?;
    if has_explicit_state_when(node, "hover") {
        return None;
    }
    part_styles.get(&format!("{part_name}:hover"))
}

fn layout_uses_automatic_hover(node: &LayoutNode, part_styles: &PartStyleMap) -> bool {
    automatic_hover_style(node, part_styles).is_some()
        || node
            .children
            .iter()
            .any(|child| layout_uses_automatic_hover(child, part_styles))
}

fn primitive_supports_automatic_focus(tag: &str) -> bool {
    matches!(
        tag,
        "HostInput" | "HostNumberInput" | "HostButton" | "HostCheckbox" | "HostRadio" | "HostLink"
    )
}

fn automatic_focus_style<'a>(
    node: &LayoutNode,
    part_styles: &'a PartStyleMap,
) -> Option<&'a PartStyleEntry> {
    let part_name = node.part_name.as_deref()?;
    if !primitive_supports_automatic_focus(&node.tag) || has_explicit_state_when(node, "focused") {
        return None;
    }
    part_styles.get(&format!("{part_name}:focused"))
}

fn layout_uses_automatic_focus(node: &LayoutNode, part_styles: &PartStyleMap) -> bool {
    automatic_focus_style(node, part_styles).is_some()
        || node
            .children
            .iter()
            .any(|child| layout_uses_automatic_focus(child, part_styles))
}

fn primitive_supports_automatic_press(tag: &str) -> bool {
    matches!(
        tag,
        "HostButton" | "HostCheckbox" | "HostRadio" | "HostLink"
    )
}

fn automatic_press_style<'a>(
    node: &LayoutNode,
    part_styles: &'a PartStyleMap,
) -> Option<&'a PartStyleEntry> {
    let part_name = node.part_name.as_deref()?;
    if !primitive_supports_automatic_press(&node.tag) || has_explicit_state_when(node, "pressed") {
        return None;
    }
    part_styles.get(&format!("{part_name}:pressed"))
}

fn layout_uses_automatic_press(node: &LayoutNode, part_styles: &PartStyleMap) -> bool {
    automatic_press_style(node, part_styles).is_some()
        || node
            .children
            .iter()
            .any(|child| layout_uses_automatic_press(child, part_styles))
}

/// One state layer collected from a node's `state-when-<X>: ( expr )` props.
///
/// `cond_expr` is the Swift expression for the predicate (already
/// camel-cased / Expr-lowered by [`collect_state_layers`]); `props` is
/// the matching `state X { ... }` block's overriding props from the
/// `.msl`.
///
/// Order in `Vec<StateLayer>` is declaration order on the node, which
/// is also the order React emits state-when spreads in — so when we
/// fold these into a nested ternary in [`swiftui_modifier_chain`], the
/// LAST layer becomes the OUTERMOST condition (highest precedence).
struct StateLayer<'a> {
    /// The Swift conditional expression text, e.g.
    /// `( r == selectedRow && c == selectedCol )`.
    cond_expr: String,
    /// The state block's overriding properties.
    props: &'a [StyleProp],
    /// Transitions used while entering this state.
    transitions: &'a [StyleTransition],
}

/// Walk a layout node's props for `state-when-<X>: ( expr )` entries
/// and produce a list of [`StateLayer`] values that the modifier-chain
/// builder can fold into nested ternaries.
///
/// The author surface:
///
/// ```text
/// Box [ cell ] (
///   state-when-selected: slot: is-selected ,
///   state-when-editing:  slot: is-editing
/// ) { ... }
/// ```
///
/// After UI34 package-resolver substitution, the slot refs typically
/// become `LayoutPropValue::Expr` values like
/// `( r == selectedRow && c == selectedCol )`.
///
/// **Trust model.**  The `cond_expr` text comes from the developer-
/// supplied `.msl` / `.mll` source and is interpolated verbatim into
/// the generated Swift inside a parenthesised position.  The moslayout
/// parser already wraps `Expr` values in `( ... )`, so balanced
/// parentheses keep the expression contained.  This mirrors what the
/// React emitter does — we don't add new escaping (see
/// `mosaic_emit_react::pipeline` lines 895–960).
fn collect_state_layers<'a>(
    node: &LayoutNode,
    part_name: &str,
    part_styles: &'a PartStyleMap,
) -> Vec<StateLayer<'a>> {
    let mut layers = Vec::new();
    for prop in &node.props {
        let Some(state_name) = prop.name.strip_prefix("state-when-") else {
            continue;
        };
        let state_key = format!("{part_name}:{state_name}");
        let Some(state_style) = part_styles.get(&state_key) else {
            // `state-when-X` declared without a matching `state X { }`
            // block — silently skip (matches React's posture).
            continue;
        };
        let cond_expr = match &prop.value {
            LayoutPropValue::Expr(t) => t.clone(),
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Keyword(k) => k.clone(),
            // EmitRef / Number / String don't make sense as boolean
            // predicates — drop the whole layer rather than emit a
            // condition that won't compile.
            _ => continue,
        };
        layers.push(StateLayer {
            cond_expr,
            props: state_style.props.as_slice(),
            transitions: state_style.transitions.as_slice(),
        });
    }
    layers
}

fn collect_state_layers_for_emission<'a>(
    node: &LayoutNode,
    part_name: &str,
    part_styles: &'a PartStyleMap,
    uses_automatic_hover: bool,
    uses_automatic_focus: bool,
    uses_automatic_press: bool,
) -> Vec<StateLayer<'a>> {
    let mut layers = Vec::new();
    if uses_automatic_hover {
        if let Some(hover_style) = part_styles.get(&format!("{part_name}:hover")) {
            layers.push(StateLayer {
                cond_expr: "__mosaicHoverActive".to_string(),
                props: hover_style.props.as_slice(),
                transitions: hover_style.transitions.as_slice(),
            });
        }
    }
    if uses_automatic_focus {
        if let Some(focus_style) = part_styles.get(&format!("{part_name}:focused")) {
            layers.push(StateLayer {
                cond_expr: "__mosaicFocusActive".to_string(),
                props: focus_style.props.as_slice(),
                transitions: focus_style.transitions.as_slice(),
            });
        }
    }
    if uses_automatic_press {
        if let Some(press_style) = part_styles.get(&format!("{part_name}:pressed")) {
            layers.push(StateLayer {
                cond_expr: "__mosaicPressActive".to_string(),
                props: press_style.props.as_slice(),
                transitions: press_style.transitions.as_slice(),
            });
        }
    }
    layers.extend(collect_state_layers(node, part_name, part_styles));
    layers
}

/// Strip a trailing `px` suffix from a CSS length value.
///
/// `strip_css_px("12px")` → `"12"`; `strip_css_px("auto")` → `"auto"`.
/// We're permissive: anything that doesn't end in `px` falls through
/// unchanged.  SwiftUI's modifier vocabulary treats numeric lengths as
/// device-independent points (the same magnitude as CSS `px` at 1×), so
/// dropping the suffix is the right lowering — no unit conversion.
fn strip_css_px(v: &str) -> &str {
    v.strip_suffix("px").unwrap_or(v)
}

/// Convert a CSS color value to a Swift `Color(...)` expression.
///
/// Recognised forms:
///
/// | input            | output                                                 |
/// |------------------|--------------------------------------------------------|
/// | `#rrggbb`        | `Color(red: R, green: G, blue: B)` (each / 255, 3 dp)  |
/// | `#rgb`           | as above, after expanding each nibble (`#abc` → `#aabbcc`) |
/// | named CSS color  | `Color.white` / `Color.black` / `Color.clear` etc.     |
/// | anything else    | `Color.clear` (safe fall-through, file still compiles) |
///
/// The 3-decimal-place rounding is load-bearing for the unit tests:
/// `0x1e / 255` is `0.117647…`, which we round to `0.118` so the
/// expected-string assertions stay short and stable.  3 dp is also
/// well below the precision SwiftUI ultimately renders at (its color
/// pipeline quantises to 8-bit per channel before display), so the
/// rounding is lossless in practice.
fn swiftui_color_value(v: &str) -> String {
    let trimmed = v.trim();

    // Hex shorthand (`#rgb`) — expand each nibble to a byte (`#a` →
    // `#aa`) and fall into the 6-digit path.
    let expanded: String;
    let hex_body: Option<&str> = if let Some(body) = trimmed.strip_prefix('#') {
        if body.len() == 3 && body.chars().all(|c| c.is_ascii_hexdigit()) {
            expanded = body.chars().flat_map(|c| [c, c]).collect::<String>();
            Some(expanded.as_str())
        } else if body.len() == 6 && body.chars().all(|c| c.is_ascii_hexdigit()) {
            Some(body)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(hex) = hex_body {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        let rf = round3(f64::from(r) / 255.0);
        let gf = round3(f64::from(g) / 255.0);
        let bf = round3(f64::from(b) / 255.0);
        return format!("Color(red: {rf}, green: {gf}, blue: {bf})");
    }

    // Named CSS colors that SwiftUI has a direct enum case for.  Unknown
    // names fall through to `Color.clear` so the generated file still
    // compiles even if the author wrote something exotic.
    match trimmed {
        "white" => "Color.white".to_string(),
        "black" => "Color.black".to_string(),
        "clear" | "transparent" => "Color.clear".to_string(),
        "red" => "Color.red".to_string(),
        "green" => "Color.green".to_string(),
        "blue" => "Color.blue".to_string(),
        "yellow" => "Color.yellow".to_string(),
        "orange" => "Color.orange".to_string(),
        "pink" => "Color.pink".to_string(),
        "purple" => "Color.purple".to_string(),
        "gray" | "grey" => "Color.gray".to_string(),
        _ => "Color.clear".to_string(),
    }
}

/// Round a float to three decimal places.
///
/// SwiftUI `Color(red:green:blue:)` takes `Double`s in the `[0, 1]`
/// range; rounding to 3 dp keeps the emitted source short and the
/// unit-test expected strings stable.  See [`swiftui_color_value`] for
/// why 3 dp is enough precision.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Convert a resolved MSL duration to deterministic Swift seconds.
fn swiftui_duration_seconds(duration: &str) -> Option<String> {
    let duration = duration.trim();
    let seconds = if let Some(milliseconds) = duration.strip_suffix("ms") {
        milliseconds.trim().parse::<f64>().ok()? / 1000.0
    } else {
        let seconds = duration.strip_suffix('s')?;
        seconds.trim().parse::<f64>().ok()?
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let mut rendered = format!("{seconds:.6}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.push('0');
    }
    Some(rendered)
}

/// Lower one resolved MSL easing curve to a SwiftUI `Animation`.
fn swiftui_animation(transition: &StyleTransition) -> Option<String> {
    let duration = swiftui_duration_seconds(&transition.duration)?;
    let easing = transition.easing.trim();
    let constructor = match easing {
        "linear" => format!("Animation.linear(duration: {duration})"),
        "ease" | "ease-in-out" => {
            format!("Animation.easeInOut(duration: {duration})")
        }
        "ease-in" => format!("Animation.easeIn(duration: {duration})"),
        "ease-out" => format!("Animation.easeOut(duration: {duration})"),
        _ => {
            let values = easing
                .strip_prefix("cubic-bezier(")?
                .strip_suffix(')')?
                .split(',')
                .map(str::trim)
                .map(str::parse::<f64>)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            if values.len() != 4 || values.iter().any(|value| !value.is_finite()) {
                return None;
            }
            format!(
                "Animation.timingCurve({}, {}, {}, {}, duration: {duration})",
                values[0], values[1], values[2], values[3]
            )
        }
    };
    Some(constructor)
}

/// Resolve the effective animation for one style property.
///
/// Base transitions are the fallback for every state change. A transition
/// declared inside a state replaces that fallback only while entering that
/// state. Leaving the state therefore uses the base transition, or no
/// animation when the part has no base transition.
fn swiftui_animation_for_property(
    property: &str,
    base_transitions: &[StyleTransition],
    state_layers: &[StateLayer],
) -> Option<String> {
    let mut animation = base_transitions
        .iter()
        .rev()
        .find(|transition| transition.property == property)
        .and_then(swiftui_animation);

    for layer in state_layers {
        let Some(state_animation) = layer
            .transitions
            .iter()
            .rev()
            .find(|transition| transition.property == property)
            .and_then(swiftui_animation)
        else {
            continue;
        };
        animation = Some(match animation {
            Some(fallback) => format!(
                "(({}) ? {} : {})",
                layer.cond_expr, state_animation, fallback
            ),
            None => format!("(({}) ? {} : nil)", layer.cond_expr, state_animation),
        });
    }
    animation
}

/// Attach property-scoped implicit animation to the current modifier.
///
/// Observing the final lowered property expression instead of the raw state
/// predicate means SwiftUI only starts the animation when that property's
/// value actually changes.
fn push_swiftui_animation(
    out: &mut String,
    pad: &str,
    property: &str,
    value_expr: &str,
    base_transitions: &[StyleTransition],
    state_layers: &[StateLayer],
) {
    if let Some(animation) =
        swiftui_animation_for_property(property, base_transitions, state_layers)
    {
        out.push_str(&format!(
            "\n{pad}.animation({animation}, value: {value_expr})"
        ));
    }
}

/// Build a SwiftUI view-modifier chain from a slice of [`StyleProp`].
///
/// Each modifier renders on its own line, prefixed by `\n` + an indent of
/// `indent` spaces.  When no property in `props` maps to a recognised
/// modifier the returned string is empty — callers MUST treat empty as
/// "do not splice anything" so a node with only unknown styles renders
/// identically to a node with no part_name at all.
///
/// ## Property → modifier table (v1 coverage)
///
/// | Mosstyle property                              | SwiftUI                                                          |
/// |------------------------------------------------|------------------------------------------------------------------|
/// | `width: Npx` + `height: Npx`                   | `.frame(width: N, height: N)` (single call)                      |
/// | `width: Npx` alone                             | `.frame(width: N)`                                               |
/// | `height: Npx` alone                            | `.frame(height: N)`                                              |
/// | `padding: Npx`                                 | `.padding(N)`                                                    |
/// | `background: <color>`                          | `.background(<color>)`                                           |
/// | `color: <color>`                               | `.foregroundColor(<color>)`                                      |
/// | `font-size: Npx` + `font-family: monospace`    | `.font(.system(size: N, design: .monospaced))`                   |
/// | `font-size: Npx` alone                         | `.font(.system(size: N))`                                        |
/// | `font-family: monospace` alone                 | `.font(.system(.body, design: .monospaced))`                     |
/// | `font-weight: bold` / `700`                    | `.fontWeight(.bold)`                                             |
/// | `font-weight: semibold` / `SemiBold` / `600`   | `.fontWeight(.semibold)`                                         |
/// | `font-weight: medium` / `500`                  | `.fontWeight(.medium)`                                           |
/// | `border-width: Npx` (+ optional `border-color`)| `.border(<color or Color.gray>, width: N)`                       |
/// | `opacity: N`                                   | `.opacity(N)`                                                    |
/// | `text-align: left`/`center`/`right` (base only)| `alignment:` arg of `.frame(...)` (`.leading`/`.center`/`.trailing`) |
/// | anything else                                  | silently skipped (React emitter does the same for v1)            |
///
/// `border-style: solid` is silently ignored — SwiftUI's `.border` is
/// always a solid stroke.  `border-color` without `border-width` is also
/// dropped (the modifier needs the width).
///
/// ## Modifier ORDER (load-bearing)
///
/// Emitted top-to-bottom: `.foregroundColor` → `.font` → `.fontWeight`
/// → `.padding` → `.frame(width:,height:,alignment:)` → `.background`
/// → `.border` → `.opacity`. Content styling and sizing come BEFORE paint
/// so the background fills, and the border strokes, the full frame rather
/// than a text-sized box. See the inline comment in the emission body.
///
/// ## `injected_width`
///
/// When `Some(expr)`, the `.frame(...)` ALWAYS emits and uses `expr`
/// (a Swift expression such as `columnWidths[Int(c)]`) for `width:`,
/// taking precedence over any `width` from the part's own props.  This
/// is how the HostTable column-width thread injects the cell width INTO
/// the chain at the correct position (before background/border) instead
/// of appending a trailing `.frame(width:)` that paints too late.
#[cfg(test)]
fn swiftui_modifier_chain(
    base_props: &[StyleProp],
    state_layers: &[StateLayer],
    indent: usize,
    injected_width: Option<&str>,
) -> String {
    swiftui_modifier_chain_with_transitions(base_props, &[], state_layers, indent, injected_width)
}

fn swiftui_modifier_chain_with_transitions(
    base_props: &[StyleProp],
    base_transitions: &[StyleTransition],
    state_layers: &[StateLayer],
    indent: usize,
    injected_width: Option<&str>,
) -> String {
    let pad = " ".repeat(indent);

    // Only accept `Npx` (or unitless `N`) for numeric length values.
    // `100%`, `auto`, `calc(...)` and other CSS forms have no clean
    // SwiftUI analog at this level — silently skip per the v1 cut.
    fn px_or_none(v: &str) -> Option<String> {
        let stripped = strip_css_px(v);
        if stripped
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
            && !stripped.is_empty()
        {
            Some(stripped.to_string())
        } else {
            None
        }
    }

    fn font_weight_swift(v: &str) -> Option<&'static str> {
        match v.trim() {
            "bold" | "700" => Some(".bold"),
            "semibold" | "SemiBold" | "600" => Some(".semibold"),
            "medium" | "500" => Some(".medium"),
            _ => None,
        }
    }

    // Map a CSS `text-align` value to a SwiftUI `Alignment`.  The
    // alignment becomes the `alignment:` argument of the `.frame(...)`
    // call so the (now full-width) cell positions its content the way
    // the author asked — `text-align: right` on a spreadsheet number
    // column pins the digits to the right edge, etc.  Unrecognised
    // values (`justify`, etc.) map to `None` so no alignment is emitted.
    fn text_align_swift(v: &str) -> Option<&'static str> {
        match v.trim() {
            "left" | "start" => Some(".leading"),
            "center" => Some(".center"),
            "right" | "end" => Some(".trailing"),
            _ => None,
        }
    }

    let layer_count = state_layers.len();

    // One bucket per logical property.  We collect base values first,
    // then walk each state layer's props with the same matcher.  Each
    // bucket tracks an `Option<String>` per layer index so
    // [`layer_value`] can fold them into a nested ternary.
    let mut width = PropBucket::new(layer_count);
    let mut height = PropBucket::new(layer_count);
    let mut padding = PropBucket::new(layer_count);
    let mut background = PropBucket::new(layer_count);
    let mut foreground = PropBucket::new(layer_count);
    let mut font_size = PropBucket::new(layer_count);
    let mut font_family_mono = PropBucket::new(layer_count);
    let mut font_weight = PropBucket::new(layer_count);
    let mut border_width = PropBucket::new(layer_count);
    let mut border_color = PropBucket::new(layer_count);
    let mut opacity = PropBucket::new(layer_count);

    // `text-align` is a static layout concern — we deliberately do NOT
    // layer it per state (a per-state alignment flip is never needed for
    // the table/spreadsheet cases this v1 targets, and it would force a
    // ternary into the `.frame(alignment:)` argument, which SwiftUI does
    // not accept there).  We therefore capture only the BASE value and
    // resolve it to a SwiftUI `Alignment` token (`.leading` / `.center`
    // / `.trailing`).  See [`text_align_swift`].
    let mut text_align: Option<&'static str> = None;

    // Helper closure: dispatch one StyleProp into the right bucket.
    // `layer_idx == None` means "base"; `Some(i)` means "state layer i".
    let mut absorb = |p: &StyleProp, layer_idx: Option<usize>| {
        let set = |bucket: &mut PropBucket, v: String| match layer_idx {
            None => bucket.base = Some(v),
            Some(i) => bucket.set_state(i, v),
        };
        match p.name.as_str() {
            "width" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut width, v);
                }
            }
            "height" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut height, v);
                }
            }
            "padding" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut padding, v);
                }
            }
            "background" | "background-color" => {
                set(&mut background, swiftui_color_value(&p.value));
            }
            "color" => set(&mut foreground, swiftui_color_value(&p.value)),
            "font-size" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut font_size, v);
                }
            }
            "font-family" => {
                // We only recognise `monospace`; other font families
                // need a Swift `Font` lookup by name which is a v2
                // feature (the CSS family list `'Menlo, monospace'`
                // doesn't map cleanly without a font-resolution pass).
                // Marker value is the literal `"true"`.
                if p.value.trim() == "monospace" {
                    set(&mut font_family_mono, "true".to_string());
                }
            }
            "font-weight" => {
                if let Some(w) = font_weight_swift(&p.value) {
                    set(&mut font_weight, w.to_string());
                }
            }
            "border-width" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut border_width, v);
                }
            }
            "border-color" => set(&mut border_color, swiftui_color_value(&p.value)),
            "opacity" => {
                if let Some(v) = px_or_none(&p.value) {
                    set(&mut opacity, v);
                }
            }
            "text-align"
                // Base-only by design (see the `text_align` binding's
                // doc comment).  A state layer that sets `text-align` is
                // intentionally ignored — only `layer_idx == None` wins.
                if layer_idx.is_none() => {
                    if let Some(a) = text_align_swift(&p.value) {
                        text_align = Some(a);
                    }
                }
            // border-style, border-collapse, outline, etc. — silently
            // skipped.  Matches the React emitter's v1 posture.
            _ => {}
        }
    };

    for p in base_props {
        absorb(p, None);
    }
    for (i, layer) in state_layers.iter().enumerate() {
        for p in layer.props {
            absorb(p, Some(i));
        }
    }

    let mut out = String::new();

    // -----------------------------------------------------------------
    // MODIFIER ORDER (this ordering is load-bearing — see the bug fix in
    // PR feat/emit-swiftui-cell-fill-and-alignment).
    //
    // SwiftUI modifiers wrap the view left-to-right: each `.background`
    // / `.border` paints around the view *at its size at that point in
    // the chain*.  So content-styling and sizing MUST come BEFORE
    // paint, or the background/border draw around the text-sized box and
    // a later `.frame` just re-centers that small painted box inside a
    // bigger invisible frame.
    //
    // Correct order, top to bottom:
    //   1. .foregroundColor  ─┐ content styling — affects the text only,
    //   2. .font              │ independent of the box geometry, so it
    //   3. .fontWeight       ─┘ goes first.
    //   4. .padding           — insets the content.
    //   5. .frame(width:,height:,alignment:) — fixes the cell's box size
    //      and positions the (padded) content within it.
    //   6. .background        — fills the FRAME (now full cell size).
    //   7. .border            — strokes the FRAME edge.
    // -----------------------------------------------------------------

    // 1. .foregroundColor
    if !foreground.empty() {
        let expr = layer_value(&foreground, state_layers, "Color.primary");
        out.push_str(&format!("\n{pad}.foregroundColor({expr})"));
        push_swiftui_animation(
            &mut out,
            &pad,
            "color",
            &expr,
            base_transitions,
            state_layers,
        );
    }

    // 2. + 3. .font — combine size + monospaced family into one call
    // when either is present.  Standalone family lowers to
    // `.system(.body, design: ...)` because `.font(.system(size:))`
    // requires the size; we use `.body` as the implicit textstyle when
    // only the family is set.
    match (!font_size.empty(), !font_family_mono.empty()) {
        (true, true) => {
            let sz = layer_value(&font_size, state_layers, "0");
            out.push_str(&format!(
                "\n{pad}.font(.system(size: {sz}, design: .monospaced))"
            ));
            push_swiftui_animation(
                &mut out,
                &pad,
                "font-size",
                &sz,
                base_transitions,
                state_layers,
            );
        }
        (true, false) => {
            let sz = layer_value(&font_size, state_layers, "0");
            out.push_str(&format!("\n{pad}.font(.system(size: {sz}))"));
            push_swiftui_animation(
                &mut out,
                &pad,
                "font-size",
                &sz,
                base_transitions,
                state_layers,
            );
        }
        (false, true) => out.push_str(&format!(
            "\n{pad}.font(.system(.body, design: .monospaced))"
        )),
        (false, false) => {}
    }

    // 3. .fontWeight
    if !font_weight.empty() {
        let expr = layer_value(&font_weight, state_layers, ".regular");
        out.push_str(&format!("\n{pad}.fontWeight({expr})"));
    }

    // 4. .padding — insets the content before the frame sizes it.
    if !padding.empty() {
        let expr = layer_value(&padding, state_layers, "0");
        out.push_str(&format!("\n{pad}.padding({expr})"));
        push_swiftui_animation(
            &mut out,
            &pad,
            "padding",
            &expr,
            base_transitions,
            state_layers,
        );
    }

    // 5. .frame(width:, height:, alignment:) — the cell's box.
    //
    // An `injected_width` (the threaded column-width Swift expression,
    // e.g. `columnWidths[Int(c)]`) takes precedence over any `width`
    // from the part's own props and FORCES the frame to emit.  The
    // alignment argument (resolved from base `text-align`) is appended
    // when present.  Three width sources, in precedence order:
    //   a. injected width        → always wins.
    //   b. part `width` prop      → used when no injected width.
    //   c. (none)                 → frame may still emit for height
    //      and/or alignment only.
    let align_arg = text_align.map(|a| format!(", alignment: {a}"));
    let frame_width: Option<String> = if let Some(iw) = injected_width {
        Some(iw.to_string())
    } else if !width.empty() {
        Some(layer_value(&width, state_layers, "0"))
    } else {
        None
    };
    let frame_height: Option<String> = if !height.empty() {
        Some(layer_value(&height, state_layers, "0"))
    } else {
        None
    };
    let animated_width = frame_width.clone();
    let animated_height = frame_height.clone();
    match (frame_width, frame_height) {
        (Some(w), Some(h)) => {
            let a = align_arg.as_deref().unwrap_or("");
            out.push_str(&format!("\n{pad}.frame(width: {w}, height: {h}{a})"));
        }
        (Some(w), None) => {
            let a = align_arg.as_deref().unwrap_or("");
            out.push_str(&format!("\n{pad}.frame(width: {w}{a})"));
        }
        (None, Some(h)) => {
            let a = align_arg.as_deref().unwrap_or("");
            out.push_str(&format!("\n{pad}.frame(height: {h}{a})"));
        }
        (None, None) => {
            // No explicit size, but an alignment was requested.  Stretch
            // to the available width so the content can actually shift to
            // the requested edge — without a width, `.frame(alignment:)`
            // alone has nothing to align within.  `.infinity` is the v1
            // cut: it claims all available horizontal space, which is the
            // right behaviour for a cell/label but may over-expand a
            // free-floating element.  A future cut could gate this on a
            // container hint.
            if let Some(a) = text_align {
                out.push_str(&format!(
                    "\n{pad}.frame(maxWidth: .infinity, alignment: {a})"
                ));
            }
        }
    }
    if let Some(value) = animated_width {
        push_swiftui_animation(
            &mut out,
            &pad,
            "width",
            &value,
            base_transitions,
            state_layers,
        );
    }
    if let Some(value) = animated_height {
        push_swiftui_animation(
            &mut out,
            &pad,
            "height",
            &value,
            base_transitions,
            state_layers,
        );
    }

    // 6. .background — fills the frame (now full cell size).  A state
    // that overrides background where there's NO base value gets
    // `Color.clear` in the "no value" branch — matches the SwiftUI
    // default for an unstyled view.
    if !background.empty() {
        let expr = layer_value(&background, state_layers, "Color.clear");
        out.push_str(&format!("\n{pad}.background({expr})"));
        push_swiftui_animation(
            &mut out,
            &pad,
            "background",
            &expr,
            base_transitions,
            state_layers,
        );
        push_swiftui_animation(
            &mut out,
            &pad,
            "background-color",
            &expr,
            base_transitions,
            state_layers,
        );
    }

    // 7. .border — strokes the frame edge.  Needs at least the width.
    // If color is unset, default to `Color.gray` so the modifier emits a
    // visible (and predictable) stroke rather than nothing.  Each
    // argument is layered independently.
    if !border_width.empty() {
        let w_expr = layer_value(&border_width, state_layers, "0");
        let c_expr = if border_color.empty() {
            "Color.gray".to_string()
        } else {
            layer_value(&border_color, state_layers, "Color.gray")
        };
        out.push_str(&format!("\n{pad}.border({c_expr}, width: {w_expr})"));
        push_swiftui_animation(
            &mut out,
            &pad,
            "border-color",
            &c_expr,
            base_transitions,
            state_layers,
        );
        push_swiftui_animation(
            &mut out,
            &pad,
            "border-width",
            &w_expr,
            base_transitions,
            state_layers,
        );
    }

    // 8. .opacity — compositing applies to the fully styled part.
    if !opacity.empty() {
        let expr = layer_value(&opacity, state_layers, "1");
        out.push_str(&format!("\n{pad}.opacity({expr})"));
        push_swiftui_animation(
            &mut out,
            &pad,
            "opacity",
            &expr,
            base_transitions,
            state_layers,
        );
    }

    out
}

/// Per-property "bucket" — collects the base value and per-state
/// overrides for one logical mosstyle property.
///
/// Used internally by [`swiftui_modifier_chain`] to layer state
/// overrides on top of the base value.  `state_values` is parallel to
/// the input `state_layers` slice: index `i` of `state_values` holds
/// the lowered Swift value (or `None`) for `state_layers[i]`.
///
/// `any_state_set` is a quick predicate: "does at least one state in
/// this chain override this property?"  When it's false, the chain is
/// just the base — no ternary, no parens.
struct PropBucket {
    base: Option<String>,
    state_values: Vec<Option<String>>,
    any_state_set: bool,
}

impl PropBucket {
    fn new(layer_count: usize) -> Self {
        Self {
            base: None,
            state_values: vec![None; layer_count],
            any_state_set: false,
        }
    }
    fn set_state(&mut self, idx: usize, v: String) {
        self.state_values[idx] = Some(v);
        self.any_state_set = true;
    }
    /// True if no base AND no state has this property — caller should
    /// emit no modifier line at all.
    fn empty(&self) -> bool {
        self.base.is_none() && !self.any_state_set
    }
}

/// Fold a [`PropBucket`] into a Swift expression string.
///
/// * Base-only → returns the raw base value.
/// * State-only or state+base → returns a nested ternary, with the
///   LAST state layer as the OUTERMOST condition.  Shape:
///   `((condN) ? valN : ((condN-1) ? valN-1 : ... : base_or_default))`.
///
/// `default_when_unset` is the fallback used when there's no base
/// value AND a state-override fires — typically `Color.clear` for
/// background, `Color.primary` for foreground, `"0"` for numerics.
///
/// When a layer doesn't override this property, its branch collapses
/// into the next layer down (or the base) — no `cond ? base : base`
/// noise.
fn layer_value(
    bucket: &PropBucket,
    state_layers: &[StateLayer],
    default_when_unset: &str,
) -> String {
    let mut inner = bucket
        .base
        .clone()
        .unwrap_or_else(|| default_when_unset.to_string());
    // Iterate from FIRST to LAST so the last layer ends up wrapping
    // all the earlier ones — that's how it becomes the outermost
    // (highest-precedence) condition.
    for (i, layer) in state_layers.iter().enumerate() {
        let v = bucket.state_values[i]
            .clone()
            .unwrap_or_else(|| inner.clone());
        if v == inner {
            continue;
        }
        inner = format!("(({}) ? {} : {})", layer.cond_expr, v, inner);
    }
    inner
}

/// Compile a three-file Mosaic pipeline triple to a SwiftUI source file.
///
/// The `style` argument's `part` blocks are inlined as SwiftUI view-
/// modifier chains attached to layout nodes whose `part_name` matches.
/// See [`swiftui_modifier_chain`] for the property→modifier table.
/// State blocks (`state X { ... }`) are folded into the chain as
/// nested ternaries when the matching node carries a
/// `state-when-X: ( expr )` prop — see [`collect_state_layers`] for
/// the predicate-extraction rules.
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

    // Build the part-style map ONCE per emission and thread it through
    // the view-tree walker.  Empty when the `.msl` declares no parts,
    // which is the no-op path — every modifier-chain lookup returns
    // None and emission proceeds identically to a styleless pipeline.
    let part_styles = build_part_style_map(style);

    let name = &interface.component;
    let mut out = String::new();

    // 2. File header. `import SwiftUI` is always required because every
    //    primitive lowering names a SwiftUI type — there is no equivalent
    //    of React's "do I need to import React?" tree-shake check here.
    writeln!(
        out,
        "// Auto-generated by mosaic-emit-swiftui. Do not edit."
    )
    .unwrap();
    writeln!(out, "import SwiftUI").unwrap();
    writeln!(out).unwrap();

    if layout_uses_automatic_hover(&layout.root, &part_styles) {
        out.push_str(HOVER_STATE_HELPER_SWIFT);
        writeln!(out).unwrap();
    }
    if layout_uses_automatic_focus(&layout.root, &part_styles) {
        out.push_str(FOCUS_STATE_HELPER_SWIFT);
        writeln!(out).unwrap();
    }
    if layout_uses_automatic_press(&layout.root, &part_styles) {
        out.push_str(PRESS_STATE_HELPER_SWIFT);
        writeln!(out).unwrap();
    }

    // 3. Event enum (analog of UI24 §3.1 event union).
    out.push_str(&emit_event_union(name, &interface.emits)?);
    writeln!(out).unwrap();

    // 4. View struct: properties + body computed property.
    out.push_str(&emit_view_struct(
        name,
        &interface.slots,
        &interface.emits,
        &layout.root,
        &part_styles,
    )?);

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
                validate_slot_or_field_name(&label).map_err(PipelineEmitError::UnsafeSlotName)?;
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
    out.push_str(&emit_event_wire_extension(component, emits)?);
    Ok(out)
}

fn host_button_event_args(
    emit: &EmitDecl,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    if emit.params.is_empty() {
        return Ok(String::new());
    }
    if emit.params.len() == 1 {
        let param = &emit.params[0];
        let field = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
        let expr = host_button_payload_expr(&param.r#type, for_payload)
            .unwrap_or_else(|| "/* TODO: payload */ fatalError(\"TODO: payload\")".to_string());
        return Ok(format!("{field}: {expr}"));
    }

    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
            Ok(format!(
                "{field}: /* TODO: payload */ fatalError(\"TODO: payload\")"
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

fn host_button_payload_expr(
    t: &EmitPayloadType,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Option<String> {
    let scope = for_payload?;
    match t {
        EmitPayloadType::Text | EmitPayloadType::Color | EmitPayloadType::Component(_) => {
            Some(scope.item.to_string())
        }
        EmitPayloadType::Number => scope.index.map(str::to_string),
        EmitPayloadType::Bool => None,
    }
}

fn emit_event_wire_extension(
    component: &str,
    emits: &[EmitDecl],
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    writeln!(out).unwrap();
    writeln!(out, "extension {component}Event {{").unwrap();
    writeln!(out, "    var mosaicName: String {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for emit in emits {
        let case_name = event_case_name(emit)?;
        let pattern = swift_event_case_pattern(&case_name, emit, false)?;
        let event_name = escape_swift_string(&emit.name);
        writeln!(out, "        case {pattern}: return \"{event_name}\"").unwrap();
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    var mosaicPayload: [String: Any] {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for emit in emits {
        let case_name = event_case_name(emit)?;
        let pattern = swift_event_case_pattern(&case_name, emit, true)?;
        let payload = swift_event_payload_dictionary(emit)?;
        writeln!(out, "        case {pattern}: return {payload}").unwrap();
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    var mosaicEnvelope: [String: Any] {{").unwrap();
    writeln!(out, "        var envelope = mosaicPayload").unwrap();
    writeln!(out, "        envelope[\"event\"] = mosaicName").unwrap();
    writeln!(out, "        return envelope").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn event_case_name(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    let lowered = strip_on_prefix(&emit.name);
    let case_name = to_camel_case_first_lower(&lowered);
    validate_emit_name(&case_name)?;
    Ok(case_name)
}

fn swift_event_case_pattern(
    case_name: &str,
    emit: &EmitDecl,
    bind_payload: bool,
) -> Result<String, PipelineEmitError> {
    if emit.params.is_empty() {
        return Ok(format!(".{case_name}"));
    }

    let mut fields = Vec::with_capacity(emit.params.len());
    for param in &emit.params {
        let label = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&label).map_err(PipelineEmitError::UnsafeSlotName)?;
        if bind_payload {
            fields.push(format!("{label}: {label}"));
        } else {
            fields.push(format!("{label}: _"));
        }
    }

    if bind_payload {
        Ok(format!("let .{case_name}({})", fields.join(", ")))
    } else {
        Ok(format!(".{case_name}({})", fields.join(", ")))
    }
}

fn swift_event_payload_dictionary(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    if emit.params.is_empty() {
        return Ok("[:]".to_string());
    }

    let mut fields = Vec::with_capacity(emit.params.len());
    for param in &emit.params {
        let label = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&label).map_err(PipelineEmitError::UnsafeSlotName)?;
        fields.push(format!("\"{label}\": {label}"));
    }
    Ok(format!("[{}]", fields.join(", ")))
}

/// Emit the SwiftUI `struct {Component}View: View { ... }` declaration.
///
/// Order matches the React backend's destructuring order: slots in source
/// order, then `dispatch` last so it stands out in code review.
fn emit_view_struct(
    component: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
    layout_root: &LayoutNode,
    part_styles: &PartStyleMap,
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
    let body = emit_view_tree(layout_root, 8, part_styles, emits, None, None, None)?;
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
fn emit_view_tree(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
    // The threaded column-width Swift expression (e.g.
    // `columnWidths[Int(c)]`) for THIS node only — `Some` exactly when
    // this is a direct body child of a width-threading HostTable cell
    // `For`.  It is consumed here (merged into this node's own modifier
    // chain, or emitted as a standalone `.frame` if the node has no part
    // style) and is NOT propagated into this node's children — every
    // recursive call below passes `None`.  See [`emit_for_swift`]'s
    // width-thread path for where `Some(...)` originates.
    injected_width: Option<&str>,
) -> Result<String, PipelineEmitError> {
    let uses_automatic_hover = automatic_hover_style(node, part_styles).is_some();
    let uses_automatic_focus = automatic_focus_style(node, part_styles).is_some();
    let uses_automatic_press = automatic_press_style(node, part_styles).is_some();
    let automatic_wrapper_count = usize::from(uses_automatic_hover)
        + usize::from(uses_automatic_focus)
        + usize::from(uses_automatic_press);
    let indent = indent + automatic_wrapper_count * 4;
    let pad = " ".repeat(indent);

    let mut inner = match node.tag.as_str() {
        // -----------------------------------------------------------------
        // Containers — open a SwiftUI view-builder block, recurse into
        // children at +4 indentation, then close.
        // -----------------------------------------------------------------
        "Box" => container(
            "Group",
            node,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?,
        // `Row` inside a HostTable lowers to `HStack(spacing: 0)` so cells
        // sit flush (matching `border-collapse: collapse` semantics in the
        // companion .msl).  Outside a HostTable we keep the default
        // 8pt SwiftUI spacing so existing `Row` users render unchanged.
        // See [`TableContext`] for the discovery path.
        "Row" => {
            if table_ctx.is_some() {
                container_table_row(node, indent, part_styles, emits, table_ctx, for_payload)?
            } else {
                container(
                    "HStack",
                    node,
                    indent,
                    part_styles,
                    emits,
                    table_ctx,
                    for_payload,
                )?
            }
        }
        // TODO(UI28 §2.2): Column is being repurposed as Grid-metadata
        // (carries a header label + sort key + width + per-cell template,
        // discarded as a SwiftUI view). For now we keep the legacy UI14
        // semantics: `Column → VStack`. The Cell/Column/Grid v3 SwiftUI
        // lowering lands in a separate follow-up PR.
        "Column" => container(
            "VStack",
            node,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?,
        // UI29 kernel partial — Stack is the z-axis / overlay container.
        // It is *not* a synonym for VStack: SwiftUI's `ZStack` overlays
        // children along the depth axis, which is the UI29 semantics.
        "Stack" => container(
            "ZStack",
            node,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?,
        // UI29 kernel partial — `HostScroll` is the kernel form of a
        // scrollable region. SwiftUI's `ScrollView` is the direct analog;
        // it implicitly handles its own scroll-state and viewport, so we
        // do not need to thread offset/extent slots through here.
        "HostScroll" => container(
            "ScrollView",
            node,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?,

        // -----------------------------------------------------------------
        // Leaf primitives — emit a single line, no children.
        // -----------------------------------------------------------------
        "Text" => {
            let expr = swift_text_expression(node);
            format!("{pad}{expr}\n")
        }
        "Spacer" => format!("{pad}Spacer()\n"),
        "Image" => {
            // SwiftUI's `Image(systemName:)` takes an SF Symbols name. We use
            // the moslayout `source` prop if it's a string literal; if it's a
            // slot ref or missing, we fall back to a placeholder symbol so
            // the file still compiles. A real image-asset pipeline (loading
            // from URLs, bundle resources, etc.) is a follow-up.
            let symbol = find_string_prop(node, "source").unwrap_or("photo");
            let escaped = escape_swift_string(symbol);
            format!("{pad}Image(systemName: \"{escaped}\")\n")
        }
        "Divider" => format!("{pad}Divider()\n"),

        // UI29 kernel partial — `HostInput` and `HostButton` are leaf
        // primitives backed by SwiftUI's `TextField` and `Button`
        // respectively. They read slot/emit refs off the node props; see
        // the per-function doc comments below for the full mapping.
        "HostInput" => emit_host_input(node, indent)?,
        "HostButton" => emit_host_button(node, indent, emits, for_payload)?,
        "HostSurface" => emit_host_surface(node, indent),

        // UI29-2 kernel — `HostCheckbox` and `HostRadio` both lower to
        // SwiftUI `Toggle` with the platform's default toggle style.
        // The semantic difference is in the dispatched payload:
        //   - HostCheckbox dispatches `checked: Bool` (every flip).
        //   - HostRadio    dispatches `value: String` (only on select).
        // Style choice (`.checkbox` macOS / `.switch` cross-platform) is
        // left to a userland modifier or a follow-up that adds
        // platform-conditional emission; for v1 the default style ships.
        "HostCheckbox" => emit_host_checkbox(node, indent)?,
        "HostRadio" => emit_host_radio(node, indent)?,

        // UI29-4 kernel — `HostLink` lowers to SwiftUI's `Link`
        // (iOS 14+/macOS 11+), `HostTooltip` to the `.help(...)`
        // view modifier on the wrapped child (macOS / iOS 16+), and
        // `HostNumberInput` to `TextField` with the `.number`
        // format binding (iOS 15+/macOS 12+).
        "HostLink" => emit_host_link(node, indent, emits, for_payload)?,
        "HostTooltip" => emit_host_tooltip(node, indent, part_styles, emits, for_payload)?,
        "HostNumberInput" => emit_host_number_input(node, indent)?,

        // UI29 kernel — `HostTable` is the semantic data-table primitive.
        // See [`emit_host_table`] for the lowering rationale (VStack +
        // HStack rows, not SwiftUI.Table — for now).
        //
        // `HostTable` is an entry point for [`TableContext`]: it discovers
        // a nested `HostTableColGroup`'s `For ( each: slot: column-widths
        // ) { Col }` shape and threads a column-widths slot down through
        // the section walker into every `Row` cell, so each cell can
        // apply `.frame(width: columnWidths[Int(<idx>)])`.  Any inherited
        // `table_ctx` from a HostTable-inside-HostTable would be confusing
        // anyway, so we start a fresh context here rather than chaining.
        "HostTable" => emit_host_table(node, indent, part_styles, emits, for_payload)?,

        // UI29-1 kernel — `HostDialog` is the modal/popover primitive.
        // It lowers to a `Color.clear` anchor view carrying a
        // `.sheet(...)` (modal=true) or `.popover(...)` (modal=false)
        // modifier. See [`emit_host_dialog`] for the lowering rationale
        // (SwiftUI exposes dialogs as view modifiers, not standalone
        // views).
        "HostDialog" => emit_host_dialog(node, indent, part_styles, emits, for_payload)?,

        // UI29 kernel — table sub-tags. When they appear OUTSIDE of a
        // HostTable parent they have nothing to attach to in SwiftUI;
        // we emit a self-documenting comment rather than erroring so the
        // generated file still type-checks (a Swift comment is a
        // statement-level no-op).
        "HostTableHead" | "HostTableBody" | "HostTableFoot" | "HostTableColGroup" => {
            format!("{pad}// {} outside HostTable — ignored\n", node.tag)
        }

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
        "For" => emit_for_swift(
            node,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
            false,
        )?,
        "If" => emit_if_swift(
            node,
            None,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?,
        // An orphan `Else` (Else not preceded by If) is rejected by the
        // moslayout analyzer, but the emitter is defensive: rather than
        // erroring at the Swift level we emit a self-documenting Swift
        // comment so the generated file still type-checks.
        "Else" => format!("{}// orphan Else — ignored\n", " ".repeat(indent)),

        other => return Err(PipelineEmitError::UnknownPrimitive(other.to_string())),
    };

    // Apply the part-style modifier chain.  Each branch above produces
    // `inner` with exactly one trailing newline; we splice the modifier
    // chain right before that newline so SwiftUI sees the chain as
    // continuation of the expression on the line(s) above.
    //
    // The lookup is `part_styles.get(part)`; an absent part_name OR an
    // empty `chain` is the no-op path (the inner emission renders
    // identically to a styleless node).
    //
    // `injected_width` (the threaded column width) is consumed HERE so it
    // lands at the correct position inside the chain (before
    // background/border — see [`swiftui_modifier_chain`]'s ordering),
    // rather than being appended as a trailing `.frame(width:)`.
    let mut consumed_injected = false;
    if let Some(part) = &node.part_name {
        let base_style = part_styles.get(part);
        let base_props = base_style
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let base_transitions = base_style
            .map(|style| style.transitions.as_slice())
            .unwrap_or(&[]);
        let state_layers = collect_state_layers_for_emission(
            node,
            part,
            part_styles,
            uses_automatic_hover,
            uses_automatic_focus,
            uses_automatic_press,
        );
        // When a width is injected we MUST run the chain even if the part
        // carries no styles, so the frame still emits.
        if !base_props.is_empty() || !state_layers.is_empty() || injected_width.is_some() {
            let chain = swiftui_modifier_chain_with_transitions(
                base_props,
                base_transitions,
                &state_layers,
                indent + 4,
                injected_width,
            );
            if injected_width.is_some() {
                consumed_injected = true;
            }
            if !chain.is_empty() {
                let mut spliced = inner.trim_end_matches('\n').to_string();
                spliced.push_str(&chain);
                spliced.push('\n');
                inner = spliced;
            }
        }
    }

    // Robustness fallback: a width was injected but the node had NO
    // part_name at all (so the chain above never ran).  Emit a standalone
    // `.frame(width: <expr>, alignment: .center)` so the threaded column
    // width is still honored.  `.center` is the neutral default — without
    // a part there's no `text-align` to consult.
    if injected_width.is_some() && !consumed_injected {
        if let Some(iw) = injected_width {
            let frame_pad = " ".repeat(indent + 4);
            let mut spliced = inner.trim_end_matches('\n').to_string();
            spliced.push_str(&format!(
                "\n{frame_pad}.frame(width: {iw}, alignment: .center)"
            ));
            spliced.push('\n');
            inner = spliced;
        }
    }

    let mut wrapper_indent = indent;
    if uses_automatic_press {
        wrapper_indent -= 4;
        let wrapper_pad = " ".repeat(wrapper_indent);
        inner = format!(
            "{wrapper_pad}_MosaicPressState {{ __mosaicPressActive in\n{inner}{wrapper_pad}}}\n"
        );
    }
    if uses_automatic_focus {
        wrapper_indent -= 4;
        let wrapper_pad = " ".repeat(wrapper_indent);
        inner = format!(
            "{wrapper_pad}_MosaicFocusState {{ __mosaicFocusActive in\n{inner}{wrapper_pad}}}\n"
        );
    }
    if uses_automatic_hover {
        wrapper_indent -= 4;
        let outer_pad = " ".repeat(wrapper_indent);
        inner = format!(
            "{outer_pad}_MosaicHoverState {{ __mosaicHoverActive in\n{inner}{outer_pad}}}\n"
        );
    }

    Ok(inner)
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
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    if node.children.is_empty() {
        // Empty containers still need a body — SwiftUI's trailing-closure
        // syntax `Group { }` is valid Swift and renders nothing.
        return Ok(format!("{pad}{swiftui_view} {{ }}\n"));
    }
    let mut out = format!("{pad}{swiftui_view} {{\n");
    // A container's own children never receive the injected width — the
    // injection target is the container node itself, consumed in
    // [`emit_view_tree`]'s splice.  Pass `None` here.
    out.push_str(&emit_children(
        &node.children,
        indent + 4,
        part_styles,
        emits,
        table_ctx,
        for_payload,
        None,
    )?);
    out.push_str(&format!("{pad}}}\n"));
    Ok(out)
}

/// Emit a `Row` container that lives inside a `HostTable`.
///
/// Differs from the generic [`container`] helper in two ways:
///
/// 1. The opener is `HStack(spacing: 0) {` instead of `HStack {`.
///    Zero spacing matches the `border-collapse: collapse` semantics
///    that VisiCalc-style .msl files declare on the sheet part — cells
///    sit flush against each other so overlapping 1pt borders read as
///    a single grid line, not as a stack of disconnected boxes.
///
/// 2. Each child is dispatched through [`emit_table_cell`] rather than
///    [`emit_view_tree`] directly, so a child `For` with an `index:`
///    binding can pick up `.frame(width: <slot>[Int(<idx>)])` from the
///    surrounding [`TableContext`].
///
/// Empty `Row`s collapse to `HStack(spacing: 0) { }` so the generated
/// source still type-checks and the spacing convention is uniform.
fn container_table_row(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    if node.children.is_empty() {
        return Ok(format!("{pad}HStack(spacing: 0) {{ }}\n"));
    }
    let mut out = format!("{pad}HStack(spacing: 0) {{\n");
    for cell in &node.children {
        out.push_str(&emit_table_cell(
            cell,
            indent + 4,
            part_styles,
            emits,
            table_ctx,
            for_payload,
        )?);
    }
    out.push_str(&format!("{pad}}}\n"));
    Ok(out)
}

/// Emit a single cell of a `Row` inside a `HostTable`.
///
/// When the cell is a `For` with both an `index:` binding AND a live
/// `column_widths_slot` in the surrounding [`TableContext`], we route
/// through [`emit_for_swift`]'s width-threading mode: each iteration's
/// view picks up a trailing `.frame(width: <slot>[Int(<idx>)])`
/// modifier so columns get explicit widths rather than auto-sizing to
/// content.
///
/// Anything else (a `Box` literal cell, an inline `Text`, a non-indexed
/// `For`, a `HostTable` context with no `HostTableColGroup`, etc.)
/// falls through to the normal [`emit_view_tree`] walker — the cell
/// still renders, just without explicit width threading.
fn emit_table_cell(
    cell: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    if let Some(ctx) = table_ctx {
        if ctx.column_widths_slot.is_some()
            && cell.tag == "For"
            && find_keyword_prop(cell, "index").is_some()
        {
            return emit_for_swift(
                cell,
                indent,
                part_styles,
                emits,
                table_ctx,
                for_payload,
                /*width_thread=*/ true,
            );
        }
    }
    emit_view_tree(
        cell,
        indent,
        part_styles,
        emits,
        table_ctx,
        for_payload,
        None,
    )
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
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
    // The threaded column-width Swift expression to inject into each
    // DIRECT child's modifier chain.  `Some` only on the body-children
    // dispatch of a width-threading HostTable cell `For`; `None`
    // everywhere else (the common case).  `If`/`Else` control-flow
    // children cannot carry a frame, so the injection only applies to
    // the plain-view dispatch below.
    injected_width: Option<&str>,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if child.tag == "If" {
            // Peek: if the next sibling is `Else`, consume both into a
            // paired emission and advance past the Else.
            let else_node = children.get(i + 1).filter(|n| n.tag == "Else");
            out.push_str(&emit_if_swift(
                child,
                else_node,
                indent,
                part_styles,
                emits,
                table_ctx,
                for_payload,
            )?);
            i += if else_node.is_some() { 2 } else { 1 };
            continue;
        }
        if child.tag == "Else" {
            // Reached only when an `Else` did NOT immediately follow an
            // `If`; emit a documenting comment instead of crashing.
            out.push_str(&format!("{}// orphan Else — ignored\n", " ".repeat(indent)));
            i += 1;
            continue;
        }
        out.push_str(&emit_view_tree(
            child,
            indent,
            part_styles,
            emits,
            table_ctx,
            for_payload,
            injected_width,
        )?);
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
                LayoutPropValue::Expr(text) => {
                    // U29-G3 expression-valued content + UI29 §3.4 scope
                    // (PR #4398) — the Expr text is the literal Swift
                    // expression to evaluate in the surrounding For-loop
                    // closure's lexical scope. mosaic-pkg-grid v0.2.0's
                    // `Text ( content: ( v ) )` shape becomes `Text(v)`
                    // (where `v` is the inner ForEach binding); the host
                    // gets the live cell text inside every body cell
                    // instead of the empty placeholder this branch used
                    // to emit before §3.4 made the scope rules explicit.
                    return format!("Text({text})");
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

    // `value: slot: x` -> `text: .constant(x)`. If `value` is an Expr
    // (e.g. `value: ( v )` where `v` is a For-loop binding from the
    // mosaic-pkg-grid v0.2.0 Cell composition), pass the expression
    // text verbatim into `.constant(...)` — Swift evaluates it in the
    // surrounding closure's lexical scope. If no `value` is bound,
    // synthesise `.constant("")` so the file still type-checks.
    let value_expr = match node.props.iter().find(|p| p.name == "value") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(slot) => {
                let camel = to_camel_case_first_lower(slot);
                validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
                camel
            }
            LayoutPropValue::Expr(text) => text.clone(),
            _ => "\"\"".to_string(),
        },
        None => "\"\"".to_string(),
    };

    // Is this input EDITABLE? An input is editable when it has both an
    // `onChange` emit handler and a bound `value` slot. For an editable input
    // we synthesise a real two-way `Binding` whose setter dispatches the change
    // on every keystroke (the same pattern the `Toggle` lowering uses), so the
    // `TextField` can actually be typed into. Without an `onChange` handler
    // there is nowhere to write edits back to, so we keep the read-only
    // `.constant(...)` form (a label-like display).
    let editable_case = match find_emit_ref_prop(node, "onChange") {
        Some(emit_name) if find_slot_ref_prop(node, "value").is_some() => {
            let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
            validate_emit_name(&case_name)?;
            Some(case_name)
        }
        _ => None,
    };

    // `text:` binding — a writable `Binding(get:set:)` for editable inputs
    // (setter dispatches the `onChange` event with the new value `$0`), or the
    // read-only `.constant(...)` otherwise.
    let text_binding = match &editable_case {
        Some(case_name) => format!(
            "Binding(get: {{ {value_expr} }}, set: {{ dispatch(.{case_name}(value: $0)) }})"
        ),
        None => format!(".constant({value_expr})"),
    };

    // The opening `TextField` expression.
    let mut line = format!("{pad}TextField({placeholder_lit}, text: {text_binding})");

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

    // The `onChange` handler is wired through the `text:` binding's setter
    // above (it dispatches the event with the new value on every keystroke),
    // so there is no separate `.onChange(of:)` modifier here. Emitting one in
    // addition would feed back: the setter dispatches -> the host updates the
    // bound slot -> `.onChange(of:)` fires -> dispatches again.

    // `.onSubmit { dispatch(.e) }`. SwiftUI fires onSubmit when the
    // user presses Enter / Return in the TextField.
    //
    // We deliberately emit the VOID form regardless of whether
    // `value:` is bound on the HostInput. The previous shape
    // `dispatch(.<case>(value: <value_expr>))` was an emitter bug:
    // it tried to call a Swift enum case with an associated value
    // that the .mil never declared (`emit onCommit ;` has no
    // payload), producing "enum case has no associated values"
    // compile errors. Matches the React backend, which dispatches
    // `{ type: "commit" }` with no value field. Authors who need
    // the live value subscribe to `onChange` (which IS the
    // value-carrying path — see the `.onChange(of:)` handler
    // above).
    //
    // If a future Mosaic component genuinely needs an onCommit
    // that carries the value, the right move is to declare
    // `emit onCommit ( value : text ) ;` in the .mil and have the
    // emitter look up the emit's payload from the interface
    // descriptor — not to guess from the HostInput's `value:` prop.
    // That richer payload-aware handling is tracked as a follow-up.
    if let Some(emit_name) = find_emit_ref_prop(node, "onCommit") {
        let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_emit_name(&case_name)?;
        line.push_str(&format!(".onSubmit {{ dispatch(.{case_name}) }}"));
    }

    // `.onExitCommand { dispatch(.e) }` — macOS Escape-key handler.
    //
    // `.onExitCommand` is a macOS-only modifier (it observes the
    // Escape key via the AppKit responder chain).  Emitting it
    // unconditionally breaks iOS / iPadOS / tvOS / watchOS builds
    // with "value of type ... has no member 'onExitCommand'".  We
    // wrap it in a `#if os(macOS)` compile-time guard so the same
    // generated file targets the whole Apple platform fleet.
    //
    // To keep the rest of the chain readable, we close the current
    // chain line, emit the guarded modifier on its own block, and
    // open a no-op continuation if any further chained modifiers
    // get added in the future.  Today `onCancel` is the last
    // modifier we emit, so the trailing newline pattern just
    // closes the expression.
    if let Some(emit_name) = find_emit_ref_prop(node, "onCancel") {
        let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
        validate_emit_name(&case_name)?;
        line.push('\n');
        line.push_str(&format!("{pad}    #if os(macOS)\n"));
        line.push_str(&format!(
            "{pad}    .onExitCommand {{ dispatch(.{case_name}) }}\n"
        ));
        line.push_str(&format!("{pad}    #endif"));
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
/// | `label: item`           | `Text(item)` for a scoped `For` binding         |
/// | `disabled: slot: x`     | `.disabled(x)` modifier                         |
/// | `disabled: true`/`false`| `.disabled(true)` / `.disabled(false)`          |
/// | `onTap: emit: onE`      | `action: { dispatch(.e) }`                      |
/// | `onClick: emit: onE`    | same, with single row payloads when declared    |
///
/// ## Generated shape
///
/// ```swift
/// Button(action: { dispatch(.tap) }) {
///     Text(label)
/// }.disabled(disabled)
/// ```
///
/// If no click/tap emit is bound the action closure is `{ }` (a no-op);
/// the file still compiles and the button is effectively decorative.
fn emit_host_button(
    node: &LayoutNode,
    indent: usize,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);

    // Action closure body.
    let action_body =
        match find_emit_ref_prop(node, "onClick").or_else(|| find_emit_ref_prop(node, "onTap")) {
            Some(emit_name) => {
                let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
                validate_emit_name(&case_name)?;
                let args = emits
                    .iter()
                    .find(|e| e.name == *emit_name)
                    .map(|e| host_button_event_args(e, for_payload))
                    .transpose()?
                    .unwrap_or_default();
                if args.is_empty() {
                    format!("dispatch(.{case_name})")
                } else {
                    format!("dispatch(.{case_name}({args}))")
                }
            }
            None => String::new(),
        };

    // Label expression. String literal → `Text("...")`; slot ref →
    // `Text(slotName)`; nothing bound → `Text("")` placeholder.
    let label_expr = match find_prop_value(node, "label") {
        Some(LayoutPropValue::String(s)) => format!("Text(\"{}\")", escape_swift_string(s)),
        Some(LayoutPropValue::SlotRef(slot)) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            format!("Text({camel})")
        }
        Some(LayoutPropValue::Keyword(name)) => {
            let camel = to_camel_case_first_lower(name);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            format!("Text({camel})")
        }
        Some(LayoutPropValue::Expr(text)) => format!("Text({})", text.trim()),
        _ => "Text(\"\")".to_string(),
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
// UI29-2 kernel — HostCheckbox and HostRadio emitters
// =====================================================================

/// Lower a UI29-2 `HostCheckbox` node to a SwiftUI `Toggle`.
///
/// ## Property handling
///
/// | Moslayout prop          | SwiftUI surface                                          |
/// |---|---|
/// | `checked: slot: c`      | `isOn:` reads `c`, writes through `dispatch` (if onToggle)|
/// | `disabled: slot: d`     | `.disabled(d)` trailing modifier                         |
/// | `disabled: true`/`false`| `.disabled(true)` / `.disabled(false)` literal           |
/// | `label: "..."`          | `Toggle("...", isOn: …)` — string label argument         |
/// | `label: slot: x`        | `Toggle(x, isOn: …)` — slot label argument               |
/// | `onToggle: emit: onX`   | `Binding(get:set:)` form whose setter dispatches         |
///
/// ## Binding pattern
///
/// SwiftUI's `Toggle` expects a `Binding<Bool>`. Mosaic slots are read-
/// only `let`s, so we synthesise the binding with a `Binding(get:set:)`
/// closure pair:
///
/// ```swift
/// Toggle("label", isOn: Binding(
///     get: { isChecked },
///     set: { newValue in dispatch(.toggle(checked: newValue)) }
/// ))
/// ```
///
/// When `onToggle` is unbound we fall back to `.constant(checked)` so
/// the generated source still type-checks; the toggle then becomes
/// effectively read-only (user taps have no effect — the host has no
/// way to learn about them).
///
/// ## What is NOT in this first cut
///
/// - The `indeterminate` slot. SwiftUI's `Toggle` has no tri-state
///   visual; rendering a "mixed" state requires either a custom
///   `ToggleStyle` or a different primitive (e.g. `Image` of the
///   `checkmark.square.fill` SF Symbol). Deferred to a follow-up that
///   adds a custom checkbox style.
/// - Explicit `.toggleStyle(.checkbox)`. That style is macOS-only; on
///   iOS the same modifier fails to compile. Setting the default
///   platform style works on both. Authors who want a checkbox visual
///   on macOS can compose a userland wrapper that adds the modifier.
fn emit_host_checkbox(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // Resolve the `checked:` slot. We need the camelCased name for the
    // binding getter. Missing slot falls back to a `false` constant so
    // the file compiles.
    let checked_expr: String = match find_slot_ref_prop(node, "checked") {
        Some(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            camel
        }
        None => "false".to_string(),
    };

    // Label argument — first positional argument to Toggle. String
    // literal → `"..."`; slot ref → bare identifier (SwiftUI accepts
    // any `StringProtocol`); missing → empty string literal.
    let label_arg: String = if let Some(s) = find_string_prop(node, "label") {
        format!("\"{}\"", escape_swift_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        camel
    } else {
        "\"\"".to_string()
    };

    // Binding form. With onToggle: a Binding(get:set:) whose setter
    // dispatches the new value. Without: a .constant() that makes the
    // toggle read-only but still type-checks.
    let binding_expr: String = match find_emit_ref_prop(node, "onToggle") {
        Some(emit_name) => {
            let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
            validate_emit_name(&case_name)?;
            format!(
                "Binding(get: {{ {checked_expr} }}, set: {{ newValue in dispatch(.{case_name}(checked: newValue)) }})"
            )
        }
        None => format!(".constant({checked_expr})"),
    };

    let mut out = String::new();
    writeln!(out, "{pad}Toggle({label_arg}, isOn: {binding_expr})").unwrap();

    // `.disabled(...)` trailing modifier. Indented one Swift-source step
    // beyond the Toggle so the modifier visually attaches to it.
    let mod_pad = " ".repeat(indent + 4);
    if let Some(slot) = find_slot_ref_prop(node, "disabled") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{mod_pad}.disabled({camel})").unwrap();
    } else if let Some(kw) = find_keyword_prop(node, "disabled") {
        if kw == "true" || kw == "false" {
            writeln!(out, "{mod_pad}.disabled({kw})").unwrap();
        }
    }

    Ok(out)
}

/// Lower a UI29-2 `HostRadio` node to a SwiftUI `Toggle`.
///
/// ## Why also a `Toggle`?
///
/// SwiftUI exposes the canonical macOS radio-group via `Picker` with
/// `.pickerStyle(.radioGroup)`, but that is a *multi-option* primitive
/// driven from a single selection state — not a standalone radio
/// button. UI29-2 §2.2 keeps each `HostRadio` as a standalone primitive
/// in v1 (the proper `RadioGroup` userland composition is reserved for
/// UI29-2.1), so the SwiftUI lowering uses `Toggle` for visual parity
/// with `HostCheckbox` and surfaces the radio semantics through the
/// emit payload: `onSelect` carries the radio's `value:` string.
///
/// ## Property handling
///
/// | Moslayout prop          | SwiftUI surface                                          |
/// |---|---|
/// | `checked: slot: c`      | `isOn:` reads `c`                                        |
/// | `group: "..."` / `slot:`| recorded in source as a `// group: ...` comment for now  |
/// | `value: "..." `         | dispatched payload literal in onSelect closure           |
/// | `value: slot: v`        | dispatched payload reads `v` at dispatch time            |
/// | `disabled: slot|bool`   | `.disabled(...)` modifier                                |
/// | `label: ...`            | `Toggle(label, isOn: …)` argument                        |
/// | `onSelect: emit: onX`   | `Binding(get:set:)` whose setter dispatches IF new state |
///
/// ## `onSelect` fires only on positive transition
///
/// The kernel-canonical `onSelect` event represents "this radio was
/// chosen", not "this radio was toggled". When the user taps a checked
/// radio (no-op), or a sibling radio causes this one to flip off, we
/// must not dispatch. The setter wraps the dispatch in `if newValue`:
///
/// ```swift
/// Binding(get: { isSelected }, set: { newValue in
///     if newValue { dispatch(.select(value: "vanilla")) }
/// })
/// ```
///
/// ## Group coordination
///
/// SwiftUI radios have no implicit grouping. Host code is responsible
/// for tracking which radio in a logical group is currently selected
/// and toggling the others' `checked:` slot to `false`. The `group:`
/// prop is preserved in the generated source as a `// group: ...`
/// comment so it remains visible for future code that consumes it
/// (e.g. a structural pass that synthesises a `Picker` from a sibling
/// run of HostRadio with shared `group:`).
fn emit_host_radio(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // Resolve `checked:` (identical to HostCheckbox).
    let checked_expr: String = match find_slot_ref_prop(node, "checked") {
        Some(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            camel
        }
        None => "false".to_string(),
    };

    // Resolve `value:` for the dispatch payload. String literal → JS
    // string literal in the dispatch call; slot ref → bare identifier
    // (host's responsibility to make the slot a `String`).
    let value_expr: String = if let Some(s) = find_string_prop(node, "value") {
        format!("\"{}\"", escape_swift_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        camel
    } else {
        // No value: bound — fall back to an empty string. The radio
        // still works visually but the host can't distinguish it from
        // any other valueless radio in the same group.
        "\"\"".to_string()
    };

    // Label argument (same shape as HostCheckbox).
    let label_arg: String = if let Some(s) = find_string_prop(node, "label") {
        format!("\"{}\"", escape_swift_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        camel
    } else {
        "\"\"".to_string()
    };

    let mut out = String::new();

    // `// group: ...` comment — preserves the group: prop in the
    // generated source for downstream tooling. Mirrors HostTable's
    // `// part: ...` comment pattern.
    //
    // Security: a raw newline (or CR) in the author's `group:` string
    // would terminate this `//` line comment and inject arbitrary Swift
    // source on the next line. `escape_swift_string` only escapes `"`
    // and `\` (it's tuned for string-literal contexts), so we have to
    // strip line terminators here ourselves. Replacing with a space
    // preserves the visible group name as best we can for human
    // readers while keeping the line-comment scope intact.
    fn escape_for_line_comment(s: &str) -> String {
        let escaped = escape_swift_string(s);
        escaped.replace(['\r', '\n'], " ")
    }
    if let Some(g) = find_string_prop(node, "group") {
        writeln!(out, "{pad}// group: {}", escape_for_line_comment(g)).unwrap();
    } else if let Some(slot) = find_slot_ref_prop(node, "group") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{pad}// group: slot {camel}").unwrap();
    }

    // Binding form. With onSelect: dispatch only when newValue is true
    // (this radio was chosen). Without: read-only constant.
    let binding_expr: String = match find_emit_ref_prop(node, "onSelect") {
        Some(emit_name) => {
            let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
            validate_emit_name(&case_name)?;
            format!(
                "Binding(get: {{ {checked_expr} }}, set: {{ newValue in if newValue {{ dispatch(.{case_name}(value: {value_expr})) }} }})"
            )
        }
        None => format!(".constant({checked_expr})"),
    };

    writeln!(out, "{pad}Toggle({label_arg}, isOn: {binding_expr})").unwrap();

    let mod_pad = " ".repeat(indent + 4);
    if let Some(slot) = find_slot_ref_prop(node, "disabled") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{mod_pad}.disabled({camel})").unwrap();
    } else if let Some(kw) = find_keyword_prop(node, "disabled") {
        if kw == "true" || kw == "false" {
            writeln!(out, "{mod_pad}.disabled({kw})").unwrap();
        }
    }

    Ok(out)
}

// =====================================================================
// UI29-4 — HostLink / HostTooltip / HostNumberInput emitters
// =====================================================================

/// Lower a `HostLink` node (UI29-4, 19th kernel primitive) to a
/// SwiftUI `Link(label, destination:)` view.
///
/// `Link` is iOS 14+/macOS 11+. It uses the OS-default URL-open
/// behavior (browser / Universal Links / scene-handler), which is
/// the right default for `target: same` and `target: new-tab` (iOS
/// has no in-window tab concept; both map to "open externally").
///
/// ## Property handling
///
/// | moslayout prop      | SwiftUI                                        |
/// |---|---|
/// | `href: "..."`       | `Link(label, destination: URL(string: "...")!)`|
/// | `label: "..."`      | first positional argument to Link              |
/// | `external: false`   | wrapped in `Button(action:)` instead, dispatches `onActivate` only (no URL open) |
/// | `onActivate: emit`  | dispatch in the Button's action closure (only when external:false) |
///
/// ## When `external: true` + `onActivate` are BOTH bound
///
/// SwiftUI's `Link` doesn't expose an "also call this closure when
/// clicked" hook. The host can observe environment changes if it
/// really needs to track activation, but the v1 emitter drops the
/// `onActivate` dispatch when `external != false` and documents
/// the limitation. The far-more-common in-app routing path
/// (`external: false`) gets the full Button-with-dispatch shape.
fn emit_host_link(
    node: &LayoutNode,
    indent: usize,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 4);

    let href = find_string_prop(node, "href").unwrap_or("#");
    let escaped_href = escape_swift_string(href);
    let label_text = host_link_label_text_expr(node, href)?;

    let external_false = matches!(find_keyword_prop(node, "external"), Some("false"));
    let on_activate = find_emit_ref_prop(node, "onActivate");

    if external_false {
        // In-app routing: Button + dispatch only, no URL open.
        let mut out = String::new();
        let action_body: String = match on_activate {
            Some(emit_name) => {
                let case = to_camel_case_first_lower(&strip_on_prefix(emit_name));
                validate_emit_name(&case)?;
                let args = host_link_event_args(emit_name, emits, for_payload, &escaped_href)?;
                if args.is_empty() {
                    format!("dispatch(.{case})")
                } else {
                    format!("dispatch(.{case}({args}))")
                }
            }
            None => "{ }".to_string(),
        };
        writeln!(out, "{pad}Button(action: {{ {action_body} }}) {{").unwrap();
        writeln!(out, "{inner_pad}{label_text}").unwrap();
        writeln!(out, "{pad}}}").unwrap();
        Ok(out)
    } else {
        // Default: SwiftUI Link with OS-default open behaviour.
        let mut out = String::new();
        writeln!(
            out,
            "{pad}Link(destination: URL(string: \"{escaped_href}\")!) {{"
        )
        .unwrap();
        writeln!(out, "{inner_pad}{label_text}").unwrap();
        writeln!(out, "{pad}}}").unwrap();
        Ok(out)
    }
}

fn host_link_label_text_expr(
    node: &LayoutNode,
    fallback_href: &str,
) -> Result<String, PipelineEmitError> {
    if let Some(label) = find_string_prop(node, "label") {
        return Ok(format!("Text(\"{}\")", escape_swift_string(label)));
    }
    if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        return Ok(format!("Text({camel})"));
    }
    if let Some(keyword) = find_keyword_prop(node, "label") {
        let camel = to_camel_case_first_lower(keyword);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        return Ok(format!("Text({camel})"));
    }
    Ok(format!("Text(\"{}\")", escape_swift_string(fallback_href)))
}

fn host_link_event_args(
    emit_name: &str,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
    escaped_href: &str,
) -> Result<String, PipelineEmitError> {
    let Some(emit) = emits.iter().find(|e| e.name == emit_name) else {
        return Ok(String::new());
    };
    if emit.params.is_empty() {
        return Ok(String::new());
    }
    if emit.params.len() == 1 {
        let param = &emit.params[0];
        let field = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
        let expr = host_link_payload_expr(&param.r#type, &field, for_payload, escaped_href)
            .unwrap_or_else(|| "/* TODO: payload */ fatalError(\"TODO: payload\")".to_string());
        return Ok(format!("{field}: {expr}"));
    }

    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field).map_err(PipelineEmitError::UnsafeSlotName)?;
            let expr = host_link_payload_expr(&param.r#type, &field, for_payload, escaped_href)
                .unwrap_or_else(|| "/* TODO: payload */ fatalError(\"TODO: payload\")".to_string());
            Ok(format!("{field}: {expr}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

fn host_link_payload_expr(
    t: &EmitPayloadType,
    field: &str,
    for_payload: Option<ForPayloadScope<'_>>,
    escaped_href: &str,
) -> Option<String> {
    if field == "href" {
        return match t {
            EmitPayloadType::Text => Some(format!("\"{escaped_href}\"")),
            _ => None,
        };
    }
    if let Some(expr) = host_button_payload_expr(t, for_payload) {
        return Some(expr);
    }
    match t {
        EmitPayloadType::Text => Some(format!("\"{escaped_href}\"")),
        _ => None,
    }
}

/// Lower a `HostTooltip` node (UI29-4, 20th kernel primitive) to
/// the SwiftUI `.help(...)` modifier on the wrapped child.
///
/// `.help(_:)` is macOS / iOS 16+. Hovering (macOS) or long-pressing
/// (iOS 16+) the modified view shows the tooltip; screen readers
/// announce it via `accessibilityHint`.
///
/// ## Generated shape
///
/// ```swift
/// VStack {
///     /* child(ren) */
/// }
/// .help("tooltip text")
/// ```
///
/// We wrap in a `VStack` (zero-frame) so the `.help` modifier
/// attaches to a single view even when multiple children are
/// present. Empty-children case emits an empty `VStack { }`.
fn emit_host_tooltip(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let mod_pad = " ".repeat(indent + 4);

    let text = find_string_prop(node, "text").unwrap_or("");
    let escaped = escape_swift_string(text);

    let mut out = String::new();
    writeln!(out, "{pad}VStack {{").unwrap();
    if !node.children.is_empty() {
        out.push_str(&emit_children(
            &node.children,
            indent + 4,
            part_styles,
            emits,
            None,
            for_payload,
            None,
        )?);
    }
    writeln!(out, "{pad}}}").unwrap();
    writeln!(out, "{mod_pad}.help(\"{escaped}\")").unwrap();
    Ok(out)
}

/// Lower a `HostNumberInput` node (UI29-4, 21st kernel primitive)
/// to a SwiftUI `TextField` with the `.number` format binding
/// (iOS 15+/macOS 12+).
///
/// ## Generated shape
///
/// ```swift
/// TextField("placeholder", value: .constant(value), format: .number)
///     .disabled(disabled)
/// ```
///
/// `.constant(value)` is used when there is no `onChange` handler.
/// When `value` and `onChange` are both present, the field uses a
/// writable `Binding(get:set:)` whose setter dispatches the new
/// numeric value back to the host.
fn emit_host_number_input(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let mod_pad = " ".repeat(indent + 4);

    let placeholder = find_string_prop(node, "placeholder").unwrap_or("");
    let escaped_placeholder = escape_swift_string(placeholder);

    let value_prop = find_prop_value(node, "value");
    let value_expr: String = match value_prop {
        Some(LayoutPropValue::SlotRef(slot)) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            camel
        }
        Some(LayoutPropValue::Expr(text)) => text.clone(),
        Some(LayoutPropValue::Number(n)) => n.to_string(),
        _ => "0".to_string(),
    };

    let editable_case = match find_emit_ref_prop(node, "onChange") {
        Some(emit_name) if value_prop.is_some() => {
            let case = to_camel_case_first_lower(&strip_on_prefix(emit_name));
            validate_emit_name(&case)?;
            Some(case)
        }
        _ => None,
    };

    let value_binding = match &editable_case {
        Some(case) => {
            format!("Binding(get: {{ {value_expr} }}, set: {{ dispatch(.{case}(value: $0)) }})")
        }
        None => format!(".constant({value_expr})"),
    };

    let mut out = String::new();
    writeln!(
        out,
        "{pad}TextField(\"{escaped_placeholder}\", value: {value_binding}, format: .number)"
    )
    .unwrap();

    // disabled — same shape HostInput uses.
    if let Some(slot) = find_slot_ref_prop(node, "disabled") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{mod_pad}.disabled({camel})").unwrap();
    } else if let Some(kw) = find_keyword_prop(node, "disabled") {
        if kw == "true" || kw == "false" {
            writeln!(out, "{mod_pad}.disabled({kw})").unwrap();
        }
    }

    // `onChange` is wired through the Binding setter above. Emitting a
    // separate `.onChange(of:)` would double-dispatch after the host
    // reflects the edited number back into the slot.

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
fn emit_host_table(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
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

    // Discover the column-widths slot from a nested `HostTableColGroup`
    // (if any).  When the For-Col shape isn't there, `ctx.column_widths_slot`
    // stays None and width threading is a no-op — only the
    // `HStack(spacing: 0)` rewrite kicks in (which still matches the
    // border-collapse semantics .msl declares on the sheet part).
    let ctx = extract_table_context(node);
    let table_ctx = Some(&ctx);

    // Walk sections in source order. We track whether we just emitted a
    // head section so we can insert a Divider before the first non-head
    // section that follows.
    let mut head_just_emitted = false;
    for section in &node.children {
        match section.tag.as_str() {
            "HostTableHead" => {
                emit_table_section_rows(
                    &mut out,
                    section,
                    indent + 4,
                    /*bold=*/ true,
                    part_styles,
                    emits,
                    table_ctx,
                    for_payload,
                )?;
                head_just_emitted = true;
            }
            "HostTableBody" => {
                if head_just_emitted {
                    writeln!(out, "{inner_pad}Divider()").unwrap();
                    head_just_emitted = false;
                }
                emit_table_section_rows(
                    &mut out,
                    section,
                    indent + 4,
                    /*bold=*/ false,
                    part_styles,
                    emits,
                    table_ctx,
                    for_payload,
                )?;
            }
            "HostTableFoot" => {
                // Per the brief, HostTableFoot is preceded by a Divider
                // unconditionally — it visually separates totals/summary
                // rows from the body above.
                writeln!(out, "{inner_pad}Divider()").unwrap();
                head_just_emitted = false;
                emit_table_section_rows(
                    &mut out,
                    section,
                    indent + 4,
                    /*bold=*/ false,
                    part_styles,
                    emits,
                    table_ctx,
                    for_payload,
                )?;
            }
            "HostTableColGroup" => {
                // The col-group itself emits no SwiftUI view — column
                // widths reach the cells via [`TableContext`] threading
                // discovered above.  The comment makes the intent
                // visible in the output for downstream tooling and
                // tracks the shape of the .mll source.
                writeln!(
                    out,
                    "{inner_pad}// HostTableColGroup — column widths threaded into cell .frame(width:)"
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

    // UI31 §3.2 RTL contract. SwiftUI's canonical layout-direction
    // knob is `.environment(\.layoutDirection, .rightToLeft)` (or
    // `.leftToRight`) attached as a modifier on the VStack. When
    // unset, the value inherits from the ambient `Environment` (the
    // system locale → window → ancestor view chain), which is the
    // correct default and matches what `dir: auto` should produce.
    //
    // | Source                 | Emits                                                              |
    // |------------------------|--------------------------------------------------------------------|
    // | `dir: rtl`             | `.environment(\.layoutDirection, .rightToLeft)`                    |
    // | `dir: ltr`             | `.environment(\.layoutDirection, .leftToRight)`                    |
    // | `dir: auto`            | (nothing — inherit ambient; matches "let the host decide")         |
    // | `dir: slot: layoutDir` | `.environment(\.layoutDirection, layoutDir)` — slot is a Swift     |
    // |                        | expression that must evaluate to `LayoutDirection`                 |
    // | unknown keyword        | (nothing — drops silently per allow-list security gate)            |
    //
    // The allow-list (`ltr` / `rtl` / `auto`) is the security gate:
    // an attacker-controlled keyword cannot break out of the Swift
    // expression position because it never reaches the format string.
    // Slot refs run through `is_safe_swift_identifier` so they can't
    // either.
    if let Some(slot) = find_slot_ref_prop(node, "dir") {
        let camel = to_camel_case_first_lower(slot);
        if is_safe_swift_identifier(&camel) {
            writeln!(out, "{pad}.environment(\\.layoutDirection, {camel})").unwrap();
        }
    } else if let Some(kw) = find_keyword_prop(node, "dir") {
        match kw {
            "rtl" => {
                writeln!(out, "{pad}.environment(\\.layoutDirection, .rightToLeft)").unwrap();
            }
            "ltr" => {
                writeln!(out, "{pad}.environment(\\.layoutDirection, .leftToRight)").unwrap();
            }
            // `auto` — let the ambient Environment value flow through.
            // SwiftUI has no `.automatic` enum case for layoutDirection;
            // the spec-mandated semantic for `auto` is "let the host
            // decide", which is exactly what the SwiftUI default does.
            "auto" => {}
            _ => {}
        }
    }

    Ok(out)
}

// =====================================================================
// UI29-1 kernel — HostDialog emitter
// =====================================================================

/// Lower a UI29-1 `HostDialog` node to a SwiftUI `.sheet` (modal) or
/// `.popover` (non-modal) view modifier.
///
/// ## Why anchor the modifier to a `Color.clear` view?
///
/// SwiftUI exposes dialog-style presentation as a *view modifier*
/// (`.sheet(isPresented:onDismiss:content:)` /
/// `.popover(isPresented:content:)`), not as a standalone view. The
/// UI29 kernel emitter walks the layout tree as a stream of standalone
/// view nodes, so the simplest way to keep `HostDialog` implementable
/// as a single tree node is to emit an invisible anchor view —
/// `Color.clear.frame(width: 0, height: 0)` — that carries the
/// modifier. The anchor renders nothing visible; the actual dialog
/// content lives inside the modifier's trailing closure.
///
/// A future enhancement can hoist `HostDialog` modifiers up to the
/// nearest container parent (so they attach to a real view rather than
/// a Color.clear), but that requires a structural pass over the tree —
/// out of scope for this first cut.
///
/// ## Property handling
///
/// | Moslayout prop                  | SwiftUI                                                |
/// |---------------------------------|--------------------------------------------------------|
/// | `open: slot: x`                 | `isPresented: .constant(x)` (see binding note)         |
/// | `modal: true` (default)         | `.sheet(...)`                                          |
/// | `modal: false`                  | `.popover(...)`                                        |
/// | `title: "..."` / `title: slot:` | `.navigationTitle(...)` inside the content closure     |
/// | `dismiss-on-backdrop: false`    | `.interactiveDismissDisabled(true)` inside the closure |
/// | `onClose: emit: onX`            | `onDismiss: { dispatch(.x) }` (`.sheet` only)          |
///
/// ## Binding choice — `.constant(value)`
///
/// SwiftUI bindings need mutable state, but Mosaic components receive
/// slots as immutable `let`s. We follow the same `.constant(value)`
/// pattern `HostInput` uses: the host owns close-via-dispatch through
/// the UI24 Flux loop. A `@State` proxy that lifts the slot into
/// mutable local state is a documented future enhancement.
///
/// ## `.popover` and `onClose`
///
/// SwiftUI's `.popover(isPresented:content:)` does **not** accept an
/// `onDismiss:` closure (the API is `.popover(isPresented:attachmentAnchor:arrowEdge:content:)`).
/// When `modal: false`, the emitter still wires the content closure
/// but omits the `onDismiss` handler — the host should observe its own
/// `open` slot change and dispatch the close event itself.
///
/// ## Generated shape (modal)
///
/// ```swift
/// Color.clear.frame(width: 0, height: 0)
///     .sheet(isPresented: .constant(open), onDismiss: { dispatch(.close) }) {
///         VStack {
///             // children
///         }
///         .navigationTitle("Save changes?")
///         .interactiveDismissDisabled(true)
///     }
/// ```
fn emit_host_dialog(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let mod_pad = " ".repeat(indent + 4);
    let inner_pad = " ".repeat(indent + 8);
    let body_pad = " ".repeat(indent + 12);

    // `open: slot: x` → `.constant(x)`. If unbound, fall back to
    // `.constant(false)` so the generated source still type-checks —
    // the host then has nothing to drive the dialog open, but the
    // file compiles.
    let open_binding = match find_slot_ref_prop(node, "open") {
        Some(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
            format!(".constant({camel})")
        }
        None => ".constant(false)".to_string(),
    };

    // `modal: false` → `.popover(...)`; anything else (including unset
    // and `modal: true`) → `.sheet(...)`. The default is modal.
    let is_modal = !matches!(find_keyword_prop(node, "modal"), Some("false"));
    let presenter = if is_modal { "sheet" } else { "popover" };

    // `onClose` → `onDismiss:` closure. Only valid on `.sheet`;
    // `.popover` does not accept an `onDismiss` argument.
    let on_dismiss_clause = if is_modal {
        match find_emit_ref_prop(node, "onClose") {
            Some(emit_name) => {
                let case_name = to_camel_case_first_lower(&strip_on_prefix(emit_name));
                validate_emit_name(&case_name)?;
                format!(", onDismiss: {{ dispatch(.{case_name}) }}")
            }
            None => String::new(),
        }
    } else {
        String::new()
    };

    let mut out = String::new();
    // The invisible anchor view. `Color.clear` is a SwiftUI primitive
    // that renders nothing; `.frame(width: 0, height: 0)` shrinks it
    // to zero size so it occupies no layout space either.
    writeln!(out, "{pad}Color.clear.frame(width: 0, height: 0)").unwrap();
    writeln!(
        out,
        "{mod_pad}.{presenter}(isPresented: {open_binding}{on_dismiss_clause}) {{"
    )
    .unwrap();

    // Content closure: a VStack wrapping the children. Even with zero
    // children we emit the VStack so the modifier signature is correct.
    writeln!(out, "{inner_pad}VStack {{").unwrap();
    if !node.children.is_empty() {
        out.push_str(&emit_children(
            &node.children,
            indent + 12,
            part_styles,
            emits,
            None,
            for_payload,
            None,
        )?);
    }
    writeln!(out, "{inner_pad}}}").unwrap();

    // `title:` → `.navigationTitle(...)` on the VStack content. Accept
    // string literals and slot refs. The modifier is a no-op outside a
    // `NavigationStack` parent, so it's safe to always emit.
    if let Some(title) = find_string_prop(node, "title") {
        let escaped = escape_swift_string(title);
        writeln!(out, "{body_pad}.navigationTitle(\"{escaped}\")").unwrap();
    } else if let Some(slot) = find_slot_ref_prop(node, "title") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel).map_err(PipelineEmitError::UnsafeSlotName)?;
        writeln!(out, "{body_pad}.navigationTitle({camel})").unwrap();
    }

    // `dismiss-on-backdrop: false` → `.interactiveDismissDisabled(true)`.
    // Any other value (including the default `true` and unset) lets
    // SwiftUI's default dismissal behaviour stand.
    if matches!(
        find_keyword_prop(node, "dismiss-on-backdrop"),
        Some("false")
    ) {
        writeln!(out, "{body_pad}.interactiveDismissDisabled(true)").unwrap();
    }

    writeln!(out, "{mod_pad}}}").unwrap();
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
// Threads the full row-lowering context through the section walk; splitting
// would obscure the flow.
#[allow(clippy::too_many_arguments)]
fn emit_table_section_rows(
    out: &mut String,
    section: &LayoutNode,
    indent: usize,
    bold: bool,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
) -> Result<(), PipelineEmitError> {
    let pad = " ".repeat(indent);

    for child in &section.children {
        if child.tag == "Row" {
            // Inside a HostTable every row is `HStack(spacing: 0)` so
            // cells sit flush — matching `border-collapse: collapse`
            // semantics from .msl.  Outside HostTable, `Row` keeps
            // SwiftUI's default 8pt HStack spacing (see [`emit_view_tree`]
            // and the regression test guarding that path).
            if child.children.is_empty() {
                writeln!(out, "{pad}HStack(spacing: 0) {{ }}").unwrap();
                continue;
            }
            writeln!(out, "{pad}HStack(spacing: 0) {{").unwrap();
            for cell in &child.children {
                // `emit_table_cell` dispatches to width-threading
                // [`emit_for_swift`] when the cell is a For with an
                // `index:` binding AND the table has a discovered
                // column-widths slot.  Other cells go through
                // [`emit_view_tree`] unchanged.
                let emitted =
                    emit_table_cell(cell, indent + 4, part_styles, emits, table_ctx, for_payload)?;
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
            // a comment so the unusual nesting is visible.  We still
            // thread `table_ctx` so nested Rows (e.g. a `For
            // (each: viewport-rows) { Row [data-row] { For { Cell } } }`
            // body) can pick up `HStack(spacing: 0)` and width
            // threading on the inner cell Fors.
            writeln!(
                out,
                "{pad}// non-Row child '{}' in table section",
                child.tag
            )
            .unwrap();
            let emitted = emit_view_tree(
                child,
                indent,
                part_styles,
                emits,
                table_ctx,
                for_payload,
                None,
            )?;
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
/// `width_thread`: when true AND a [`TableContext::column_widths_slot`]
/// is live AND the For node has an `index:` binding, each iteration's
/// body gets a trailing `.frame(width: <slot>[Int(<idx>)])` modifier so
/// the generated SwiftUI cells take the column widths from the host's
/// `columnWidths` slot.  Callers set this flag only when the For is in
/// cell-position inside a HostTable Row; nested / outer Fors (e.g. the
/// `viewport-rows` iterator that produces rows, not cells) do not get
/// width threading.  See [`emit_table_cell`].
fn emit_for_swift(
    node: &LayoutNode,
    indent: usize,
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
    width_thread: bool,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // `each:` — required. `validate_for_node` guarantees SlotRef,
    // Expr, OR (UI29 §3.4) Keyword that names an enclosing For's
    // `as:`/`index:` binding. The emitter is defensive: if none of
    // those, fall back to an empty array literal so the generated
    // file still type-checks.
    let coll_expr = match node.props.iter().find(|p| p.name == "each") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Expr(text) => text.clone(),
            // UI29 §3.4 — Keyword names an outer For's binding. The
            // moslayout validator has already verified the name is in
            // scope. The peer emitters (React, Qt, XAML) add an
            // explicit identifier check after camel-casing as
            // defense-in-depth; SwiftUI matches that pattern here.
            // Today's NAME lexer (`[a-zA-Z_][a-zA-Z0-9_-]*`) +
            // `to_camel_case_first_lower` cannot produce an unsafe
            // Swift identifier, but the explicit gate keeps the
            // emitter robust if the lexer's NAME ever loosens.
            LayoutPropValue::Keyword(name) => {
                let camel = to_camel_case_first_lower(name);
                if !is_safe_swift_identifier(&camel) {
                    return Err(PipelineEmitError::UnsafeSlotName(camel));
                }
                camel
            }
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
            //
            // The closure parameter binds the offset as the
            // **Swift-shadowed** name `_swiftIdx<idx>` (an `Int`),
            // and we immediately re-bind `<idx>` itself to a `Double`
            // cast inside the body.  This sidesteps the Swift
            // type-checker's strict Int-vs-Double comparison: the
            // moslayout author typically writes expressions like
            // `( r == editRow && c == editCol )` where `editRow` is
            // a Double-typed `number` slot.  `Array.enumerated()`
            // yields an `Int` offset; Swift refuses
            // `Int == Double` without an explicit cast.  Shadowing
            // the binding as a Double lets the author's verbatim
            // expression text compile across every backend without
            // per-backend rewriting.
            //
            // The `_swiftIdx<idx>` binding is referenced exactly
            // once (in the `Double(...)` cast on the next line); the
            // moslayout NAME grammar `[a-zA-Z][a-zA-Z0-9]*(-...)*`
            // cannot produce a leading-underscore identifier, so
            // `_swiftIdx<idx>` is collision-free against any
            // author-supplied name.
            format!(
                "{pad}ForEach(Array({coll}.enumerated()), id: \\.offset) {{ (_swiftIdx{idx}, {asn}) in\n\
                 {pad}    let {idx}: Double = Double(_swiftIdx{idx})\n",
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

    // -----------------------------------------------------------------
    // HostTable column-width threading.
    //
    // When this For sits in cell-position inside a HostTable Row AND the
    // enclosing TableContext carries a `column_widths_slot` AND we have
    // an `index:` binding, each iteration's cell view must take an
    // explicit column width (`columnWidths[Int(<idx>)]`) rather than
    // auto-sizing to its text content.
    //
    // We do NOT append a trailing `.frame(width:)` after the cell's own
    // modifier chain (that paints too late — the cell's `.background` /
    // `.border` would already have drawn around the text-sized box, and
    // a trailing width frame just re-centers that tiny painted box in a
    // wide invisible column).  Instead we thread the width EXPRESSION
    // into the body child's modifier chain via `injected_width`, where
    // it merges with the cell's own height + alignment and lands BEFORE
    // background/border.  See [`swiftui_modifier_chain`] and the
    // body-cell ordering fix in PR
    // feat/emit-swiftui-cell-fill-and-alignment.
    //
    // The injection is scoped to the For's DIRECT body children only;
    // anything nested deeper gets `None`.
    //
    // Discoverability gates (all must hold; otherwise existing behaviour
    // is preserved unchanged):
    //
    //   1. `width_thread` true (caller is [`emit_table_cell`]).
    //   2. `table_ctx` carries a `column_widths_slot`.
    //   3. The For has an `index:` binding (an `as:`-only For has no
    //      column index to address).
    //
    // The slot name has already been camel-cased and identifier-checked
    // when the TableContext was extracted in [`extract_table_context`].
    let width_expr: Option<String> = if width_thread {
        match (
            &index_name,
            table_ctx.and_then(|c| c.column_widths_slot.as_deref()),
        ) {
            (Some(idx), Some(slot)) => Some(format!("{slot}[Int({idx})]")),
            _ => None,
        }
    } else {
        None
    };

    let scoped_payload = Some(ForPayloadScope {
        item: as_name.as_str(),
        index: index_name
            .as_deref()
            .or(for_payload.and_then(|scope| scope.index)),
    });

    let mut out = header;
    if node.children.is_empty() {
        // Empty body — SwiftUI's view builder requires *something* in
        // the closure. `EmptyView()` is the canonical no-op.
        out.push_str(&format!("{}EmptyView()\n", " ".repeat(body_indent)));
    } else {
        out.push_str(&emit_children(
            &node.children,
            body_indent,
            part_styles,
            emits,
            table_ctx,
            scoped_payload,
            width_expr.as_deref(),
        )?);
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
    part_styles: &PartStyleMap,
    emits: &[EmitDecl],
    table_ctx: Option<&TableContext>,
    for_payload: Option<ForPayloadScope<'_>>,
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
        out.push_str(&emit_children(
            &if_node.children,
            indent + 4,
            part_styles,
            emits,
            table_ctx,
            for_payload,
            None,
        )?);
    }

    if let Some(en) = else_node {
        out.push_str(&format!("{pad}}} else {{\n"));
        if en.children.is_empty() {
            out.push_str(&format!("{}EmptyView()\n", " ".repeat(indent + 4)));
        } else {
            out.push_str(&emit_children(
                &en.children,
                indent + 4,
                part_styles,
                emits,
                table_ctx,
                for_payload,
                None,
            )?);
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
fn find_prop_value<'a>(node: &'a LayoutNode, prop_name: &str) -> Option<&'a LayoutPropValue> {
    node.props
        .iter()
        .find(|p| p.name == prop_name)
        .map(|p| &p.value)
}

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

/// Lower a host-owned native surface to the typed `AnyView` node supplied by
/// the component host. A missing binding remains an empty native view so
/// generated preview/project shells stay buildable before a host is attached.
fn emit_host_surface(node: &LayoutNode, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let expression = find_slot_ref_prop(node, "content")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "AnyView(EmptyView())".to_string());
    format!("{pad}{expression}\n")
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
    // Tests build `EmitOptions::default()` then set one field; the sequential
    // form reads clearly and is behavior-identical to an initializer.
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use moslayout_compiler::{LayoutNode, LayoutProp};
    use mosmodel_compiler::EmitParam;
    use mosstyle_compiler::{PartStyle, StateStyle, StyleDef, StyleProp, StyleTransition};

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

    fn node_with_props(tag: &str, props: Vec<LayoutProp>, children: Vec<LayoutNode>) -> LayoutNode {
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
                slot("tags", SlotType::List(Box::new(ListInnerType::Text)), true),
            ],
            vec![],
        );
        let l = box_layout("Profile");
        let out = from_pipeline(&m, &l, &empty_style("Profile"))
            .unwrap()
            .output;

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
                    vec![
                        param("row", EmitPayloadType::Number),
                        param("col", EmitPayloadType::Number),
                    ],
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
        assert!(out.contains("extension GridEvent {"));
        assert!(out.contains("var mosaicName: String"));
        assert!(out.contains("case .navigate(row: _, col: _): return \"onNavigate\""));
        assert!(out.contains("case .editCommit(value: _): return \"onEditCommit\""));
        assert!(out.contains("case .clear: return \"onClear\""));
        assert!(out.contains(
            "case let .navigate(row: row, col: col): return [\"row\": row, \"col\": col]"
        ));
        assert!(out.contains("case let .editCommit(value: value): return [\"value\": value]"));
        assert!(out.contains("case .clear: return [:]"));
        assert!(out.contains("envelope[\"event\"] = mosaicName"));
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
            let layout = layout_with("X", container_node("Box", vec![leaf(tag, vec![])]));
            let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
                .unwrap()
                .output;
            // Containers print `{ }` for empty bodies (one space inside the
            // braces), so we use a strict prefix to allow either form.
            assert!(
                out.contains(expected)
                    || out.contains(&format!("{} }}", expected.trim_end_matches('{').trim())),
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
                vec![leaf("Text", vec![prop_slot_ref("content", "display-name")])],
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
        let layout2 = layout_with("Pic2", container_node("Box", vec![leaf("Image", vec![])]));
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
        let layout = layout_with("X", container_node("Box", vec![leaf("Scroll", vec![])]));
        let err = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
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
        let out = from_pipeline(&m, &layout, &empty_style("Header"))
            .unwrap()
            .output;

        // Header lines in the expected order.
        let import_pos = out.find("import SwiftUI").expect("import present");
        let enum_pos = out.find("enum HeaderEvent {").expect("enum present");
        let struct_pos = out
            .find("struct HeaderView: View {")
            .expect("struct present");
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
    // Test 13 — version pin: the crate version is 0.5.0. Catches accidental
    // version bumps before they merge.
    // ---------------------------------------------------------------------

    #[test]
    fn version_is_0_5_0() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.5.0");
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
        let layout = layout_with("S", container_node("Box", vec![leaf("Stack", vec![])]));
        let out = from_pipeline(&component("S", vec![], vec![]), &layout, &empty_style("S"))
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
            &component("Form", vec![slot("query", SlotType::Text, true)], vec![]),
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
    // Test 16b — a `HostInput` with BOTH a `value` slot and an `onChange`
    // handler is EDITABLE: it lowers to a writable `Binding(get:set:)` whose
    // setter dispatches the change per keystroke (so the field can be typed
    // into), instead of the read-only `.constant(...)`. The separate
    // `.onChange(of:)` modifier is NOT emitted (the setter is the dispatch
    // path; emitting both would feed back).
    // ---------------------------------------------------------------------

    #[test]
    fn host_input_with_on_change_emits_editable_binding() {
        let layout = layout_with(
            "Form",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_string("placeholder", "Enter formula"),
                        prop_slot_ref("value", "formula"),
                        prop_emit_ref("onChange", "onFormulaChange"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "Form",
                vec![slot("formula", SlotType::Text, true)],
                vec![emit(
                    "onFormulaChange",
                    vec![param("value", EmitPayloadType::Text)],
                )],
            ),
            &layout,
            &empty_style("Form"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(
                "text: Binding(get: { formula }, set: { dispatch(.formulaChange(value: $0)) })"
            ),
            "expected an editable Binding setter, got:\n{out}"
        );
        assert!(
            !out.contains(".constant(formula)"),
            "editable input must not use a read-only .constant binding, got:\n{out}"
        );
        assert!(
            !out.contains(".onChange(of:"),
            "the setter is the dispatch path; no separate .onChange modifier, got:\n{out}"
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
        // Always emit the void-form dispatch. The previous
        // `dispatch(.commit(value: value))` shape was an emitter bug:
        // when the Mosaic component declares `emit onCommit ;` (no
        // payload — which is the common case), the Swift compiler
        // rejected the value-carrying form as "enum case has no
        // associated values". Authors that need the live value
        // subscribe to `onChange` instead (which IS the value-
        // carrying path; see `.onChange(of: ...)` test elsewhere).
        assert!(
            out.contains(".onSubmit { dispatch(.commit) }"),
            "expected void-form .onSubmit dispatch, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // Test 18b — onCancel emits `.onExitCommand` guarded by
    // `#if os(macOS)`.
    //
    // `.onExitCommand` is macOS-only.  Emitting it unconditionally
    // broke iOS builds with "value of type ... has no member
    // 'onExitCommand'".  Now wrapped in a `#if os(macOS)` compile-time
    // guard so the same generated file targets the whole Apple
    // platform fleet — macOS, iOS, iPadOS, tvOS, watchOS.
    // ---------------------------------------------------------------------

    #[test]
    fn host_input_on_cancel_wraps_on_exit_command_in_os_macos_guard() {
        let layout = layout_with(
            "F",
            container_node(
                "Box",
                vec![leaf(
                    "HostInput",
                    vec![
                        prop_slot_ref("value", "value"),
                        prop_emit_ref("onCancel", "onCancel"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "F",
                vec![slot("value", SlotType::Text, true)],
                vec![emit("onCancel", vec![])],
            ),
            &layout,
            &empty_style("F"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("#if os(macOS)"),
            "expected #if os(macOS) guard around .onExitCommand, got:\n{out}"
        );
        assert!(
            out.contains(".onExitCommand { dispatch(.cancel) }"),
            "expected .onExitCommand dispatch inside the guard, got:\n{out}"
        );
        assert!(
            out.contains("#endif"),
            "expected #endif closing the guard, got:\n{out}"
        );
        // Regression: the guard must actually wrap onExitCommand,
        // not be elsewhere in the file.
        let if_pos = out.find("#if os(macOS)").unwrap();
        let cmd_pos = out.find(".onExitCommand").unwrap();
        let endif_pos = out.find("#endif").unwrap();
        assert!(
            if_pos < cmd_pos && cmd_pos < endif_pos,
            "expected #if/.onExitCommand/#endif in order:\n{out}"
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
    fn host_button_on_click_emits_button_with_action_and_label() {
        let layout = layout_with(
            "Bar",
            container_node(
                "Box",
                vec![leaf(
                    "HostButton",
                    vec![
                        prop_slot_ref("label", "caption"),
                        prop_emit_ref("onClick", "onClick"),
                    ],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "Bar",
                vec![slot("caption", SlotType::Text, true)],
                vec![emit("onClick", vec![])],
            ),
            &layout,
            &empty_style("Bar"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("Button(action: { dispatch(.click) }) {"),
            "expected Button(action:) opener, got:\n{out}"
        );
        assert!(
            out.contains("Text(caption)"),
            "expected label closure with Text(caption), got:\n{out}"
        );
    }

    #[test]
    fn host_button_inside_indexed_for_dispatches_index_payload() {
        let layout = layout_with(
            "ListGroup",
            container_node(
                "Column",
                vec![node_with_props(
                    "For",
                    vec![
                        prop_slot_ref("each", "items"),
                        prop_keyword("as", "item"),
                        prop_keyword("index", "i"),
                    ],
                    vec![leaf(
                        "HostButton",
                        vec![
                            prop_keyword("label", "item"),
                            prop_emit_ref("onClick", "onSelect"),
                        ],
                    )],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "ListGroup",
                vec![slot(
                    "items",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                )],
                vec![emit(
                    "onSelect",
                    vec![param("index", EmitPayloadType::Number)],
                )],
            ),
            &layout,
            &empty_style("ListGroup"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("let i: Double = Double(_swiftIdxi)"),
            "expected numeric For index binding, got:\n{out}"
        );
        assert!(
            out.contains("dispatch(.select(index: i))"),
            "expected HostButton to dispatch index payload, got:\n{out}"
        );
        assert!(
            out.contains("Text(item)"),
            "expected HostButton label to use For item binding, got:\n{out}"
        );
    }

    #[test]
    fn host_button_inside_for_dispatches_text_item_payload() {
        let layout = layout_with(
            "SelectMenu",
            container_node(
                "Column",
                vec![node_with_props(
                    "For",
                    vec![
                        prop_slot_ref("each", "options"),
                        prop_keyword("as", "option"),
                    ],
                    vec![leaf(
                        "HostButton",
                        vec![
                            prop_keyword("label", "option"),
                            prop_emit_ref("onClick", "onChange"),
                        ],
                    )],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
                "SelectMenu",
                vec![slot(
                    "options",
                    SlotType::List(Box::new(ListInnerType::Text)),
                    true,
                )],
                vec![emit(
                    "onChange",
                    vec![param("value", EmitPayloadType::Text)],
                )],
            ),
            &layout,
            &empty_style("SelectMenu"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("dispatch(.change(value: option))"),
            "expected HostButton to dispatch item payload, got:\n{out}"
        );
        assert!(
            out.contains("Text(option)"),
            "expected HostButton label to use For item binding, got:\n{out}"
        );
    }

    #[test]
    fn host_scroll_lowers_to_scroll_view() {
        let layout = layout_with(
            "S",
            container_node(
                "HostScroll",
                vec![leaf("Text", vec![prop_string("content", "row")])],
            ),
        );
        let out = from_pipeline(&component("S", vec![], vec![]), &layout, &empty_style("S"))
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
        for tag in [
            "Stack",
            "HostScroll",
            "HostInput",
            "HostButton",
            "HostTable",
            "HostDialog",
        ] {
            let layout = layout_with("X", container_node("Box", vec![leaf(tag, vec![])]));
            let r = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"));
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
            let layout = layout_with("X", container_node("Box", vec![leaf(tag, vec![])]));
            let r = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"));
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
        let layout = layout_with("T", container_node("Box", vec![leaf("HostTable", vec![])]));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
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
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // HostTable rows now lower to `HStack(spacing: 0)` so cells sit
        // flush — matches the `border-collapse: collapse` semantics .msl
        // declares on the sheet part.  See the section walker for the
        // rationale.
        assert!(
            out.contains("HStack(spacing: 0) {"),
            "expected HStack(spacing: 0) opener, got:\n{out}"
        );
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
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // Two HStacks (one per Row), four Texts total, no .bold().
        // HostTable rows use `HStack(spacing: 0)` per border-collapse
        // semantics — see [`emit_table_section_rows`].
        let hstack_count = out.matches("HStack(spacing: 0) {").count();
        assert_eq!(
            hstack_count, 2,
            "expected 2 HStacks, got {hstack_count}:\n{out}"
        );
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
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
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
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
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
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // ColGroup is now part of the column-widths threading pipeline;
        // the inline comment documents that intent (and that an empty
        // ColGroup falls back to no width threading).
        assert!(
            out.contains("// HostTableColGroup"),
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
        let result = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"));
        let out = result.expect("orphan sub-tag must not error").output;
        assert!(
            out.contains("// HostTableHead outside HostTable — ignored"),
            "expected orphan sub-tag comment, got:\n{out}"
        );

        // Sanity: HostTableBody / HostTableFoot / HostTableColGroup
        // orphans take the same path.
        for tag in ["HostTableBody", "HostTableFoot", "HostTableColGroup"] {
            let layout = layout_with("T", container_node("Box", vec![leaf(tag, vec![])]));
            let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
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
        let layout = layout_with("T", container_node("Box", vec![table_node]));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        let part_pos = out
            .find("// part: data-table")
            .expect("part comment present");
        let vstack_pos = out
            .find("VStack(alignment: .leading, spacing: 0)")
            .expect("VStack present");
        assert!(
            part_pos < vstack_pos,
            "expected // part comment to precede VStack, got:\n{out}"
        );
    }

    // =================================================================
    // UI31 — HostTable a11y gate + RTL contract (SwiftUI backend)
    //
    // Mirrors the React (#4143), HTML (#4156), WebComponent (#4162),
    // Flutter (#4166), and Qt (#4185) precedents:
    //
    // - **A11y gate**: the SwiftUI lowering must continue to emit
    //   the structural VStack-of-HStack rows that VoiceOver and
    //   Switch Control walk as a coherent table. A flat ZStack
    //   would break that.
    // - **RTL gate**: when `dir:` is authored, the VStack carries
    //   `.environment(\.layoutDirection, .rightToLeft)` (or the
    //   matching `.leftToRight` / slot binding). Allow-list is
    //   `ltr|rtl|auto`; unknown keywords drop silently.
    // =================================================================

    /// Helper: build a `HostTable` with one `HostTableBody` row and a
    /// `dir:` prop set to the given value. Used as the fixture for
    /// the UI31 RTL tests.
    fn host_table_with_dir(value: LayoutPropValue) -> LayoutDef {
        let table_node = LayoutNode {
            tag: "HostTable".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "dir".to_string(),
                value,
            }],
            children: vec![table_section("HostTableBody", vec![vec!["x"]])],
        };
        layout_with("T", container_node("Box", vec![table_node]))
    }

    /// UI31 §3.1 a11y gate — `HostTable` MUST continue to lower to
    /// the structural `VStack(alignment: .leading, spacing: 0)` of
    /// `HStack` rows. A regression to a flat `ZStack` or `Group`
    /// would break VoiceOver's table-row traversal.
    #[test]
    fn ui31_a11y_host_table_uses_structural_vstack() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section("HostTableBody", vec![vec!["a"]])],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            out.contains("VStack(alignment: .leading, spacing: 0)"),
            "HostTable must lower to VStack, got:\n{out}"
        );
        assert!(
            out.contains("HStack"),
            "HostTable body must include HStack rows, got:\n{out}"
        );
    }

    /// UI31 §3.2 RTL contract — `dir: rtl` keyword attaches
    /// `.environment(\.layoutDirection, .rightToLeft)` as a modifier
    /// on the VStack. SwiftUI's `Environment(\.layoutDirection)` is
    /// the canonical RTL knob; every horizontally-laid SwiftUI
    /// container respects it.
    #[test]
    fn ui31_rtl_host_table_dir_rtl_keyword_emits_environment_modifier() {
        let layout = host_table_with_dir(LayoutPropValue::Keyword("rtl".to_string()));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            out.contains(".environment(\\.layoutDirection, .rightToLeft)"),
            "expected .environment(\\.layoutDirection, .rightToLeft), got:\n{out}"
        );
    }

    /// `dir: ltr` keyword emits the explicit-leftToRight form.
    /// Symmetry with the rtl test; explicit `.leftToRight` is the
    /// right thing when an author needs to override an ambient RTL
    /// ancestor (e.g. a HostTable inside a RTL window where they
    /// want this particular table to stay LTR — useful for data
    /// like number-heavy spreadsheets).
    #[test]
    fn ui31_rtl_host_table_dir_ltr_keyword_emits_left_to_right() {
        let layout = host_table_with_dir(LayoutPropValue::Keyword("ltr".to_string()));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            out.contains(".environment(\\.layoutDirection, .leftToRight)"),
            "expected .environment(\\.layoutDirection, .leftToRight), got:\n{out}"
        );
    }

    /// `dir: auto` keyword is the spec-mandated "let the host
    /// decide". SwiftUI has no `.automatic` enum case for
    /// layoutDirection; the right behaviour is to NOT emit the
    /// modifier so the ambient `Environment(\.layoutDirection)`
    /// flows through unchanged. This matches the system locale → app
    /// → ancestor view cascade that SwiftUI uses by default.
    #[test]
    fn ui31_rtl_host_table_dir_auto_keyword_does_not_emit_modifier() {
        let layout = host_table_with_dir(LayoutPropValue::Keyword("auto".to_string()));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            !out.contains(".environment(\\.layoutDirection"),
            "auto must NOT emit a layoutDirection modifier, got:\n{out}"
        );
        assert!(
            out.contains("VStack(alignment: .leading, spacing: 0)"),
            "bare VStack should still render, got:\n{out}"
        );
    }

    /// `dir: slot: layout-direction` interpolates the bound slot
    /// (camel-cased to `layoutDirection`) into the modifier. The
    /// slot is expected to evaluate to a `LayoutDirection`; this is
    /// the contract the host must honour. Slot name goes through
    /// `is_safe_swift_identifier` so it can't smuggle malicious
    /// Swift through the modifier's expression position.
    #[test]
    fn ui31_rtl_host_table_dir_slot_ref_interpolates_camel_case_identifier() {
        let table_node = LayoutNode {
            tag: "HostTable".to_string(),
            part_name: None,
            props: vec![LayoutProp {
                name: "dir".to_string(),
                value: LayoutPropValue::SlotRef("layout-direction".to_string()),
            }],
            children: vec![table_section("HostTableBody", vec![vec!["x"]])],
        };
        let layout = layout_with("T", container_node("Box", vec![table_node]));
        let out = from_pipeline(
            &component(
                "T",
                vec![slot("layout-direction", SlotType::Text, true)],
                vec![],
            ),
            &layout,
            &empty_style("T"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".environment(\\.layoutDirection, layoutDirection)"),
            "expected .environment(\\.layoutDirection, layoutDirection), got:\n{out}"
        );
    }

    /// Unknown `dir:` keywords (anything outside the `ltr|rtl|auto`
    /// allow-list) MUST drop silently. This is the security gate:
    /// an attacker-controlled keyword cannot inject Swift code
    /// because it never reaches the format string. Test payload
    /// `".rightToLeft).onAppear { pwn() }"` is specifically shaped
    /// to break out of the modifier-call argument list if naively
    /// interpolated.
    #[test]
    fn ui31_rtl_host_table_unknown_dir_keyword_drops_silently() {
        let layout = host_table_with_dir(LayoutPropValue::Keyword(
            ".rightToLeft).onAppear { pwn() }".to_string(),
        ));
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            !out.contains("pwn()"),
            "unknown keyword payload must not appear, got:\n{out}"
        );
        assert!(
            !out.contains(".environment(\\.layoutDirection"),
            "unknown keyword must NOT emit a modifier, got:\n{out}"
        );
        assert!(
            out.contains("VStack(alignment: .leading, spacing: 0)"),
            "bare VStack should still render, got:\n{out}"
        );
    }

    /// Regression guard — `HostTable` with no `dir:` prop emits no
    /// `.environment(\.layoutDirection ...)` line. A future
    /// refactor that always-emits would break authors who rely on
    /// the ambient Environment value.
    #[test]
    fn ui31_rtl_host_table_without_dir_prop_emits_no_modifier() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section("HostTableBody", vec![vec!["x"]])],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            !out.contains(".environment(\\.layoutDirection"),
            "no layoutDirection modifier expected when dir absent, got:\n{out}"
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
            vec![prop_slot_ref("each", "rows"), prop_keyword("as", "row")],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
            .unwrap()
            .output;
        // The closure parameter binds the offset under a
        // Swift-shadowed name and the body re-binds the
        // moslayout-author's `index:` identifier (here `r`) to a
        // `Double` cast.  See `emit_for_swift` for the rationale —
        // Swift refuses `Int == Double` so we lift the index into
        // `Double` to make verbatim Expr text like
        // `( r == editRow && c == editCol )` compile across every
        // backend.
        assert!(
            out.contains("ForEach(Array(rows.enumerated()), id: \\.offset) { (_swiftIdxr, row) in"),
            "expected enumerated ForEach with shadowed Int offset, got:\n{out}"
        );
        assert!(
            out.contains("let r: Double = Double(_swiftIdxr)"),
            "expected explicit Double cast of index binding, got:\n{out}"
        );
    }

    /// The Int→Double cast lets a verbatim Expr referencing both
    /// the index and a `number` slot compile cleanly.  Pins the
    /// fix on the visicalc-style cell-edit predicate shape.
    #[test]
    fn for_with_index_and_double_slot_comparison_compiles() {
        let cell = node_with_props(
            "Box",
            vec![LayoutProp {
                name: "state-when-editing".to_string(),
                value: LayoutPropValue::Expr("( r == editRow && c == editCol )".to_string()),
            }],
            vec![],
        );
        let inner = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "row"),
                prop_keyword("as", "v"),
                prop_keyword("index", "c"),
            ],
            vec![cell],
        );
        let outer = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "rows"),
                prop_keyword("as", "row"),
                prop_keyword("index", "r"),
            ],
            vec![inner],
        );
        let layout = layout_with("Grid", container_node("Box", vec![outer]));
        let out = from_pipeline(
            &component(
                "Grid",
                vec![
                    slot("edit-row", SlotType::Number, true),
                    slot("edit-col", SlotType::Number, true),
                    slot("rows", SlotType::List(Box::new(ListInnerType::Text)), true),
                ],
                vec![],
            ),
            &layout,
            &empty_style("Grid"),
        )
        .unwrap()
        .output;
        // Both outer and inner indices must be cast to Double so the
        // expression's `r == editRow && c == editCol` compares
        // Double-to-Double on both sides.
        assert!(out.contains("let r: Double = Double(_swiftIdxr)"));
        assert!(out.contains("let c: Double = Double(_swiftIdxc)"));
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
            vec![prop_expr("each", "cols.visible"), prop_keyword("as", "col")],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
            vec![prop_slot_ref("each", "cols"), prop_keyword("as", "col")],
            vec![leaf("Text", vec![prop_string("content", "c")])],
        );
        let outer_for = node_with_props(
            "For",
            vec![prop_slot_ref("each", "rows"), prop_keyword("as", "row")],
            vec![inner_for],
        );
        let layout = layout_with("C", container_node("Box", vec![outer_for]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let layout = layout_with("C", container_node("Box", vec![if_node, else_node]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let layout = layout_with("C", container_node("Box", vec![if_node, else_node]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
            .unwrap()
            .output;
        assert!(out.contains("if a {"), "expected first if, got:\n{out}");
        assert!(out.contains("if b {"), "expected second if, got:\n{out}");
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
            vec![prop_slot_ref("each", "rows"), prop_keyword("as", "row")],
            vec![if_node, else_node],
        );
        let layout = layout_with("C", container_node("Box", vec![for_node]));
        let out = from_pipeline(&component("C", vec![], vec![]), &layout, &empty_style("C"))
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

    // =================================================================
    // UI29-1 — `HostDialog` kernel-primitive tests (42 through 49).
    //
    // HostDialog lowers to an invisible `Color.clear` anchor view
    // carrying a `.sheet(...)` (modal=true, default) or `.popover(...)`
    // (modal=false) view modifier. The dialog children become the
    // modifier's content closure, wrapped in a `VStack`.
    //
    // See [`emit_host_dialog`] for the lowering rationale and the
    // full prop→modifier mapping.
    // =================================================================

    // -----------------------------------------------------------------
    // Test 42 — Empty HostDialog emits a Color.clear anchor and a
    // `.sheet` modifier. The content closure still contains a VStack
    // so the modifier signature is well-formed under SwiftUI's
    // `@ViewBuilder` rules even when no children are present.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_empty_emits_color_clear_anchor_and_sheet() {
        let layout = layout_with("D", container_node("Box", vec![leaf("HostDialog", vec![])]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains("Color.clear.frame(width: 0, height: 0)"),
            "expected Color.clear anchor, got:\n{out}"
        );
        assert!(
            out.contains(".sheet(isPresented: .constant(false)) {"),
            "expected .sheet modifier with .constant(false) fallback, got:\n{out}"
        );
        assert!(
            out.contains("VStack {"),
            "expected content-closure VStack, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 43 — `open: slot: x` lowers to `.constant(x)` where `x` is
    // the camelCased slot identifier. This mirrors HostInput's
    // immutable-slot-via-.constant lowering choice (see the binding
    // note in the emitter doc).
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_open_slot_drives_constant_binding() {
        let dialog = leaf("HostDialog", vec![prop_slot_ref("open", "dialog-open")]);
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(".sheet(isPresented: .constant(dialogOpen)) {"),
            "expected .constant(dialogOpen) binding from open slot, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 44 — `modal: true` (the default) selects `.sheet(...)`.
    // Even when the keyword is explicit, we get the modal presenter.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_modal_true_uses_sheet() {
        let dialog = leaf(
            "HostDialog",
            vec![prop_slot_ref("open", "open"), prop_keyword("modal", "true")],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(".sheet(isPresented: .constant(open)) {"),
            "expected modal .sheet presenter, got:\n{out}"
        );
        assert!(
            !out.contains(".popover("),
            "expected no .popover when modal=true, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 45 — `modal: false` switches to `.popover(...)`. Per the
    // emitter doc and SwiftUI's API, `.popover` does NOT accept an
    // `onDismiss:` argument, so even if `onClose` is bound, the
    // popover form must NOT emit one.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_modal_false_uses_popover_without_on_dismiss() {
        let dialog = leaf(
            "HostDialog",
            vec![
                prop_slot_ref("open", "open"),
                prop_keyword("modal", "false"),
                prop_emit_ref("onClose", "onClose"),
            ],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(".popover(isPresented: .constant(open)) {"),
            "expected .popover presenter, got:\n{out}"
        );
        assert!(
            !out.contains("onDismiss:"),
            ".popover does not accept onDismiss; must not be emitted, got:\n{out}"
        );
        assert!(
            !out.contains(".sheet("),
            "expected no .sheet when modal=false, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 46 — children render inside the content closure's VStack,
    // walked through `emit_children` so nested kernel primitives lower
    // the same way they do anywhere else.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_children_render_inside_content_vstack() {
        let dialog = node_with_props(
            "HostDialog",
            vec![prop_slot_ref("open", "open")],
            vec![
                leaf("Text", vec![prop_string("content", "Save changes?")]),
                leaf("HostButton", vec![prop_string("label", "Cancel")]),
            ],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains("VStack {"),
            "expected content VStack, got:\n{out}"
        );
        assert!(
            out.contains(r#"Text("Save changes?")"#),
            "expected child Text inside dialog, got:\n{out}"
        );
        assert!(
            out.contains(r#"Text("Cancel")"#),
            "expected child HostButton's label inside dialog, got:\n{out}"
        );
        // The VStack must appear AFTER the sheet opener, not before.
        let sheet_idx = out.find(".sheet(isPresented:").expect("sheet present");
        let vstack_idx = out.find("VStack {").expect("vstack present");
        assert!(
            sheet_idx < vstack_idx,
            "expected VStack to follow .sheet opener, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 47 — `onClose: emit: onClose` wires the `onDismiss:`
    // argument on `.sheet`. The emit name follows the same
    // strip-`on`-prefix + camelCase convention as every other emit
    // in this backend, so `onClose` becomes `.close` in dispatch.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_on_close_wires_on_dismiss_callback() {
        let dialog = leaf(
            "HostDialog",
            vec![
                prop_slot_ref("open", "open"),
                prop_emit_ref("onClose", "onDialogClose"),
            ],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(
                ".sheet(isPresented: .constant(open), onDismiss: { dispatch(.dialogClose) })"
            ),
            "expected onDismiss wired to dispatch(.dialogClose), got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test 48 — `title: slot: t` emits a `.navigationTitle(t)`
    // modifier inside the content closure. The slot is camelCased
    // (kebab → camel) before reaching the Swift source.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_title_slot_emits_navigation_title() {
        let dialog = leaf(
            "HostDialog",
            vec![
                prop_slot_ref("open", "open"),
                prop_slot_ref("title", "dialog-title"),
            ],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(".navigationTitle(dialogTitle)"),
            "expected .navigationTitle(dialogTitle) modifier, got:\n{out}"
        );

        // Sanity: a string-literal title also lowers to .navigationTitle.
        let dialog2 = leaf(
            "HostDialog",
            vec![
                prop_slot_ref("open", "open"),
                prop_string("title", "Save changes?"),
            ],
        );
        let layout2 = layout_with("D", container_node("Box", vec![dialog2]));
        let out2 = from_pipeline(&component("D", vec![], vec![]), &layout2, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out2.contains(r#".navigationTitle("Save changes?")"#),
            "expected string-literal navigation title, got:\n{out2}"
        );
    }

    // -----------------------------------------------------------------
    // Test 49 — `dismiss-on-backdrop: false` emits
    // `.interactiveDismissDisabled(true)` inside the content closure.
    // The default (unset, or `true`) does NOT emit the modifier — we
    // let SwiftUI's built-in dismissal behaviour stand.
    // -----------------------------------------------------------------

    #[test]
    fn host_dialog_dismiss_on_backdrop_false_disables_interactive_dismiss() {
        let dialog = leaf(
            "HostDialog",
            vec![
                prop_slot_ref("open", "open"),
                prop_keyword("dismiss-on-backdrop", "false"),
            ],
        );
        let layout = layout_with("D", container_node("Box", vec![dialog]));
        let out = from_pipeline(&component("D", vec![], vec![]), &layout, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            out.contains(".interactiveDismissDisabled(true)"),
            "expected .interactiveDismissDisabled(true), got:\n{out}"
        );

        // Negative: without the keyword (default), the modifier must
        // NOT appear.
        let default_dialog = leaf("HostDialog", vec![prop_slot_ref("open", "open")]);
        let layout2 = layout_with("D", container_node("Box", vec![default_dialog]));
        let out2 = from_pipeline(&component("D", vec![], vec![]), &layout2, &empty_style("D"))
            .unwrap()
            .output;
        assert!(
            !out2.contains(".interactiveDismissDisabled("),
            "default backdrop behaviour must not emit modifier, got:\n{out2}"
        );
    }

    // =====================================================================
    // UI29-2 — HostCheckbox + HostRadio (SwiftUI Toggle)
    // =====================================================================

    /// UI29-2 SwiftUI test 1 — a bare `HostCheckbox` (no props) emits a
    /// `Toggle("", isOn: .constant(false))` line. The empty label and
    /// `.constant(false)` binding form is the type-checking degraded
    /// shape; a real component will bind `checked:` and `onToggle:`.
    #[test]
    fn host_checkbox_empty_emits_toggle_with_constant_binding() {
        let checkbox = leaf("HostCheckbox", vec![]);
        let layout = layout_with("X", container_node("Box", vec![checkbox]));
        let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
            .unwrap()
            .output;
        assert!(
            out.contains("Toggle(\"\", isOn: .constant(false))"),
            "expected `Toggle(\"\", isOn: .constant(false))`, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 2 — `checked: slot: c` flows the camelCased
    /// slot name into the `.constant(c)` getter when no onToggle is
    /// bound. The toggle is read-only but type-checks.
    #[test]
    fn host_checkbox_checked_slot_drives_constant_binding() {
        let checkbox = leaf("HostCheckbox", vec![prop_slot_ref("checked", "is-checked")]);
        let layout = layout_with("X", container_node("Box", vec![checkbox]));
        let out = from_pipeline(
            &component("X", vec![slot("is-checked", SlotType::Bool, true)], vec![]),
            &layout,
            &empty_style("X"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".constant(isChecked)"),
            "expected `.constant(isChecked)` binding, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 3 — `onToggle: emit: onChange` plus a
    /// `checked:` slot produces the Binding(get:set:) pair whose setter
    /// dispatches the new value. The `checked: bool` payload field is
    /// kernel-canonical (UI29-2 §2.2).
    #[test]
    fn host_checkbox_on_toggle_dispatches_via_binding_setter() {
        let checkbox = leaf(
            "HostCheckbox",
            vec![
                prop_slot_ref("checked", "is-checked"),
                prop_emit_ref("onToggle", "onChange"),
            ],
        );
        let layout = layout_with("X", container_node("Box", vec![checkbox]));
        let out = from_pipeline(
            &component(
                "X",
                vec![slot("is-checked", SlotType::Bool, true)],
                vec![emit(
                    "onChange",
                    vec![param("checked", EmitPayloadType::Bool)],
                )],
            ),
            &layout,
            &empty_style("X"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(
                "Binding(get: { isChecked }, set: { newValue in dispatch(.change(checked: newValue)) })"
            ),
            "expected Binding(get:set:) dispatching change(checked:), got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 4 — `label: "Remember me"` becomes the first
    /// positional argument to `Toggle(...)` as a Swift string literal.
    #[test]
    fn host_checkbox_string_label_becomes_first_argument() {
        let checkbox = leaf("HostCheckbox", vec![prop_string("label", "Remember me")]);
        let layout = layout_with("X", container_node("Box", vec![checkbox]));
        let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
            .unwrap()
            .output;
        assert!(
            out.contains("Toggle(\"Remember me\", isOn:"),
            "expected `Toggle(\"Remember me\", isOn: …)`, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 5 — `disabled: slot: d` adds a trailing
    /// `.disabled(d)` modifier to the Toggle.
    #[test]
    fn host_checkbox_disabled_slot_emits_disabled_modifier() {
        let checkbox = leaf("HostCheckbox", vec![prop_slot_ref("disabled", "locked")]);
        let layout = layout_with("X", container_node("Box", vec![checkbox]));
        let out = from_pipeline(
            &component("X", vec![slot("locked", SlotType::Bool, true)], vec![]),
            &layout,
            &empty_style("X"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".disabled(locked)"),
            "expected `.disabled(locked)` modifier, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 6 — a bare `HostRadio` (no props) emits the
    /// same degraded-but-compiling shape as a bare HostCheckbox:
    /// `Toggle("", isOn: .constant(false))`.
    #[test]
    fn host_radio_empty_emits_toggle_with_constant_binding() {
        let radio = leaf("HostRadio", vec![]);
        let layout = layout_with("X", container_node("Box", vec![radio]));
        let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
            .unwrap()
            .output;
        assert!(
            out.contains("Toggle(\"\", isOn: .constant(false))"),
            "expected bare Toggle shape, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 7 — `group: "flavor"` is preserved as a
    /// `// group: flavor` comment ahead of the Toggle. SwiftUI has no
    /// implicit radio grouping, so the comment is the canonical place
    /// to keep the group metadata visible until a structural pass
    /// synthesises a Picker from sibling radios.
    #[test]
    fn host_radio_group_string_emits_comment() {
        let radio = leaf("HostRadio", vec![prop_string("group", "flavor")]);
        let layout = layout_with("X", container_node("Box", vec![radio]));
        let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
            .unwrap()
            .output;
        assert!(
            out.contains("// group: flavor"),
            "expected `// group: flavor` comment, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 8 — `onSelect: emit: onPick` + `value:
    /// "vanilla"` produces the positive-transition Binding(get:set:)
    /// whose setter dispatches `.pick(value: "vanilla")` only when
    /// `newValue` is true (the radio was chosen, not deselected).
    #[test]
    fn host_radio_on_select_dispatches_only_on_positive_transition() {
        let radio = leaf(
            "HostRadio",
            vec![
                prop_slot_ref("checked", "is-selected"),
                prop_string("value", "vanilla"),
                prop_emit_ref("onSelect", "onPick"),
            ],
        );
        let layout = layout_with("X", container_node("Box", vec![radio]));
        let out = from_pipeline(
            &component(
                "X",
                vec![slot("is-selected", SlotType::Bool, true)],
                vec![emit("onPick", vec![param("value", EmitPayloadType::Text)])],
            ),
            &layout,
            &empty_style("X"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(
                "Binding(get: { isSelected }, set: { newValue in if newValue { dispatch(.pick(value: \"vanilla\")) } })"
            ),
            "expected positive-transition Binding, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 9a — regression: a `group:` string with an
    /// embedded newline must not break out of the `//` line comment.
    /// Pre-fix, `group: "x\nimport Foo"` would emit:
    /// ```swift
    /// // group: x
    /// import Foo
    /// Toggle(...)
    /// ```
    /// which lets an attacker-controlled group prop inject arbitrary
    /// Swift code into the generated output. The fix replaces `\n` and
    /// `\r` with spaces in the comment text only (the line-comment
    /// scope ends at the next newline, so collapsing them is the
    /// minimal change that closes the vector).
    #[test]
    fn host_radio_group_string_with_newline_is_neutralised_in_comment() {
        let radio = leaf(
            "HostRadio",
            vec![prop_string("group", "x\nimport Foo\nstruct Evil {}")],
        );
        let layout = layout_with("X", container_node("Box", vec![radio]));
        let out = from_pipeline(&component("X", vec![], vec![]), &layout, &empty_style("X"))
            .unwrap()
            .output;
        // The line-comment scope ends at the FIRST \n in the output, so
        // requiring `// group:` and `import Foo` on the SAME line is
        // the tight invariant: there must be no newline between them.
        let comment_line = out
            .lines()
            .find(|l| l.contains("// group:"))
            .expect("group comment present");
        assert!(
            comment_line.contains("import Foo"),
            "newline injection must be neutralised — `import Foo` must \
             stay on the same line as `// group:`, got line:\n{comment_line}"
        );
        // Belt-and-suspenders: the literal `\nimport Foo` substring
        // (real newline byte) must not appear anywhere in the output.
        assert!(
            !out.contains("\nimport Foo"),
            "found a raw newline followed by `import Foo` — injection \
             vector still open, got:\n{out}"
        );
    }

    /// UI29-2 SwiftUI test 9 — `value: slot: v` flows the slot
    /// identifier into the dispatch payload, allowing the value to be
    /// computed at runtime by the host rather than baked into the
    /// generated source.
    #[test]
    fn host_radio_value_slot_flows_into_dispatch_payload() {
        let radio = leaf(
            "HostRadio",
            vec![
                prop_slot_ref("value", "radio-value"),
                prop_emit_ref("onSelect", "onPick"),
            ],
        );
        let layout = layout_with("X", container_node("Box", vec![radio]));
        let out = from_pipeline(
            &component(
                "X",
                vec![slot("radio-value", SlotType::Text, true)],
                vec![emit("onPick", vec![param("value", EmitPayloadType::Text)])],
            ),
            &layout,
            &empty_style("X"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("dispatch(.pick(value: radioValue))"),
            "expected dispatch with `radioValue` bare identifier, got:\n{out}"
        );
    }

    // =====================================================================
    // UI29-4 — HostLink / HostTooltip / HostNumberInput (SwiftUI)
    // =====================================================================

    /// UI29-4 SwiftUI test 1 — bare `HostLink href + label` lowers
    /// to SwiftUI's `Link(label, destination: URL(string: href)!)`.
    /// Default behavior is OS-managed URL open.
    #[test]
    fn host_link_string_href_and_label_emits_swiftui_link() {
        let m = component("X", vec![], vec![]);
        let l = layout_with(
            "X",
            container_node(
                "Box",
                vec![leaf(
                    "HostLink",
                    vec![
                        prop_string("href", "https://example.com"),
                        prop_string("label", "Click me"),
                    ],
                )],
            ),
        );
        let r = from_pipeline(&component("X", vec![], vec![]), &l, &empty_style("X"))
            .unwrap()
            .output;
        let _ = m;
        assert!(
            r.contains("Link(destination: URL(string: \"https://example.com\")!) {"),
            "expected SwiftUI Link with URL, got:\n{r}"
        );
        assert!(
            r.contains("Text(\"Click me\")"),
            "expected SwiftUI Link label, got:\n{r}"
        );
    }

    /// UI29-4 SwiftUI test 2 — `external: false` + `onActivate` swaps
    /// to a Button wrapper that dispatches the named emit without
    /// opening the URL (host's in-app router takes over). Pins the
    /// in-app routing path.
    #[test]
    fn host_link_external_false_with_on_activate_emits_button_dispatch() {
        let m = component(
            "X",
            vec![],
            vec![emit(
                "onNavigate",
                vec![param("href", EmitPayloadType::Text)],
            )],
        );
        let l = layout_with(
            "X",
            container_node(
                "Box",
                vec![leaf(
                    "HostLink",
                    vec![
                        prop_string("href", "/about"),
                        prop_keyword("external", "false"),
                        prop_emit_ref("onActivate", "onNavigate"),
                    ],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            r.contains("Button(action: { dispatch(.navigate(href: \"/about\")) })"),
            "expected Button + dispatch, got:\n{r}"
        );
        assert!(
            !r.contains("URL(string:"),
            "in-app routing path must NOT emit URL open, got:\n{r}"
        );
    }

    #[test]
    fn host_link_inside_indexed_for_dispatches_index_payload() {
        let layout = layout_with(
            "Nav",
            container_node(
                "Column",
                vec![node_with_props(
                    "For",
                    vec![
                        prop_slot_ref("each", "items"),
                        prop_keyword("as", "item"),
                        prop_keyword("index", "i"),
                    ],
                    vec![leaf(
                        "HostLink",
                        vec![
                            prop_string("href", "#"),
                            prop_keyword("label", "item"),
                            prop_keyword("external", "false"),
                            prop_emit_ref("onActivate", "onSelect"),
                        ],
                    )],
                )],
            ),
        );
        let out = from_pipeline(
            &component(
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
            ),
            &layout,
            &empty_style("Nav"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains("let i: Double = Double(_swiftIdxi)"),
            "expected numeric For index binding, got:\n{out}"
        );
        assert!(
            out.contains("dispatch(.select(index: i))"),
            "expected HostLink to dispatch index payload, got:\n{out}"
        );
        assert!(
            out.contains("Text(item)"),
            "expected HostLink label to use For item binding, got:\n{out}"
        );
    }

    /// UI29-4 SwiftUI test 3 — `HostTooltip` wraps its child in a
    /// `VStack` and attaches `.help("text")` to it. The `.help`
    /// modifier is macOS / iOS 16+.
    #[test]
    fn host_tooltip_wraps_child_in_vstack_with_help_modifier() {
        let m = component("X", vec![], vec![]);
        let l = layout_with(
            "X",
            container_node(
                "Box",
                vec![LayoutNode {
                    tag: "HostTooltip".to_string(),
                    part_name: None,
                    props: vec![prop_string("text", "Click to submit")],
                    children: vec![leaf("HostButton", vec![prop_string("label", "Submit")])],
                }],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(r.contains("VStack {"), "expected VStack wrapper, got:\n{r}");
        assert!(
            r.contains(".help(\"Click to submit\")"),
            "expected .help modifier, got:\n{r}"
        );
    }

    /// UI29-4 SwiftUI test 4 — bare `HostNumberInput` with
    /// `placeholder` + `value` slot lowers to `TextField(placeholder,
    /// value: .constant(slot), format: .number)`.
    #[test]
    fn host_number_input_with_placeholder_and_value_slot_emits_textfield() {
        let m = component("X", vec![slot("count", SlotType::Number, true)], vec![]);
        let l = layout_with(
            "X",
            container_node(
                "Box",
                vec![leaf(
                    "HostNumberInput",
                    vec![
                        prop_string("placeholder", "Enter a number"),
                        prop_slot_ref("value", "count"),
                    ],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            r.contains("TextField(\"Enter a number\", value: .constant(count), format: .number)"),
            "expected TextField with .number format, got:\n{r}"
        );
    }

    /// UI29-4 SwiftUI test 5 — `onChange: emit: onSet` makes the
    /// number TextField editable by routing writes through a binding
    /// setter that dispatches the new numeric value.
    #[test]
    fn host_number_input_on_change_emits_binding_setter_dispatch() {
        let m = component(
            "X",
            vec![slot("count", SlotType::Number, true)],
            vec![emit("onSet", vec![param("value", EmitPayloadType::Number)])],
        );
        let l = layout_with(
            "X",
            container_node(
                "Box",
                vec![leaf(
                    "HostNumberInput",
                    vec![
                        prop_slot_ref("value", "count"),
                        prop_emit_ref("onChange", "onSet"),
                    ],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            r.contains(
                "TextField(\"\", value: Binding(get: { count }, set: { dispatch(.set(value: $0)) }), format: .number)"
            ),
            "expected Binding setter with dispatch, got:\n{r}"
        );
        assert!(
            !r.contains(".onChange(of:"),
            "HostNumberInput should dispatch through the binding setter only, got:\n{r}"
        );
    }

    // =================================================================
    // UI32-K-swiftui — `--emit-project` SwiftPM shell tests
    //
    // Covers UI32 spec §3.1-§3.8 per-PR gates:
    //   §3.4 Composable     : default options = no project shell.
    //   §3.5 Banner          : every emitted file starts with banner.
    //   §3.1 Reproducible    : two runs produce byte-identical output.
    //   §3.6.1 Validation    : Swift keyword collision → fail-loud.
    //   §3.6.2 SwiftUI row   : keyword reject-list enforced.
    //   §3.6.3 Pinning       : Package.swift carries pinned versions.
    //   §3.7 Output paths    : only the spec §2.2 enumeration.
    //   §3.8 No env reads    : no /Users/, $HOME, etc. in output.
    // =================================================================

    #[test]
    fn ui32_emit_project_false_is_backward_compatible_with_from_pipeline() {
        let m = component("X", vec![], vec![]);
        let l = layout_with("X", container_node("Box", vec![]));
        let s = empty_style("X");

        let legacy = from_pipeline(&m, &l, &s).unwrap();
        let extended = from_pipeline_with_options(&m, &l, &s, &EmitOptions::default()).unwrap();

        assert_eq!(legacy.output, extended.output, ".swift bytes diverged");
        assert_eq!(legacy.component_name, extended.component_name);
        assert!(
            extended.project.is_none(),
            "default options must NOT emit a project shell"
        );
    }

    #[test]
    fn ui32_emit_project_true_returns_project_files() {
        let m = component("Hello", vec![], vec![]);
        let l = layout_with("Hello", container_node("Box", vec![]));
        let s = empty_style("Hello");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let r = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert!(
            r.project.is_some(),
            "emit_project: true must produce a shell"
        );
    }

    #[test]
    fn ui32_every_emitted_side_file_carries_auto_generated_banner() {
        let m = component("Hello", vec![], vec![]);
        let l = layout_with("Hello", container_node("Box", vec![]));
        let s = empty_style("Hello");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .expect("project shell expected");

        // Package.swift starts with `// swift-tools-version:` per
        // SwiftPM convention; the banner sits on line 2.
        assert!(
            proj.package_swift.starts_with("// swift-tools-version:"),
            "Package.swift must start with `// swift-tools-version:`"
        );
        assert!(
            proj.package_swift
                .contains("AUTO-GENERATED by mosaic-compile"),
            "Package.swift missing banner"
        );
        // App.swift starts with the banner directly.
        assert!(
            proj.app_swift.starts_with("// AUTO-GENERATED"),
            "Sources/App/App.swift must START with banner"
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
        let l = layout_with("Deterministic", container_node("Box", vec![]));
        let s = empty_style("Deterministic");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;

        let a = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        let b = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert_eq!(a.output, b.output, ".swift is not deterministic");
        assert_eq!(a.project, b.project, "project shell is not deterministic");
    }

    /// §3.6.1 + §3.6.2 SwiftUI row defense-in-depth: a component
    /// name that's a valid mosmodel `NAME` (`/[A-Za-z][A-Za-z0-9]*(-...)*/`)
    /// but NOT a valid Swift identifier (because hyphens are
    /// allowed in mosmodel but rejected by Swift) MUST fail-loud
    /// — otherwise `<Foo-Bar>View()` would produce an obscure
    /// `swift build` error. Flagged by the L7 security review.
    #[test]
    fn ui32_hyphenated_component_name_returns_invalid_swift_identifier_error() {
        let m = component("Foo-Bar", vec![], vec![]);
        let l = layout_with("Foo-Bar", container_node("Box", vec![]));
        let s = empty_style("Foo-Bar");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;

        let err =
            from_pipeline_with_options(&m, &l, &s, &opts).expect_err("hyphenated name must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("'Foo-Bar'") && msg.contains("Swift identifier"),
            "expected invalid Swift identifier error, got: {msg}"
        );
    }

    /// §3.6.1 + §3.6.2 SwiftUI row: a component name that's a Swift
    /// reserved keyword (e.g., `Class`, `Protocol`, `Actor`) MUST
    /// fail-loud, not silently emit backtick-quoted Swift.
    #[test]
    fn ui32_swift_keyword_component_name_returns_error() {
        let m = component("Class", vec![], vec![]);
        let l = layout_with("Class", container_node("Box", vec![]));
        let s = empty_style("Class");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;

        let err = from_pipeline_with_options(&m, &l, &s, &opts)
            .expect_err("Swift keyword collision must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("'Class'") && msg.contains("Swift reserved keyword"),
            "expected Swift keyword collision error, got: {msg}"
        );
    }

    /// §3.6.3 Pinning. Package.swift carries the default pinned
    /// swift-tools-version and macOS deployment target.
    #[test]
    fn ui32_package_swift_carries_pinned_default_versions_exactly() {
        let m = component("X", vec![], vec![]);
        let l = layout_with("X", container_node("Box", vec![]));
        let s = empty_style("X");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        assert!(
            proj.package_swift.contains("// swift-tools-version: 5.10"),
            "expected swift-tools-version 5.10, got:\n{}",
            proj.package_swift
        );
        assert!(
            proj.package_swift
                .contains("platforms: [.macOS(.v13), .iOS(.v16)]"),
            "expected macOS .v13 and iOS .v16 platform pins, got:\n{}",
            proj.package_swift
        );
    }

    /// README platform wording for the generated SwiftUI shell.
    #[test]
    fn ui32_readme_documents_macos_and_ios_ready_shell() {
        let m = component("LanguageDeck", vec![], vec![]);
        let l = layout_with("LanguageDeck", container_node("Box", vec![]));
        let s = empty_style("LanguageDeck");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();

        assert!(proj.readme.contains("SwiftUI macOS and iOS-ready shell"));
        assert!(proj.readme.contains("## Run on macOS"));
        assert!(proj.readme.contains("## Use from iOS"));
        assert!(proj
            .readme
            .contains("`Package.swift` pins both `.macOS(.v13)` and `.iOS(.v16)`"));
        assert!(proj.readme.contains("Sources/App/LanguageDeck.swift"));
        assert!(
            !proj.readme.contains("mv LanguageDeck.swift"),
            "README should not tell package builds to move a file that pkg already nests"
        );
    }

    /// §3.7 Output paths tripwire.
    #[test]
    fn ui32_project_files_struct_exposes_only_spec_22_swiftui_files() {
        let m = component("X", vec![], vec![]);
        let l = layout_with("X", container_node("Box", vec![]));
        let s = empty_style("X");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let ProjectFiles {
            package_swift,
            app_swift,
            readme,
        } = proj;
        assert!(!package_swift.is_empty(), "Package.swift empty");
        assert!(!app_swift.is_empty(), "Sources/App/App.swift empty");
        assert!(!readme.is_empty(), "README.md empty");
    }

    #[test]
    fn ui32_emitted_files_contain_no_environment_specific_strings() {
        let m = component("X", vec![], vec![]);
        let l = layout_with("X", container_node("Box", vec![]));
        let s = empty_style("X");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let all = format!(
            "{}\n{}\n{}",
            proj.package_swift, proj.app_swift, proj.readme
        );
        for banned in ["/Users/", "/home/", "C:\\Users\\", "$HOME"] {
            assert!(
                !all.contains(banned),
                "emitted shell contains environment-specific fragment `{banned}`"
            );
        }
    }

    /// App.swift mounts the component's `View` struct in a
    /// WindowGroup. The component-name suffix `View` comes from the
    /// emitter's documented `{component_name}View` convention.
    #[test]
    fn ui32_app_swift_mounts_component_view_in_window_group() {
        let m = component("MyWidget", vec![], vec![]);
        let l = layout_with("MyWidget", container_node("Box", vec![]));
        let s = empty_style("MyWidget");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        // Must use @main App + WindowGroup + the {Component}View
        // initializer.
        assert!(
            proj.app_swift.contains("@main"),
            "App.swift must declare @main"
        );
        assert!(
            proj.app_swift.contains("WindowGroup(\"MyWidget\")"),
            "App.swift must wrap in WindowGroup titled with component name"
        );
        assert!(
            proj.app_swift.contains("MyWidgetView("),
            "App.swift must instantiate MyWidgetView"
        );
        assert!(
            proj.app_swift.contains("dispatch: { event in"),
            "App.swift must provide a dispatch closure"
        );
        assert!(
            proj.app_swift.contains("host.dispatch(event)"),
            "App.swift should dispatch the Mosaic wire envelope through the host"
        );
    }

    #[test]
    fn ui32_app_swift_exposes_optional_mosaic_host_bridge() {
        let m = component(
            "Hostable",
            vec![
                slot("title", SlotType::Text, true),
                slot("count", SlotType::Number, true),
                slot("enabled", SlotType::Bool, true),
                slot("items", SlotType::List(Box::new(ListInnerType::Text)), true),
            ],
            vec![emit("onTap", vec![])],
        );
        let l = layout_with("Hostable", container_node("Box", vec![]));
        let s = empty_style("Hostable");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();

        assert!(proj.app_swift.contains("import Combine"));
        assert!(proj
            .app_swift
            .contains("@StateObject private var host = MosaicHostState()"));
        assert!(proj
            .app_swift
            .contains("@Published var lastHostIntent: [String: Any]? = nil"));
        assert!(proj
            .app_swift
            .contains("private func applyHostResponse(_ response: [String: Any]?)"));
        assert!(proj.app_swift.contains("self.lastHostIntent = intent"));
        assert!(proj
            .app_swift
            .contains("@objc protocol MosaicHostBridgeObject"));
        assert!(proj.app_swift.contains("MosaicHostBridge.load()"));
        assert!(proj
            .app_swift
            .contains("[\"App.MosaicHost\", \"MosaicHost\"]"));
        assert!(proj.app_swift.contains("NSClassFromString(className)"));
        assert!(proj.app_swift.contains(
            "title: MosaicHostValue.string(host.props, \"title\", fallback: \"Sample Title\"),"
        ));
        assert!(proj
            .app_swift
            .contains("count: MosaicHostValue.double(host.props, \"count\", fallback: 0),"));
        assert!(proj
            .app_swift
            .contains("enabled: MosaicHostValue.bool(host.props, \"enabled\", fallback: false),"));
        assert!(proj
            .app_swift
            .contains("items: MosaicHostValue.stringList(host.props, \"items\", fallback: []),"));
        assert!(proj.app_swift.contains("host.dispatch(event)"));
        assert!(proj
            .app_swift
            .contains("applyHostResponse(bridge.handleEvent(event.mosaicEnvelope as NSDictionary"));
    }

    #[test]
    fn ui32_app_swift_passes_sample_slot_values_to_component_view() {
        let mut display_name = slot("display-name", SlotType::Text, false);
        display_name.default = Some(SlotDefault::Text("Ada".to_string()));
        let m = component(
            "ProfileCard",
            vec![
                display_name,
                slot("age", SlotType::Number, true),
                slot("is-active", SlotType::Bool, true),
                slot("avatar-url", SlotType::Image, true),
                slot("accent", SlotType::Color, true),
                slot("tags", SlotType::List(Box::new(ListInnerType::Text)), true),
            ],
            vec![],
        );
        let l = layout_with("ProfileCard", container_node("Box", vec![]));
        let s = empty_style("ProfileCard");
        let mut opts = EmitOptions::default();
        opts.emit_project = true;
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();

        assert!(
            proj.app_swift.contains("ProfileCardView("),
            "App.swift must instantiate ProfileCardView"
        );
        assert!(
            proj.app_swift
                .contains("displayName: MosaicHostValue.string(host.props, \"display-name\", fallback: \"Ada\"),"),
            "text defaults should flow into the generated initializer fallback"
        );
        assert!(
            proj.app_swift
                .contains("age: MosaicHostValue.double(host.props, \"age\", fallback: 0),"),
            "number slots need a sample value"
        );
        assert!(
            proj.app_swift.contains(
                "isActive: MosaicHostValue.bool(host.props, \"is-active\", fallback: false),"
            ),
            "bool slots need a sample value"
        );
        assert!(
            proj.app_swift
                .contains("avatarUrl: MosaicHostValue.string(host.props, \"avatar-url\", fallback: \"sample-image\"),"),
            "image slots need a sample value"
        );
        assert!(
            proj.app_swift.contains(
                "accent: MosaicHostValue.string(host.props, \"accent\", fallback: \"#808080\"),"
            ),
            "color slots need a sample value"
        );
        assert!(
            proj.app_swift
                .contains("tags: MosaicHostValue.stringList(host.props, \"tags\", fallback: []),"),
            "list slots need an empty sample value"
        );
        assert!(
            proj.app_swift.contains("dispatch: { event in"),
            "the generated initializer must pass dispatch last"
        );
        assert!(
            proj.app_swift.contains("host.dispatch(event)"),
            "dispatch should route through the optional Mosaic host state"
        );
    }

    /// Truth table for is_swift_keyword. Covers the
    /// PascalCase subset of Swift reserved words.
    #[test]
    fn ui32_is_swift_keyword_truth_table() {
        // Rejected (PascalCase Swift keywords)
        assert!(is_swift_keyword("Class"));
        assert!(is_swift_keyword("Protocol"));
        assert!(is_swift_keyword("Actor"));
        assert!(is_swift_keyword("Self"));
        assert!(is_swift_keyword("Any"));
        assert!(is_swift_keyword("Type"));
        // Accepted (non-keyword PascalCase names)
        assert!(!is_swift_keyword("Hello"));
        assert!(!is_swift_keyword("HostTable"));
        assert!(!is_swift_keyword("MyWidget"));
        assert!(!is_swift_keyword("Component"));
        // The check is case-sensitive; lowercase variants would never
        // pass the upstream validator anyway (PascalCase required).
        assert!(!is_swift_keyword("class")); // not in our PascalCase set
    }

    // =====================================================================
    // Part-style lowering tests (UI33 style-inlining v1)
    //
    // Cover the new `build_part_style_map` / `swiftui_color_value` /
    // `swiftui_modifier_chain` helpers and confirm the modifier chain is
    // spliced onto the right node by `emit_view_tree`.
    // =====================================================================

    /// Small helper to build a `StyleProp` quickly.
    fn sp(name: &str, value: &str) -> StyleProp {
        StyleProp {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    fn transition(property: &str, duration: &str, easing: &str) -> StyleTransition {
        StyleTransition {
            property: property.to_string(),
            duration: duration.to_string(),
            easing: easing.to_string(),
        }
    }

    /// Build a `StyleDef` with a single `part name { base props }`.
    fn style_with_part(component: &str, part_name: &str, props: Vec<StyleProp>) -> StyleDef {
        StyleDef {
            component_name: component.to_string(),
            parts: vec![PartStyle {
                name: part_name.to_string(),
                base: props,
                transitions: vec![],
                states: Vec::new(),
            }],
        }
    }

    /// Build a Box layout whose root carries a `part_name`.
    fn box_layout_with_part(component: &str, part: &str) -> LayoutDef {
        LayoutDef {
            component_name: component.to_string(),
            root: LayoutNode {
                tag: "Box".to_string(),
                part_name: Some(part.to_string()),
                props: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    // ---------------------------------------------------------------------
    // T1 — `build_part_style_map` returns a map keyed by part name.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_map_keys_by_part_name() {
        let s = StyleDef {
            component_name: "Demo".to_string(),
            parts: vec![
                PartStyle {
                    name: "cell".to_string(),
                    base: vec![sp("padding", "2px")],
                    transitions: vec![],
                    states: Vec::new(),
                },
                PartStyle {
                    name: "header".to_string(),
                    base: vec![sp("font-weight", "bold")],
                    transitions: vec![],
                    states: Vec::new(),
                },
            ],
        };
        let map = build_part_style_map(&s);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("cell"));
        assert!(map.contains_key("header"));
        // Empty StyleDef → empty map (the `empty_style` helper path).
        assert!(build_part_style_map(&empty_style("Demo")).is_empty());
    }

    // ---------------------------------------------------------------------
    // T2 — Unknown / unsupported properties are silently skipped.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_unknown_properties_skipped_silently() {
        // border-collapse, outline, cursor — none map to a SwiftUI modifier.
        let props = vec![
            sp("border-collapse", "collapse"),
            sp("outline", "none"),
            sp("cursor", "pointer"),
        ];
        let chain = swiftui_modifier_chain(&props, &[], 0, None);
        assert_eq!(chain, "");
    }

    // ---------------------------------------------------------------------
    // T3 — width+height collapse to a single `.frame(width: N, height: N)`.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_width_height_collapses_to_single_frame_call() {
        let props = vec![sp("width", "80px"), sp("height", "22px")];
        let chain = swiftui_modifier_chain(&props, &[], 0, None);
        assert_eq!(chain, "\n.frame(width: 80, height: 22)");
        // Singletons render as the one-side form so the user gets
        // SwiftUI's "intrinsic on the other axis" default.
        let only_w = swiftui_modifier_chain(&[sp("width", "80px")], &[], 0, None);
        assert_eq!(only_w, "\n.frame(width: 80)");
        let only_h = swiftui_modifier_chain(&[sp("height", "22px")], &[], 0, None);
        assert_eq!(only_h, "\n.frame(height: 22)");
    }

    // ---------------------------------------------------------------------
    // T4 — `#1e1e1e` → `Color(red: 0.118, green: 0.118, blue: 0.118)`.
    //
    // The 3-decimal-place rounding is load-bearing: `0x1e/255` is
    // `0.11764…`; we round to 3 dp so the generated source stays short
    // and unit-test assertions stay stable.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_hex_background_converts_to_color_init() {
        assert_eq!(
            swiftui_color_value("#1e1e1e"),
            "Color(red: 0.118, green: 0.118, blue: 0.118)"
        );
        // 3-char shorthand `#abc` → `#aabbcc`.
        assert_eq!(
            swiftui_color_value("#fff"),
            "Color(red: 1, green: 1, blue: 1)"
        );
        // Pure black / pure white round to integers (the trailing `.0`
        // is dropped by Rust's float formatting).
        assert_eq!(
            swiftui_color_value("#000000"),
            "Color(red: 0, green: 0, blue: 0)"
        );
        // Named keyword passes through.
        assert_eq!(swiftui_color_value("white"), "Color.white");
        // Unknown name falls back to clear so the file still compiles.
        assert_eq!(swiftui_color_value("rebeccapurple"), "Color.clear");

        // Confirm the .background modifier line is emitted end-to-end.
        let chain = swiftui_modifier_chain(&[sp("background", "#1e1e1e")], &[], 0, None);
        assert_eq!(
            chain,
            "\n.background(Color(red: 0.118, green: 0.118, blue: 0.118))"
        );
    }

    // ---------------------------------------------------------------------
    // T5 — font-size + font-family monospace combine to a single .font().
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_font_size_and_monospace_combine_to_single_font_call() {
        let props = vec![sp("font-size", "12px"), sp("font-family", "monospace")];
        let chain = swiftui_modifier_chain(&props, &[], 0, None);
        assert_eq!(chain, "\n.font(.system(size: 12, design: .monospaced))");

        // Standalone font-size emits the size-only form.
        let only_size = swiftui_modifier_chain(&[sp("font-size", "14px")], &[], 0, None);
        assert_eq!(only_size, "\n.font(.system(size: 14))");

        // Standalone monospace family emits the .body shape.
        let only_mono = swiftui_modifier_chain(&[sp("font-family", "monospace")], &[], 0, None);
        assert_eq!(only_mono, "\n.font(.system(.body, design: .monospaced))");
    }

    // ---------------------------------------------------------------------
    // T6 — border-width + border-color → `.border(Color(...), width: N)`.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_border_width_and_color_emit_border_modifier() {
        let props = vec![sp("border-width", "1px"), sp("border-color", "#3f3f46")];
        let chain = swiftui_modifier_chain(&props, &[], 0, None);
        assert_eq!(
            chain,
            "\n.border(Color(red: 0.247, green: 0.247, blue: 0.275), width: 1)"
        );

        // border-width alone defaults to Color.gray so the modifier
        // still emits something predictable.
        let only_w = swiftui_modifier_chain(&[sp("border-width", "2px")], &[], 0, None);
        assert_eq!(only_w, "\n.border(Color.gray, width: 2)");

        // border-color WITHOUT border-width emits nothing (no width to
        // anchor the modifier to). `border-style: solid` is silently
        // skipped — SwiftUI's .border is always a solid stroke.
        let only_color = swiftui_modifier_chain(
            &[sp("border-color", "#aaa"), sp("border-style", "solid")],
            &[],
            0,
            None,
        );
        assert_eq!(only_color, "");
    }

    // ---------------------------------------------------------------------
    // T7 — End-to-end: Box with `[ cell ]` part_name + cell styles
    //                 produces the expected modifier chain in from_pipeline.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_cell_styles_appear_in_from_pipeline_output() {
        let m = component("Grid", vec![], vec![]);
        let l = box_layout_with_part("Grid", "cell");
        let s = style_with_part(
            "Grid",
            "cell",
            vec![
                sp("border-width", "1px"),
                sp("border-color", "#3f3f46"),
                sp("padding", "2px"),
                sp("height", "22px"),
                sp("background", "#1e1e1e"),
            ],
        );
        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(out.contains(".frame(height: 22)"), "out = {out}");
        assert!(out.contains(".padding(2)"), "out = {out}");
        assert!(
            out.contains(".background(Color(red: 0.118, green: 0.118, blue: 0.118))"),
            "out = {out}"
        );
        assert!(
            out.contains(".border(Color(red: 0.247, green: 0.247, blue: 0.275), width: 1)"),
            "out = {out}"
        );
    }

    // ---------------------------------------------------------------------
    // T8 — A node with NO part_name produces NO modifier chain.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_node_without_part_name_emits_no_modifier_chain() {
        let m = component("Plain", vec![], vec![]);
        let l = box_layout("Plain"); // root has part_name: None
                                     // Style declares a `cell` part — but the root isn't named `cell`,
                                     // so no chain should be spliced.
        let s = style_with_part(
            "Plain",
            "cell",
            vec![sp("padding", "8px"), sp("background", "#fff")],
        );
        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(!out.contains(".padding"), "out = {out}");
        assert!(!out.contains(".background"), "out = {out}");
    }

    // ---------------------------------------------------------------------
    // T9 — An empty PartStyleMap produces NO modifier chain even when the
    //      node has a part_name.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_empty_map_emits_no_modifier_chain() {
        let m = component("Bare", vec![], vec![]);
        // Root carries part_name "cell" but style is empty — the
        // map.get("cell") returns None, no chain emitted.
        let l = box_layout_with_part("Bare", "cell");
        let s = empty_style("Bare");
        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(!out.contains(".padding"));
        assert!(!out.contains(".background"));
        assert!(!out.contains(".border"));
        assert!(!out.contains(".frame"));
    }

    // ---------------------------------------------------------------------
    // T10 — `font-weight: 600` and `font-weight: SemiBold` both lower to
    //       `.fontWeight(.semibold)`. Same for the bold (700) and medium
    //       (500) variants — covers numeric + CamelCase + lowercase
    //       spellings in one go.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_font_weight_variants_all_lower_to_swiftui_weight() {
        // semibold synonyms.
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "600")], &[], 0, None),
            "\n.fontWeight(.semibold)"
        );
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "SemiBold")], &[], 0, None),
            "\n.fontWeight(.semibold)"
        );
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "semibold")], &[], 0, None),
            "\n.fontWeight(.semibold)"
        );
        // bold synonyms.
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "700")], &[], 0, None),
            "\n.fontWeight(.bold)"
        );
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "bold")], &[], 0, None),
            "\n.fontWeight(.bold)"
        );
        // medium synonyms.
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "500")], &[], 0, None),
            "\n.fontWeight(.medium)"
        );
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "medium")], &[], 0, None),
            "\n.fontWeight(.medium)"
        );
        // Unknown weight — silently skipped (matches React emitter posture).
        assert_eq!(
            swiftui_modifier_chain(&[sp("font-weight", "ultraheavy")], &[], 0, None),
            ""
        );
    }

    // ---------------------------------------------------------------------
    // T11 — `strip_css_px` strips `px` suffix; falls through unchanged
    //       for non-px values (sanity check the helper).
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_strip_css_px_helper_behaviour() {
        assert_eq!(strip_css_px("12px"), "12");
        assert_eq!(strip_css_px("0px"), "0");
        // No suffix → passes through.
        assert_eq!(strip_css_px("auto"), "auto");
        assert_eq!(strip_css_px("12"), "12");
    }

    // ---------------------------------------------------------------------
    // T12 — Indentation: the chain renders with the requested `indent`
    //       spaces in front of each modifier line.
    // ---------------------------------------------------------------------

    #[test]
    fn part_style_chain_respects_indent_argument() {
        let chain = swiftui_modifier_chain(&[sp("padding", "4px")], &[], 12, None);
        assert_eq!(chain, "\n            .padding(4)");
    }

    // ---------------------------------------------------------------------
    // T13 — `text-align` maps to the `.frame(alignment:)` argument.
    //
    //   right/end   → .trailing
    //   center      → .center
    //   left/start  → .leading
    // ---------------------------------------------------------------------

    #[test]
    fn text_align_right_sets_frame_trailing_alignment() {
        // With width+height present, the alignment rides the SAME frame
        // call (no separate `.frame`).
        let chain = swiftui_modifier_chain(
            &[
                sp("width", "80px"),
                sp("height", "22px"),
                sp("text-align", "right"),
            ],
            &[],
            0,
            None,
        );
        assert_eq!(
            chain,
            "\n.frame(width: 80, height: 22, alignment: .trailing)"
        );
        // `end` is the logical synonym for `right`.
        let chain_end = swiftui_modifier_chain(
            &[sp("height", "22px"), sp("text-align", "end")],
            &[],
            0,
            None,
        );
        assert_eq!(chain_end, "\n.frame(height: 22, alignment: .trailing)");
    }

    #[test]
    fn text_align_center_and_left_map_to_center_and_leading() {
        let center = swiftui_modifier_chain(
            &[sp("width", "60px"), sp("text-align", "center")],
            &[],
            0,
            None,
        );
        assert_eq!(center, "\n.frame(width: 60, alignment: .center)");

        let left = swiftui_modifier_chain(
            &[sp("width", "60px"), sp("text-align", "left")],
            &[],
            0,
            None,
        );
        assert_eq!(left, "\n.frame(width: 60, alignment: .leading)");

        // `start` is the logical synonym for `left`.
        let start = swiftui_modifier_chain(
            &[sp("width", "60px"), sp("text-align", "start")],
            &[],
            0,
            None,
        );
        assert_eq!(start, "\n.frame(width: 60, alignment: .leading)");
    }

    #[test]
    fn text_align_without_width_or_height_emits_maxwidth_frame() {
        // No width/height — alignment still needs SOMETHING to align
        // within, so we stretch to `.infinity` (v1 cut).
        let chain = swiftui_modifier_chain(&[sp("text-align", "right")], &[], 0, None);
        assert_eq!(chain, "\n.frame(maxWidth: .infinity, alignment: .trailing)");
    }

    #[test]
    fn text_align_unrecognised_value_emits_no_alignment() {
        // `justify` has no SwiftUI Alignment analog → no frame at all
        // (no width/height/alignment present).
        let chain = swiftui_modifier_chain(&[sp("text-align", "justify")], &[], 0, None);
        assert_eq!(chain, "");
    }

    // ---------------------------------------------------------------------
    // T14 — Modifier ORDER (the cell-fill bug fix).  For a part with
    // padding + height + background + border, the chain MUST be:
    //   .padding  →  .frame  →  .background  →  .border
    // so the background fills, and border strokes, the full frame.
    // ---------------------------------------------------------------------

    #[test]
    fn modifier_order_padding_before_frame_before_background_and_border() {
        let chain = swiftui_modifier_chain(
            &[
                sp("padding", "2px"),
                sp("height", "22px"),
                sp("background", "#1e1e1e"),
                sp("border-width", "1px"),
            ],
            &[],
            0,
            None,
        );
        let pad_at = chain.find(".padding").expect("padding present");
        let frame_at = chain.find(".frame").expect("frame present");
        let bg_at = chain.find(".background").expect("background present");
        let border_at = chain.find(".border").expect("border present");
        assert!(pad_at < frame_at, "padding must precede frame:\n{chain}");
        assert!(frame_at < bg_at, "frame must precede background:\n{chain}");
        assert!(frame_at < border_at, "frame must precede border:\n{chain}");
        assert!(
            bg_at < border_at,
            "background must precede border:\n{chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T15 — `injected_width` FORCES the frame to emit (even with no width
    // prop) and takes precedence over any part `width`.  It merges with
    // the part's own height + alignment and lands BEFORE background/border.
    // ---------------------------------------------------------------------

    #[test]
    fn injected_width_merges_into_frame_before_paint() {
        let chain = swiftui_modifier_chain(
            &[
                sp("height", "22px"),
                sp("text-align", "right"),
                sp("background", "#1e1e1e"),
                sp("border-width", "1px"),
            ],
            &[],
            0,
            Some("columnWidths[Int(c)]"),
        );
        // Single merged frame with injected width + height + alignment.
        assert!(
            chain.contains(".frame(width: columnWidths[Int(c)], height: 22, alignment: .trailing)"),
            "expected merged frame, got:\n{chain}"
        );
        // It appears BEFORE both background and border.
        let frame_at = chain.find(".frame(width: columnWidths").unwrap();
        let bg_at = chain.find(".background").unwrap();
        let border_at = chain.find(".border").unwrap();
        assert!(frame_at < bg_at && frame_at < border_at, "got:\n{chain}");
        // No SECOND, trailing `.frame(width:` after the border.
        assert_eq!(
            chain.matches(".frame(width:").count(),
            1,
            "expected exactly one width frame, got:\n{chain}"
        );
    }

    #[test]
    fn injected_width_overrides_part_own_width() {
        // The part sets its own `width: 80px`, but the injected width
        // wins.
        let chain = swiftui_modifier_chain(
            &[sp("width", "80px"), sp("height", "22px")],
            &[],
            0,
            Some("columnWidths[Int(c)]"),
        );
        assert_eq!(chain, "\n.frame(width: columnWidths[Int(c)], height: 22)");
        assert!(
            !chain.contains("width: 80"),
            "injected width must win:\n{chain}"
        );
    }

    // =====================================================================
    // State-block lowering tests (UI33 style-inlining v2)
    //
    // Cover the `collect_state_layers` helper, the composite-key shape of
    // `build_part_style_map`, and the nested-ternary layering in
    // `swiftui_modifier_chain`.  Mirror the React emitter's `state-when-X`
    // mechanism — see `mosaic_emit_react::pipeline` lines 895–960.
    // =====================================================================

    /// Build a `StyleDef` with a single `part` containing both `base`
    /// props and one or more named state blocks.
    fn style_with_part_and_states(
        component: &str,
        part_name: &str,
        base: Vec<StyleProp>,
        states: Vec<(&str, Vec<StyleProp>)>,
    ) -> StyleDef {
        StyleDef {
            component_name: component.to_string(),
            parts: vec![PartStyle {
                name: part_name.to_string(),
                base,
                transitions: vec![],
                states: states
                    .into_iter()
                    .map(|(name, props)| StateStyle {
                        state: name.to_string(),
                        transitions: vec![],
                        props,
                    })
                    .collect(),
            }],
        }
    }

    /// Build a Box node with a `part_name` and a list of layout props
    /// (typically `state-when-*: <Expr/SlotRef>`).
    fn box_node_with_part_and_props(part: &str, props: Vec<LayoutProp>) -> LayoutNode {
        LayoutNode {
            tag: "Box".to_string(),
            part_name: Some(part.to_string()),
            props,
            children: Vec::new(),
        }
    }

    // ---------------------------------------------------------------------
    // T13 — `build_part_style_map` now includes `{part}:{state}` keys for
    //       each non-empty state block.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_part_style_map_includes_composite_keys() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("background", "#1e1e1e")],
            vec![
                ("selected", vec![sp("background", "#264f78")]),
                ("editing", vec![sp("background", "#1f4f3f")]),
            ],
        );
        let map = build_part_style_map(&s);
        assert!(map.contains_key("cell"), "base key missing: {map:?}");
        assert!(
            map.contains_key("cell:selected"),
            "selected composite missing: {map:?}"
        );
        assert!(
            map.contains_key("cell:editing"),
            "editing composite missing: {map:?}"
        );
        // Empty state block (`state hover { }`) is skipped — saves
        // downstream code from having to check `.is_empty()` again.
        let s2 = style_with_part_and_states("Demo", "cell", vec![], vec![("hover", vec![])]);
        let map2 = build_part_style_map(&s2);
        assert!(!map2.contains_key("cell:hover"));
    }

    // ---------------------------------------------------------------------
    // T14 — `collect_state_layers` returns layers in declaration order
    //       (so the LAST `state-when` wins as the outermost ternary).
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_collect_layers_returns_declaration_order() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![],
            vec![
                ("selected", vec![sp("background", "#264f78")]),
                ("editing", vec![sp("background", "#1f4f3f")]),
            ],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![
                prop_expr("state-when-selected", "( isSel )"),
                prop_expr("state-when-editing", "( isEdit )"),
            ],
        );
        let layers = collect_state_layers(&node, "cell", &map);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].cond_expr, "( isSel )");
        assert_eq!(layers[1].cond_expr, "( isEdit )");
    }

    // ---------------------------------------------------------------------
    // T15 — A node with a `state-when-selected` Expr prop and base+state
    //       `background` produces `.background((expr) ? state : base)`.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_single_state_emits_ternary_background() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("background", "#1e1e1e")],
            vec![("selected", vec![sp("background", "#264f78")])],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![prop_expr(
                "state-when-selected",
                "( r == selectedRow && c == selectedCol )",
            )],
        );
        let base_props = map
            .get("cell")
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain(base_props, &layers, 0, None);
        // Modifier wraps the bucket value in `(...)`; the bucket value
        // is itself a parenthesised ternary `((cond) ? v : base)` where
        // `cond` is the moslayout-parser-supplied Expr text (already
        // wrapped in its own `( ... )`).  Final shape:
        // `.background(((( r == ... )) ? v : base))`.
        assert!(
            chain.contains(".background(((( r == selectedRow && c == selectedCol )) ? Color(red: 0.149, green: 0.31, blue: 0.471) : Color(red: 0.118, green: 0.118, blue: 0.118)))"),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T16 — A state that overrides a property with NO base value uses
    //       `Color.clear` (background) in the "no value" branch.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_no_base_falls_back_to_color_clear() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![],
            vec![("selected", vec![sp("background", "#264f78")])],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![prop_expr("state-when-selected", "( isSel )")],
        );
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain(&[], &layers, 0, None);
        assert!(
            chain.contains(".background(((( isSel )) ? Color(red: 0.149, green: 0.31, blue: 0.471) : Color.clear))"),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T17 — Two states layered: the LATER `state-when-X` becomes the
    //       OUTERMOST condition.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_two_states_last_is_outermost_ternary() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("background", "#1e1e1e")],
            vec![
                ("selected", vec![sp("background", "#264f78")]),
                ("editing", vec![sp("background", "#1f4f3f")]),
            ],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![
                prop_expr("state-when-selected", "( sel )"),
                prop_expr("state-when-editing", "( edit )"),
            ],
        );
        let base_props = map
            .get("cell")
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain(base_props, &layers, 0, None);
        // editing wraps selected; selected wraps base.  The modifier
        // adds one outer `(...)` and each layer_value wraps in
        // `((cond) ? v : inner)`.
        let inner_sel = "((( sel )) ? Color(red: 0.149, green: 0.31, blue: 0.471) : Color(red: 0.118, green: 0.118, blue: 0.118))";
        let expected = format!(
            ".background(((( edit )) ? Color(red: 0.122, green: 0.31, blue: 0.247) : {inner_sel}))"
        );
        assert!(chain.contains(&expected), "chain = {chain}");
        // And `edit` (the later one) is the OUTER condition — it appears
        // before `sel` in the modifier text.
        let edit_pos = chain.find("( edit )").expect("edit cond present");
        let sel_pos = chain.find("( sel )").expect("sel cond present");
        assert!(
            edit_pos < sel_pos,
            "expected edit outside sel, chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T18 — A SlotRef state-when cond resolves to a camelCased identifier.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_slot_ref_cond_resolves_to_camel_case() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![],
            vec![("selected", vec![sp("background", "#264f78")])],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![prop_slot_ref("state-when-selected", "is-selected")],
        );
        let layers = collect_state_layers(&node, "cell", &map);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].cond_expr, "isSelected");
    }

    // ---------------------------------------------------------------------
    // T19 — An Expr state-when cond passes through verbatim.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_expr_cond_passes_through_verbatim() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![],
            vec![("selected", vec![sp("background", "#264f78")])],
        );
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "cell",
            vec![prop_expr(
                "state-when-selected",
                "( r == selectedRow && c == selectedCol )",
            )],
        );
        let layers = collect_state_layers(&node, "cell", &map);
        assert_eq!(layers.len(), 1);
        assert_eq!(
            layers[0].cond_expr,
            "( r == selectedRow && c == selectedCol )"
        );
    }

    // ---------------------------------------------------------------------
    // T20 — A state that overrides `color` but NOT `background` leaves
    //       `.background` at the base value (no ternary).
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_partial_override_leaves_other_props_alone() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("background", "#1e1e1e"), sp("color", "#cccccc")],
            vec![("selected", vec![sp("color", "#ffffff")])],
        );
        let map = build_part_style_map(&s);
        let node =
            box_node_with_part_and_props("cell", vec![prop_expr("state-when-selected", "( sel )")]);
        let base_props = map
            .get("cell")
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain(base_props, &layers, 0, None);
        // .background stays at the base value — no ternary.
        assert!(
            chain.contains(".background(Color(red: 0.118, green: 0.118, blue: 0.118))"),
            "background should NOT be a ternary, chain = {chain}"
        );
        assert!(
            !chain.contains(".background((( sel ))"),
            "background should NOT layer, chain = {chain}"
        );
        // .foregroundColor IS a ternary (4 opening parens: modifier
        // wrap + layer_value wrap + cond_expr's own parens).
        assert!(
            chain.contains(".foregroundColor(((( sel )) ? Color(red: 1, green: 1, blue: 1) : Color(red: 0.8, green: 0.8, blue: 0.8)))"),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T21 — A state block whose props are all unrecognised (e.g. only
    //       `outline`) produces no extra modifiers and no extra parens.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_with_only_unrecognised_props_emits_no_extra_modifier() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("padding", "4px")],
            vec![("selected", vec![sp("outline", "1px solid #007acc")])],
        );
        let map = build_part_style_map(&s);
        let node =
            box_node_with_part_and_props("cell", vec![prop_expr("state-when-selected", "( sel )")]);
        let base_props = map
            .get("cell")
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain(base_props, &layers, 0, None);
        // Only `.padding(4)` — no ternary, no `( sel )`, no extra parens.
        assert_eq!(chain, "\n.padding(4)", "chain = {chain}");
    }

    // ---------------------------------------------------------------------
    // T22 — `state-when-X` without a matching `state X { }` block in the
    //       `.msl` is silently ignored (matches React posture).
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_state_when_without_matching_state_block_is_ignored() {
        let s = style_with_part_and_states(
            "Demo",
            "cell",
            vec![sp("background", "#1e1e1e")],
            // No "selected" state block at all.
            vec![],
        );
        let map = build_part_style_map(&s);
        let node =
            box_node_with_part_and_props("cell", vec![prop_expr("state-when-selected", "( sel )")]);
        let layers = collect_state_layers(&node, "cell", &map);
        assert!(layers.is_empty());
        let base_props = map
            .get("cell")
            .map(|style| style.props.as_slice())
            .unwrap_or(&[]);
        let chain = swiftui_modifier_chain(base_props, &layers, 0, None);
        // No ternary, just the base background.
        assert!(
            chain.contains(".background(Color(red: 0.118, green: 0.118, blue: 0.118))"),
            "chain = {chain}"
        );
        assert!(!chain.contains("( sel )"), "chain = {chain}");
    }

    // ---------------------------------------------------------------------
    // T23 — End-to-end: a small Box+cell triple with selected+editing
    //       state-when props produces the full conditional modifier chain
    //       through `from_pipeline`.
    // ---------------------------------------------------------------------

    #[test]
    fn state_block_end_to_end_from_pipeline_emits_layered_modifiers() {
        let m = component("Cell", vec![], vec![]);
        let l = LayoutDef {
            component_name: "Cell".to_string(),
            root: box_node_with_part_and_props(
                "cell",
                vec![
                    prop_expr(
                        "state-when-selected",
                        "( r == selectedRow && c == selectedCol )",
                    ),
                    prop_expr("state-when-editing", "( r == editRow && c == editCol )"),
                ],
            ),
        };
        let s = style_with_part_and_states(
            "Cell",
            "cell",
            vec![
                sp("padding", "4px"),
                sp("background", "#1e1e1e"),
                sp("color", "#cccccc"),
            ],
            vec![
                (
                    "selected",
                    vec![sp("background", "#264f78"), sp("color", "#ffffff")],
                ),
                ("editing", vec![sp("background", "#1f4f3f")]),
            ],
        );
        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        // Padding (no state override) stays a plain modifier.
        assert!(out.contains(".padding(4)"), "out = {out}");
        // Background carries both layers; editing wraps selected.
        assert!(
            out.contains(
                "(( r == editRow && c == editCol )) ? Color(red: 0.122, green: 0.31, blue: 0.247)"
            ),
            "out = {out}"
        );
        assert!(
            out.contains(
                "(( r == selectedRow && c == selectedCol )) ? Color(red: 0.149, green: 0.31, blue: 0.471)"
            ),
            "out = {out}"
        );
        // Foreground only has the selected layer (editing didn't touch it).
        assert!(
            out.contains(
                ".foregroundColor(((( r == selectedRow && c == selectedCol )) ? Color(red: 1, green: 1, blue: 1) : Color(red: 0.8, green: 0.8, blue: 0.8)))"
            ),
            "out = {out}"
        );
    }

    // ---------------------------------------------------------------------
    // T24 — MSL transition durations and easing curves lower to native
    //       SwiftUI Animation constructors.
    // ---------------------------------------------------------------------

    #[test]
    fn transition_values_lower_to_swiftui_animations() {
        assert_eq!(swiftui_duration_seconds("80ms").as_deref(), Some("0.08"));
        assert_eq!(swiftui_duration_seconds("1s").as_deref(), Some("1.0"));
        assert_eq!(
            swiftui_animation(&transition("opacity", "150ms", "ease-out")).as_deref(),
            Some("Animation.easeOut(duration: 0.15)")
        );
        assert_eq!(
            swiftui_animation(&transition(
                "opacity",
                "300ms",
                "cubic-bezier(0.34, 1.56, 0.64, 1)"
            ))
            .as_deref(),
            Some("Animation.timingCurve(0.34, 1.56, 0.64, 1, duration: 0.3)")
        );
        assert!(swiftui_duration_seconds("fast").is_none());
    }

    // ---------------------------------------------------------------------
    // T25 — A part-level transition watches the final lowered property
    //       expression, so unrelated state changes do not start it.
    // ---------------------------------------------------------------------

    #[test]
    fn base_transition_emits_property_scoped_animation() {
        let s = StyleDef {
            component_name: "Demo".to_string(),
            parts: vec![PartStyle {
                name: "cell".to_string(),
                base: vec![sp("background", "#1e1e1e")],
                transitions: vec![transition("background", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "selected".to_string(),
                    props: vec![sp("background", "#264f78")],
                    transitions: vec![],
                }],
            }],
        };
        let map = build_part_style_map(&s);
        let node =
            box_node_with_part_and_props("cell", vec![prop_expr("state-when-selected", "( sel )")]);
        let base = map.get("cell").expect("base style");
        let layers = collect_state_layers(&node, "cell", &map);
        let chain = swiftui_modifier_chain_with_transitions(
            &base.props,
            &base.transitions,
            &layers,
            0,
            None,
        );
        let property_value = "((( sel )) ? Color(red: 0.149, green: 0.31, blue: 0.471) : Color(red: 0.118, green: 0.118, blue: 0.118))";
        assert!(
            chain.contains(&format!(
                ".animation(Animation.easeOut(duration: 0.08), value: {property_value})"
            )),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T26 — State-local transitions replace the base curve while entering
    //       that state; leaving falls back to the part-level curve.
    // ---------------------------------------------------------------------

    #[test]
    fn state_local_transition_overrides_only_while_entering_state() {
        let s = StyleDef {
            component_name: "Demo".to_string(),
            parts: vec![PartStyle {
                name: "root".to_string(),
                base: vec![sp("opacity", "1")],
                transitions: vec![transition("opacity", "150ms", "ease-out")],
                states: vec![StateStyle {
                    state: "disabled".to_string(),
                    props: vec![sp("opacity", "0.4")],
                    transitions: vec![transition("opacity", "300ms", "linear")],
                }],
            }],
        };
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "root",
            vec![prop_expr("state-when-disabled", "( disabled )")],
        );
        let base = map.get("root").expect("base style");
        let layers = collect_state_layers(&node, "root", &map);
        let chain = swiftui_modifier_chain_with_transitions(
            &base.props,
            &base.transitions,
            &layers,
            0,
            None,
        );
        assert!(
            chain.contains(".opacity(((( disabled )) ? 0.4 : 1))"),
            "chain = {chain}"
        );
        assert!(
            chain.contains(
                ".animation(((( disabled )) ? Animation.linear(duration: 0.3) : Animation.easeOut(duration: 0.15)), value: ((( disabled )) ? 0.4 : 1))"
            ),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T27 — A transition-only state remains addressable and disables
    //       animation on exit when no base transition exists.
    // ---------------------------------------------------------------------

    #[test]
    fn transition_only_state_is_collected_with_nil_exit_animation() {
        let s = StyleDef {
            component_name: "Demo".to_string(),
            parts: vec![PartStyle {
                name: "root".to_string(),
                base: vec![sp("opacity", "1")],
                transitions: vec![],
                states: vec![StateStyle {
                    state: "disabled".to_string(),
                    props: vec![sp("opacity", "0.4")],
                    transitions: vec![transition("opacity", "300ms", "ease-in")],
                }],
            }],
        };
        let map = build_part_style_map(&s);
        let node = box_node_with_part_and_props(
            "root",
            vec![prop_expr("state-when-disabled", "( disabled )")],
        );
        let base = map.get("root").expect("base style");
        let layers = collect_state_layers(&node, "root", &map);
        assert_eq!(layers.len(), 1);
        let chain = swiftui_modifier_chain_with_transitions(
            &base.props,
            &base.transitions,
            &layers,
            0,
            None,
        );
        assert!(
            chain.contains(
                ".animation(((( disabled )) ? Animation.easeIn(duration: 0.3) : nil), value: ((( disabled )) ? 0.4 : 1))"
            ),
            "chain = {chain}"
        );
    }

    // ---------------------------------------------------------------------
    // T28 — End-to-end pipeline output consumes transitions from StyleDef.
    // ---------------------------------------------------------------------

    #[test]
    fn transition_end_to_end_from_pipeline_emits_swiftui_animation() {
        let m = component("Fade", vec![], vec![]);
        let l = LayoutDef {
            component_name: "Fade".to_string(),
            root: box_node_with_part_and_props(
                "root",
                vec![prop_expr("state-when-disabled", "( disabled )")],
            ),
        };
        let s = StyleDef {
            component_name: "Fade".to_string(),
            parts: vec![PartStyle {
                name: "root".to_string(),
                base: vec![sp("opacity", "1")],
                transitions: vec![transition("opacity", "150ms", "ease-out")],
                states: vec![StateStyle {
                    state: "disabled".to_string(),
                    props: vec![sp("opacity", "0.4")],
                    transitions: vec![],
                }],
            }],
        };
        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            out.contains(
                ".animation(Animation.easeOut(duration: 0.15), value: ((( disabled )) ? 0.4 : 1))"
            ),
            "out = {out}"
        );
    }

    // ---------------------------------------------------------------------
    // T29 — UI15's built-in hover state is activated without requiring a
    //       redundant state-when-hover layout predicate.
    // ---------------------------------------------------------------------

    #[test]
    fn built_in_hover_state_emits_row_local_wrapper_and_animation() {
        let m = component("HoverCard", vec![], vec![]);
        let l = LayoutDef {
            component_name: "HoverCard".to_string(),
            root: box_node_with_part_and_props("card", vec![]),
        };
        let s = StyleDef {
            component_name: "HoverCard".to_string(),
            parts: vec![PartStyle {
                name: "card".to_string(),
                base: vec![sp("background", "#ffffff")],
                transitions: vec![transition("background", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "hover".to_string(),
                    props: vec![sp("background", "#e8f0ff")],
                    transitions: vec![],
                }],
            }],
        };

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            out.contains("private struct _MosaicHoverState<Content: View>: View"),
            "out = {out}"
        );
        assert!(
            out.contains("_MosaicHoverState { __mosaicHoverActive in"),
            "out = {out}"
        );
        assert!(
            out.contains(
                ".background(((__mosaicHoverActive) ? Color(red: 0.91, green: 0.941, blue: 1) : Color(red: 1, green: 1, blue: 1)))"
            ),
            "out = {out}"
        );
        assert!(
            out.contains(
                ".animation(Animation.easeOut(duration: 0.08), value: ((__mosaicHoverActive) ? Color(red: 0.91, green: 0.941, blue: 1) : Color(red: 1, green: 1, blue: 1)))"
            ),
            "out = {out}"
        );
    }

    #[test]
    fn built_in_hover_state_is_local_to_each_for_iteration() {
        let m = component(
            "HoverRows",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let mut row = box_node_with_part_and_props("row", vec![]);
        row.children.push(node_with_props(
            "Text",
            vec![prop_keyword("value", "row")],
            vec![],
        ));
        let l = LayoutDef {
            component_name: "HoverRows".to_string(),
            root: node_with_props(
                "For",
                vec![prop_slot_ref("each", "rows"), prop_keyword("as", "row")],
                vec![row],
            ),
        };
        let s = style_with_part_and_states(
            "HoverRows",
            "row",
            vec![sp("opacity", "0.8")],
            vec![("hover", vec![sp("opacity", "1")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        let for_pos = out
            .find("ForEach(rows, id: \\.self) { row in")
            .expect("ForEach");
        let hover_pos = out[for_pos..]
            .find("_MosaicHoverState { __mosaicHoverActive in")
            .expect("row-local hover wrapper");
        assert!(hover_pos > 0, "out = {out}");
        assert!(
            out[for_pos..].contains(".opacity(((__mosaicHoverActive) ? 1 : 0.8))"),
            "out = {out}"
        );
    }

    #[test]
    fn explicit_hover_predicate_remains_author_controlled() {
        let m = component("ManualHover", vec![], vec![]);
        let l = LayoutDef {
            component_name: "ManualHover".to_string(),
            root: box_node_with_part_and_props(
                "root",
                vec![prop_expr("state-when-hover", "( forceHover )")],
            ),
        };
        let s = style_with_part_and_states(
            "ManualHover",
            "root",
            vec![sp("opacity", "0.8")],
            vec![("hover", vec![sp("opacity", "1")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            !out.contains("_MosaicHoverState"),
            "explicit predicate should not install native hover tracking:\n{out}"
        );
        assert!(
            out.contains(".opacity(((( forceHover )) ? 1 : 0.8))"),
            "out = {out}"
        );
    }

    // ---------------------------------------------------------------------
    // T30 — UI15's built-in pressed state is activated by row-local native
    //       SwiftUI gesture state for pressable host controls.
    // ---------------------------------------------------------------------

    #[test]
    fn built_in_pressed_state_emits_native_press_wrapper_and_animation() {
        let m = component("PressedButton", vec![], vec![]);
        let l = LayoutDef {
            component_name: "PressedButton".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: Some("button".to_string()),
                props: vec![prop_string("label", "Save")],
                children: vec![],
            },
        };
        let s = StyleDef {
            component_name: "PressedButton".to_string(),
            parts: vec![PartStyle {
                name: "button".to_string(),
                base: vec![sp("opacity", "1")],
                transitions: vec![transition("opacity", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "pressed".to_string(),
                    props: vec![sp("opacity", "0.7")],
                    transitions: vec![],
                }],
            }],
        };

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            out.contains("private struct _MosaicPressState<Content: View>: View"),
            "out = {out}"
        );
        assert!(
            out.contains("@GestureState private var isPressed = false"),
            "out = {out}"
        );
        assert!(
            out.contains("_MosaicPressState { __mosaicPressActive in"),
            "out = {out}"
        );
        assert!(
            out.contains("DragGesture(minimumDistance: 0)"),
            "out = {out}"
        );
        assert!(
            out.contains(".opacity(((__mosaicPressActive) ? 0.7 : 1))"),
            "out = {out}"
        );
    }

    #[test]
    fn explicit_pressed_predicate_remains_author_controlled() {
        let m = component("ManualPress", vec![], vec![]);
        let l = LayoutDef {
            component_name: "ManualPress".to_string(),
            root: LayoutNode {
                tag: "HostButton".to_string(),
                part_name: Some("button".to_string()),
                props: vec![
                    prop_string("label", "Save"),
                    prop_expr("state-when-pressed", "( forcePress )"),
                ],
                children: vec![],
            },
        };
        let s = style_with_part_and_states(
            "ManualPress",
            "button",
            vec![sp("opacity", "1")],
            vec![("pressed", vec![sp("opacity", "0.7")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            !out.contains("_MosaicPressState"),
            "explicit predicate should not install native press tracking:\n{out}"
        );
        assert!(out.contains("forcePress"), "out = {out}");
    }

    #[test]
    fn non_pressable_layout_node_does_not_install_press_tracking() {
        let m = component("DecorativePress", vec![], vec![]);
        let l = LayoutDef {
            component_name: "DecorativePress".to_string(),
            root: box_node_with_part_and_props("panel", vec![]),
        };
        let s = style_with_part_and_states(
            "DecorativePress",
            "panel",
            vec![sp("opacity", "1")],
            vec![("pressed", vec![sp("opacity", "0.7")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            !out.contains("_MosaicPressState"),
            "non-pressable layout nodes must not gain native press tracking:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // T31 — UI15's built-in focused state is activated by native SwiftUI
    //       focus for focus-capable host controls.
    // ---------------------------------------------------------------------

    #[test]
    fn built_in_focused_state_emits_native_focus_wrapper_and_animation() {
        let m = component("FocusField", vec![], vec![]);
        let l = LayoutDef {
            component_name: "FocusField".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: Some("field".to_string()),
                props: vec![prop_string("placeholder", "Search")],
                children: vec![],
            },
        };
        let s = StyleDef {
            component_name: "FocusField".to_string(),
            parts: vec![PartStyle {
                name: "field".to_string(),
                base: vec![sp("border-width", "1"), sp("border-color", "#d0d0d0")],
                transitions: vec![transition("border-color", "80ms", "ease-out")],
                states: vec![StateStyle {
                    state: "focused".to_string(),
                    props: vec![sp("border-color", "#e0942a")],
                    transitions: vec![],
                }],
            }],
        };

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            out.contains("private struct _MosaicFocusState<Content: View>: View"),
            "out = {out}"
        );
        assert!(
            out.contains("@FocusState private var isFocused: Bool"),
            "out = {out}"
        );
        assert!(
            out.contains("_MosaicFocusState { __mosaicFocusActive in"),
            "out = {out}"
        );
        assert!(out.contains(".focused($isFocused)"), "out = {out}");
        assert!(
            out.contains("__mosaicFocusActive")
                && out.contains("Animation.easeOut(duration: 0.08)"),
            "out = {out}"
        );
    }

    #[test]
    fn built_in_focused_state_is_local_to_each_for_iteration() {
        let m = component(
            "FocusRows",
            vec![slot(
                "rows",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![],
        );
        let field = LayoutNode {
            tag: "HostInput".to_string(),
            part_name: Some("field".to_string()),
            props: vec![
                prop_expr("value", "( row )"),
                prop_string("placeholder", "Edit"),
            ],
            children: vec![],
        };
        let l = LayoutDef {
            component_name: "FocusRows".to_string(),
            root: node_with_props(
                "For",
                vec![prop_slot_ref("each", "rows"), prop_keyword("as", "row")],
                vec![field],
            ),
        };
        let s = style_with_part_and_states(
            "FocusRows",
            "field",
            vec![sp("border-color", "#d0d0d0")],
            vec![("focused", vec![sp("border-color", "#e0942a")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        let for_pos = out
            .find("ForEach(rows, id: \\.self) { row in")
            .expect("ForEach");
        let focus_pos = out[for_pos..]
            .find("_MosaicFocusState { __mosaicFocusActive in")
            .expect("row-local focus wrapper");
        assert!(focus_pos > 0, "out = {out}");
        assert!(out[for_pos..].contains("TextField(\"Edit\""), "out = {out}");
    }

    #[test]
    fn explicit_focused_predicate_remains_author_controlled() {
        let m = component("ManualFocus", vec![], vec![]);
        let l = LayoutDef {
            component_name: "ManualFocus".to_string(),
            root: LayoutNode {
                tag: "HostInput".to_string(),
                part_name: Some("field".to_string()),
                props: vec![
                    prop_string("placeholder", "Search"),
                    prop_expr("state-when-focused", "( forceFocus )"),
                ],
                children: vec![],
            },
        };
        let s = style_with_part_and_states(
            "ManualFocus",
            "field",
            vec![sp("border-width", "1"), sp("border-color", "#d0d0d0")],
            vec![("focused", vec![sp("border-color", "#e0942a")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            !out.contains("_MosaicFocusState"),
            "explicit predicate should not install native focus tracking:\n{out}"
        );
        assert!(out.contains("forceFocus"), "out = {out}");
    }

    #[test]
    fn non_focusable_layout_node_does_not_install_focus_tracking() {
        let m = component("DecorativeFocus", vec![], vec![]);
        let l = LayoutDef {
            component_name: "DecorativeFocus".to_string(),
            root: box_node_with_part_and_props("panel", vec![]),
        };
        let s = style_with_part_and_states(
            "DecorativeFocus",
            "panel",
            vec![sp("border-color", "#d0d0d0")],
            vec![("focused", vec![sp("border-color", "#e0942a")])],
        );

        let out = from_pipeline(&m, &l, &s).expect("emit ok").output;
        assert!(
            !out.contains("_MosaicFocusState"),
            "non-focusable layout nodes must not gain native focus:\n{out}"
        );
    }

    // =====================================================================
    // HostTable column-widths threading tests
    //
    // These tests pin down the contract of [`TableContext`] +
    // [`extract_table_context`] + the cell-position width threading in
    // [`emit_for_swift`]:
    //
    //   1. HostTable without a HostTableColGroup → no width threading,
    //      cells stay text-sized (but rows still use spacing: 0).
    //   2. HostTableColGroup with the canonical `For (each: slot:
    //      column-widths) { Col }` shape → context captured, body Fors
    //      with `index:` get `.frame(width: columnWidths[Int(<idx>)])`.
    //   3. Body For with NO index binding → no width modifier (the
    //      index is what indexes into the slot).
    //   4. Header For inside HostTableHead → cells get the width
    //      modifier too (header's `ch` is the column index).
    //   5. End-to-end smoke against the Grid.mll shape → both
    //      `HStack(spacing: 0)` AND `.frame(width: columnWidths[...])`
    //      appear in the output.
    //   6. HStack outside HostTable (i.e. a `Row` container) still uses
    //      default spacing (regression guard).
    //   7. Variable spacing inside non-table Rows is unaffected by the
    //      threading change (regression guard).
    //   8. A HostTableColGroup with a non-canonical For body (no `Col`)
    //      falls back to no threading.
    // =====================================================================

    /// Helper: build a `HostTableColGroup { For (each: slot: <widths>,
    /// as: w, index: cw) { Col [col] (width: (w)) } }` block matching
    /// the `mosaic-pkg-grid::Grid.mll` shape.
    fn col_group_widths(slot: &str) -> LayoutNode {
        let col_node = LayoutNode {
            tag: "Col".to_string(),
            part_name: Some("col".to_string()),
            props: vec![prop_expr("width", "w")],
            children: Vec::new(),
        };
        let for_node = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", slot),
                prop_keyword("as", "w"),
                prop_keyword("index", "cw"),
            ],
            vec![col_node],
        );
        container_node("HostTableColGroup", vec![for_node])
    }

    /// Helper: build a HostTableBody whose only child is a For over a
    /// slot, with the given index binding name.  Each iteration emits a
    /// `Row [data-row]` whose only child is a For over the row binding
    /// — i.e. the canonical Grid.mll shape but with a configurable
    /// inner-index name and a configurable body leaf (so we can drop
    /// out the inner For to exercise the no-index path).
    fn body_for_rows_then_inner_for(outer_index: &str, inner_index: Option<&str>) -> LayoutNode {
        let leaf = leaf("Text", vec![prop_slot_ref("content", "v")]);
        let inner_for_props = match inner_index {
            Some(idx) => vec![
                LayoutProp {
                    name: "each".to_string(),
                    value: LayoutPropValue::Keyword("row".to_string()),
                },
                prop_keyword("as", "v"),
                prop_keyword("index", idx),
            ],
            None => vec![
                LayoutProp {
                    name: "each".to_string(),
                    value: LayoutPropValue::Keyword("row".to_string()),
                },
                prop_keyword("as", "v"),
            ],
        };
        let inner_for = node_with_props("For", inner_for_props, vec![leaf]);
        let row = LayoutNode {
            tag: "Row".to_string(),
            part_name: Some("data-row".to_string()),
            props: Vec::new(),
            children: vec![inner_for],
        };
        let outer_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "viewport-rows"),
                prop_keyword("as", "row"),
                prop_keyword("index", outer_index),
            ],
            vec![row],
        );
        container_node("HostTableBody", vec![outer_for])
    }

    // -----------------------------------------------------------------
    // Test W1 — HostTable WITHOUT a HostTableColGroup: no column-width
    // threading, but rows still use `HStack(spacing: 0)`.
    //
    // The fall-back path: cells render at their natural text-content
    // width and the visible diff vs pre-PR is only the row spacing.
    // -----------------------------------------------------------------

    #[test]
    fn host_table_without_col_group_does_not_thread_widths() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![table_section("HostTableBody", vec![vec!["a", "b"]])],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // spacing: 0 still applies (HostTable always uses it).
        assert!(
            out.contains("HStack(spacing: 0) {"),
            "expected spacing: 0 even without ColGroup, got:\n{out}"
        );
        // No `.frame(width:` modifier anywhere — there's no slot to
        // address.
        assert!(
            !out.contains(".frame(width:"),
            "expected NO width threading without ColGroup, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W2 — HostTableColGroup with the canonical For-Col shape is
    // detected by [`extract_table_context`] and the slot name flows
    // into a body cell's `.frame(width: columnWidths[Int(c)])`.
    // -----------------------------------------------------------------

    #[test]
    fn host_table_col_group_canonical_shape_threads_widths_into_body_cells() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        col_group_widths("column-widths"),
                        body_for_rows_then_inner_for("r", Some("c")),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // The inner For (`index: c`) injects the column width INTO the
        // cell's frame.  The cell body here is a bare `Text` (no part
        // style), so the standalone-frame fallback fires and emits
        // `.frame(width: columnWidths[Int(c)], alignment: .center)`.  We
        // match the width prefix (not the closing paren) so the
        // alignment argument doesn't break the assertion.
        assert!(
            out.contains(".frame(width: columnWidths[Int(c)]"),
            "expected width threading on body inner For, got:\n{out}"
        );
        // The OUTER For (`index: r`, iterates rows) must NOT get a
        // width modifier — `r` is a row index, not a column index.
        // Only the inner-For (cell-position) variant of `.frame(width:
        // ...)` may appear.
        assert!(
            !out.contains(".frame(width: columnWidths[Int(r)])"),
            "expected NO width threading on outer row For, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W3 — Body For WITHOUT an `index:` binding falls through to
    // no-width threading.  The slot indexer needs an index expression;
    // an as-only For has no column index to feed it.
    // -----------------------------------------------------------------

    #[test]
    fn body_for_without_index_skips_width_threading() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        col_group_widths("column-widths"),
                        // Inner For has no `index:` binding.
                        body_for_rows_then_inner_for("r", None),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // ColGroup is present and the slot was extracted, but the body
        // For lacks `index:` so no width modifier fires.
        assert!(
            !out.contains(".frame(width: columnWidths"),
            "expected NO width threading when For lacks `index:`, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W4 — Header For inside HostTableHead also gets width
    // threading.  The header's `ch` binding IS a column index
    // (mosaic-pkg-grid::Grid.mll convention), so column-widths[Int(ch)]
    // is the right address.
    // -----------------------------------------------------------------

    #[test]
    fn header_for_with_index_threads_width_into_header_cells() {
        let header_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "column-headers"),
                prop_keyword("as", "h"),
                prop_keyword("index", "ch"),
            ],
            vec![leaf("Text", vec![prop_slot_ref("content", "h")])],
        );
        let header_row = LayoutNode {
            tag: "Row".to_string(),
            part_name: Some("header-row".to_string()),
            props: Vec::new(),
            children: vec![header_for],
        };
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        col_group_widths("column-widths"),
                        container_node("HostTableHead", vec![header_row]),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            out.contains(".frame(width: columnWidths[Int(ch)]"),
            "expected width threading on header For, got:\n{out}"
        );
        // The header text still gets `.bold()` — width threading does
        // not interfere with the existing bolding path.
        assert!(
            out.contains(".bold()"),
            "expected header bold still applied, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W5 — End-to-end smoke: the full Grid.mll shape (header For +
    // col-group For-Col + body For-Row-For) produces BOTH
    // `HStack(spacing: 0)` AND `.frame(width: columnWidths[Int(<idx>)])`
    // in the output.
    // -----------------------------------------------------------------

    #[test]
    fn host_table_full_grid_shape_emits_spacing_zero_and_width_threading() {
        let header_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "column-headers"),
                prop_keyword("as", "h"),
                prop_keyword("index", "ch"),
            ],
            vec![leaf("Text", vec![prop_slot_ref("content", "h")])],
        );
        let header_row = LayoutNode {
            tag: "Row".to_string(),
            part_name: Some("header-row".to_string()),
            props: Vec::new(),
            children: vec![header_for],
        };
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        col_group_widths("column-widths"),
                        container_node("HostTableHead", vec![header_row]),
                        body_for_rows_then_inner_for("r", Some("c")),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // Spacing-zero on every HostTable row.
        assert!(
            out.contains("HStack(spacing: 0) {"),
            "expected spacing: 0 HStack, got:\n{out}"
        );
        // Width threading on both header and body cells (the bare-Text
        // cells take the standalone-frame fallback, so the frame also
        // carries an `alignment:` argument — match the width prefix).
        assert!(
            out.contains(".frame(width: columnWidths[Int(ch)]"),
            "expected header width threading, got:\n{out}"
        );
        assert!(
            out.contains(".frame(width: columnWidths[Int(c)]"),
            "expected body width threading, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W6 — Regression guard: an `Row` container OUTSIDE any
    // HostTable still lowers to a plain `HStack {` with SwiftUI's
    // default 8pt spacing.  The spacing: 0 edit must NOT leak into
    // ordinary Row users (a freestanding toolbar, a Box-of-Rows, etc.).
    // -----------------------------------------------------------------

    #[test]
    fn row_outside_host_table_keeps_default_spacing() {
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "Row",
                    vec![
                        leaf("Text", vec![prop_string("content", "left")]),
                        leaf("Text", vec![prop_string("content", "right")]),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // A plain `HStack {` (default spacing) must appear; a
        // `HStack(spacing: 0)` opener must NOT appear (there's no
        // HostTable in scope to trigger the override).
        assert!(
            out.contains("HStack {\n"),
            "expected default-spacing HStack opener, got:\n{out}"
        );
        assert!(
            !out.contains("HStack(spacing: 0)"),
            "spacing: 0 must NOT leak into non-HostTable Rows, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W7 — Regression guard: a `Row` nested inside a `Box` that
    // is itself NOT under any HostTable must also keep default
    // spacing.  Exercises the recursion path — the absence of
    // `table_ctx` should propagate down through container nesting.
    // -----------------------------------------------------------------

    #[test]
    fn nested_row_outside_host_table_keeps_default_spacing() {
        let inner_row =
            container_node("Row", vec![leaf("Text", vec![prop_string("content", "x")])]);
        let layout = layout_with(
            "T",
            container_node("Box", vec![container_node("Box", vec![inner_row])]),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        assert!(
            out.contains("HStack {\n"),
            "expected default-spacing HStack opener, got:\n{out}"
        );
        assert!(
            !out.contains("HStack(spacing: 0)"),
            "nested non-table Row must keep default spacing, got:\n{out}"
        );
        assert!(
            !out.contains(".frame(width:"),
            "no HostTable in scope → no width threading, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W8 — HostTableColGroup with a malformed For body (no `Col`)
    // falls back to no width threading.  The structural match is
    // permissive (any Col counts) but we require *some* Col to be
    // present; absent that, `column_widths_slot` stays None and only
    // the spacing: 0 edit takes effect.
    // -----------------------------------------------------------------

    #[test]
    fn col_group_without_col_body_falls_back_to_no_threading() {
        // ColGroup contains a For whose body is a Text (not Col).
        // The structural match should reject this.
        let malformed_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "column-widths"),
                prop_keyword("as", "w"),
                prop_keyword("index", "cw"),
            ],
            vec![leaf("Text", vec![prop_string("content", "x")])],
        );
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![
                        container_node("HostTableColGroup", vec![malformed_for]),
                        body_for_rows_then_inner_for("r", Some("c")),
                    ],
                )],
            ),
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &empty_style("T"))
            .unwrap()
            .output;
        // No width threading — the ColGroup's For did not contain a Col.
        assert!(
            !out.contains(".frame(width: columnWidths"),
            "expected NO width threading with malformed ColGroup, got:\n{out}"
        );
        // Spacing: 0 still fires (HostTable is present).
        assert!(
            out.contains("HStack(spacing: 0) {"),
            "expected spacing: 0 HStack even with malformed ColGroup, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W9 — Injected width on a cell that HAS a `cell` part with
    // height + border (the visicalc shape).  The threaded column width
    // must merge into the part's OWN frame and land BEFORE
    // `.background`/`.border`, with NO trailing standalone
    // `.frame(width:)` after the border.  This is the headline cell-fill
    // bug-fix assertion at the end-to-end (`from_pipeline`) level.
    // -----------------------------------------------------------------

    #[test]
    fn injected_width_merges_into_cell_part_frame_before_paint_e2e() {
        // Body For whose iteration body is a `Box [cell]` (a styled cell,
        // not a bare Text), inside a HostTable with a ColGroup so the
        // width thread is live.
        let cell_box = LayoutNode {
            tag: "Box".to_string(),
            part_name: Some("cell".to_string()),
            props: Vec::new(),
            children: vec![leaf("Text", vec![prop_slot_ref("content", "v")])],
        };
        let inner_for = node_with_props(
            "For",
            vec![
                LayoutProp {
                    name: "each".to_string(),
                    value: LayoutPropValue::Keyword("row".to_string()),
                },
                prop_keyword("as", "v"),
                prop_keyword("index", "c"),
            ],
            vec![cell_box],
        );
        let row = LayoutNode {
            tag: "Row".to_string(),
            part_name: Some("data-row".to_string()),
            props: Vec::new(),
            children: vec![inner_for],
        };
        let outer_for = node_with_props(
            "For",
            vec![
                prop_slot_ref("each", "viewport-rows"),
                prop_keyword("as", "row"),
                prop_keyword("index", "r"),
            ],
            vec![row],
        );
        let body = container_node("HostTableBody", vec![outer_for]);
        let layout = layout_with(
            "T",
            container_node(
                "Box",
                vec![container_node(
                    "HostTable",
                    vec![col_group_widths("column-widths"), body],
                )],
            ),
        );
        // `cell` part: padding + height + border + right-align background.
        let style = style_with_part(
            "T",
            "cell",
            vec![
                sp("padding", "2px"),
                sp("height", "22px"),
                sp("text-align", "right"),
                sp("background", "#1e1e1e"),
                sp("border-width", "1px"),
            ],
        );
        let out = from_pipeline(&component("T", vec![], vec![]), &layout, &style)
            .unwrap()
            .output;

        // A SINGLE merged frame carrying the injected width + the part's
        // own height + the base text-align alignment.
        assert!(
            out.contains(".frame(width: columnWidths[Int(c)], height: 22, alignment: .trailing)"),
            "expected merged width+height+alignment frame, got:\n{out}"
        );
        // The merged frame precedes both `.background` and `.border`.
        let frame_at = out.find(".frame(width: columnWidths[Int(c)]").unwrap();
        let bg_at = out.find(".background(").unwrap();
        let border_at = out.find(".border(").unwrap();
        assert!(
            frame_at < bg_at && frame_at < border_at,
            "frame must precede background+border, got:\n{out}"
        );
        // Exactly ONE injected-width frame — no trailing
        // `.frame(width: columnWidths...)` after the border (the old,
        // buggy placement).  We match the `columnWidths` indexer rather
        // than a bare `.frame(width:` so the explanatory ColGroup
        // comment line (which mentions `.frame(width:)`) is not counted.
        assert_eq!(
            out.matches(".frame(width: columnWidths").count(),
            1,
            "expected exactly one injected-width frame, got:\n{out}"
        );
    }

    // -----------------------------------------------------------------
    // Test W10 — Injected width on a cell with NO part style.  The
    // standalone-frame fallback must still honor the threaded column
    // width (with a neutral `.center` alignment).
    // -----------------------------------------------------------------

    #[test]
    fn injected_width_on_unstyled_cell_emits_standalone_frame() {
        // The body cell is a bare `Text` — no part_name, so the part
        // chain never runs; the fallback must emit the frame anyway.
        let out = from_pipeline(
            &component("T", vec![], vec![]),
            &layout_with(
                "T",
                container_node(
                    "Box",
                    vec![container_node(
                        "HostTable",
                        vec![
                            col_group_widths("column-widths"),
                            body_for_rows_then_inner_for("r", Some("c")),
                        ],
                    )],
                ),
            ),
            &empty_style("T"),
        )
        .unwrap()
        .output;
        assert!(
            out.contains(".frame(width: columnWidths[Int(c)], alignment: .center)"),
            "expected standalone width frame on unstyled cell, got:\n{out}"
        );
    }
}
