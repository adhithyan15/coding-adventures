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
//! | `HostDraggable`      | native `Draggable<_MosaicDragData>` + keyboard/semantics controller |
//! | `HostDropTarget`     | native `DragTarget<_MosaicDragData>` + scoped target registry |
//! | `HostLink`           | `InkWell(onTap: () => launchUrl(...), child: Text(...))` (UI29-4) |
//! | `HostTooltip`        | `Tooltip(message: ..., child: ...)` (UI29-4)        |
//! | `HostNumberInput`    | `TextField(keyboardType: TextInputType.number, ...)` (UI29-4) |
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

use moslayout_compiler::{LayoutDef, LayoutNode, LayoutProp, LayoutPropValue};
use mosmodel_compiler::{
    EmitDecl, EmitPayloadType, ListInnerType, MosmodelComponent, SlotDecl, SlotDefault, SlotType,
};
#[cfg(test)]
use mosstyle_compiler::PartStyle;
use mosstyle_compiler::{StyleDef, StyleProp};

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
    ComponentNameMismatch { mosmodel: String, moslayout: String },
    UnsafeSlotName(String),
    UnsafeEmitName(String),
    UnknownPrimitive(String),
    /// #13052: a `HostLink.href` literal uses an explicit URI scheme
    /// outside the `http`/`https`/`mailto` allowlist, checked when
    /// `external` is not `false` (the path a real `launchUrl` call
    /// will eventually use -- currently still a `/* TODO: launchUrl(...)
    /// */` comment, but this is checked preventatively ahead of that
    /// landing rather than waiting for it to become a live gap).
    UnsafeUriScheme(String),
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
                "moslayout primitive '{t}' is not yet supported by the Flutter pipeline emitter"
            ),
            PipelineEmitError::UnsafeUriScheme(href) => write!(
                f,
                "HostLink href {href:?} does not use an allowed URI scheme (http, https, mailto)"
            ),
        }
    }
}

impl std::error::Error for PipelineEmitError {}

// =====================================================================
// UI32-K-flutter — `--emit-project` Flutter app shell
//
// Mirrors L2 (React, PR #4297), L3 (HTML, PR #4309), L4
// (WebComponent, PR #4315): EmitOptions / ProjectFiles /
// from_pipeline_with_options.
//
// When `--emit-project` is on, emits a flutter-create-shaped
// scaffold alongside the component .dart. Author runs
// `flutter pub get && flutter run -d <device>` to see the
// component on a connected simulator/emulator/desktop window.
// =====================================================================

/// Options controlling the Flutter emitter's behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Also emit `pubspec.yaml`, `analysis_options.yaml`, `lib/main.dart`,
    /// `test/widget_test.dart`, and `README.md`
    /// alongside the component `.dart` file. Default `false`.
    pub emit_project: bool,

    /// Make the generated application shell require Mosaic's standard Rust
    /// runtime and runtime-provided props. This removes the preview/sample
    /// paths but does not change the reusable component artifact. Default
    /// `false` preserves permissive standalone emission.
    pub require_runtime: bool,

    /// Pinned Flutter SDK constraint to write into
    /// `pubspec.yaml`'s `environment.flutter`. UI32 spec §3.6.3
    /// requires exact pinning. Default `">=3.32.0 <4.0.0"` — the first
    /// stable Flutter release whose native Table exposes explicit
    /// table/row/cell semantics roles.
    /// Caret-pinning is not idiomatic for Flutter SDK constraints
    /// (which use range syntax), so this is the closest exact
    /// equivalent.
    pub pinned_flutter_sdk: String,

    /// Pinned Dart SDK constraint to write into
    /// `pubspec.yaml`'s `environment.sdk`. Default
    /// `">=3.5.0 <4.0.0"` — compatible with the Flutter 3.32 floor.
    pub pinned_dart_sdk: String,

    /// Pubspec package name to write into `pubspec.yaml` `name:`.
    /// If `None`, derived from the component name by kebab→snake
    /// casing and prefixing `mosaic_` (Dart pub requires snake_case
    /// per §3.6.2 Flutter row; the prefix avoids collisions with
    /// Dart's own packages).
    pub package_name: Option<String>,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            emit_project: false,
            require_runtime: false,
            pinned_flutter_sdk: ">=3.32.0 <4.0.0".to_string(),
            pinned_dart_sdk: ">=3.5.0 <4.0.0".to_string(),
            package_name: None,
        }
    }
}

/// Project-shaped artifacts emitted when `EmitOptions::emit_project`
/// is on. These files are enough for `flutter create`, `flutter analyze`,
/// `flutter test`, and `flutter run` to operate without retaining the stock
/// counter-app test or a lint include that is absent from the pubspec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFiles {
    /// `pubspec.yaml` — pinned Flutter + Dart SDK constraints,
    /// `flutter:` block with the SDK dep. Package name follows
    /// Dart pub rules (snake_case).
    pub pubspec_yaml: String,
    /// `analysis_options.yaml` — the standard Flutter lint baseline with
    /// generated-code warnings kept visible to `flutter analyze`.
    pub analysis_options_yaml: String,
    /// `lib/main.dart` — `MaterialApp` shell that mounts the
    /// component as the `home:` widget. Imports the component
    /// sibling-relative from the project root.
    pub main_dart: String,
    /// `lib/mosaic_host.dart` — default no-op host hook. App packages
    /// can overwrite this file with a backend-specific bridge via
    /// manifest host assets while the generated shell remains runnable
    /// without a host installed.
    pub mosaic_host_dart: String,
    /// `test/widget_test.dart` — package-name-correct smoke coverage for the
    /// generated Mosaic application shell. Permissive shells mount the app;
    /// runtime-required shells verify the exported widget type without trying
    /// to load a target native library in the test process.
    pub widget_test_dart: String,
    /// `README.md` — prereqs (Flutter SDK), `flutter pub get` +
    /// `flutter run` commands, file map.
    pub readme: String,
}

/// Error shapes specific to the project-shell emission path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectShellError {
    /// The derived Dart-pub name fails the Dart pub naming
    /// convention: lowercase letters/digits/underscores,
    /// MUST start with a letter, no leading underscore.
    /// Per UI32 spec §3.6.2 Flutter row.
    InvalidDartPubName(String),
}

impl std::fmt::Display for ProjectShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectShellError::InvalidDartPubName(n) => write!(
                f,
                "derived Dart pub name '{n}' violates the pub naming convention (snake_case: lowercase + digits + underscores, must start with letter)"
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

/// Extended pipeline result — same as `PipelineEmitResult` but
/// carries the optional `ProjectFiles` when `emit_project` is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEmitResultWithProject {
    pub output: String,
    pub component_name: String,
    pub project: Option<ProjectFiles>,
}

/// Compile a three-file Mosaic pipeline triple to Dart with
/// explicit emit options.
pub fn from_pipeline_with_options(
    interface: &MosmodelComponent,
    layout: &LayoutDef,
    style: &StyleDef,
    options: &EmitOptions,
) -> Result<PipelineEmitResultWithProject, PipelineEmitError> {
    let component = from_pipeline(interface, layout, style)?;

    let project = if options.emit_project {
        Some(build_flutter_project_files(interface, options)?)
    } else {
        None
    };

    Ok(PipelineEmitResultWithProject {
        output: component.output,
        component_name: component.component_name,
        project,
    })
}

/// Build the three Flutter app-shell side files for a single
/// component.
fn build_flutter_project_files(
    interface: &MosmodelComponent,
    options: &EmitOptions,
) -> Result<ProjectFiles, ProjectShellError> {
    let name = &interface.component;
    let pub_name = match &options.package_name {
        Some(p) => p.clone(),
        None => format!("mosaic_{}", pascal_to_snake_for_pub(name)),
    };
    if !is_valid_dart_pub_name(&pub_name) {
        return Err(ProjectShellError::InvalidDartPubName(pub_name));
    }

    Ok(ProjectFiles {
        pubspec_yaml: build_pubspec_yaml(&pub_name, options),
        analysis_options_yaml: build_analysis_options_yaml(),
        main_dart: build_main_dart(name, &interface.slots, options.require_runtime),
        mosaic_host_dart: build_mosaic_host_dart(options.require_runtime),
        widget_test_dart: build_widget_test_dart(&pub_name, options.require_runtime),
        readme: format!(
            "{}\n## Analyze and test\n\nMosaic supplies `analysis_options.yaml`, the matching `flutter_lints` dependency, and `test/widget_test.dart` before Flutter creates platform runners. The bootstrap therefore preserves a package-name-correct smoke test instead of installing the stock counter-app test.\n\n```sh\nflutter analyze\nflutter test\n```\n",
            build_flutter_readme(&pub_name, name, options.require_runtime)
        ),
    })
}

/// PascalCase → snake_case for Dart pub naming. `Hello` → `hello`;
/// `ProfileCard` → `profile_card`.
fn pascal_to_snake_for_pub(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        for d in c.to_lowercase() {
            out.push(d);
        }
    }
    out
}

/// Validate Dart pub name per §3.6.2 Flutter row:
/// `[a-z][a-z0-9_]*` (lowercase, digits, underscores; must start
/// with letter; no leading underscore). Dart pub rejects names
/// with hyphens, uppercase, or leading digit/underscore.
fn is_valid_dart_pub_name(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

const BANNER_DART: &str = "// AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n// Fork the file (remove this banner) to customise.\n";
const BANNER_YAML: &str = "# AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit.\n# Fork the file (remove this banner) to customise.\n";
const BANNER_MD: &str = "<!-- AUTO-GENERATED by mosaic-compile --emit-project. Edits will be overwritten on next emit. -->\n<!-- Fork the file (remove this banner) to customise. -->\n";

fn build_pubspec_yaml(pub_name: &str, options: &EmitOptions) -> String {
    format!(
        "{BANNER_YAML}name: {pub_name}\ndescription: Auto-generated Flutter shell for a Mosaic component.\npublish_to: 'none'\nversion: 0.0.0\n\nenvironment:\n  sdk: '{}'\n  flutter: '{}'\n\ndependencies:\n  flutter:\n    sdk: flutter\n\ndev_dependencies:\n  flutter_lints: '>=6.0.0 <7.0.0'\n  flutter_test:\n    sdk: flutter\n\nflutter:\n  uses-material-design: true\n",
        options.pinned_dart_sdk, options.pinned_flutter_sdk,
    )
}

fn build_analysis_options_yaml() -> String {
    format!(
        "{BANNER_YAML}include: package:flutter_lints/flutter.yaml\n\nanalyzer:\n  errors:\n    unused_import: error\n    unused_local_variable: error\n\nlinter:\n  rules:\n    # Mosaic component artifacts retain their PascalCase component identity.\n    file_names: false\n\n"
    )
}

fn build_widget_test_dart(pub_name: &str, require_runtime: bool) -> String {
    if require_runtime {
        return format!(
            "{BANNER_DART}import 'package:flutter_test/flutter_test.dart';\nimport 'package:{pub_name}/main.dart';\n\nvoid main() {{\n  test('exports the generated Mosaic app widget', () {{\n    expect(MosaicApp, isNotNull);\n  }});\n}}\n"
        );
    }

    format!(
        "{BANNER_DART}import 'package:flutter/material.dart';\nimport 'package:flutter_test/flutter_test.dart';\nimport 'package:{pub_name}/main.dart';\n\nvoid main() {{\n  testWidgets('mounts the generated Mosaic app shell', (tester) async {{\n    await tester.pumpWidget(const MosaicApp());\n    await tester.pump();\n\n    expect(find.byType(MaterialApp), findsOneWidget);\n  }});\n}}\n"
    )
}

fn build_main_dart(component_name: &str, slots: &[SlotDecl], require_runtime: bool) -> String {
    if require_runtime {
        return build_runtime_required_main_dart(component_name, slots);
    }
    build_permissive_main_dart(component_name, slots)
}

fn build_permissive_main_dart(component_name: &str, slots: &[SlotDecl]) -> String {
    let root_widget = build_root_widget_constructor(component_name, slots);
    let host_props_field = if slots.is_empty() {
        String::new()
    } else {
        "  Map<String, Object?> _hostProps = const <String, Object?>{};\n".to_string()
    };
    let apply_host_props = if slots.is_empty() {
        String::new()
    } else {
        concat!(
            "    final nextProps = mosaicMap(response['props']);\n",
            "    if (nextProps.isNotEmpty) {\n",
            "      setState(() {\n",
            "        _hostProps = nextProps;\n",
            "      });\n",
            "    }\n"
        )
        .to_string()
    };
    format!(
        concat!(
            "{banner}",
            "import 'dart:async';\n",
            "import 'package:flutter/material.dart';\n",
            "import '{component_name}.dart';\n",
            "import 'mosaic_host.dart';\n\n",
            "void main() {{\n",
            "  runApp(const MosaicApp());\n",
            "}}\n\n",
            "class MosaicApp extends StatefulWidget {{\n",
            "  const MosaicApp({{super.key, this.mosaicHost}});\n\n",
            "  final MosaicHost? mosaicHost;\n\n",
            "  @override\n",
            "  State<MosaicApp> createState() => _MosaicAppState();\n",
            "}}\n\n",
            "class _MosaicAppState extends State<MosaicApp> {{\n",
            "  late final MosaicHost? _mosaicHost;\n",
            "{host_props_field}\n",
            "  @override\n",
            "  void initState() {{\n",
            "    super.initState();\n",
            "    _mosaicHost = widget.mosaicHost ?? MosaicHost.load();\n",
            "    _mosaicHost?.setPropsChangedHandler(() =>\n",
            "        _queueMosaicResponse(_mosaicHost.props()));\n",
            "    _queueMosaicResponse(_mosaicHost?.props());\n",
            "  }}\n\n",
            "  @override\n",
            "  void dispose() {{\n",
            "    _mosaicHost?.dispose();\n",
            "    super.dispose();\n",
            "  }}\n\n",
            "  void _queueMosaicResponse(\n",
            "    FutureOr<Map<String, Object?>?>? responseOrFuture,\n",
            "  ) {{\n",
            "    if (responseOrFuture == null) return;\n",
            "    Future<Map<String, Object?>?>.value(responseOrFuture)\n",
            "        .then(_applyMosaicResponse)\n",
            "        .catchError((Object error) {{\n",
            "      debugPrint('host error: $error');\n",
            "    }});\n",
            "  }}\n\n",
            "  void _applyMosaicResponse(Map<String, Object?>? response) {{\n",
            "    if (response == null) return;\n",
            "    if (!mounted) return;\n",
            "    final hostIntent = mosaicMap(response['hostIntent']);\n",
            "    final error = response['error'];\n",
            "{apply_host_props}",
            "    if (hostIntent.isNotEmpty) {{\n",
            "      debugPrint('hostIntent: $hostIntent');\n",
            "    }}\n",
            "    if (error != null) {{\n",
            "      debugPrint('host error: $error');\n",
            "    }}\n",
            "  }}\n\n",
            "  @override\n",
            "  Widget build(BuildContext context) {{\n",
            "    return MaterialApp(\n",
            "      title: '{component_name}',\n",
            "      home: Scaffold(\n",
            "        appBar: AppBar(title: const Text('{component_name}')),\n",
            "        body: Center(\n",
            "          child: {root_widget},\n",
            "        ),\n",
            "      ),\n",
            "    );\n",
            "  }}\n",
            "}}\n\n",
            "Map<String, Object?> mosaicMap(Object? value) {{\n",
            "  if (value is Map<String, Object?>) return value;\n",
            "  if (value is Map) {{\n",
            "    return Map<String, Object?>.fromEntries(\n",
            "      value.entries.where((entry) => entry.key is String).map(\n",
            "        (entry) => MapEntry(entry.key as String, entry.value),\n",
            "      ),\n",
            "    );\n",
            "  }}\n",
            "  return const <String, Object?>{{}};\n",
            "}}\n\n",
            "String mosaicString(Map<String, Object?> props, String name, String fallback) =>\n",
            "    props[name]?.toString() ?? fallback;\n\n",
            "double mosaicDouble(Map<String, Object?> props, String name, double fallback) {{\n",
            "  final value = props[name];\n",
            "  if (value is num) return value.toDouble();\n",
            "  if (value is String) return double.tryParse(value) ?? fallback;\n",
            "  return fallback;\n",
            "}}\n\n",
            "bool mosaicBoolean(Map<String, Object?> props, String name, bool fallback) {{\n",
            "  final value = props[name];\n",
            "  if (value is bool) return value;\n",
            "  if (value is String) {{\n",
            "    final lowered = value.toLowerCase();\n",
            "    if (lowered == 'true') return true;\n",
            "    if (lowered == 'false') return false;\n",
            "  }}\n",
            "  return fallback;\n",
            "}}\n\n",
            "List<String> mosaicStringList(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is List) {{\n",
            "    return value.map((item) => item.toString()).toList(growable: false);\n",
            "  }}\n",
            "  return const <String>[];\n",
            "}}\n\n",
            "List<double> mosaicDoubleList(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is List) {{\n",
            "    return value\n",
            "        .map((item) {{\n",
            "          if (item is num) return item.toDouble();\n",
            "          if (item is String) return double.tryParse(item);\n",
            "          return null;\n",
            "        }})\n",
            "        .whereType<double>()\n",
            "        .toList(growable: false);\n",
            "  }}\n",
            "  return const <double>[];\n",
            "}}\n\n",
            "List<bool> mosaicBooleanList(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is List) {{\n",
            "    return value\n",
            "        .map((item) {{\n",
            "          if (item is bool) return item;\n",
            "          if (item is String) {{\n",
            "            final lowered = item.toLowerCase();\n",
            "            if (lowered == 'true') return true;\n",
            "            if (lowered == 'false') return false;\n",
            "          }}\n",
            "          return null;\n",
            "        }})\n",
            "        .whereType<bool>()\n",
            "        .toList(growable: false);\n",
            "  }}\n",
            "  return const <bool>[];\n",
            "}}\n\n",
            "Widget mosaicWidget(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            "  Widget fallback,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  return value is Widget ? value : fallback;\n",
            "}}\n"
        ),
        banner = BANNER_DART,
        component_name = component_name,
        root_widget = root_widget,
        host_props_field = host_props_field,
        apply_host_props = apply_host_props,
    )
}

fn build_runtime_required_main_dart(component_name: &str, slots: &[SlotDecl]) -> String {
    let root_widget = build_runtime_required_root_widget_constructor(component_name, slots);
    let host_props_field = if slots.is_empty() {
        String::new()
    } else {
        "  Map<String, Object?> _hostProps = const <String, Object?>{};\n".to_string()
    };
    let next_props = if slots.is_empty() {
        String::new()
    } else {
        "    final nextProps = mosaicMap(response['props']);\n".to_string()
    };
    let assign_host_props = if slots.is_empty() {
        String::new()
    } else {
        "      _hostProps = nextProps;\n".to_string()
    };
    format!(
        concat!(
            "{banner}",
            "import 'dart:async';\n",
            "import 'package:flutter/material.dart';\n",
            "import '{component_name}.dart';\n",
            "import 'mosaic_host.dart';\n\n",
            "void main() {{\n",
            "  runApp(MosaicApp(mosaicHost: MosaicHost.loadRequired()));\n",
            "}}\n\n",
            "class MosaicApp extends StatefulWidget {{\n",
            "  const MosaicApp({{super.key, required this.mosaicHost}});\n\n",
            "  final MosaicHost mosaicHost;\n\n",
            "  @override\n",
            "  State<MosaicApp> createState() => _MosaicAppState();\n",
            "}}\n\n",
            "class _MosaicAppState extends State<MosaicApp> {{\n",
            "  late final MosaicHost _mosaicHost;\n",
            "{host_props_field}",
            "  bool _hostReady = false;\n\n",
            "  @override\n",
            "  void initState() {{\n",
            "    super.initState();\n",
            "    _mosaicHost = widget.mosaicHost;\n",
            "    _mosaicHost.setPropsChangedHandler(() =>\n",
            "        _queueMosaicResponse(_mosaicHost.props()));\n",
            "    _queueMosaicResponse(_mosaicHost.props());\n",
            "  }}\n\n",
            "  @override\n",
            "  void dispose() {{\n",
            "    _mosaicHost.dispose();\n",
            "    super.dispose();\n",
            "  }}\n\n",
            "  void _queueMosaicResponse(\n",
            "    FutureOr<Map<String, Object?>?>? responseOrFuture,\n",
            "  ) {{\n",
            "    if (responseOrFuture == null) {{\n",
            "      throw StateError('Mosaic runtime returned no response');\n",
            "    }}\n",
            "    Future<Map<String, Object?>?>.value(responseOrFuture).then((response) {{\n",
            "      if (response == null) {{\n",
            "        throw StateError('Mosaic runtime returned no response');\n",
            "      }}\n",
            "      _applyMosaicResponse(response);\n",
            "    }});\n",
            "  }}\n\n",
            "  void _applyMosaicResponse(Map<String, Object?> response) {{\n",
            "    if (!response.containsKey('props')) {{\n",
            "      throw StateError('Mosaic runtime response did not include props');\n",
            "    }}\n",
            "{next_props}",
            "    final hostIntent = mosaicMap(response['hostIntent']);\n",
            "    final error = response['error'];\n",
            "    if (!mounted) return;\n",
            "    setState(() {{\n",
            "{assign_host_props}",
            "      _hostReady = true;\n",
            "    }});\n",
            "    if (hostIntent.isNotEmpty) {{\n",
            "      debugPrint('hostIntent: $hostIntent');\n",
            "    }}\n",
            "    if (error != null) {{\n",
            "      debugPrint('host error: $error');\n",
            "    }}\n",
            "  }}\n\n",
            "  @override\n",
            "  Widget build(BuildContext context) {{\n",
            "    return MaterialApp(\n",
            "      title: '{component_name}',\n",
            "      home: Scaffold(\n",
            "        appBar: AppBar(title: const Text('{component_name}')),\n",
            "        body: Center(\n",
            "          child: _hostReady\n",
            "              ? {root_widget}\n",
            "              : Semantics(\n",
            "                  label: 'Starting {component_name}',\n",
            "                  child: const CircularProgressIndicator(),\n",
            "                ),\n",
            "        ),\n",
            "      ),\n",
            "    );\n",
            "  }}\n",
            "}}\n\n",
            "Map<String, Object?> mosaicMap(Object? value) {{\n",
            "  if (value is Map<String, Object?>) return value;\n",
            "  if (value is Map) {{\n",
            "    return Map<String, Object?>.fromEntries(\n",
            "      value.entries.where((entry) => entry.key is String).map(\n",
            "        (entry) => MapEntry(entry.key as String, entry.value),\n",
            "      ),\n",
            "    );\n",
            "  }}\n",
            "  return const <String, Object?>{{}};\n",
            "}}\n\n",
            "String? mosaicOptionalString(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  return value?.toString();\n",
            "}}\n\n",
            "double? mosaicOptionalDouble(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is num) return value.toDouble();\n",
            "  if (value is String) {{\n",
            "    final parsed = double.tryParse(value);\n",
            "    if (parsed != null) return parsed;\n",
            "  }}\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a number\");\n",
            "}}\n\n",
            "bool? mosaicOptionalBoolean(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is bool) return value;\n",
            "  if (value is String) {{\n",
            "    final lowered = value.toLowerCase();\n",
            "    if (lowered == 'true') return true;\n",
            "    if (lowered == 'false') return false;\n",
            "  }}\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a boolean\");\n",
            "}}\n\n",
            "List<String>? mosaicOptionalStringList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime prop '$name' is not a list\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item == null) {{\n",
            "      throw StateError(\"Mosaic runtime prop '$name' contains null\");\n",
            "    }}\n",
            "    return item.toString();\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "List<double>? mosaicOptionalDoubleList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime prop '$name' is not a list\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item is num) return item.toDouble();\n",
            "    if (item is String) {{\n",
            "      final parsed = double.tryParse(item);\n",
            "      if (parsed != null) return parsed;\n",
            "    }}\n",
            "    throw StateError(\"Mosaic runtime prop '$name' contains a non-number\");\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "List<bool>? mosaicOptionalBooleanList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime prop '$name' is not a list\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item is bool) return item;\n",
            "    if (item is String) {{\n",
            "      final lowered = item.toLowerCase();\n",
            "      if (lowered == 'true') return true;\n",
            "      if (lowered == 'false') return false;\n",
            "    }}\n",
            "    throw StateError(\"Mosaic runtime prop '$name' contains a non-boolean\");\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "Widget? mosaicOptionalWidget(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is Widget) return value;\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a Widget\");\n",
            "}}\n\n",
            "T? mosaicOptionalValue<T>(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value == null) return null;\n",
            "  if (value is T) return value as T;\n",
            "  throw StateError(\"Mosaic runtime prop '$name' has the wrong type\");\n",
            "}}\n\n",
            "String mosaicRequiredString(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value != null) return value.toString();\n",
            "  throw StateError(\"Mosaic runtime omitted required prop '$name'\");\n",
            "}}\n\n",
            "double mosaicRequiredDouble(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is num) return value.toDouble();\n",
            "  if (value is String) {{\n",
            "    final parsed = double.tryParse(value);\n",
            "    if (parsed != null) return parsed;\n",
            "  }}\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a number\");\n",
            "}}\n\n",
            "bool mosaicRequiredBoolean(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is bool) return value;\n",
            "  if (value is String) {{\n",
            "    final lowered = value.toLowerCase();\n",
            "    if (lowered == 'true') return true;\n",
            "    if (lowered == 'false') return false;\n",
            "  }}\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a boolean\");\n",
            "}}\n\n",
            "List<String> mosaicRequiredStringList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime omitted required list prop '$name'\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item == null) {{\n",
            "      throw StateError(\"Mosaic runtime prop '$name' contains null\");\n",
            "    }}\n",
            "    return item.toString();\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "List<double> mosaicRequiredDoubleList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime omitted required list prop '$name'\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item is num) return item.toDouble();\n",
            "    if (item is String) {{\n",
            "      final parsed = double.tryParse(item);\n",
            "      if (parsed != null) return parsed;\n",
            "    }}\n",
            "    throw StateError(\"Mosaic runtime prop '$name' contains a non-number\");\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "List<bool> mosaicRequiredBooleanList(\n",
            "  Map<String, Object?> props,\n",
            "  String name,\n",
            ") {{\n",
            "  final value = props[name];\n",
            "  if (value is! List) {{\n",
            "    throw StateError(\"Mosaic runtime omitted required list prop '$name'\");\n",
            "  }}\n",
            "  return value.map((item) {{\n",
            "    if (item is bool) return item;\n",
            "    if (item is String) {{\n",
            "      final lowered = item.toLowerCase();\n",
            "      if (lowered == 'true') return true;\n",
            "      if (lowered == 'false') return false;\n",
            "    }}\n",
            "    throw StateError(\"Mosaic runtime prop '$name' contains a non-boolean\");\n",
            "  }}).toList(growable: false);\n",
            "}}\n\n",
            "Widget mosaicRequiredWidget(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is Widget) return value;\n",
            "  throw StateError(\"Mosaic runtime prop '$name' is not a Widget\");\n",
            "}}\n\n",
            "T mosaicRequiredValue<T>(Map<String, Object?> props, String name) {{\n",
            "  final value = props[name];\n",
            "  if (value is T) return value;\n",
            "  throw StateError(\"Mosaic runtime prop '$name' has the wrong type\");\n",
            "}}\n"
        ),
        banner = BANNER_DART,
        component_name = component_name,
        root_widget = root_widget,
        host_props_field = host_props_field,
        next_props = next_props,
        assign_host_props = assign_host_props,
    )
}

fn build_runtime_required_root_widget_constructor(
    component_name: &str,
    slots: &[SlotDecl],
) -> String {
    let mut out = format!("{component_name}(\n");
    for slot in slots {
        let field = to_camel_case_first_lower(&slot.name);
        let value = runtime_required_host_value_for_slot(slot);
        writeln!(out, "            {field}: {value},").unwrap();
    }
    out.push_str("            dispatch: (event) {\n");
    out.push_str(
        "              _queueMosaicResponse(_mosaicHost.handleEvent(event.mosaicEnvelope));\n",
    );
    out.push_str("            },\n");
    out.push_str("          )");
    out
}

fn runtime_required_host_value_for_slot(slot: &SlotDecl) -> String {
    let slot_name = escape_dart_string(&slot.name);
    if let Some(default) = &slot.default {
        return match default {
            SlotDefault::Text(value) => format!(
                "mosaicOptionalString(_hostProps, \"{slot_name}\") ?? \"{}\"",
                escape_dart_string(value)
            ),
            SlotDefault::Number(value) if value.is_finite() => format!(
                "mosaicOptionalDouble(_hostProps, \"{slot_name}\") ?? {}",
                dart_double_literal(*value)
            ),
            SlotDefault::Number(_) => {
                format!("mosaicOptionalDouble(_hostProps, \"{slot_name}\") ?? 0.0")
            }
            SlotDefault::Bool(value) => {
                format!("mosaicOptionalBoolean(_hostProps, \"{slot_name}\") ?? {value}")
            }
        };
    }
    if !slot.required {
        return match &slot.r#type {
            SlotType::Text | SlotType::Image | SlotType::Color => {
                format!("mosaicOptionalString(_hostProps, \"{slot_name}\")")
            }
            SlotType::Number => {
                format!("mosaicOptionalDouble(_hostProps, \"{slot_name}\")")
            }
            SlotType::Bool => {
                format!("mosaicOptionalBoolean(_hostProps, \"{slot_name}\")")
            }
            SlotType::List(inner) => match inner.as_ref() {
                ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                    format!("mosaicOptionalStringList(_hostProps, \"{slot_name}\")")
                }
                ListInnerType::Number => {
                    format!("mosaicOptionalDoubleList(_hostProps, \"{slot_name}\")")
                }
                ListInnerType::Bool => {
                    format!("mosaicOptionalBooleanList(_hostProps, \"{slot_name}\")")
                }
                _ => format!(
                    "mosaicOptionalValue<{}>(_hostProps, \"{slot_name}\")",
                    slot_type_to_dart(&slot.r#type)
                ),
            },
            SlotType::Node => format!("mosaicOptionalWidget(_hostProps, \"{slot_name}\")"),
            SlotType::Component(_) => format!(
                "mosaicOptionalValue<{}>(_hostProps, \"{slot_name}\")",
                slot_type_to_dart(&slot.r#type)
            ),
        };
    }
    match &slot.r#type {
        SlotType::Text | SlotType::Image | SlotType::Color => {
            format!("mosaicRequiredString(_hostProps, \"{slot_name}\")")
        }
        SlotType::Number => format!("mosaicRequiredDouble(_hostProps, \"{slot_name}\")"),
        SlotType::Bool => format!("mosaicRequiredBoolean(_hostProps, \"{slot_name}\")"),
        SlotType::List(inner) => match inner.as_ref() {
            ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                format!("mosaicRequiredStringList(_hostProps, \"{slot_name}\")")
            }
            ListInnerType::Number => {
                format!("mosaicRequiredDoubleList(_hostProps, \"{slot_name}\")")
            }
            ListInnerType::Bool => {
                format!("mosaicRequiredBooleanList(_hostProps, \"{slot_name}\")")
            }
            _ => format!(
                "mosaicRequiredValue<{}>(_hostProps, \"{slot_name}\")",
                slot_type_to_dart(&slot.r#type)
            ),
        },
        SlotType::Node => format!("mosaicRequiredWidget(_hostProps, \"{slot_name}\")"),
        SlotType::Component(_) => format!(
            "mosaicRequiredValue<{}>(_hostProps, \"{slot_name}\")",
            slot_type_to_dart(&slot.r#type)
        ),
    }
}

fn build_root_widget_constructor(component_name: &str, slots: &[SlotDecl]) -> String {
    let mut out = format!("{component_name}(\n");
    for slot in slots {
        let field = to_camel_case_first_lower(&slot.name);
        let value = host_value_for_slot(slot);
        writeln!(out, "            {field}: {value},").unwrap();
    }
    out.push_str("            dispatch: (event) {\n");
    out.push_str(
        "              final response = _mosaicHost?.handleEvent(event.mosaicEnvelope);\n",
    );
    out.push_str("              if (response == null) {\n");
    out.push_str("                debugPrint(\"event: ${event.mosaicEnvelope}\");\n");
    out.push_str("              }\n");
    out.push_str("              _queueMosaicResponse(response);\n");
    out.push_str("            },\n");
    out.push_str("          )");
    out
}

fn host_value_for_slot(slot: &SlotDecl) -> String {
    let slot_name = escape_dart_string(&slot.name);
    let fallback = sample_value_for_slot(slot);
    match &slot.r#type {
        SlotType::Text | SlotType::Image | SlotType::Color => {
            format!("mosaicString(_hostProps, \"{slot_name}\", {fallback})")
        }
        SlotType::Number => format!("mosaicDouble(_hostProps, \"{slot_name}\", {fallback})"),
        SlotType::Bool => format!("mosaicBoolean(_hostProps, \"{slot_name}\", {fallback})"),
        SlotType::List(inner) => match inner.as_ref() {
            ListInnerType::Text | ListInnerType::Image | ListInnerType::Color => {
                format!("mosaicStringList(_hostProps, \"{slot_name}\")")
            }
            ListInnerType::Number => format!("mosaicDoubleList(_hostProps, \"{slot_name}\")"),
            ListInnerType::Bool => format!("mosaicBooleanList(_hostProps, \"{slot_name}\")"),
            _ => fallback,
        },
        SlotType::Node | SlotType::Component(_) => {
            format!("mosaicWidget(_hostProps, \"{slot_name}\", {fallback})")
        }
    }
}

fn build_mosaic_host_dart(require_runtime: bool) -> String {
    let mut out = String::from(BANNER_DART);
    out.push_str("import 'dart:async';\n\n");
    out.push_str("class MosaicHost {\n");
    out.push_str("  const MosaicHost();\n\n");
    out.push_str("  static MosaicHost? load() => null;\n\n");
    if require_runtime {
        out.push_str("  static MosaicHost loadRequired() {\n");
        out.push_str(
            "    throw StateError('native-complete requires the Mosaic Rust application runtime');\n",
        );
        out.push_str("  }\n\n");
    }
    out.push_str("  FutureOr<Map<String, Object?>?> props() => null;\n\n");
    out.push_str(
        "  FutureOr<Map<String, Object?>?> handleEvent(Map<String, Object?> event) => null;\n\n",
    );
    out.push_str("  void setPropsChangedHandler(void Function()? handler) {}\n\n");
    out.push_str("  void dispose() {}\n");
    out.push_str("}\n");
    out
}

fn sample_value_for_slot(slot: &SlotDecl) -> String {
    match &slot.default {
        Some(SlotDefault::Text(value)) => format!("\"{}\"", escape_dart_string(value)),
        Some(SlotDefault::Number(value)) if value.is_finite() => dart_double_literal(*value),
        Some(SlotDefault::Number(_)) => "0.0".to_string(),
        Some(SlotDefault::Bool(value)) => value.to_string(),
        None => sample_value_for_slot_type(&slot.r#type, &slot.name),
    }
}

fn sample_value_for_slot_type(slot_type: &SlotType, slot_name: &str) -> String {
    match slot_type {
        SlotType::Text => format!("\"Sample {}\"", kebab_to_pascal_case_for_label(slot_name)),
        SlotType::Number => "0.0".to_string(),
        SlotType::Bool => "false".to_string(),
        SlotType::Image => "\"sample-image\"".to_string(),
        SlotType::Color => "\"#808080\"".to_string(),
        SlotType::Node => "const SizedBox.shrink()".to_string(),
        SlotType::Component(_) => "throw UnimplementedError()".to_string(),
        SlotType::List(_) => "const []".to_string(),
    }
}

fn dart_double_literal(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
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

fn build_flutter_readme(pub_name: &str, component_name: &str, require_runtime: bool) -> String {
    if !require_runtime {
        return format!(
        "{BANNER_MD}# {component_name} — Flutter app shell\n\nAuto-generated by `mosaic-compile --backend flutter --emit-project`.\n\n## Prerequisites\n\n- Flutter SDK 3.32+ (run `flutter --version` to check).\n- A device target: iOS simulator, Android emulator, or desktop (`flutter config --enable-macos-desktop` / `--enable-linux-desktop` / `--enable-windows-desktop`).\n\n## Run\n\nChoose the platforms this host will ship, let Flutter add their standard runner files, then build or run normally. Flutter preserves the Mosaic-generated `lib/` sources:\n\n```sh\nflutter create --platforms=macos,windows,linux .\nflutter pub get\nflutter run -d <device-id>   # or `flutter run` to pick interactively\n```\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `lib/{component_name}.dart` | The Mosaic-compiled component mounted by the app shell. |\n| `pubspec.yaml` | Dart pub manifest. Pinned Flutter + Dart SDKs per UI32 spec §3.6.3. |\n| `lib/main.dart` | MaterialApp shell that mounts `{component_name}(...)`, hydrates slot values from an optional Mosaic host, and forwards Mosaic event envelopes. |\n| `lib/mosaic_host.dart` | Default no-op Mosaic host hook. App packages can overwrite it with a real bridge. |\n| `README.md` | This file. |\n\nDart pub name: `{pub_name}`.\n\n## Editing\n\nEvery shell file carries an AUTO-GENERATED banner. Re-running `mosaic-compile --emit-project` will overwrite them. To customise the shell, remove the banner from a file and rename or relocate it; the next `--emit-project` run will recreate the original at its original name without touching your forked copy.\n"
        );
    }

    format!(
        "{BANNER_MD}# {component_name} — Flutter native-complete app shell\n\nAuto-generated by `mosaic-compile --backend flutter --emit-project --profile native-complete`.\n\nThis shell requires Mosaic's standard Rust application runtime at startup. It waits for the first runtime props envelope before mounting `{component_name}` and never substitutes preview/sample values for missing required props.\n\n## Prerequisites\n\n- Flutter SDK 3.32+ (run `flutter --version` to check).\n- A built Mosaic Rust application library, available through `MOSAIC_APP_LIBRARY` or the platform's conventional `mosaic_app` library name.\n- A device target: iOS simulator, Android emulator, or desktop (`flutter config --enable-macos-desktop` / `--enable-linux-desktop` / `--enable-windows-desktop`).\n\n## Run\n\nChoose the platforms this host will ship, let Flutter add their standard runner files, then build or run normally. Flutter preserves the Mosaic-generated `lib/` sources:\n\n```sh\nflutter create --platforms=macos,windows,linux .\nflutter pub get\nMOSAIC_APP_LIBRARY=/absolute/path/to/libmosaic_app.dylib flutter run -d <device-id>\n```\n\nUse the platform-appropriate library filename on Linux or Windows. Startup fails explicitly when the Rust runtime cannot be loaded or does not provide the required props envelope.\n\n## What's in this directory\n\n| File | Purpose |\n|---|---|\n| `lib/{component_name}.dart` | The Mosaic-compiled component mounted by the app shell. |\n| `pubspec.yaml` | Dart pub manifest. Pinned Flutter + Dart SDKs per UI32 spec §3.6.3. |\n| `lib/main.dart` | Runtime-required MaterialApp shell that waits for Rust-provided props and forwards Mosaic event envelopes. |\n| `lib/mosaic_host.dart` | Mosaic host contract. The package builder installs the standard Dart FFI binding here. |\n| `README.md` | This file. |\n\nDart pub name: `{pub_name}`.\n\n## Editing\n\nEvery shell file carries an AUTO-GENERATED banner. Re-running `mosaic-compile --emit-project` will overwrite them. To customise the shell, remove the banner from a file and rename or relocate it; the next `--emit-project` run will recreate the original at its original name without touching your forked copy.\n"
    )
}

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
    writeln!(
        out,
        "// Auto-generated by mosaic-emit-flutter. Do not edit."
    )
    .unwrap();
    let uses_checkbox = layout_contains_tag(&layout.root, "HostCheckbox");
    let uses_radio = layout_contains_tag(&layout.root, "HostRadio");
    let uses_slider = layout_contains_tag(&layout.root, "HostSlider");
    let uses_tooltip = layout_contains_tag(&layout.root, "HostTooltip");
    let uses_drag = layout_contains_tag(&layout.root, "HostDraggable")
        || layout_contains_tag(&layout.root, "HostDropTarget");
    let uses_dialog = layout_contains_native_dialog(&layout.root);
    if uses_radio {
        writeln!(out, "// ignore_for_file: deprecated_member_use").unwrap();
    }
    if uses_drag {
        // The generated private helper exposes the complete UI35 callback
        // contract even when a particular component binds only a subset.
        // `SemanticsService.announce` is retained for the documented Flutter
        // 3.24 floor; newer SDKs deprecate it in favor of a multi-view API.
        writeln!(
            out,
            "// ignore_for_file: unused_element_parameter, deprecated_member_use"
        )
        .unwrap();
    } else if uses_dialog {
        // #13010: `_MosaicDialogHost.barrierDismissible` is a
        // constructor parameter with a default, exposed for every
        // `HostDialog` even when this particular file's authored
        // layout(s) never set `dismiss-on-backdrop: false` -- same
        // "shared helper exposes the complete contract" shape as the
        // drag helper above, just for a different lint trigger.
        writeln!(out, "// ignore_for_file: unused_element_parameter").unwrap();
    }
    writeln!(
        out,
        "import 'package:flutter/material.dart' hide Checkbox, Radio, Slider, Tooltip;"
    )
    .unwrap();
    let mut material_aliases = Vec::new();
    if uses_checkbox {
        material_aliases.push("Checkbox");
    }
    if uses_radio {
        material_aliases.push("Radio");
    }
    if uses_slider {
        material_aliases.push("Slider");
    }
    if uses_tooltip {
        material_aliases.push("Tooltip");
    }
    if !material_aliases.is_empty() {
        writeln!(
            out,
            "import 'package:flutter/material.dart' as material show {};",
            material_aliases.join(", ")
        )
        .unwrap();
    }
    if uses_drag {
        writeln!(out, "import 'package:flutter/services.dart';").unwrap();
        writeln!(out, "import 'package:flutter/semantics.dart';").unwrap();
    }
    writeln!(out).unwrap();

    // 2. Event union — sealed base class + one subclass per emit.
    out.push_str(&emit_event_union(name, &interface.emits)?);
    writeln!(out).unwrap();
    if uses_drag {
        out.push_str(&emit_drag_helpers());
        writeln!(out).unwrap();
    }
    if uses_dialog {
        out.push_str(&emit_dialog_helper());
        writeln!(out).unwrap();
    }

    // 3. Pre-compute the per-part style map. Same shape as the React
    //    emitter's `build_part_style_map`: kebab part-name → joined
    //    `key: value;` string the widget builder can consume.
    let part_styles = build_part_style_map(style);

    // 4. The widget class itself.
    out.push_str(&emit_widget_class(
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

fn layout_contains_tag(node: &LayoutNode, tag: &str) -> bool {
    node.tag == tag
        || node
            .children
            .iter()
            .any(|child| layout_contains_tag(child, tag))
}

/// #13010: like [`layout_contains_tag`] for `"HostDialog"`, but only
/// counts a node that actually lowers to `_MosaicDialogHost` (i.e.
/// [`host_dialog_has_native_semantics`] is true for it). A file whose
/// only `HostDialog` is `modal: false` must not pay for the shared
/// helper class it never instantiates.
fn layout_contains_native_dialog(node: &LayoutNode) -> bool {
    (node.tag == "HostDialog" && host_dialog_has_native_semantics(node))
        || node.children.iter().any(layout_contains_native_dialog)
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
///   final num row;
///   final num col;
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
    writeln!(out, "  String get mosaicName;").unwrap();
    writeln!(
        out,
        "  Map<String, Object?> get mosaicPayload => const {{}};"
    )
    .unwrap();
    writeln!(
        out,
        "  Map<String, Object?> get mosaicEnvelope => {{'event': mosaicName, ...mosaicPayload}};"
    )
    .unwrap();
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
        writeln!(out, "  @override").unwrap();
        writeln!(
            out,
            "  String get mosaicName => \"{}\";",
            escape_dart_string(&e.name)
        )
        .unwrap();
        if !e.params.is_empty() {
            let payload_entries = e
                .params
                .iter()
                .map(|p| {
                    let field = to_camel_case_first_lower(&p.name);
                    format!("'{}': {field}", escape_dart_string(&field))
                })
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "  @override").unwrap();
            writeln!(
                out,
                "  Map<String, Object?> get mosaicPayload => {{{payload_entries}}};"
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
    }

    Ok(out)
}

/// Emit the package-independent Flutter drag/drop runtime used by generated
/// components. Pointer and keyboard releases both call `accept`, so the
/// proposal payload and accepted outcome cannot diverge. The scope is mounted
/// once per generated component instance, which keeps keyboard target
/// enumeration out of sibling or nested component instances.
fn emit_drag_helpers() -> String {
    r#"class _MosaicDragData {
  const _MosaicDragData({required this.key, required this.kind, required this.label});

  final String key;
  final String kind;
  final String label;
}

class _MosaicDragScope extends StatefulWidget {
  const _MosaicDragScope({required this.child});

  final Widget child;

  static _MosaicDragScopeState of(BuildContext context) {
    final host = context.dependOnInheritedWidgetOfExactType<_MosaicDragScopeHost>();
    assert(host != null, 'Mosaic drag primitives require a component drag scope');
    return host!.state;
  }

  @override
  State<_MosaicDragScope> createState() => _MosaicDragScopeState();
}

class _MosaicDragScopeHost extends InheritedWidget {
  const _MosaicDragScopeHost({required this.state, required super.child});

  final _MosaicDragScopeState state;

  @override
  bool updateShouldNotify(_MosaicDragScopeHost oldWidget) => false;
}

class _MosaicDragScopeState extends State<_MosaicDragScope> {
  final List<_MosaicDropTargetState> _targets = <_MosaicDropTargetState>[];
  _MosaicDragData? _active;
  void Function(bool dropped)? _onDragEnd;
  _MosaicDropTargetState? _activeTarget;

  void register(_MosaicDropTargetState target) {
    if (!_targets.contains(target)) _targets.add(target);
  }

  void unregister(_MosaicDropTargetState target) {
    _targets.remove(target);
    if (identical(_activeTarget, target)) _activeTarget = null;
  }

  void announce(String message) {
    SemanticsService.announce(message, Directionality.of(context));
  }

  void grab(
    _MosaicDragData data,
    VoidCallback? onDragStart,
    void Function(bool dropped)? onDragEnd,
  ) {
    if (_active != null) return;
    _active = data;
    _onDragEnd = onDragEnd;
    _activeTarget = null;
    onDragStart?.call();
    announce('Grabbed ${data.label}. Use arrow keys to choose a target, then press Space or Enter to drop.');
  }

  List<_MosaicDropTargetState> get _eligibleTargets {
    final data = _active;
    if (data == null) return const <_MosaicDropTargetState>[];
    return _targets.where((target) => target.accepts(data)).toList(growable: false);
  }

  void step(int delta) {
    final data = _active;
    final targets = _eligibleTargets;
    if (data == null || targets.isEmpty) {
      announce('No available drop targets.');
      return;
    }
    final previous = _activeTarget;
    var targetIndex = previous == null ? -1 : targets.indexOf(previous);
    targetIndex = (targetIndex + delta) % targets.length;
    if (targetIndex < 0) targetIndex += targets.length;
    final next = targets[targetIndex];
    if (!identical(previous, next)) {
      previous?.leave(data);
      next.enter(data);
    }
    next.hover(data, 'into');
    _activeTarget = next;
    announce('Move to ${next.widget.targetKey}, position ${targetIndex + 1} of ${targets.length}.');
  }

  bool drop() {
    final data = _active;
    final target = _activeTarget;
    if (data == null || target == null || !target.accepts(data)) return false;
    target.accept(data, 'into');
    _finish(true);
    return true;
  }

  void cancel() {
    final data = _active;
    if (data == null) return;
    _activeTarget?.leave(data);
    announce('Cancelled drag.');
    _finish(false);
  }

  void _finish(bool dropped) {
    final onDragEnd = _onDragEnd;
    _active = null;
    _onDragEnd = null;
    _activeTarget = null;
    onDragEnd?.call(dropped);
  }

  @override
  Widget build(BuildContext context) =>
      _MosaicDragScopeHost(state: this, child: widget.child);
}

class _MosaicDraggable extends StatelessWidget {
  const _MosaicDraggable({
    required this.data,
    required this.disabled,
    required this.child,
    this.onDragStart,
    this.onDragEnd,
  });

  final _MosaicDragData data;
  final bool disabled;
  final Widget child;
  final VoidCallback? onDragStart;
  final void Function(bool dropped)? onDragEnd;

  void _toggle(BuildContext context) {
    final scope = _MosaicDragScope.of(context);
    if (scope._active == null) {
      scope.grab(data, onDragStart, onDragEnd);
    } else if (!scope.drop()) {
      scope.cancel();
    }
  }

  KeyEventResult _onKey(BuildContext context, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;
    final scope = _MosaicDragScope.of(context);
    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.escape && scope._active != null) {
      scope.cancel();
      return KeyEventResult.handled;
    }
    if (disabled) return KeyEventResult.ignored;
    if (key == LogicalKeyboardKey.space || key == LogicalKeyboardKey.enter) {
      _toggle(context);
      return KeyEventResult.handled;
    }
    if (scope._active != null &&
        (key == LogicalKeyboardKey.arrowDown || key == LogicalKeyboardKey.arrowRight)) {
      scope.step(1);
      return KeyEventResult.handled;
    }
    if (scope._active != null &&
        (key == LogicalKeyboardKey.arrowUp || key == LogicalKeyboardKey.arrowLeft)) {
      scope.step(-1);
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final scope = _MosaicDragScope.of(context);
    final semanticChild = Semantics(
      button: true,
      label: 'Draggable ${data.label}. Press Space or Enter to grab.',
      onTap: disabled && scope._active == null ? null : () => _toggle(context),
      child: child,
    );
    return Focus(
      canRequestFocus: !disabled,
      onKeyEvent: (node, event) => _onKey(context, event),
      child: Draggable<_MosaicDragData>(
        data: data,
        maxSimultaneousDrags: disabled ? 0 : 1,
        onDragStarted: () {
          onDragStart?.call();
          scope.announce('Grabbed ${data.label}.');
        },
        onDragEnd: (details) {
          onDragEnd?.call(details.wasAccepted);
          if (!details.wasAccepted) scope.announce('Cancelled drag.');
        },
        feedback: Material(
          type: MaterialType.transparency,
          child: Opacity(opacity: 0.8, child: child),
        ),
        childWhenDragging: Opacity(opacity: 0.45, child: semanticChild),
        child: semanticChild,
      ),
    );
  }
}

class _MosaicDropTarget extends StatefulWidget {
  const _MosaicDropTarget({
    required this.targetKey,
    required this.disabled,
    required this.child,
    this.acceptsKinds,
    this.onDragEnter,
    this.onDragLeave,
    this.onDropHover,
    this.onDrop,
  });

  final String targetKey;
  final bool disabled;
  final List<String>? acceptsKinds;
  final Widget child;
  final void Function(_MosaicDragData data)? onDragEnter;
  final void Function(_MosaicDragData data)? onDragLeave;
  final void Function(_MosaicDragData data, String position)? onDropHover;
  final void Function(_MosaicDragData data, String position)? onDrop;

  @override
  State<_MosaicDropTarget> createState() => _MosaicDropTargetState();
}

class _MosaicDropTargetState extends State<_MosaicDropTarget> {
  _MosaicDragScopeState? _scope;
  String _position = 'into';

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final next = _MosaicDragScope.of(context);
    if (!identical(_scope, next)) {
      _scope?.unregister(this);
      _scope = next;
      next.register(this);
    }
  }

  @override
  void dispose() {
    _scope?.unregister(this);
    super.dispose();
  }

  bool accepts(_MosaicDragData data) =>
      !widget.disabled &&
      (widget.acceptsKinds == null || widget.acceptsKinds!.contains(data.kind));

  String _positionFor(Offset globalOffset) {
    final renderObject = context.findRenderObject();
    if (renderObject is! RenderBox || renderObject.size.height <= 0) return 'into';
    final ratio = renderObject.globalToLocal(globalOffset).dy / renderObject.size.height;
    return ratio < 1 / 3 ? 'before' : ratio > 2 / 3 ? 'after' : 'into';
  }

  void accept(_MosaicDragData data, String position) {
    if (!accepts(data)) return;
    widget.onDrop?.call(data, position);
    _scope?.announce('Dropped ${data.label} on ${widget.targetKey}.');
  }

  void enter(_MosaicDragData data) => widget.onDragEnter?.call(data);

  void leave(_MosaicDragData data) => widget.onDragLeave?.call(data);

  void hover(_MosaicDragData data, String position) {
    _position = position;
    widget.onDropHover?.call(data, position);
  }

  @override
  Widget build(BuildContext context) {
    return DragTarget<_MosaicDragData>(
      onWillAcceptWithDetails: (details) {
        final accepted = accepts(details.data);
        if (accepted) enter(details.data);
        return accepted;
      },
      onMove: (details) {
        if (!accepts(details.data)) return;
        hover(details.data, _positionFor(details.offset));
      },
      onLeave: (data) {
        if (data != null) leave(data);
      },
      onAcceptWithDetails: (details) => accept(details.data, _position),
      builder: (context, candidateData, rejectedData) => Semantics(
        container: true,
        label: 'Drop target ${widget.targetKey}',
        child: widget.child,
      ),
    );
  }
}
"#
    .to_string()
}

/// Emit the `StatelessWidget` class: fields for every slot, a
/// `dispatch` field (always present, matches React's required prop),
/// a const constructor with named-required parameters, and the
/// `build` method returning the widget tree.
fn emit_widget_class(
    component: &str,
    slots: &[SlotDecl],
    emits: &[EmitDecl],
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
        // A MIL default makes the native field non-null while keeping the
        // named argument optional. Truly optional hand-built IR without a
        // default remains nullable.
        let nullable = !s.required && s.default.is_none();
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
        let default = s
            .default
            .as_ref()
            .map(|_| format!(" = {}", sample_value_for_slot(s)))
            .unwrap_or_default();
        writeln!(out, "    {prefix}this.{field}{default},").unwrap();
    }
    writeln!(out, "    required this.dispatch,").unwrap();
    writeln!(out, "  }});").unwrap();

    let mut tree = emit_widget_tree(
        layout_root,
        6,
        part_styles,
        component,
        emits,
        TableCtx::default(),
    )?;

    if layout_contains_tag(layout_root, "HostDraggable")
        || layout_contains_tag(layout_root, "HostDropTarget")
    {
        let nested = tree
            .trim_end()
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        tree = format!("      _MosaicDragScope(\n        child:\n{nested},\n      )\n");
    }

    // Mosaic conditions use value truthiness, while Dart requires a
    // statically-typed bool. Keep the conversion on the generated widget so
    // multiple emitted component files cannot introduce a top-level helper
    // collision. Emit it only for a tree that uses it so otherwise-clean
    // package exports do not carry an unused private member warning.
    if tree.contains("_mosaicTruthy(") {
        writeln!(out).unwrap();
        writeln!(out, "  bool _mosaicTruthy(Object? value) {{").unwrap();
        writeln!(out, "    if (value == null) return false;").unwrap();
        writeln!(out, "    if (value is bool) return value;").unwrap();
        writeln!(out, "    if (value is num) return value != 0;").unwrap();
        writeln!(out, "    if (value is String) return value.isNotEmpty;").unwrap();
        writeln!(out, "    if (value is Iterable) return value.isNotEmpty;").unwrap();
        writeln!(out, "    if (value is Map) return value.isNotEmpty;").unwrap();
        writeln!(out, "    return true;").unwrap();
        writeln!(out, "  }}").unwrap();
    }

    // 3. build method.
    writeln!(out).unwrap();
    writeln!(out, "  @override").unwrap();
    writeln!(out, "  Widget build(BuildContext context) {{").unwrap();
    writeln!(out, "    return").unwrap();
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
// Table context — column-widths threading + cell-position tracking
// =====================================================================

/// Threaded down the widget walker so cell-position widgets pick up the
/// right `width:` and parents know whether to *spread* a `For` child
/// into their `children: [...]` list (orientation-correct) instead of
/// nesting a standalone `Column`.
///
/// Two facts travel together:
///
///   * `column_widths_slot` — the camelCased name of the host's
///     column-widths array (`columnWidths`), discovered at `HostTable`
///     entry from the `HostTableColGroup > For (each: slot: …) { Col }`
///     shape.  Cells index into it (`columnWidths[c]`) to render at a
///     stable column width.  Mirrors the SwiftUI backend's
///     `TableContext` (PR #4393 lineage).
///   * `cell_index` — set to the enclosing cell-position `For`'s index
///     binding (`c` for body cells, `ch` for header cells) while that
///     `For`'s body is being emitted, so the cell's `Container` can use
///     `columnWidths[<cell_index>]`.
///
/// `Copy` so it threads by value with zero ceremony.  The default
/// (`None`/`None`) is the non-table case — every existing single-shot
/// emit path keeps working unchanged.
#[derive(Clone, Copy, Default)]
struct TableCtx<'a> {
    column_widths_slot: Option<&'a str>,
    cell_index: Option<&'a str>,
    /// Nearest enclosing `For` row binding. Buttons inside toolkit rows
    /// use this to dispatch the selected item without each backend
    /// inventing a separate row-selection primitive.
    for_item: Option<&'a str>,
    /// Nearest enclosing `For` index binding. Number-typed button emits
    /// use this when the interface declares a single numeric payload.
    for_index: Option<&'a str>,
    /// The table's own (`sheet`) part base text colour / font, threaded
    /// so cells fall back to the sheet's `color` / `font-family` /
    /// `font-size` instead of `null`. Each is the already-lowered Dart
    /// expression (`const Color(0xFFCCCCCC)`) or family/size literal.
    sheet_text_color: Option<&'a str>,
    sheet_font_family: Option<&'a str>,
    sheet_font_size: Option<&'a str>,
    /// True only while emitting a widget that is a direct child of a
    /// `Row`. Flutter text fields require a finite horizontal constraint,
    /// so direct row inputs lower through `Expanded`.
    direct_row_child: bool,
}

/// Scan a `HostTable`'s children for the
/// `HostTableColGroup > For (each: slot: <name>) { Col … }` shape and
/// return the camelCased column-widths slot name.  `None` when the
/// table has no such col-group (cells then size to content, matching
/// the pre-fix behaviour).  Structural twin of the SwiftUI backend's
/// `extract_table_context`.
fn extract_column_widths_slot(host_table: &LayoutNode) -> Option<String> {
    for child in &host_table.children {
        if child.tag != "HostTableColGroup" {
            continue;
        }
        for cg_child in &child.children {
            if cg_child.tag != "For" {
                continue;
            }
            // The For body must contain a `Col` (the colgroup cell tag).
            if !cg_child.children.iter().any(|n| n.tag == "Col") {
                continue;
            }
            if let Some(slot) = find_slot_ref_prop(cg_child, "each") {
                let camel = to_camel_case_first_lower(slot);
                if is_safe_dart_identifier(&camel) {
                    return Some(camel);
                }
            }
        }
    }
    None
}

/// The native Flutter table lowering deliberately accepts the canonical
/// UI31/Grid shape rather than pretending every arbitrary HostTable tree can
/// be represented by `DataTable`:
///
/// ```text
/// HostTable
///   HostTableHead > Row > For > <header widget>
///   HostTableBody > For > Row > For > <cell widget>
/// ```
///
/// `DataTable` accepts dynamically-created `List<DataColumn>` and
/// `List<DataRow>` values, so both `For` loops can remain dynamic. Unsupported
/// shapes retain the visual Column fallback and, importantly, remain visible
/// to native-complete degradation analysis.
struct FlutterDataTableShape<'a> {
    header_cells: &'a LayoutNode,
    body_rows: &'a LayoutNode,
    body_cells: &'a LayoutNode,
}

fn flutter_data_table_shape(host_table: &LayoutNode) -> Option<FlutterDataTableShape<'_>> {
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

    Some(FlutterDataTableShape {
        header_cells,
        body_rows,
        body_cells,
    })
}

/// Returns whether a HostTable layout has the canonical dynamic structure that
/// this emitter lowers to Flutter's native DataTable family.
///
/// Package capability analysis uses the same predicate so strict-profile
/// reporting cannot drift away from the actual lowering.
pub fn host_table_has_native_semantics(host_table: &LayoutNode) -> bool {
    flutter_data_table_shape(host_table).is_some()
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
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // --- Routing: kernel primitives with custom lowerings ---
    if node.tag == "HostSurface" {
        return emit_host_surface(node, indent);
    }
    if node.tag == "HostInput" {
        return emit_host_input(
            node,
            indent,
            part_styles,
            component,
            emits,
            ctx.direct_row_child,
        );
    }
    if node.tag == "HostButton" {
        return emit_host_button(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostCheckbox" {
        return emit_host_checkbox(node, indent, part_styles, component, emits);
    }
    if node.tag == "HostRadio" {
        return emit_host_radio(node, indent, part_styles, component, emits);
    }
    if node.tag == "HostSlider" {
        return emit_host_slider(node, indent, component, emits);
    }
    if node.tag == "HostScroll" {
        return emit_host_scroll(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostDialog" {
        return emit_host_dialog(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostTable" {
        return emit_host_table(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostDraggable" {
        return emit_host_draggable(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostDropTarget" {
        return emit_host_drop_target(node, indent, part_styles, component, emits, ctx);
    }
    // UI29-4 kernel — three new primitives. `HostLink` lowers to an
    // `InkWell` wrapping a `Text` (with a `url_launcher` TODO comment
    // since Flutter has no built-in URL-launch capability without an
    // external package), `HostTooltip` to Flutter's first-class
    // `Tooltip(message:, child:)` widget, and `HostNumberInput` to a
    // `TextField` configured with `TextInputType.number` so mobile
    // devices show the numeric keypad.
    if node.tag == "HostLink" {
        return emit_host_link(node, indent, component, emits, ctx);
    }
    if node.tag == "HostTooltip" {
        return emit_host_tooltip(node, indent, part_styles, component, emits, ctx);
    }
    if node.tag == "HostNumberInput" {
        return emit_host_number_input(node, indent, component);
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
        // X5 (Flutter analog): semantic-glyph lowering before the
        // default `Icon(Icons.<source>)` path.  When the glyph is a
        // *semantic* name (currently only `"spinner"`), emit the
        // Material widget that natively expresses that semantic —
        // `CircularProgressIndicator()` for `"spinner"`.  Without
        // this, the toolkit's `Icon (glyph: "spinner")` would render
        // as a static star (the `unwrap_or("star")` default, because
        // the Flutter emitter looks for `source` not `glyph` —
        // see the prop-name compatibility note below).
        //
        // The semantic table mirrors mosaic-emit-xaml's X5 fix.  Each
        // backend ships its own native-widget map for the same
        // semantic names so the kernel layout stays backend-agnostic.
        let semantic_name =
            find_string_prop(node, "glyph").or_else(|| find_string_prop(node, "source"));
        if let Some(name) = semantic_name {
            if let Some(widget) = semantic_glyph_flutter_widget(name) {
                return Ok(format!("{pad}{widget}\n"));
            }
        }
        // Default to a placeholder symbol if no `source` keyword is
        // supplied. Authors can override via the source prop pointing
        // at a `Icons.<name>` identifier (we pass it through verbatim,
        // assuming the host imports `material.dart` which provides
        // the `Icons` constants).  For prop-name compatibility we
        // also accept `glyph` (the toolkit's preferred name) — keeps
        // the same .mll source working on both XAML and Flutter
        // without duplicating Icon declarations.
        let source = find_string_prop(node, "source")
            .or_else(|| find_string_prop(node, "glyph"))
            .unwrap_or("star");
        let safe = sanitize_dart_identifier(source);
        return Ok(format!("{pad}Icon(Icons.{safe})\n"));
    }

    // --- Routing: container primitives with generic flexbox-style children walks ---
    let container = match node.tag.as_str() {
        "Box" => Some("Container"),
        "Row" => Some("Row"),
        "Column" => Some("Column"),
        "Stack" => Some("Stack"),
        // UI28-1 / U29-D1 — HostTable structural sub-tags lower to
        // `Column` containers on Flutter (Flutter has no semantic
        // `<thead>`/`<tbody>`/`<colgroup>` equivalent — DataTable
        // requires up-front DataColumn/DataRow lists that don't fit
        // the For-driven dynamic shape). The visual nesting matches:
        // HostTableHead becomes a `Column` containing the header
        // Row; HostTableBody becomes a `Column` containing data
        // Rows; HostTableFoot the same. Accessibility for the table
        // semantics is the host's responsibility on Flutter today.
        "HostTableHead" | "HostTableBody" | "HostTableFoot" | "HostTableColGroup" => Some("Column"),
        _ => None,
    };
    // `Col` is a sub-tag of HostTableColGroup with no visual
    // contribution on Flutter (Flutter columns don't have a
    // pre-allocated `<col>` analog — column widths come from cell
    // intrinsic sizing or explicit SizedBox wrappers around cells).
    // Emit a zero-height SizedBox so the parent Column doesn't get
    // an empty slot that breaks rendering.
    if node.tag == "Col" {
        let pad = " ".repeat(indent);
        return Ok(format!(
            "{pad}const SizedBox.shrink() /* Col — Flutter colgroup has no visual analog */\n"
        ));
    }
    if let Some(widget) = container {
        return emit_container(node, widget, indent, part_styles, component, emits, ctx);
    }

    // --- Routing: meta-primitives ---
    //
    // `For` / `If` / `Else` are control-flow primitives (UI29 §3.1
    // and §3.2). They lower to Dart control-flow expressions wrapped
    // in a self-contained widget so they slot into any parent that
    // expects a single child:
    //
    //   - `For`        → `Column(children: <coll>.map(...).toList())`
    //   - `If`         → `(<cond>) ? <then> : const SizedBox.shrink()`
    //   - `If`+`Else`  → `(<cond>) ? <then> : <else>` — but the
    //     pairing happens at the parent (see `emit_paired_children`)
    //     because we need the next sibling to combine them.
    //
    // A bare `Else` here means the moslayout analyzer let through an
    // orphan (the validator should reject this, but the emitter is
    // defensive). Emit a comment widget so the file still compiles.
    //
    // The standalone routes here cover the case where `For`/`If`
    // appear as the SINGLE child of a parent (`child: For(...)`) —
    // i.e. when the parent's emitter calls `emit_widget_tree` directly
    // on the meta-primitive instead of routing through
    // `emit_paired_children`. The container walker
    // (`emit_container_paired_children`) handles the multi-child case
    // where `If`/`Else` siblings need to combine.
    match node.tag.as_str() {
        // Standalone `For` — NOT a direct child of a Row/Column
        // children-list (the spread path in `emit_paired_children`
        // handles that case). Falls back to a self-contained
        // `Column(children: …map().toList())` so it slots into any
        // single-child parent.
        "For" => return emit_for_dart(node, indent, part_styles, component, emits, ctx),
        // Standalone If — no Else paired. The container walker fuses
        // sibling pairs, so this branch fires only when If is the lone
        // child or the parent didn't pair-walk.
        "If" => return emit_if_dart(node, None, indent, part_styles, component, emits, ctx),
        "Else" => {
            return Ok(format!(
                "{pad}/* orphan Else — analyzer should have rejected this */ const SizedBox.shrink()\n"
            ));
        }
        _ => {}
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
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 2);
    let child_ctx = TableCtx {
        direct_row_child: widget == "Row",
        ..ctx
    };

    let style_props = node
        .part_name
        .as_deref()
        .and_then(|p| part_styles.get(p).map(String::as_str))
        .unwrap_or("");

    // Special case for Box → Container.
    if widget == "Container" {
        // Styled-cell path (Bug B). A Box that carries a `part_name`
        // with real styling — a border, background, height, text-align,
        // and/or `state-when-*` highlights — must lower to a `Container`
        // whose visual properties live in a `BoxDecoration` (a Container
        // can't take BOTH `color:` and `decoration:`; the background goes
        // INSIDE the decoration when a border is also present). The cell
        // also threads the column width (`columnWidths[<idx>]`) and folds
        // its `state-when-selected` / `state-when-editing` predicates into
        // conditional background + text colour. See `emit_styled_box`.
        if let Some(part) = node.part_name.as_deref() {
            if part_has_decoration(style_props) || node_has_state_when(node) {
                return emit_styled_box(
                    node,
                    part,
                    indent,
                    part_styles,
                    component,
                    emits,
                    child_ctx,
                );
            }
        }

        // Plain Box (no decorative styling) — keep the lightweight inline
        // form. A `Container` with no children just collapses to the box;
        // multiple children need a child Column wrapper since Container
        // only accepts one direct child.
        let style_args = style_to_container_args(style_props);
        if node.children.len() == 1 && style_args.is_empty() {
            // If the resolved Mosaic style has no Flutter-visible container
            // properties, this wrapper changes neither layout nor visuals.
            return emit_widget_tree(
                &node.children[0],
                indent,
                part_styles,
                component,
                emits,
                child_ctx,
            );
        }
        if node.children.is_empty() {
            return Ok(format!(
                "{pad}Container({})\n",
                style_to_container_args(style_props)
            ));
        }
        if node.children.len() == 1 {
            let child_src = emit_widget_tree(
                &node.children[0],
                indent + 2,
                part_styles,
                component,
                emits,
                child_ctx,
            )?;
            let child_src = child_src.trim_end_matches('\n');
            let style_prefix = if style_args.is_empty() {
                String::new()
            } else {
                format!("{inner_pad}{style_args},\n")
            };
            return Ok(format!(
                "{pad}Container(\n{style_prefix}{inner_pad}child: {child_src}\n{pad})\n",
            ));
        }
        let children = emit_paired_children(
            &node.children,
            indent + 4,
            part_styles,
            component,
            emits,
            child_ctx,
        )?;
        let style_prefix = if style_args.is_empty() {
            String::new()
        } else {
            format!("{inner_pad}{style_args},\n")
        };
        return Ok(format!(
            "{pad}Container(\n{style_prefix}{pad}  child: Column(children: [\n{children}{pad}  ])\n{pad})\n"
        ));
    }

    // Row / Column / Stack — direct Flutter widgets with a children list.
    // `emit_paired_children` handles the For-spread (Bug A): a `For`
    // child of this Row/Column SPREADS its mapped widgets into THIS
    // `children: [...]` list, so the Row lays cells across and the
    // Column stacks rows down — the parent controls orientation.
    let children = emit_paired_children(
        &node.children,
        indent + 4,
        part_styles,
        component,
        emits,
        child_ctx,
    )?;

    if children.is_empty() {
        return Ok(format!("{pad}const {widget}(children: [])\n"));
    }
    Ok(format!(
        "{pad}{widget}(\n{inner_pad}children: [\n{children}{inner_pad}],\n{pad})\n"
    ))
}

/// Walk a sibling list with two pieces of sibling-aware behaviour:
///
/// 1. An `If` followed immediately by `Else` is consumed as a pair
///    and lowered via [`emit_if_dart`] with the Else branch wired
///    in. The Else is skipped on the next loop step.
/// 2. Everything else delegates to [`emit_widget_tree`].
///
/// Returns the comma-joined widget list ready to splice into a
/// `children: [...]` argument. Each emitted child has the trailing
/// comma and newline appended by this function — the caller does
/// **not** add them again.
///
/// Mirrors `emit_children` in the SwiftUI backend; the routing logic
/// is identical because both languages model `If`/`Else` as
/// expression-form conditionals returning a view.
fn emit_paired_children(
    children: &[LayoutNode],
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let mut out = String::new();
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if child.tag == "If" {
            // Peek for `Else` immediately after; pair if present.
            let else_node = children.get(i + 1).filter(|n| n.tag == "Else");
            let if_src =
                emit_if_dart(child, else_node, indent, part_styles, component, emits, ctx)?;
            let trimmed = if_src.trim_end_matches('\n');
            out.push_str(trimmed);
            out.push_str(",\n");
            i += if else_node.is_some() { 2 } else { 1 };
            continue;
        }
        // Bug A — a `For` that is a DIRECT child of this container's
        // children-list SPREADS its mapped widgets into the parent's
        // `children: [ ... ]` using Dart's collection-spread (`...expr`),
        // instead of nesting a standalone `Column`. The parent (a Row vs
        // a Column) then controls orientation: header `Row` + cell `Row`
        // lay their `For`-mapped cells ACROSS; the outer body `Column`
        // stacks `For`-mapped rows DOWN. Nesting a `Column` here was the
        // bug that made the header A–E and each row's cells render as a
        // vertical stack.
        if child.tag == "For" {
            let spread = emit_for_spread(child, indent, part_styles, component, emits, ctx)?;
            out.push_str(spread.trim_end_matches('\n'));
            out.push_str(",\n");
            i += 1;
            continue;
        }
        // Orphan Else falls through to the standalone routing in
        // `emit_widget_tree`, which emits a documenting placeholder.
        let sub = emit_widget_tree(child, indent, part_styles, component, emits, ctx)?;
        let sub = sub.trim_end_matches('\n');
        out.push_str(sub);
        out.push_str(",\n");
        i += 1;
    }
    Ok(out)
}

/// Lower a `For` that sits directly inside a parent's `children: [...]`
/// list to a Dart **collection spread** — `...<coll>.asMap().entries
/// .map((entry) { final <idx> = entry.key; final <as> = entry.value;
/// return <body>; })` — so the mapped widgets flatten into the parent's
/// child list and the PARENT (Row vs Column) decides orientation.
///
/// This is the Bug-A fix. The standalone `Column(children: …map()
/// .toList())` form (in [`emit_for_dart`]) is kept ONLY for a `For` that
/// is not a direct child of a Row/Column children-list (single-child
/// parents, the layout root).
///
/// When this `For` is in *cell position* (its index binding names the
/// column, and the surrounding [`TableCtx`] carries a `columnWidths`
/// slot), the index is threaded into `ctx.cell_index` so the cell
/// `Container` deeper down picks up `width: columnWidths[<idx>]`.
fn emit_for_spread(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    let coll_expr = for_collection_expr(node);
    let as_name = find_keyword_prop(node, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());
    let index_name = find_keyword_prop(node, "index").map(to_camel_case_first_lower);

    // Thread the column index into the body so a cell Container can read
    // `columnWidths[<idx>]`. Only meaningful when an `index:` binding is
    // present and the table carries a column-widths slot.
    let body_ctx = TableCtx {
        cell_index: match (&index_name, ctx.column_widths_slot) {
            (Some(idx), Some(_)) => Some(idx.as_str()),
            _ => ctx.cell_index,
        },
        for_item: Some(as_name.as_str()),
        for_index: index_name.as_deref().or(ctx.for_index),
        ..ctx
    };

    let body_pad = indent + 4;
    let body = for_body_widget(node, body_pad, part_styles, component, emits, body_ctx)?;
    let body_trimmed = body.trim_start();
    let uses_item = dart_source_references_identifier(body_trimmed, &as_name);

    match index_name {
        Some(idx) if dart_source_references_identifier(body_trimmed, &idx) => {
            let item_binding = if uses_item {
                format!("{}final {as_name} = entry.value;\n", " ".repeat(indent + 2))
            } else {
                String::new()
            };
            Ok(format!(
                "{pad}...{coll}.asMap().entries.map((entry) {{\n\
                 {p2}final {idx} = entry.key;\n\
                 {item_binding}\
                 {p2}return {body};\n\
                 {pad}}})\n",
                coll = coll_expr,
                idx = idx,
                body = body_trimmed,
                p2 = " ".repeat(indent + 2),
            ))
        }
        Some(_) | None => Ok(format!(
            "{pad}...{coll}.map(({parameter}) => {body})\n",
            coll = coll_expr,
            parameter = if uses_item { as_name.as_str() } else { "_" },
            body = body_trimmed,
        )),
    }
}

fn dart_source_references_identifier(source: &str, identifier: &str) -> bool {
    source.match_indices(identifier).any(|(start, _)| {
        let before = source[..start].chars().next_back();
        let after = source[start + identifier.len()..].chars().next();
        !before.is_some_and(is_dart_identifier_char) && !after.is_some_and(is_dart_identifier_char)
    })
}

fn is_dart_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '$')
}

/// Resolve a `For`'s `each:` prop to its Dart collection expression.
///
/// `each:` may be a `SlotRef` (lowered to its camelCase name), an
/// `Expr` (passed through verbatim — author-controlled), or a UI29 §3.4
/// `Keyword` that names an enclosing `For`'s `as:`/`index:` binding
/// (also camelCased). Falls back to `<dynamic>[]` so the file
/// type-checks if validation was somehow skipped.
fn for_collection_expr(node: &LayoutNode) -> String {
    match node.props.iter().find(|p| p.name == "each") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Expr(text) => text.clone(),
            LayoutPropValue::Keyword(name) => to_camel_case_first_lower(name),
            _ => "<dynamic>[]".to_string(),
        },
        None => "<dynamic>[]".to_string(),
    }
}

/// Render a `For`'s body as a single Dart widget expression (indented
/// to `body_pad`), suitable as the `=> <body>` of an arrow map or the
/// `return <body>;` of a block map. Empty body → `const
/// SizedBox.shrink()`; single child → recurse; multiple children →
/// wrap in a `Column`.
fn for_body_widget(
    node: &LayoutNode,
    body_pad: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    if node.children.is_empty() {
        return Ok(format!("{}const SizedBox.shrink()", " ".repeat(body_pad)));
    }
    if node.children.len() == 1 {
        let child = &node.children[0];
        return Ok(
            emit_widget_tree(child, body_pad, part_styles, component, emits, ctx)?
                .trim_end_matches('\n')
                .to_string(),
        );
    }
    let inner = emit_paired_children(
        &node.children,
        body_pad + 4,
        part_styles,
        component,
        emits,
        ctx,
    )?;
    Ok(format!(
        "{}Column(children: [\n{}{}])",
        " ".repeat(body_pad),
        inner,
        " ".repeat(body_pad + 2),
    ))
}

/// Lower a UI29 `For` (§3.1) to a Dart `Column` whose `children:`
/// list is built by mapping over the iterated collection.
///
/// Three shapes, depending on which optional bindings are present:
///
/// | `For` shape                       | Dart output                                         |
/// |-----------------------------------|-----------------------------------------------------|
/// | `For ( each: X, as: y )`          | `Column(children: X.map((y) => <body>).toList())`   |
/// | `For ( each: X, as: y, index: i)` | `Column(children: X.asMap().entries.map((entry) {`  |
/// |                                   | `  final i = entry.key;`                            |
/// |                                   | `  final y = entry.value;`                          |
/// |                                   | `  return KeyedSubtree(key: ValueKey(i), child: <body>);`|
/// |                                   | `}).toList())`                                      |
///
/// The wrapping `KeyedSubtree` carrying `ValueKey(i)` is what gives
/// Flutter's element tree a stable identity for each iteration —
/// roughly the equivalent of React's `key={i}` (UI28-1 §5
/// performance property). Without it, Flutter's diff would mis-
/// associate widget state when rows reorder.
///
/// `each:` may be a `SlotRef` (lowered to its camelCase name) or an
/// `Expr` (passed through verbatim — author-controlled). A defensive
/// fall-back to `<dynamic>[]` keeps the file type-checking when the
/// validator somehow allowed a bad shape.
fn emit_for_dart(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    let coll_expr = for_collection_expr(node);

    // `as:` — required, always a Keyword per UI29 §3.1. Defensive
    // fallback to `item` matches SwiftUI.
    let as_name = find_keyword_prop(node, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());

    // `index:` — optional, always a Keyword when present.
    let index_name = find_keyword_prop(node, "index").map(to_camel_case_first_lower);
    let body_ctx = TableCtx {
        for_item: Some(as_name.as_str()),
        for_index: index_name.as_deref().or(ctx.for_index),
        ..ctx
    };

    // Body is the children of the For.
    let body_pad = indent + 6;
    let body = for_body_widget(node, body_pad, part_styles, component, emits, body_ctx)?;
    let body_trimmed = body.trim_start();
    let uses_item = dart_source_references_identifier(body_trimmed, &as_name);

    match index_name {
        Some(idx) => {
            // Indexed form: enumerate via .asMap().entries, bind key+value,
            // wrap the body in a KeyedSubtree so the index becomes the
            // element-tree stable key (UI28-1 §5).
            let item_binding = if uses_item {
                format!("{}final {as_name} = entry.value;\n", " ".repeat(indent + 2))
            } else {
                String::new()
            };
            Ok(format!(
                "{pad}Column(children: {coll}.asMap().entries.map((entry) {{\n\
                 {p2}final {idx} = entry.key;\n\
                 {item_binding}\
                 {p2}return KeyedSubtree(key: ValueKey({idx}), child: {body});\n\
                 {pad}}}).toList())\n",
                coll = coll_expr,
                idx = idx,
                body = body_trimmed,
                p2 = " ".repeat(indent + 2),
            ))
        }
        None => {
            // Plain form: single-arg arrow function returning the body.
            // No key — the author opted out by not binding `index:`.
            Ok(format!(
                "{pad}Column(children: {coll}.map(({parameter}) => {body}).toList())\n",
                coll = coll_expr,
                parameter = if uses_item { as_name.as_str() } else { "_" },
                body = body_trimmed,
            ))
        }
    }
}

/// Lower a UI29 `If` (§3.2) — optionally paired with a following
/// `Else` sibling — to a Dart ternary returning a Widget.
///
/// | shape                   | Dart                                       |
/// |-------------------------|--------------------------------------------|
/// | `If { then }`           | `(<cond>) ? <then> : const SizedBox.shrink()` |
/// | `If { then } Else { e }`| `(<cond>) ? <then> : <else>`              |
///
/// `<cond>` is the camelCased name for a `SlotRef`, or the
/// expression source text verbatim for an `Expr` (author-controlled),
/// wrapped in the generated `_mosaicTruthy` conversion so text, number,
/// list, record, and boolean values all produce a Dart `bool`.
/// The ternary's branches are recursed through [`emit_widget_tree`]
/// so nested `If`/`Else` still pairs naturally.
///
/// Empty branches collapse to `const SizedBox.shrink()` so the
/// ternary always returns a concrete Widget.
fn emit_if_dart(
    if_node: &LayoutNode,
    else_node: Option<&LayoutNode>,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // `when:` — required. `validate_if_node` guarantees SlotRef or
    // Expr; fall back to `false` so the file compiles even if
    // validation was skipped.
    let cond_expr = match if_node.props.iter().find(|p| p.name == "when") {
        Some(p) => match &p.value {
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Expr(text) => text.clone(),
            _ => "false".to_string(),
        },
        None => "false".to_string(),
    };

    let body_pad = indent + 2;
    let then_branch = render_branch(
        &if_node.children,
        body_pad,
        part_styles,
        component,
        emits,
        ctx,
    )?;
    let else_branch = match else_node {
        Some(en) => render_branch(&en.children, body_pad, part_styles, component, emits, ctx)?,
        None => "const SizedBox.shrink()".to_string(),
    };

    Ok(format!(
        "{pad}((_mosaicTruthy({cond})) ? {then_b} : {else_b})\n",
        cond = cond_expr,
        then_b = then_branch,
        else_b = else_branch,
    ))
}

/// Render a single conditional branch (the body of an `If` or `Else`)
/// as a single-Widget expression suitable as a ternary operand.
///
/// - Empty body → `const SizedBox.shrink()`
/// - Single child → recurse, trimming the trailing newline
/// - Multiple children → wrap in `Column(children: [...])`
fn render_branch(
    children: &[LayoutNode],
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    if children.is_empty() {
        return Ok("const SizedBox.shrink()".to_string());
    }
    if children.len() == 1 {
        let s = emit_widget_tree(&children[0], indent, part_styles, component, emits, ctx)?;
        return Ok(s.trim_end_matches('\n').trim_start().to_string());
    }
    let inner = emit_paired_children(children, indent + 2, part_styles, component, emits, ctx)?;
    Ok(format!(
        "Column(children: [\n{}{}])",
        inner,
        " ".repeat(indent),
    ))
}

/// Render a container's part-style props as inline Flutter
/// `Container` properties: `color: Color(0xFFRRGGBB), padding:
/// EdgeInsets.all(N)`, etc.
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
        "padding" => Some(format!(
            "padding: const EdgeInsets.all({})",
            parse_pixel_value(value)
        )),
        "width" => Some(format!("width: {}", parse_pixel_value(value))),
        "height" => Some(format!("height: {}", parse_pixel_value(value))),
        "min-height" => Some(format!(
            "constraints: const BoxConstraints(minHeight: {})",
            parse_pixel_value(value)
        )),
        "background-color" | "color" => css_color_to_dart(value).map(|c| format!("color: {c}")),
        _ => None,
    }
}

/// Parse a CSS-style pixel value (`"18px"` or `"18"`) to a bare
/// Dart numeric expression. Defaults to `0` on parse failure rather
/// than panicking — generated source still type-checks.
fn parse_pixel_value(s: &str) -> String {
    let s = s.trim().trim_end_matches("px");
    s.parse::<f64>()
        .map(|f| format!("{f}"))
        .unwrap_or_else(|_| "0".to_string())
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
// Styled cell / header-cell lowering (Bug B)
// =====================================================================

/// Split a joined `"key: value; key: value"` part-style string into a
/// `key → value` map. Both halves are trimmed; values keep their CSS
/// units (`22px`, `#3f3f46`) for downstream parsing.
fn parse_style_props(style_props: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for prop in style_props.split(';') {
        if let Some((k, v)) = prop.split_once(':') {
            let k = k.trim();
            if k.is_empty() {
                continue;
            }
            map.insert(k.to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    map
}

/// True when a part's base style declares anything that needs a real
/// `Container` decoration / sizing pass: a border, a background, an
/// explicit height/width, or a text alignment. Plain parts (no visual
/// styling) keep the lightweight inline form in [`emit_container`].
fn part_has_decoration(style_props: &str) -> bool {
    let m = parse_style_props(style_props);
    m.contains_key("border-width")
        || m.contains_key("border-color")
        || m.contains_key("background")
        || m.contains_key("background-color")
        || m.contains_key("height")
        || m.contains_key("width")
        || m.contains_key("text-align")
}

/// True when the node carries any `state-when-*` predicate prop — the
/// signal that this box wants conditional (selected / editing) styling
/// folded in even if its base part is otherwise plain.
fn node_has_state_when(node: &LayoutNode) -> bool {
    node.props.iter().any(|p| p.name.starts_with("state-when-"))
}

/// Map a mosstyle `text-align` value to a Flutter `Alignment` constant.
fn text_align_to_alignment(value: &str) -> &'static str {
    match value.trim() {
        "right" => "Alignment.centerRight",
        "center" => "Alignment.center",
        "left" => "Alignment.centerLeft",
        _ => "Alignment.centerLeft",
    }
}

/// One conditional state layer on a styled box: the Dart predicate text
/// plus that state's resolved background + text colours (either may be
/// absent if the `state X { }` block didn't set them).
struct StateLayer {
    cond: String,
    background: Option<String>,
    text_color: Option<String>,
}

/// Collect `state-when-<X>: ( expr )` props on a node, pairing each with
/// the resolved `{part}:{X}` style block's background + text colour.
/// Declaration order is preserved; the cell-styling fold treats the
/// FIRST matching layer as highest precedence (selected beats editing),
/// matching the `.msl` author order and the other backends.
fn collect_cell_state_layers(
    node: &LayoutNode,
    part: &str,
    part_styles: &HashMap<String, String>,
) -> Vec<StateLayer> {
    let mut layers = Vec::new();
    for prop in &node.props {
        let Some(state_name) = prop.name.strip_prefix("state-when-") else {
            continue;
        };
        let Some(state_style) = part_styles.get(&format!("{part}:{state_name}")) else {
            continue;
        };
        let cond = match &prop.value {
            LayoutPropValue::Expr(t) => t.clone(),
            LayoutPropValue::SlotRef(s) => to_camel_case_first_lower(s),
            LayoutPropValue::Keyword(k) => k.clone(),
            // EmitRef / Number / String can't be boolean predicates.
            _ => continue,
        };
        let m = parse_style_props(state_style);
        let background = m
            .get("background")
            .or_else(|| m.get("background-color"))
            .and_then(|v| css_color_to_dart(v));
        let text_color = m.get("color").and_then(|v| css_color_to_dart(v));
        layers.push(StateLayer {
            cond,
            background,
            text_color,
        });
    }
    layers
}

/// Build a nested-ternary Dart expression for a colour that flips with
/// state. `layers` are tried in order; each contributes
/// `(cond) ? <color> :` when it carries the requested colour. `base` is
/// the final fallback (`null` for "no fill"). Returns just `base` when
/// no layer supplies the colour, so the cheapest expression is emitted.
fn state_color_expr(
    layers: &[StateLayer],
    pick: impl Fn(&StateLayer) -> Option<&String>,
    base: &str,
) -> String {
    let mut acc = base.to_string();
    // Fold from the LAST layer to the FIRST so the first layer ends up
    // the outermost (highest-precedence) condition.
    for layer in layers.iter().rev() {
        if let Some(color) = pick(layer) {
            acc = format!(
                "_mosaicTruthy(( {} )) ? {} : {}",
                layer.cond.trim(),
                color,
                acc
            );
        }
    }
    acc
}

/// Lower a styled `Box [part]` (a spreadsheet cell or header-cell) to a
/// fully-decorated Flutter `Container`. This is the Bug-B fix.
///
/// Produces (cell example):
///
/// ```dart
/// Container(
///   width: columnWidths[c],
///   height: 22,
///   alignment: Alignment.centerRight,
///   padding: const EdgeInsets.symmetric(horizontal: 2),
///   decoration: BoxDecoration(
///     color: (( r == selectedRow && c == selectedCol )) ? const Color(0xFF264F78)
///          : (( r == editRow && c == editCol )) ? const Color(0xFF1F4F3F)
///          : null,
///     border: Border.all(color: const Color(0xFF3F3F46), width: 1),
///   ),
///   child: DefaultTextStyle.merge(
///     style: TextStyle(color: (( r == … )) ? const Color(0xFFFFFFFF) : const Color(0xFFCCCCCC)),
///     child: <inner>,
///   ),
/// )
/// ```
///
/// A `Container` cannot take BOTH `color:` and `decoration:`, so the
/// background ALWAYS rides inside `BoxDecoration(color: …)` here. The
/// per-state text colour is applied with `DefaultTextStyle.merge` so it
/// reaches the child whether the child is a bare `Text` or an
/// `If`/`Else` ternary (Text vs editing TextField).
fn emit_styled_box(
    node: &LayoutNode,
    part: &str,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let ip = " ".repeat(indent + 2);

    let style_props = part_styles.get(part).map(String::as_str).unwrap_or("");
    let base = parse_style_props(style_props);
    let layers = collect_cell_state_layers(node, part, part_styles);

    // --- Sizing / alignment / padding args ---------------------------
    let mut args: Vec<String> = Vec::new();

    // width: a discovered column width (`columnWidths[c]`) wins; else an
    // explicit base `width`.
    if let (Some(slot), Some(idx)) = (ctx.column_widths_slot, ctx.cell_index) {
        args.push(format!("width: {slot}[{idx}]"));
    } else if let Some(w) = base.get("width") {
        args.push(format!("width: {}", parse_pixel_value(w)));
    }
    if let Some(h) = base.get("height") {
        args.push(format!("height: {}", parse_pixel_value(h)));
    }
    if let Some(min_height) = base.get("min-height") {
        args.push(format!(
            "constraints: const BoxConstraints(minHeight: {})",
            parse_pixel_value(min_height)
        ));
    }
    if let Some(ta) = base.get("text-align") {
        args.push(format!("alignment: {}", text_align_to_alignment(ta)));
    }
    if let Some(p) = base.get("padding") {
        args.push(format!(
            "padding: const EdgeInsets.symmetric(horizontal: {})",
            parse_pixel_value(p)
        ));
    }

    // --- BoxDecoration: background (state-conditional) + border -------
    let base_bg = base
        .get("background")
        .or_else(|| base.get("background-color"))
        .and_then(|v| css_color_to_dart(v));
    let bg_expr = state_color_expr(
        &layers,
        |l| l.background.as_ref(),
        base_bg.as_deref().unwrap_or("null"),
    );
    let mut deco_parts: Vec<String> = vec![format!("color: {bg_expr}")];
    if let (Some(bc), Some(bw)) = (
        base.get("border-color").and_then(|v| css_color_to_dart(v)),
        base.get("border-width"),
    ) {
        deco_parts.push(format!(
            "border: Border.all(color: {bc}, width: {})",
            parse_pixel_value(bw)
        ));
    }
    args.push(format!(
        "decoration: BoxDecoration({})",
        deco_parts.join(", ")
    ));

    // --- Child, wrapped in a per-state text colour -------------------
    //
    // The Box [cell] body is an `If (is-editing) { HostInput } Else
    // { Text }` pair — exactly one rendered widget after fusing. Detect
    // the leading `If` (+ optional `Else`) and emit the single ternary
    // directly, so the cell's `alignment:` actually positions the
    // content (a `Column` wrapper would expand to fill and defeat it).
    // Other shapes fall back to single-child / Column wrapping.
    let inner_child = if node.children.is_empty() {
        format!("{ip}const SizedBox.shrink()")
    } else if node.children[0].tag == "If" {
        let else_node = node.children.get(1).filter(|n| n.tag == "Else");
        emit_if_dart(
            &node.children[0],
            else_node,
            indent + 4,
            part_styles,
            component,
            emits,
            ctx,
        )?
        .trim_end_matches('\n')
        .to_string()
    } else if node.children.len() == 1 {
        emit_widget_tree(
            &node.children[0],
            indent + 4,
            part_styles,
            component,
            emits,
            ctx,
        )?
        .trim_end_matches('\n')
        .to_string()
    } else {
        let kids = emit_paired_children(
            &node.children,
            indent + 6,
            part_styles,
            component,
            emits,
            ctx,
        )?;
        format!("{ip}Column(children: [\n{kids}{ip}])")
    };
    let inner_child = inner_child.trim_start();

    // Text colour: the part's own base `color`, else the sheet's
    // inherited `color` (threaded via [`TableCtx`]), with the
    // per-state overrides folded on top. A `TextStyle` whose `color:`
    // is a runtime ternary can't be `const`.
    let base_text = base
        .get("color")
        .and_then(|v| css_color_to_dart(v))
        .or_else(|| ctx.sheet_text_color.map(str::to_string))
        .unwrap_or_else(|| "null".to_string());
    let text_color_expr = state_color_expr(&layers, |l| l.text_color.as_ref(), &base_text);

    // Font family / size: the part's own, else the sheet's (the
    // VisiCalc monospace 12px lives on the `sheet` part, not the cell).
    let font_family = base
        .get("font-family")
        .map(String::as_str)
        .or(ctx.sheet_font_family);
    let font_size = base
        .get("font-size")
        .map(|v| parse_pixel_value(v))
        .or_else(|| ctx.sheet_font_size.map(str::to_string));

    let mut text_style_parts: Vec<String> = vec![format!("color: {text_color_expr}")];
    if let Some(ff) = font_family {
        text_style_parts.push(format!("fontFamily: \"{}\"", escape_dart_string(ff)));
    }
    if let Some(fs) = font_size {
        text_style_parts.push(format!("fontSize: {fs}"));
    }
    let child_expr = format!(
        "DefaultTextStyle.merge(style: TextStyle({}), child: {inner_child})",
        text_style_parts.join(", ")
    );
    args.push(format!("child: {child_expr}"));

    // --- Assemble ----------------------------------------------------
    let mut out = String::new();
    out.push_str(&format!("{pad}Container(\n"));
    for a in &args {
        out.push_str(&format!("{ip}{a},\n"));
    }
    out.push_str(&format!("{pad})\n"));
    Ok(out)
}

// =====================================================================
// Text + Image leaves
// =====================================================================

/// Lower a `Text` node to a `Text("...")` widget. Accepts the
/// `content` prop as a string literal, slot ref, or (UI28-1 / U29-D1)
/// an Expr that evaluates in the surrounding closure scope. Expr
/// passes verbatim into `Text(...)` so For-loop bindings like
/// `Text ( content: ( v ) )` reach the Flutter widget unchanged.
fn emit_text(node: &LayoutNode, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let text = if let Some(s) = find_string_prop(node, "content") {
        format!("Text(\"{}\")", escape_dart_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "content") {
        let camel = to_camel_case_first_lower(slot);
        format!("Text({camel})")
    } else if let Some(expr_text) = node
        .props
        .iter()
        .find(|p| p.name == "content")
        .and_then(|p| match &p.value {
            LayoutPropValue::Expr(t) => Some(t.as_str()),
            _ => None,
        })
    {
        // UI28-1 / U29-D1 — Expr content passes verbatim into Text so
        // surrounding For-loop bindings remain live.
        format!("Text({expr_text})")
    } else {
        "const Text(\"\")".to_string()
    };

    let hidden = matches!(find_prop_value(node, "a11y-role"), Some(LayoutPropValue::Keyword(value)) if value == "none")
        || matches!(find_prop_value(node, "a11y-hidden"), Some(LayoutPropValue::Keyword(value)) if value == "true");
    if hidden {
        return format!("{pad}ExcludeSemantics(child: {text})\n");
    }

    let label = match find_prop_value(node, "a11y-label") {
        Some(LayoutPropValue::String(value)) => Some(format!("\"{}\"", escape_dart_string(value))),
        Some(LayoutPropValue::SlotRef(slot)) => Some(to_camel_case_first_lower(slot)),
        _ => None,
    };
    let heading = matches!(find_prop_value(node, "a11y-role"), Some(LayoutPropValue::Keyword(value)) if value == "heading");
    if label.is_none() && !heading {
        return format!("{pad}{text}\n");
    }
    let mut args = Vec::new();
    if let Some(label) = label {
        args.push(format!("label: {label}"));
        args.push("excludeSemantics: true".to_string());
    }
    if heading {
        args.push("header: true".to_string());
    }
    args.push(format!("child: {text}"));
    format!("{pad}Semantics({})\n", args.join(", "))
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
    format!("{pad}Image.{factory}(\"{}\")\n", escape_dart_string(source))
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
    component: &str,
    emits: &[EmitDecl],
    direct_row_child: bool,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let field_pad = if direct_row_child {
        " ".repeat(indent + 2)
    } else {
        pad.clone()
    };
    // UI28-1 / U29-D1 — accept Expr in `value:` so
    // mosaic-pkg-grid v0.2.0's `HostInput ( value: ( v ) )` shape
    // reaches the TextField with the For-bound cell text. The Expr
    // is the literal Dart expression evaluated in the surrounding
    // closure's scope; verbatim pass-through matches Text content.
    let value_expr: String = if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else if let Some(s) = find_string_prop(node, "value") {
        format!("\"{}\"", escape_dart_string(s))
    } else if let Some(expr_text) =
        node.props
            .iter()
            .find(|p| p.name == "value")
            .and_then(|p| match &p.value {
                LayoutPropValue::Expr(t) => Some(t.clone()),
                _ => None,
            })
    {
        expr_text
    } else {
        "\"\"".to_string()
    };

    let mut out = String::new();
    if direct_row_child {
        writeln!(out, "{pad}Expanded(").unwrap();
        writeln!(out, "{field_pad}child: TextField(").unwrap();
    } else {
        writeln!(out, "{field_pad}TextField(").unwrap();
    }
    writeln!(
        out,
        "{field_pad}  controller: TextEditingController(text: {value_expr}),"
    )
    .unwrap();

    if let Some(p) = find_string_prop(node, "placeholder") {
        writeln!(
            out,
            "{field_pad}  decoration: InputDecoration(hintText: \"{}\"),",
            escape_dart_string(p)
        )
        .unwrap();
    }

    if let Some(read_only) = bool_prop_expression(node, "read-only")? {
        writeln!(out, "{field_pad}  readOnly: {read_only},").unwrap();
    }

    // onChange — wraps the new value in a dispatched event.
    //
    // The dispatched event subclass is named `<Component>Event<Case>`
    // — matching the sealed-class hierarchy `emit_widget_class`
    // generates at the top of the file (see also the
    // `dispatch: void Function(<Component>Event)` field).  Both
    // pieces — the component name and the case — are in scope here,
    // so we produce a real call.  v0.1.0 of the emitter wrote a
    // literal `/* TODO: ... */` placeholder in the dispatch argument
    // position, which broke compilation of every generated widget
    // that wired an onChange handler.
    //
    // Single named field `value: String` matches the .mil
    // declaration `emit on<Case> ( value : text )` used by VisiCalc's
    // FormulaBar.  Components whose `onChange` carries a different
    // payload shape will need to extend this codegen — for now any
    // alternate shape would produce a Dart compile error rather than
    // a silent TODO that ships at runtime.
    if let Some(emit_name) = find_emit_ref_prop(node, "onChange") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = host_input_event_args(emits, emit_name, "value")?;
        writeln!(
            out,
            "{field_pad}  onChanged: (value) => dispatch({component}Event{case}({args})),"
        )
        .unwrap();
    }
    if let Some(emit_name) = find_emit_ref_prop(node, "onCommit") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = host_input_event_args(emits, emit_name, "value")?;
        writeln!(
            out,
            "{field_pad}  onSubmitted: (value) => dispatch({component}Event{case}({args})),"
        )
        .unwrap();
    }
    writeln!(out, "{field_pad})").unwrap();
    if direct_row_child {
        writeln!(out, "{pad})").unwrap();
    }
    Ok(out)
}

/// Build the named event arguments supplied by a native text input callback.
/// A text input owns exactly one callback value, so zero- and one-payload
/// events are fully synthesised. For a malformed multi-payload binding we
/// still name every required constructor argument and fail loudly at runtime
/// instead of generating Dart that cannot compile.
fn host_input_event_args(
    emits: &[EmitDecl],
    emit_name: &str,
    value: &str,
) -> Result<String, PipelineEmitError> {
    let Some(emit) = emits.iter().find(|emit| emit.name == emit_name) else {
        // Retain the historical single-text-field fallback for direct emitter
        // callers that provide a layout without its interface emits.
        return Ok(format!("value: {value}"));
    };
    if emit.params.is_empty() {
        return Ok(String::new());
    }
    if emit.params.len() == 1 {
        let param = &emit.params[0];
        let field = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&field)?;
        let expression = match &param.r#type {
            EmitPayloadType::Text | EmitPayloadType::Color | EmitPayloadType::Component(_) => {
                value.to_string()
            }
            EmitPayloadType::Number => format!("double.tryParse({value}) ?? 0"),
            EmitPayloadType::Bool => format!("{value}.toLowerCase() == \"true\""),
        };
        return Ok(format!("{field}: {expression}"));
    }
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            Ok(format!(
                "{field}: /* TODO: payload */ throw UnimplementedError()"
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

/// `HostButton` → `ElevatedButton`. Label can be a string literal,
/// a slot ref, or empty; `disabled` toggles via `onPressed: null`.
fn emit_host_button(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let label_expr: String = match find_prop_value(node, "label") {
        Some(LayoutPropValue::String(s)) => format!("Text(\"{}\")", escape_dart_string(s)),
        Some(LayoutPropValue::SlotRef(slot)) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel)?;
            format!("Text({camel})")
        }
        Some(LayoutPropValue::Keyword(name)) => {
            let camel = to_camel_case_first_lower(name);
            validate_slot_or_field_name(&camel)?;
            format!("Text({camel})")
        }
        Some(LayoutPropValue::Expr(text)) => format!("Text({})", text.trim()),
        _ => "Text(\"\")".to_string(),
    };

    // onPressed callback.  `disabled: true` (compile-time keyword)
    // overrides with `null`.  Otherwise dispatch the bound onTap
    // event — now that `component` is threaded down (after the
    // FormulaBar fix), we can finally produce a real
    // `dispatch(<Component>Event<Case>())` call.  Buttons have no
    // inherent payload; row-scoped single-payload events are handled
    // by the current behavior note below.
    // Current behavior: zero-payload events keep the `EventCase()`
    // shape, while single text-like/number payloads borrow the nearest
    // `For` item/index when present.
    let callback: String = if let Some(emit_name) =
        find_emit_ref_prop(node, "onClick").or_else(|| find_emit_ref_prop(node, "onTap"))
    {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = emits
            .iter()
            .find(|e| e.name == *emit_name)
            .map(|e| host_button_event_args(e, ctx))
            .transpose()?
            .unwrap_or_default();
        format!("() => dispatch({component}Event{case}({args}))")
    } else {
        "() {}".to_string()
    };
    let on_pressed_expr = match bool_prop_expression(node, "disabled")?.as_deref() {
        Some("true") => "null".to_string(),
        Some("false") | None => callback,
        Some(disabled) => format!("{disabled} ? null : {callback}"),
    };

    let style_arg = host_button_style_arg(node, part_styles);
    Ok(format!(
        "{pad}ElevatedButton(onPressed: {on_pressed_expr}{style_arg}, child: {label_expr})\n"
    ))
}

fn host_button_event_args(emit: &EmitDecl, ctx: TableCtx) -> Result<String, PipelineEmitError> {
    if emit.params.is_empty() {
        return Ok(String::new());
    }
    if emit.params.len() == 1 {
        let param = &emit.params[0];
        let field = to_camel_case_first_lower(&param.name);
        validate_slot_or_field_name(&field)?;
        let expr = host_button_payload_expr(&param.r#type, ctx)
            .unwrap_or_else(|| "/* TODO: payload */ throw UnimplementedError()".to_string());
        return Ok(format!("{field}: {expr}"));
    }
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            Ok(format!(
                "{field}: /* TODO: payload */ throw UnimplementedError()"
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

fn host_button_payload_expr(t: &EmitPayloadType, ctx: TableCtx) -> Option<String> {
    match t {
        EmitPayloadType::Text | EmitPayloadType::Color | EmitPayloadType::Component(_) => {
            ctx.for_item.map(str::to_string)
        }
        EmitPayloadType::Number => ctx.for_index.map(str::to_string),
        EmitPayloadType::Bool => None,
    }
}

/// Lower a styled Mosaic button part into a Flutter `ButtonStyle`.
///
/// This intentionally covers the same conservative subset the component
/// packages already use: fill colour, foreground colour, border, corner
/// radius, and padding. Unknown CSS-shaped props stay ignored here rather
/// than leaking untranslated style text into Dart.
fn host_button_style_arg(node: &LayoutNode, part_styles: &HashMap<String, String>) -> String {
    let Some(part) = node.part_name.as_deref() else {
        return String::new();
    };
    let Some(style_props) = part_styles.get(part) else {
        return String::new();
    };

    let props = parse_style_props(style_props);
    let mut style_parts: Vec<String> = Vec::new();

    if let Some(color) = props
        .get("background")
        .or_else(|| props.get("background-color"))
        .and_then(|v| css_color_to_dart(v))
    {
        style_parts.push(format!("backgroundColor: WidgetStatePropertyAll({color})"));
    }
    if let Some(color) = props.get("color").and_then(|v| css_color_to_dart(v)) {
        style_parts.push(format!("foregroundColor: WidgetStatePropertyAll({color})"));
    }
    if let Some(padding) = props.get("padding").map(|v| parse_pixel_value(v)) {
        style_parts.push(format!(
            "padding: WidgetStatePropertyAll(const EdgeInsets.all({padding}))"
        ));
    }

    let border_color = props.get("border-color").and_then(|v| css_color_to_dart(v));
    let border_width = props.get("border-width").map(|v| parse_pixel_value(v));
    let border_radius = props.get("border-radius").map(|v| parse_pixel_value(v));
    let mut shape_args: Vec<String> = Vec::new();
    if let Some(radius) = border_radius {
        shape_args.push(format!("borderRadius: BorderRadius.circular({radius})"));
    }
    if border_color.is_some() || border_width.is_some() {
        let mut side_args: Vec<String> = Vec::new();
        if let Some(color) = border_color {
            side_args.push(format!("color: {color}"));
        }
        if let Some(width) = border_width {
            side_args.push(format!("width: {width}"));
        }
        shape_args.push(format!("side: BorderSide({})", side_args.join(", ")));
    }
    if !shape_args.is_empty() {
        style_parts.push(format!(
            "shape: WidgetStatePropertyAll(RoundedRectangleBorder({}))",
            shape_args.join(", ")
        ));
    }

    if style_parts.is_empty() {
        String::new()
    } else {
        format!(", style: ButtonStyle({})", style_parts.join(", "))
    }
}

/// `HostCheckbox` → `Checkbox`. Two-state for v1; the `indeterminate`
/// slot is accepted but ignored (Flutter `Checkbox` has a
/// `tristate: true` mode, but the visual is a dash — close enough for
/// a follow-up).
fn emit_host_checkbox(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
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

    // onToggle dispatch wiring.  Mirrors the HostInput onChange
    // pattern: look up the `onToggle` emit ref, derive
    // `<Component>Event<Case>(value: v)` where `v` is Flutter's
    // Checkbox's new-bool payload.  If no `onToggle` binding is
    // present, the callback is a no-op so the Checkbox still
    // renders interactive.
    let on_changed_body: String = if let Some(emit_name) = find_emit_ref_prop(node, "onToggle") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = emits
            .iter()
            .find(|emit| emit.name == *emit_name)
            .map(host_checkbox_event_args)
            .transpose()?
            .unwrap_or_default();
        format!("dispatch({component}Event{case}({args}))")
    } else {
        "/* no onToggle bound */".to_string()
    };
    let body = format!(
        "material.Checkbox(value: {checked_expr}, onChanged: (v) {{ {on_changed_body}; }})"
    );
    let inner = match label {
        Some(l) => format!("Row(children: [{body}, {l}])"),
        None => body,
    };
    Ok(format!("{pad}{inner}\n"))
}

fn host_checkbox_event_args(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            let value = match &param.r#type {
                EmitPayloadType::Bool => "v ?? false",
                EmitPayloadType::Number => "(v ?? false) ? 1 : 0",
                EmitPayloadType::Text | EmitPayloadType::Color => "(v ?? false).toString()",
                EmitPayloadType::Component(_) => "throw UnimplementedError()",
            };
            Ok(format!("{field}: {value}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

/// `HostRadio` → `Radio<String>`. Group coordination via `groupValue`
/// matches HTML's shared-`name` pattern; the host owns the
/// currently-selected value.
fn emit_host_radio(
    node: &LayoutNode,
    indent: usize,
    _part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
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

    // onSelect dispatch wiring.  Flutter's `Radio.onChanged(T?)`
    // fires with the newly-selected value; we forward it as the
    // `value` field of the dispatched event.
    let on_changed_body: String = if let Some(emit_name) = find_emit_ref_prop(node, "onSelect") {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = emits
            .iter()
            .find(|emit| emit.name == *emit_name)
            .map(host_radio_event_args)
            .transpose()?
            .unwrap_or_default();
        format!("dispatch({component}Event{case}({args}))")
    } else {
        "/* no onSelect bound */".to_string()
    };
    let body = format!(
        "material.Radio<String>(value: {value_expr}, groupValue: {group_value_expr}, onChanged: (v) {{ {on_changed_body}; }})"
    );
    let inner = match label {
        Some(l) => format!("Row(children: [{body}, {l}])"),
        None => body,
    };
    Ok(format!("{pad}{inner}\n"))
}

fn host_radio_event_args(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            let value = match &param.r#type {
                EmitPayloadType::Text | EmitPayloadType::Color => "v ?? \"\"",
                EmitPayloadType::Number => "num.tryParse(v ?? \"\") ?? 0",
                EmitPayloadType::Bool => "(v ?? \"\").isNotEmpty",
                EmitPayloadType::Component(_) => "throw UnimplementedError()",
            };
            Ok(format!("{field}: {value}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

/// Lower `HostSlider` to Flutter Material's native adjustable slider.
///
/// Flutter already exposes the portable split directly: `onChanged` fires for
/// every drag tick and `onChangeEnd` carries the final released value. Positive
/// Mosaic steps become Flutter divisions (intervals, rather than Compose's
/// interior-stop count); `step: 0` leaves `divisions` unset for continuous
/// motion.
fn emit_host_slider(
    node: &LayoutNode,
    indent: usize,
    component: &str,
    emits: &[EmitDecl],
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let number_expr = |name: &str, default: &str| -> Result<String, PipelineEmitError> {
        match find_prop_value(node, name) {
            Some(LayoutPropValue::SlotRef(slot)) => {
                let field = to_camel_case_first_lower(slot);
                validate_slot_or_field_name(&field)?;
                Ok(field)
            }
            Some(LayoutPropValue::Number(number)) => Ok(number.to_string()),
            Some(LayoutPropValue::Expr(expression)) => Ok(expression.clone()),
            _ => Ok(default.to_string()),
        }
    };
    let value = number_expr("value", "0")?;
    let min = number_expr("min", "0")?;
    let max = number_expr("max", "100")?;
    let disabled = bool_prop_expression(node, "disabled")?.unwrap_or_else(|| "false".into());

    let event_callback = |prop: &str| -> Result<Option<String>, PipelineEmitError> {
        let Some(emit_name) = find_emit_ref_prop(node, prop) else {
            return Ok(None);
        };
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        let args = emits
            .iter()
            .find(|emit| emit.name == *emit_name)
            .map(host_slider_event_args)
            .transpose()?
            .unwrap_or_default();
        Ok(Some(format!(
            "(value) {{ dispatch({component}Event{case}({args})); }}"
        )))
    };
    let on_change =
        event_callback("onChange")?.unwrap_or_else(|| "(value) { /* no onChange bound */ }".into());
    let on_commit = event_callback("onCommit")?;

    let divisions = match find_prop_value(node, "step") {
        Some(LayoutPropValue::Number(step)) if *step > 0.0 => {
            let range = match (find_prop_value(node, "min"), find_prop_value(node, "max")) {
                (Some(LayoutPropValue::Number(min)), Some(LayoutPropValue::Number(max))) => {
                    max - min
                }
                _ => 100.0,
            };
            Some(((range / step).round() as i64).max(1).to_string())
        }
        Some(LayoutPropValue::Number(_)) => None,
        Some(LayoutPropValue::SlotRef(_) | LayoutPropValue::Expr(_)) => {
            let step_expr = number_expr("step", "1")?;
            let intervals = format!(
                "((({max}).toDouble() - ({min}).toDouble()) / ({step_expr}).toDouble()).round()"
            );
            Some(format!(
                "({step_expr}).toDouble() > 0 ? ({intervals} < 1 ? 1 : {intervals}) : null"
            ))
        }
        _ => Some("100".to_string()),
    };

    let mut args = vec![
        format!("value: ({value}).toDouble()"),
        format!("min: ({min}).toDouble()"),
        format!("max: ({max}).toDouble()"),
        format!("onChanged: {disabled} ? null : {on_change}"),
    ];
    if let Some(on_commit) = on_commit {
        args.push(format!("onChangeEnd: {disabled} ? null : {on_commit}"));
    }
    if let Some(divisions) = divisions {
        args.push(format!("divisions: {divisions}"));
    }
    let slider = format!("material.Slider({})", args.join(", "));
    let output = match find_prop_value(node, "a11y-label") {
        Some(LayoutPropValue::String(label)) => format!(
            "Semantics(label: \"{}\", child: {slider})",
            escape_dart_string(label)
        ),
        Some(LayoutPropValue::SlotRef(slot)) => {
            let field = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&field)?;
            format!("Semantics(label: {field}, child: {slider})")
        }
        _ => slider,
    };
    Ok(format!("{pad}{output}\n"))
}

fn host_slider_event_args(emit: &EmitDecl) -> Result<String, PipelineEmitError> {
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            let value = match &param.r#type {
                EmitPayloadType::Number => "value",
                EmitPayloadType::Text | EmitPayloadType::Color => "value.toString()",
                EmitPayloadType::Bool => "value != 0",
                EmitPayloadType::Component(_) => "throw UnimplementedError()",
            };
            Ok(format!("{field}: {value}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

/// `HostScroll` → `SingleChildScrollView`. Multi-child case wraps
/// the children in a `Column`. The legacy Mosaic spec keeps
/// `HostScroll` direction-agnostic; Flutter's default is vertical
/// scroll which matches the most common use case.
fn emit_host_scroll(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    if node.children.is_empty() {
        return Ok(format!("{pad}const SingleChildScrollView()\n"));
    }
    if node.children.len() == 1 {
        let child = emit_widget_tree(
            &node.children[0],
            indent + 2,
            part_styles,
            component,
            emits,
            ctx,
        )?;
        let child = child.trim_end_matches('\n');
        return Ok(format!(
            "{pad}SingleChildScrollView(\n{pad}  child: {child},\n{pad})\n"
        ));
    }
    // Multi-child path. Use the paired walker so an `If`/`Else`
    // sibling pair (Cell-style conditionals inside a scroll viewport)
    // is consumed correctly.
    let children = emit_paired_children(
        &node.children,
        indent + 6,
        part_styles,
        component,
        emits,
        ctx,
    )?;
    Ok(format!(
        "{pad}SingleChildScrollView(\n{pad}  child: Column(\n{pad}    children: [\n{children}{pad}    ],\n{pad}  ),\n{pad})\n"
    ))
}

/// #13010: does this `HostDialog` node lower to a real native dialog on
/// the Flutter backend, or does it still fall back to the zero-size
/// placeholder? `modal: false` is the one case still unimplemented --
/// Flutter's `showDialog` is inherently modal (a full-screen barrier +
/// route), with no vanilla-Flutter equivalent to SwiftUI's `.popover`/
/// Qt's non-modal `Popup` short of a custom `Overlay`, which is out of
/// scope here. `modal: true` (the default, and the only value the
/// toolkit's own `Modal` component ever authors) gets a real dialog.
pub fn host_dialog_has_native_semantics(node: &LayoutNode) -> bool {
    !matches!(find_keyword_prop(node, "modal"), Some("false"))
}

/// `HostDialog` -> a declarative-triggered imperative `showDialog`,
/// wrapped in the shared `_MosaicDialogHost` `StatefulWidget` (emitted
/// once per file, see [`emit_dialog_helper`]). Flutter's `showDialog`
/// is an imperative call, not a widget that sits in the tree the way
/// SwiftUI's `.sheet` modifier or Compose's conditional composition
/// does -- `_MosaicDialogHost` bridges the two: it watches its `open`
/// property and calls `showDialog`/`Navigator.pop` from lifecycle
/// callbacks so the rest of this emitter can still treat `HostDialog`
/// as an ordinary declarative tree node.
///
/// `modal: false` is not implemented (see
/// [`host_dialog_has_native_semantics`]) -- it keeps the previous
/// zero-size placeholder rather than emitting a wrong-shaped dialog.
///
/// ## Property handling
///
/// | Moslayout prop        | Flutter                                          |
/// |---|---|
/// | `open: slot: x`       | `_MosaicDialogHost.open: x`                      |
/// | `title: "..."` / slot | `AlertDialog(title: Text(...))`                  |
/// | `dismiss-on-backdrop: false` | `barrierDismissible: false`               |
/// | `onClose: emit: onX`  | `onClose: () { dispatch(XEventX()); }`           |
/// | children              | `AlertDialog(content: ...)`                      |
///
/// ## Generated shape
///
/// ```dart
/// _MosaicDialogHost(
///   open: open,
///   onClose: () { dispatch(const XEventClose()); },
///   builder: (context) => AlertDialog(
///     title: Text(title),
///     content: Column(children: [ ... ]),
///   ),
/// )
/// ```
fn emit_host_dialog(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 2);
    let body_pad = " ".repeat(indent + 4);

    if !host_dialog_has_native_semantics(node) {
        return Ok(format!(
            "{pad}const SizedBox.shrink() /* TODO: HostDialog non-modal (modal: false) is not yet implemented */\n"
        ));
    }

    // `open: slot: x` -> the bare identifier. Unbound falls back to a
    // literal `false` so the file still type-checks (matches the
    // XAML/SwiftUI backends' identical fallback for an unbound open).
    let open_expr: String = match find_slot_ref_prop(node, "open") {
        Some(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel)?;
            camel
        }
        None => "false".to_string(),
    };

    // `dismiss-on-backdrop: false` -> `barrierDismissible: false`.
    // Anything else (including unset) keeps Flutter's own default of
    // `true`, so no attribute is emitted at all.
    let barrier_dismissible_attr =
        if matches!(find_keyword_prop(node, "dismiss-on-backdrop"), Some("false")) {
            format!("{inner_pad}barrierDismissible: false,\n")
        } else {
            String::new()
        };

    // `onClose: emit: onX` -> dispatch closure. Optional: a dialog
    // with no onClose still opens and closes (backdrop-dismissible by
    // default), it just doesn't notify the host.
    let on_close_attr = match find_emit_ref_prop(node, "onClose") {
        Some(emit_name) => {
            let case_name = pascalize(&strip_on_prefix(emit_name));
            validate_emit_name(&case_name)?;
            format!(
                "{inner_pad}onClose: () {{ dispatch(const {component}Event{case_name}()); }},\n"
            )
        }
        None => String::new(),
    };

    // `title: "..."` / `title: slot: x` -> `AlertDialog(title: Text(...))`.
    let title_attr = match find_string_prop(node, "title") {
        Some(literal) => format!(
            "{body_pad}title: Text(\"{}\"),\n",
            escape_dart_string(literal)
        ),
        None => match find_slot_ref_prop(node, "title") {
            Some(slot) => {
                let camel = to_camel_case_first_lower(slot);
                validate_slot_or_field_name(&camel)?;
                format!("{body_pad}title: Text({camel}),\n")
            }
            None => String::new(),
        },
    };

    // Children become the dialog's `content`. Same single-vs-multi
    // shape as `emit_host_scroll`: a lone child is passed directly,
    // several are wrapped in a `Column` via the paired walker (so an
    // `If`/`Else` sibling pair inside the dialog body is handled).
    let content_attr = if node.children.is_empty() {
        String::new()
    } else if node.children.len() == 1 {
        let child = emit_widget_tree(
            &node.children[0],
            indent + 6,
            part_styles,
            component,
            emits,
            ctx,
        )?;
        let child = child.trim_end_matches('\n');
        format!("{body_pad}content: {child},\n")
    } else {
        let children_pad = " ".repeat(indent + 8);
        let children = emit_paired_children(
            &node.children,
            indent + 8,
            part_styles,
            component,
            emits,
            ctx,
        )?;
        format!(
            "{body_pad}content: Column(\n{children_pad}mainAxisSize: MainAxisSize.min,\n{children_pad}children: [\n{children}{children_pad}],\n{body_pad}),\n"
        )
    };

    Ok(format!(
        "{pad}_MosaicDialogHost(\n{inner_pad}open: {open_expr},\n{barrier_dismissible_attr}{on_close_attr}{inner_pad}builder: (context) => AlertDialog(\n{title_attr}{content_attr}{inner_pad}),\n{pad})\n"
    ))
}

/// Shared `StatefulWidget` bridging a declarative `open: bool` to
/// Flutter's imperative `showDialog`/`Navigator` API. Emitted once per
/// file (gated on `uses_dialog`, mirroring [`emit_drag_helpers`]'s
/// `uses_drag` gate), reused by every `HostDialog` in that file.
///
/// - `open` flips false -> true: schedules `showDialog` on the next
///   frame (an `addPostFrameCallback`, since `showDialog` needs a
///   `BuildContext` already in the tree -- calling it synchronously
///   from `didUpdateWidget`/`initState` can race the current build).
/// - `open` flips true -> false while the dialog is still showing
///   (the host closed it via its own slot, not via backdrop-tap or an
///   in-dialog control): pops the route programmatically.
/// - Either dismissal path (backdrop tap or host-driven pop) resolves
///   `showDialog`'s returned `Future`, which is where `onClose` fires
///   -- exactly once per open/close cycle, regardless of which side
///   initiated the close.
fn emit_dialog_helper() -> String {
    r#"class _MosaicDialogHost extends StatefulWidget {
  final bool open;
  final bool barrierDismissible;
  final WidgetBuilder builder;
  final VoidCallback? onClose;
  const _MosaicDialogHost({
    required this.open,
    required this.builder,
    this.barrierDismissible = true,
    this.onClose,
  });
  @override
  State<_MosaicDialogHost> createState() => _MosaicDialogHostState();
}

class _MosaicDialogHostState extends State<_MosaicDialogHost> {
  bool _isShowing = false;

  @override
  void initState() {
    super.initState();
    if (widget.open) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _open());
    }
  }

  @override
  void didUpdateWidget(covariant _MosaicDialogHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.open && !_isShowing) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _open());
    } else if (!widget.open && _isShowing) {
      Navigator.of(context).maybePop();
    }
  }

  Future<void> _open() async {
    _isShowing = true;
    await showDialog<void>(
      context: context,
      barrierDismissible: widget.barrierDismissible,
      builder: widget.builder,
    );
    _isShowing = false;
    widget.onClose?.call();
  }

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}
"#
    .to_string()
}

/// `HostDraggable` and `HostDropTarget` lower through a small generated
/// runtime that keeps native pointer drag/drop and the keyboard interaction
/// on one payload path. The surrounding `_MosaicDragScope` is mounted once
/// per generated component instance by `emit_widget_class`.
fn emit_host_draggable(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let key = drag_text_prop_expression(node, "drag-key", "")?;
    let kind = drag_text_prop_expression(node, "drag-kind", "")?;
    let label = drag_text_prop_expression(node, "drag-label", &key)?;
    let disabled = bool_prop_expression(node, "drag-disabled")?
        .or(bool_prop_expression(node, "disabled")?)
        .unwrap_or_else(|| "false".to_string());
    let child = emit_container(
        node,
        "Container",
        indent + 4,
        part_styles,
        component,
        emits,
        ctx,
    )?;

    let mut callbacks = String::new();
    if let Some(call) = drag_event_dispatch(
        node,
        "onDragStart",
        component,
        emits,
        &[("key", &key), ("kind", &kind)],
    )? {
        writeln!(callbacks, "{pad}  onDragStart: () {{ {call}; }},").unwrap();
    }
    if let Some(call) = drag_event_dispatch(
        node,
        "onDragEnd",
        component,
        emits,
        &[("key", &key), ("kind", &kind), ("dropped", "dropped")],
    )? {
        writeln!(callbacks, "{pad}  onDragEnd: (dropped) {{ {call}; }},").unwrap();
    }

    Ok(format!(
        "{pad}_MosaicDraggable(\n\
         {pad}  data: _MosaicDragData(key: {key}, kind: {kind}, label: {label}),\n\
         {pad}  disabled: {disabled},\n\
         {callbacks}\
         {pad}  child:\n{child}\
         {pad})\n"
    ))
}

fn emit_host_drop_target(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let target_key = drag_text_prop_expression(node, "drop-key", "")?;
    let disabled = bool_prop_expression(node, "drop-disabled")?
        .or(bool_prop_expression(node, "disabled")?)
        .unwrap_or_else(|| "false".to_string());
    let accepts = drag_accepts_expression(node)?;
    let child = emit_container(
        node,
        "Container",
        indent + 4,
        part_styles,
        component,
        emits,
        ctx,
    )?;

    let enter_values = [("key", "data.key"), ("kind", "data.kind")];
    let hover_values = [
        ("key", "data.key"),
        ("kind", "data.kind"),
        ("targetKey", target_key.as_str()),
        ("position", "position"),
    ];
    let mut callbacks = String::new();
    for (prop, callback, values) in [
        (
            "onDragEnter",
            "onDragEnter: (data)",
            enter_values.as_slice(),
        ),
        (
            "onDragLeave",
            "onDragLeave: (data)",
            enter_values.as_slice(),
        ),
        (
            "onDropHover",
            "onDropHover: (data, position)",
            hover_values.as_slice(),
        ),
        (
            "onDrop",
            "onDrop: (data, position)",
            hover_values.as_slice(),
        ),
    ] {
        if let Some(call) = drag_event_dispatch(node, prop, component, emits, values)? {
            writeln!(callbacks, "{pad}  {callback} {{ {call}; }},").unwrap();
        }
    }

    let accepts_arg = accepts
        .map(|value| format!("{pad}  acceptsKinds: {value},\n"))
        .unwrap_or_default();
    Ok(format!(
        "{pad}_MosaicDropTarget(\n\
         {pad}  targetKey: {target_key},\n\
         {pad}  disabled: {disabled},\n\
         {accepts_arg}\
         {callbacks}\
         {pad}  child:\n{child}\
         {pad})\n"
    ))
}

fn drag_text_prop_expression(
    node: &LayoutNode,
    name: &str,
    fallback: &str,
) -> Result<String, PipelineEmitError> {
    let value = match find_prop_value(node, name) {
        Some(LayoutPropValue::String(value)) => {
            format!("\"{}\"", escape_dart_string(value))
        }
        Some(LayoutPropValue::SlotRef(slot)) | Some(LayoutPropValue::Keyword(slot)) => {
            let field = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&field)?;
            format!("{field}.toString()")
        }
        Some(LayoutPropValue::Expr(expression)) => {
            format!("({}).toString()", expression.trim())
        }
        Some(LayoutPropValue::Number(number)) => format!("\"{number}\""),
        Some(LayoutPropValue::EmitRef(_)) | None => {
            if fallback.starts_with('"') || fallback.ends_with(".toString()") {
                fallback.to_string()
            } else {
                format!("\"{}\"", escape_dart_string(fallback))
            }
        }
    };
    Ok(value)
}

fn drag_accepts_expression(node: &LayoutNode) -> Result<Option<String>, PipelineEmitError> {
    let Some(value) = find_prop_value(node, "accepts") else {
        // Historical Mosaic layouts omitted `accepts`; preserve that useful
        // accept-all behavior. An explicitly authored empty list remains an
        // empty list and accepts nothing, as UI35 specifies.
        return Ok(None);
    };
    let expression = match value {
        LayoutPropValue::String(kind) => {
            format!("const <String>[\"{}\"]", escape_dart_string(kind))
        }
        LayoutPropValue::SlotRef(slot) | LayoutPropValue::Keyword(slot) => {
            let field = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&field)?;
            field
        }
        LayoutPropValue::Expr(expression) => {
            format!("List<String>.from(({}) as Iterable)", expression.trim())
        }
        LayoutPropValue::Number(_) | LayoutPropValue::EmitRef(_) => "const <String>[]".to_string(),
    };
    Ok(Some(expression))
}

fn drag_event_dispatch(
    node: &LayoutNode,
    prop: &str,
    component: &str,
    emits: &[EmitDecl],
    values: &[(&str, &str)],
) -> Result<Option<String>, PipelineEmitError> {
    let Some(emit_name) = find_emit_ref_prop(node, prop) else {
        return Ok(None);
    };
    let case = pascalize(&strip_on_prefix(emit_name));
    validate_emit_name(&case)?;
    let Some(emit) = emits.iter().find(|emit| emit.name == emit_name) else {
        return Ok(Some(format!("dispatch({component}Event{case}())")));
    };
    let args = emit
        .params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            let expression = values
                .iter()
                .find_map(|(name, value)| (*name == field).then_some(*value))
                .map(ToString::to_string)
                .unwrap_or_else(|| match &param.r#type {
                    EmitPayloadType::Text | EmitPayloadType::Color => "\"\"".to_string(),
                    EmitPayloadType::Number => "0".to_string(),
                    EmitPayloadType::Bool => "false".to_string(),
                    EmitPayloadType::Component(_) => "throw UnimplementedError()".to_string(),
                });
            Ok(format!("{field}: {expression}"))
        })
        .collect::<Result<Vec<_>, PipelineEmitError>>()?
        .join(", ");
    Ok(Some(format!("dispatch({component}Event{case}({args}))")))
}

fn emit_native_data_table(
    shape: FlutterDataTableShape<'_>,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let field_pad = " ".repeat(indent + 2);
    let closure_pad = " ".repeat(indent + 4);
    let binding_pad = " ".repeat(indent + 6);
    let return_pad = " ".repeat(indent + 6);

    let header_collection = for_collection_expr(shape.header_cells);
    let header_item = find_keyword_prop(shape.header_cells, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());
    let header_index =
        find_keyword_prop(shape.header_cells, "index").map(to_camel_case_first_lower);
    let header_ctx = TableCtx {
        cell_index: header_index
            .as_deref()
            .filter(|_| ctx.column_widths_slot.is_some()),
        for_item: Some(header_item.as_str()),
        for_index: header_index.as_deref().or(ctx.for_index),
        direct_row_child: false,
        ..ctx
    };
    let header_widget = for_body_widget(
        shape.header_cells,
        indent + 8,
        part_styles,
        component,
        emits,
        header_ctx,
    )?;
    let header_widget = header_widget.trim();

    let row_collection = for_collection_expr(shape.body_rows);
    let row_item = find_keyword_prop(shape.body_rows, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());
    let row_index = find_keyword_prop(shape.body_rows, "index").map(to_camel_case_first_lower);
    let cell_collection = for_collection_expr(shape.body_cells);
    let cell_item = find_keyword_prop(shape.body_cells, "as")
        .map(to_camel_case_first_lower)
        .unwrap_or_else(|| "item".to_string());
    let cell_index = find_keyword_prop(shape.body_cells, "index").map(to_camel_case_first_lower);
    let cell_ctx = TableCtx {
        cell_index: cell_index
            .as_deref()
            .filter(|_| ctx.column_widths_slot.is_some()),
        for_item: Some(cell_item.as_str()),
        for_index: cell_index
            .as_deref()
            .or(row_index.as_deref())
            .or(ctx.for_index),
        direct_row_child: false,
        ..ctx
    };
    let cell_widget = for_body_widget(
        shape.body_cells,
        indent + 12,
        part_styles,
        component,
        emits,
        cell_ctx,
    )?;
    let cell_widget = cell_widget.trim();

    let mut out = String::new();
    writeln!(
        out,
        "{pad}(({header_collection}).isEmpty ? const SizedBox.shrink() : DataTable("
    )
    .unwrap();
    match header_index
        .as_deref()
        .filter(|index| dart_source_references_identifier(header_widget, index))
    {
        Some(index) => {
            writeln!(
                out,
                "{field_pad}columns: {header_collection}.asMap().entries.map((mosaicColumnEntry) {{"
            )
            .unwrap();
            writeln!(out, "{binding_pad}final {index} = mosaicColumnEntry.key;").unwrap();
            writeln!(
                out,
                "{binding_pad}final {header_item} = mosaicColumnEntry.value;"
            )
            .unwrap();
            writeln!(
                out,
                "{return_pad}return DataColumn(label: {header_widget});"
            )
            .unwrap();
            writeln!(out, "{closure_pad}}}).toList(),").unwrap();
        }
        None => {
            writeln!(
                out,
                "{field_pad}columns: {header_collection}.map(({header_item}) => DataColumn(label: {header_widget})).toList(),"
            )
            .unwrap();
        }
    }

    match row_index {
        Some(index) => {
            writeln!(
                out,
                "{field_pad}rows: {row_collection}.asMap().entries.map((mosaicRowEntry) {{"
            )
            .unwrap();
            writeln!(out, "{binding_pad}final {index} = mosaicRowEntry.key;").unwrap();
            writeln!(out, "{binding_pad}final {row_item} = mosaicRowEntry.value;").unwrap();
            writeln!(out, "{return_pad}return DataRow(").unwrap();
            writeln!(out, "{return_pad}  key: ValueKey({index}),").unwrap();
        }
        None => {
            writeln!(out, "{field_pad}rows: {row_collection}.map(({row_item}) {{").unwrap();
            writeln!(out, "{return_pad}return DataRow(").unwrap();
        }
    }

    match cell_index
        .as_deref()
        .filter(|index| dart_source_references_identifier(cell_widget, index))
    {
        Some(index) => {
            writeln!(
                out,
                "{return_pad}  cells: {cell_collection}.asMap().entries.map((mosaicCellEntry) {{"
            )
            .unwrap();
            writeln!(out, "{return_pad}    final {index} = mosaicCellEntry.key;").unwrap();
            writeln!(
                out,
                "{return_pad}    final {cell_item} = mosaicCellEntry.value;"
            )
            .unwrap();
            writeln!(out, "{return_pad}    return DataCell({cell_widget});").unwrap();
            writeln!(out, "{return_pad}  }}).toList(),").unwrap();
        }
        None => {
            writeln!(
                out,
                "{return_pad}  cells: {cell_collection}.map(({cell_item}) => DataCell({cell_widget})).toList(),"
            )
            .unwrap();
        }
    }
    writeln!(out, "{return_pad});").unwrap();
    writeln!(out, "{closure_pad}}}).toList(),").unwrap();
    writeln!(out, "{pad}))").unwrap();
    Ok(out)
}

/// `HostTable` lowers the canonical UI31/Grid structure to Flutter's native
/// `DataTable`. Other shapes keep the visual fallback and remain degradations
/// in the native-complete report.
///
/// UI31 §3.2 RTL contract — when authored with a `dir:` prop, the
/// `DataTable` is wrapped in `Directionality(textDirection: ...,
/// child: ...)`. Flutter's directionality is enum-typed
/// (`TextDirection.ltr` / `TextDirection.rtl`) and *cannot* be a
/// dynamic string, so the lowering has three shapes:
///
/// | Source                 | Emits                                                              |
/// |------------------------|--------------------------------------------------------------------|
/// | `dir: ltr`             | `Directionality(textDirection: TextDirection.ltr, child: …)`       |
/// | `dir: rtl`             | `Directionality(textDirection: TextDirection.rtl, child: …)`       |
/// | `dir: auto`            | (no wrap — Flutter has no auto; inherit ambient `Directionality`)  |
/// | `dir: slot: layoutDir` | `Directionality(textDirection: layoutDir, child: …)` — slot is a   |
/// |                        | Dart expression that must evaluate to `TextDirection`              |
/// | unknown keyword        | (no wrap — drops silently per the allow-list security gate)        |
///
/// The allow-list (`ltr` / `rtl` / `auto`) is the security gate: an
/// attacker-controlled keyword can't sneak a `child: pwn()` payload
/// into the generated source because it never reaches the format
/// string. Slot refs go through `is_safe_dart_identifier` so they
/// can't either.
fn emit_host_table(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    parent_ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // Discover the column-widths slot from the `HostTableColGroup >
    // For (each: slot: …) { Col }` shape and thread it down so each
    // cell `Container` renders at a stable column width
    // (`columnWidths[<idx>]`). `None` → cells size to content (the
    // pre-fix behaviour); other backends thread the same fact.
    let column_widths_slot = extract_column_widths_slot(node);

    // The HostTable's own (`sheet`) part carries the inherited text
    // colour + monospace font. Thread them so cells / header-cells fall
    // back to the sheet's `color` / `font-family` / `font-size` rather
    // than `null` (which would inherit whatever the ambient theme says).
    let sheet_style = node
        .part_name
        .as_deref()
        .and_then(|p| part_styles.get(p).map(String::as_str))
        .map(parse_style_props)
        .unwrap_or_default();
    let sheet_text_color = sheet_style.get("color").and_then(|v| css_color_to_dart(v));
    let sheet_font_family = sheet_style.get("font-family").cloned();
    let sheet_font_size = sheet_style.get("font-size").map(|v| parse_pixel_value(v));

    let ctx = TableCtx {
        column_widths_slot: column_widths_slot.as_deref(),
        cell_index: parent_ctx.cell_index,
        for_item: parent_ctx.for_item,
        for_index: parent_ctx.for_index,
        sheet_text_color: sheet_text_color.as_deref(),
        sheet_font_family: sheet_font_family.as_deref(),
        sheet_font_size: sheet_font_size.as_deref(),
        direct_row_child: false,
    };

    let table_body = if let Some(shape) = flutter_data_table_shape(node) {
        emit_native_data_table(shape, 0, part_styles, component, emits, ctx)?
            .trim()
            .to_string()
    } else {
        let body_inner = if node.children.is_empty() {
            format!("{}const SizedBox.shrink()\n", " ".repeat(indent + 2))
        } else {
            emit_paired_children(
                &node.children,
                indent + 4,
                part_styles,
                component,
                emits,
                ctx,
            )?
        };
        format!(
            "Column(\n{p}children: [\n{body}{p}],\n{pad})",
            p = " ".repeat(indent + 2),
            body = body_inner,
            pad = pad,
        )
    };
    let table_body = table_body.as_str();

    // Resolve the directionality expression. `None` means: do not
    // wrap (either no `dir` prop, or `dir: auto`, or unknown keyword).
    let dir_expr: Option<String> = if let Some(slot) = find_slot_ref_prop(node, "dir") {
        let camel = to_camel_case_first_lower(slot);
        if is_safe_dart_identifier(&camel) {
            Some(camel)
        } else {
            None
        }
    } else if let Some(kw) = find_keyword_prop(node, "dir") {
        match kw {
            "ltr" => Some("TextDirection.ltr".to_string()),
            "rtl" => Some("TextDirection.rtl".to_string()),
            // `auto` is the spec-mandated keyword for "let the host
            // decide". Flutter has no enum value for it — the right
            // behaviour is to NOT wrap so the ambient Directionality
            // (from `MaterialApp` / explicit ancestors) flows through.
            "auto" => None,
            _ => None,
        }
    } else {
        None
    };

    if let Some(dir) = dir_expr {
        let inner = " ".repeat(indent + 2);
        return Ok(format!(
            "{pad}Directionality(\n\
             {inner}textDirection: {dir},\n\
             {inner}child: {table_body},\n\
             {pad})\n"
        ));
    }

    Ok(format!("{pad}{table_body}\n"))
}

// =====================================================================
// UI29-4 host primitives
//
// Three primitives promoted in UI29-4 (kernel positions 19/20/21):
//
// - `HostLink`        → `InkWell` wrapping `Text`. Flutter has no
//                       built-in URL launcher; the standard idiom is
//                       the `url_launcher` package's `launchUrl(...)`.
//                       We emit an `InkWell(onTap: () { /* TODO:
//                       launchUrl */ }, child: Text(...))` so the
//                       widget renders + responds to taps today; the
//                       host wires `launchUrl` in. When `external:
//                       false` is set, no URL-launch comment is
//                       emitted — the host is expected to handle
//                       routing via the `onActivate` dispatch.
// - `HostTooltip`     → `Tooltip(message:, child:)`. Flutter's
//                       built-in tooltip widget handles hover (web /
//                       desktop) + long-press (mobile) automatically.
// - `HostNumberInput` → `TextField(keyboardType: TextInputType.number,
//                       ...)`. The `inputFormatters` list with
//                       `FilteringTextInputFormatter.digitsOnly` is
//                       skipped in v1 because it bans the decimal
//                       point — authors who want integer-only entry
//                       can wrap the generated widget; the default
//                       allows decimals (matching the spec's
//                       `step: 0.01` default-for-decimal note).
// =====================================================================

/// `HostLink` (kernel primitive #19, UI29-4) → `InkWell` wrapping a
/// `Text`. Material's `InkWell` gives a tap ripple + hover cursor on
/// desktop/web, the closest stock-Flutter analogue of a hyperlink.
///
/// Actual URL launching requires the `url_launcher` package, which is
/// not a Flutter SDK dependency. To keep this emitter zero-deps, we
/// emit a `/* TODO: launchUrl */` comment in the `onTap` callback;
/// hosts wire `launchUrl(Uri.parse(href))` (or their preferred router
/// for `external: false`) by importing the package. The text and the
/// `onActivate` dispatch (if bound) are wired today.
///
/// ## Security
///
/// Both `href` and `label` are escaped through `escape_dart_string`
/// (handles `\`, `"`, `$` — the latter critical because Dart
/// interpolates `$ident` inside double-quoted strings). The
/// `external` and `target` keywords are validated to a small allow-
/// list before being interpolated into comments, so a malicious
/// keyword like `false*/dispatch(evil())/*` can't terminate the
/// `/* ... */` block-comment early.
/// #13052: does `href` carry an explicit URI scheme outside the
/// `http`/`https`/`mailto` allowlist? A relative reference (no scheme
/// at all -- `"#"`, `"/about"`) returns `false`. Only a *present,
/// disallowed* scheme (`javascript:`, `data:`, `intent:`, a custom
/// protocol handler, ...) returns `true`. A colon appearing after a
/// `/`, `?`, or `#` is data, not a scheme separator.
///
/// Security-review hardening: normalizes exactly like the WHATWG URL
/// parser does before looking for a scheme -- trims leading/trailing
/// C0-control-or-space and strips every embedded tab/CR/LF. Without
/// this, `" javascript:alert(1)"` or `"java\tscript:alert(1)"` fails
/// the alphabetic-first-character check below and gets classified as
/// "no scheme, therefore a safe relative reference." A first review
/// pass of #13052 missed this; caught and fixed in the same PR.
fn has_disallowed_uri_scheme(href: &str) -> bool {
    let normalized: String = href
        .trim_matches(|c: char| (c as u32) <= 0x20)
        .chars()
        .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
        .collect();
    let href = normalized.as_str();
    let Some(colon) = href.find(':') else {
        return false;
    };
    let prefix = &href[..colon];
    if prefix.is_empty() || prefix.contains(['/', '?', '#']) {
        return false;
    }
    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        return false;
    }
    const ALLOWED: [&str; 3] = ["http", "https", "mailto"];
    !ALLOWED.iter().any(|s| s.eq_ignore_ascii_case(prefix))
}

fn emit_host_link(
    node: &LayoutNode,
    indent: usize,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // external — keyword allow-list. Defaults to `true` (open in OS
    // browser via url_launcher). `false` means in-app routing; host
    // handles via onActivate, no `launchUrl` comment is emitted.
    let external = find_keyword_prop(node, "external")
        .map(|v| !matches!(v, "false")) // anything other than "false" → true
        .unwrap_or(true);

    // href — slot ref takes priority over literal (slot refs are
    // identifiers we validated upstream; literals get escape_dart_string).
    // #13052: reject a literal that carries an explicit disallowed
    // scheme when `external` can reach the eventual `launchUrl` call
    // -- no escaping makes an unsafe scheme safe. `external: false`
    // never reaches that call (dispatch-only), so a routing
    // placeholder like `href: "#"` stays valid there.
    let href_expr: String = if let Some(slot) = find_slot_ref_prop(node, "href") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else if let Some(s) = find_string_prop(node, "href") {
        if external && has_disallowed_uri_scheme(s) {
            return Err(PipelineEmitError::UnsafeUriScheme(s.to_string()));
        }
        format!("\"{}\"", escape_dart_string(s))
    } else {
        "\"\"".to_string()
    };

    // label — same slot-ref-first preference.
    let label_expr: String = if let Some(s) = find_string_prop(node, "label") {
        format!("Text(\"{}\")", escape_dart_string(s))
    } else if let Some(slot) = find_slot_ref_prop(node, "label") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        format!("Text({camel})")
    } else if let Some(keyword) = find_keyword_prop(node, "label") {
        let camel = to_camel_case_first_lower(keyword);
        validate_slot_or_field_name(&camel)?;
        format!("Text({camel})")
    } else {
        "Text(\"\")".to_string()
    };

    // target — keyword allow-list (defensive). Maps to comment-only
    // hint today; Flutter's url_launcher mode is host-controlled.
    let target = match find_keyword_prop(node, "target").unwrap_or("same") {
        "same" | "new-tab" | "parent" | "top" => {
            find_keyword_prop(node, "target").unwrap_or("same")
        }
        _ => "same",
    };

    // Sanitize href for use inside a `/* ... */` block comment.
    //
    // SECURITY: Dart's block-comment tokenizer is greedy and does NOT
    // respect string-literal quotes inside the comment — the first
    // `*/` terminates the comment regardless. So an href like
    // `x*/Future.delayed(...);/*` wrapped in `"..."` and spliced into
    // `/* TODO: launchUrl(Uri.parse("x*/Future...;/*")) */` would let
    // the injected code run inside the onTap closure.
    //
    // `escape_dart_string` (which produced `href_expr`) does NOT
    // escape `*/` because that sequence is not a Dart string-escape
    // concern. Here we additionally replace `*/` with `*/` —
    // inside a Dart string literal, `/` decodes to `/`, so the
    // runtime URL is unchanged, but the comment-terminator sequence
    // is broken at the source level. Regression test:
    // `host_link_with_comment_terminator_in_href_is_neutralised`.
    let href_in_comment = href_expr.replace("*/", "*\\u002f");

    // onActivate dispatch (optional).  Real call now that
    // `component` is threaded down: the host receives the resolved
    // href in the dispatched event.  Sealed-class hierarchy at the
    // top of the generated file already declares the
    // `<Component>Event<Case>({required String href})` constructor.
    let on_activate_call: Option<String> =
        if let Some(emit_name) = find_emit_ref_prop(node, "onActivate") {
            let case = pascalize(&strip_on_prefix(emit_name));
            validate_emit_name(&case)?;
            let args = emits
                .iter()
                .find(|emit| emit.name == *emit_name)
                .map(|emit| host_link_event_args(emit, &href_expr, ctx))
                .transpose()?
                .unwrap_or_default();
            Some(format!("dispatch({component}Event{case}({args}));"))
        } else {
            None
        };

    // Compose the onTap body. Two TODO comments in the external case
    // (url_launcher + dispatch), one in the internal case (dispatch
    // only). Both comments are block-style `/* ... */`; `href_in_comment`
    // has its `*/` sequences neutralised. The `target`/`external`
    // keywords passed through grammar-level validation upstream + an
    // allow-list above — no injection vector.
    let on_tap_body = match (external, on_activate_call.as_deref()) {
        (true, Some(call)) => format!(
            "() {{ /* TODO: launchUrl(Uri.parse({href_in_comment})) — target={target} */ {call} }}"
        ),
        (true, None) => format!(
            "() {{ /* TODO: launchUrl(Uri.parse({href_in_comment})) — target={target} */ }}"
        ),
        (false, Some(call)) => format!("() {{ {call} }}"),
        (false, None) => "() {}".to_string(),
    };

    Ok(format!(
        "{pad}InkWell(onTap: {on_tap_body}, child: {label_expr})\n"
    ))
}

fn host_link_event_args(
    emit: &EmitDecl,
    href_expr: &str,
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    emit.params
        .iter()
        .map(|param| {
            let field = to_camel_case_first_lower(&param.name);
            validate_slot_or_field_name(&field)?;
            let value = match &param.r#type {
                EmitPayloadType::Number => ctx.for_index.unwrap_or("0").to_string(),
                EmitPayloadType::Text | EmitPayloadType::Color => {
                    if field == "href" {
                        href_expr.to_string()
                    } else {
                        ctx.for_item.unwrap_or(href_expr).to_string()
                    }
                }
                EmitPayloadType::Bool => "false".to_string(),
                EmitPayloadType::Component(_) => "throw UnimplementedError()".to_string(),
            };
            Ok(format!("{field}: {value}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(", "))
}

/// `HostTooltip` (kernel primitive #20, UI29-4) → `Tooltip(message:,
/// child: )`. Flutter's built-in `Tooltip` handles hover (desktop /
/// web) and long-press (mobile) triggers, and renders above other
/// content via the overlay layer.
///
/// The single-child shape is enforced by the IR — the spec defines
/// HostTooltip's `target` as "the element the tooltip annotates,
/// passed as the single child of HostTooltip." We emit
/// `SizedBox.shrink()` when no child is present (degenerate case;
/// shouldn't happen if the .mil declared `target` as required).
///
/// ## Security
///
/// `text` is escaped through `escape_dart_string` before splicing
/// into the `"..."` literal. Slot-ref form passes a validated
/// identifier; no interpolation through unvalidated input is
/// possible.
fn emit_host_tooltip(
    node: &LayoutNode,
    indent: usize,
    part_styles: &HashMap<String, String>,
    component: &str,
    emits: &[EmitDecl],
    ctx: TableCtx,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let inner_pad = " ".repeat(indent + 2);

    // text — slot ref or literal. Slot ref bypasses escaping (it's an
    // identifier validated by validate_slot_or_field_name); literal
    // gets escape_dart_string.
    let message_expr: String = if let Some(slot) = find_slot_ref_prop(node, "text") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        camel
    } else if let Some(s) = find_string_prop(node, "text") {
        format!("\"{}\"", escape_dart_string(s))
    } else {
        "\"\"".to_string()
    };

    let child_src: String = if node.children.is_empty() {
        format!("{inner_pad}const SizedBox.shrink()\n")
    } else if node.children.len() == 1 {
        emit_widget_tree(
            &node.children[0],
            indent + 2,
            part_styles,
            component,
            emits,
            ctx,
        )?
    } else {
        // Multiple children — wrap in Column. Shouldn't happen for a
        // spec-conformant HostTooltip but we handle it defensively
        // (and through the paired walker so `If`/`Else` still fuses).
        let children = emit_paired_children(
            &node.children,
            indent + 6,
            part_styles,
            component,
            emits,
            ctx,
        )?;
        format!(
            "{inner_pad}Column(\n{inner_pad}  children: [\n{children}{inner_pad}  ],\n{inner_pad})\n"
        )
    };
    let child_src = child_src.trim_end_matches('\n');

    Ok(format!(
        "{pad}material.Tooltip(\n{inner_pad}message: {message_expr},\n{inner_pad}child: {child_src},\n{pad})\n"
    ))
}

/// `HostNumberInput` (kernel primitive #21, UI29-4) → `TextField`
/// with `keyboardType: TextInputType.number`. The mobile-keypad
/// surfacing is the primary win — on iOS/Android the numeric pad
/// pops up instead of the full text keyboard.
///
/// `min`/`max`/`step` are emitted as `/* min: N, max: N, step: N */`
/// hints today. Flutter's stock `TextField` has no built-in range
/// validation; a follow-up could wire `inputFormatters` with a
/// custom `TextInputFormatter` that clamps to range. The numeric
/// values come from the IR's `LayoutPropValue::Number(f64)` so
/// they're never user-controlled strings — no injection vector.
///
/// `onChange` dispatch matches the spec's "fires on commit (Enter
/// / blur)" semantics: we wire `onSubmitted` (Enter) rather than
/// `onChanged` (per-keystroke), because spec §3.3 explicitly
/// rejects per-keystroke dispatch for numeric fields ("12 while
/// typing 12.5 isn't a meaningful value").
fn emit_host_number_input(
    node: &LayoutNode,
    indent: usize,
    component: &str,
) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);

    // value — must be a slot ref (numeric input must have a host-owned
    // controller backing it). Literal value isn't a useful shape here;
    // we accept it but emit it as an initial-text string.
    let value_expr: String = if let Some(slot) = find_slot_ref_prop(node, "value") {
        let camel = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&camel)?;
        // `.toString()` because the slot is typed `double`/`number`
        // but TextEditingController wants a String.
        format!("{camel}.toString()")
    } else {
        "\"\"".to_string()
    };

    // min/max/step — numeric literals only. find_number_prop returns
    // f64 from LayoutPropValue::Number — never a string — so no
    // escaping is needed.
    let min_opt = find_number_prop(node, "min");
    let max_opt = find_number_prop(node, "max");
    let step_opt = find_number_prop(node, "step");

    // disabled — keyword: `true` means readOnly=true + enabled=false.
    let disabled = matches!(find_keyword_prop(node, "disabled"), Some("true"));

    // onChange dispatch — fires on commit (Enter/blur), not keystroke.
    // Real dispatch — `<Component>Event<Case>(value: parsed)` where
    // `parsed` is `double.tryParse(v) ?? 0` so the host always
    // receives a number even if the user typed gibberish.  The
    // sealed-class hierarchy that emit_widget_class generates
    // already declares the value-carrying constructor; this is the
    // call-site that the old TODO blocked.
    let on_submitted_arg: Option<String> = if let Some(emit_name) =
        find_emit_ref_prop(node, "onChange")
    {
        let case = pascalize(&strip_on_prefix(emit_name));
        validate_emit_name(&case)?;
        Some(format!(
            "onSubmitted: (v) {{ dispatch({component}Event{case}(value: double.tryParse(v) ?? 0)); }}"
        ))
    } else {
        None
    };

    // Build decoration. hintText if placeholder present; helperText
    // carries the min/max/step hint comment (visible in dev; cleared
    // in production via a follow-up theme).
    let mut decoration_parts: Vec<String> = Vec::new();
    if let Some(p) = find_string_prop(node, "placeholder") {
        decoration_parts.push(format!("hintText: \"{}\"", escape_dart_string(p)));
    }

    // Compose the range hint as a single comment so unit tests can
    // assert the values are present. Numeric values from the IR are
    // already-validated f64 — no injection possible.
    let range_hint: String = {
        let mut parts: Vec<String> = Vec::new();
        if let Some(n) = min_opt {
            parts.push(format!("min: {n}"));
        }
        if let Some(n) = max_opt {
            parts.push(format!("max: {n}"));
        }
        if let Some(n) = step_opt {
            parts.push(format!("step: {n}"));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" /* {} */", parts.join(", "))
        }
    };

    let mut out = String::new();
    writeln!(out, "{pad}TextField(").unwrap();
    writeln!(
        out,
        "{pad}  keyboardType: TextInputType.number,{range_hint}"
    )
    .unwrap();
    writeln!(
        out,
        "{pad}  controller: TextEditingController(text: {value_expr}),"
    )
    .unwrap();
    if disabled {
        writeln!(out, "{pad}  enabled: false,").unwrap();
    }
    if !decoration_parts.is_empty() {
        writeln!(
            out,
            "{pad}  decoration: InputDecoration({}),",
            decoration_parts.join(", ")
        )
        .unwrap();
    }
    if let Some(arg) = on_submitted_arg {
        writeln!(out, "{pad}  {arg},").unwrap();
    }
    writeln!(out, "{pad})").unwrap();
    Ok(out)
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
        map.insert(part.name.clone(), format_props(&part.base));
        // State blocks (`state selected { ... }`) are surfaced under a
        // composite key `{part}:{state}` so the cell-styling path can
        // look up `cell:selected` / `cell:editing` without having to
        // walk `style.parts` again.  Mirrors the React + SwiftUI
        // backends' `build_part_style_map` shape — same composite-key
        // convention, so the same `.msl` `state X { ... }` block drives
        // the per-cell highlight on every backend.
        for state in &part.states {
            let fragment = format_props(&state.props);
            if !fragment.is_empty() {
                map.insert(format!("{}:{}", part.name, state.state), fragment);
            }
        }
    }
    map
}

/// Render a slice of style props as a joined `"key: value; key: value"`
/// string the widget builder can split + match on. `StyleProp.value` is
/// already a `String` in the IR — we don't need to re-format scalar /
/// keyword distinctions here. Shared by the base-part and
/// `state X { ... }` paths.
fn format_props(props: &[StyleProp]) -> String {
    let mut joined = String::new();
    for p in props {
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
        EmitPayloadType::Number => "num".to_string(),
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
        "abstract",
        "as",
        "assert",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extends",
        "extension",
        "false",
        "final",
        "finally",
        "for",
        "function",
        "get",
        "if",
        "implements",
        "import",
        "in",
        "interface",
        "is",
        "library",
        "mixin",
        "new",
        "null",
        "operator",
        "part",
        "rethrow",
        "return",
        "set",
        "static",
        "super",
        "switch",
        "sync",
        "this",
        "throw",
        "true",
        "try",
        "typedef",
        "var",
        "void",
        "while",
        "with",
        "yield",
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

fn find_prop_value<'a>(node: &'a LayoutNode, name: &str) -> Option<&'a LayoutPropValue> {
    node.props.iter().find(|p| p.name == name).map(|p| &p.value)
}

/// Map a semantic glyph name to a Material widget that natively
/// expresses that semantic.  Returns `None` for any name not in the
/// table — the caller falls back to the standard
/// `Icon(Icons.<name>)` lowering.
///
/// Currently recognized:
///
/// | semantic name | Flutter widget                  |
/// |---|---|
/// | `"spinner"`   | `CircularProgressIndicator()`   |
///
/// Mirrors `mosaic-emit-xaml::semantic_glyph_xaml_element` so the
/// same toolkit `.mll` source (`Icon (glyph: "spinner")`) renders as
/// the correct native widget on every backend that has one.  New
/// entries land case-by-case as the toolkit demo surfaces them.
///
/// **Security:** values are `&'static str` literals only.  The table
/// must NEVER accept runtime input — that would be how a user-
/// controlled glyph name leaks into the lowering-decision space.
fn semantic_glyph_flutter_widget(name: &str) -> Option<&'static str> {
    match name {
        "spinner" => Some("CircularProgressIndicator()"),
        _ => None,
    }
}

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

fn bool_prop_expression(
    node: &LayoutNode,
    name: &str,
) -> Result<Option<String>, PipelineEmitError> {
    let Some(value) = find_prop_value(node, name) else {
        return Ok(None);
    };
    let expression = match value {
        LayoutPropValue::SlotRef(slot) => {
            let camel = to_camel_case_first_lower(slot);
            validate_slot_or_field_name(&camel)?;
            format!("_mosaicTruthy({camel})")
        }
        LayoutPropValue::Keyword(keyword) if keyword == "true" || keyword == "false" => {
            keyword.clone()
        }
        LayoutPropValue::Expr(expression) => {
            format!("_mosaicTruthy(({}))", expression.trim())
        }
        _ => return Ok(None),
    };
    Ok(Some(expression))
}

fn emit_host_surface(node: &LayoutNode, indent: usize) -> Result<String, PipelineEmitError> {
    let pad = " ".repeat(indent);
    let expression = if let Some(slot) = find_slot_ref_prop(node, "content") {
        let field = to_camel_case_first_lower(slot);
        validate_slot_or_field_name(&field)?;
        field
    } else {
        "const SizedBox.shrink()".to_string()
    };
    Ok(format!("{pad}{expression}\n"))
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

/// Numeric-literal lookup. Used by HostNumberInput for `min`/`max`/
/// `step`. The IR carries these as `LayoutPropValue::Number(f64)`,
/// so there's no path for a user-controlled string to flow into the
/// generated Dart — the value is `Display`-formatted into a `/* min:
/// N, max: N */` comment, safe from injection.
fn find_number_prop(node: &LayoutNode, name: &str) -> Option<f64> {
    node.props.iter().find_map(|p| {
        if p.name == name {
            if let LayoutPropValue::Number(n) = &p.value {
                return Some(*n);
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
    use mosmodel_compiler::{EmitParam, ListInnerType};

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
        assert!(r.output.contains(
            "import 'package:flutter/material.dart' hide Checkbox, Radio, Slider, Tooltip;"
        ));
        assert!(r.output.contains("class X extends StatelessWidget"));
        assert!(r.output.contains("Container("));
        assert!(!r.output.contains("_mosaicTruthy"));
    }

    #[test]
    fn plain_single_child_box_elides_unnecessary_container() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Box",
                vec![],
                vec![node_with(
                    "Text",
                    vec![LayoutProp {
                        name: "content".into(),
                        value: LayoutPropValue::String("hello".into()),
                    }],
                    vec![],
                )],
            ),
        );
        let output = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;

        assert!(output.contains("Text(\"hello\")"), "{output}");
        assert!(!output.contains("Container(\n"), "{output}");
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
        assert!(r.output.contains("String get mosaicName;"));
        assert!(r
            .output
            .contains("Map<String, Object?> get mosaicPayload => const {};"));
        assert!(r.output.contains("Map<String, Object?> get mosaicEnvelope"));
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
                    EmitParam {
                        name: "row".into(),
                        r#type: EmitPayloadType::Number,
                    },
                    EmitParam {
                        name: "col".into(),
                        r#type: EmitPayloadType::Number,
                    },
                ],
            )],
        );
        let l = layout("Grid", node("Box"));
        let r = from_pipeline(&m, &l, &empty_style("Grid")).unwrap();
        let out = &r.output;
        assert!(out.contains("class GridEventNavigate extends GridEvent"));
        assert!(out.contains("final num row;"));
        assert!(out.contains("final num col;"));
        assert!(out.contains("required this.row,"));
        assert!(out.contains("required this.col,"));
        assert!(out.contains("String get mosaicName => \"onNavigate\";"));
        assert!(out.contains("Map<String, Object?> get mosaicPayload => {'row': row, 'col': col};"));
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

    #[test]
    fn defaulted_slots_are_non_nullable_with_constructor_defaults() {
        let mut title = slot("title", SlotType::Text, false);
        title.default = Some(SlotDefault::Text("Ready".into()));
        let mut count = slot("count", SlotType::Number, false);
        count.default = Some(SlotDefault::Number(3.0));
        let mut enabled = slot("enabled", SlotType::Bool, false);
        enabled.default = Some(SlotDefault::Bool(true));
        let m = component("Profile", vec![title, count, enabled], vec![]);
        let l = layout("Profile", node("Box"));
        let out = from_pipeline(&m, &l, &empty_style("Profile"))
            .unwrap()
            .output;

        assert!(out.contains("final String title;"), "{out}");
        assert!(out.contains("final double count;"), "{out}");
        assert!(out.contains("final bool enabled;"), "{out}");
        assert!(out.contains("this.title = \"Ready\","), "{out}");
        assert!(out.contains("this.count = 3.0,"), "{out}");
        assert!(out.contains("this.enabled = true,"), "{out}");
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
        assert!(
            out.contains("Text(\"Hello\")"),
            "expected Hello, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"World\")"),
            "expected World, got:\n{out}"
        );
    }

    // ----- Text with slot ref --------------------------------------------

    #[test]
    fn text_with_slot_ref_uses_bare_identifier() {
        let m = component("X", vec![slot("greeting", SlotType::Text, true)], vec![]);
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

    #[test]
    fn text_accessibility_metadata_lowers_to_flutter_semantics() {
        let m = component(
            "Title",
            vec![slot("spoken-title", SlotType::Text, true)],
            vec![],
        );
        let l = layout(
            "Title",
            node_with(
                "Text",
                vec![
                    LayoutProp {
                        name: "content".into(),
                        value: LayoutPropValue::String("Visible title".into()),
                    },
                    LayoutProp {
                        name: "a11y-label".into(),
                        value: LayoutPropValue::SlotRef("spoken-title".into()),
                    },
                    LayoutProp {
                        name: "a11y-role".into(),
                        value: LayoutPropValue::Keyword("heading".into()),
                    },
                ],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Title")).unwrap().output;
        assert!(out.contains(
            "Semantics(label: spokenTitle, excludeSemantics: true, header: true, child: Text(\"Visible title\"))"
        ));

        let hidden = layout(
            "Hidden",
            node_with(
                "Text",
                vec![LayoutProp {
                    name: "a11y-hidden".into(),
                    value: LayoutPropValue::Keyword("true".into()),
                }],
                vec![],
            ),
        );
        let hidden_out = from_pipeline(
            &component("Hidden", vec![], vec![]),
            &hidden,
            &empty_style("Hidden"),
        )
        .unwrap()
        .output;
        assert!(hidden_out.contains("ExcludeSemantics(child: const Text(\"\"))"));
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
    fn host_button_with_on_click_emits_dispatch_handler() {
        let m = component("X", vec![], vec![emit("onClick", vec![])]);
        let l = layout(
            "X",
            node_with(
                "HostButton",
                vec![
                    LayoutProp {
                        name: "label".into(),
                        value: LayoutPropValue::String("Save".into()),
                    },
                    LayoutProp {
                        name: "onClick".into(),
                        value: LayoutPropValue::EmitRef("onClick".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("onPressed: () => dispatch(XEventClick())"));
        assert!(out.contains("Text(\"Save\")"));
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
            vec![emit(
                "onSelect",
                vec![EmitParam {
                    name: "index".into(),
                    r#type: EmitPayloadType::Number,
                }],
            )],
        );
        let l = layout(
            "ListGroup",
            node_with(
                "Column",
                vec![],
                vec![node_with(
                    "For",
                    vec![
                        LayoutProp {
                            name: "each".into(),
                            value: LayoutPropValue::SlotRef("items".into()),
                        },
                        LayoutProp {
                            name: "as".into(),
                            value: LayoutPropValue::Keyword("item".into()),
                        },
                        LayoutProp {
                            name: "index".into(),
                            value: LayoutPropValue::Keyword("i".into()),
                        },
                    ],
                    vec![node_with(
                        "HostButton",
                        vec![
                            LayoutProp {
                                name: "label".into(),
                                value: LayoutPropValue::Keyword("item".into()),
                            },
                            LayoutProp {
                                name: "onClick".into(),
                                value: LayoutPropValue::EmitRef("onSelect".into()),
                            },
                        ],
                        vec![],
                    )],
                )],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("ListGroup"))
            .unwrap()
            .output;
        assert!(
            out.contains("final i = entry.key;"),
            "expected indexed For binding, got:\n{out}"
        );
        assert!(
            out.contains("dispatch(ListGroupEventSelect(index: i))"),
            "expected HostButton to dispatch index payload, got:\n{out}"
        );
        assert!(
            out.contains("Text(item)"),
            "expected HostButton label to use For item binding, got:\n{out}"
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
            vec![emit(
                "onChange",
                vec![EmitParam {
                    name: "value".into(),
                    r#type: EmitPayloadType::Text,
                }],
            )],
        );
        let l = layout(
            "SelectMenu",
            node_with(
                "Column",
                vec![],
                vec![node_with(
                    "For",
                    vec![
                        LayoutProp {
                            name: "each".into(),
                            value: LayoutPropValue::SlotRef("options".into()),
                        },
                        LayoutProp {
                            name: "as".into(),
                            value: LayoutPropValue::Keyword("option".into()),
                        },
                    ],
                    vec![node_with(
                        "HostButton",
                        vec![
                            LayoutProp {
                                name: "label".into(),
                                value: LayoutPropValue::Keyword("option".into()),
                            },
                            LayoutProp {
                                name: "onClick".into(),
                                value: LayoutPropValue::EmitRef("onChange".into()),
                            },
                        ],
                        vec![],
                    )],
                )],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("SelectMenu"))
            .unwrap()
            .output;
        assert!(
            out.contains("dispatch(SelectMenuEventChange(value: option))"),
            "expected HostButton to dispatch item payload, got:\n{out}"
        );
        assert!(
            out.contains("Text(option)"),
            "expected HostButton label to use For item binding, got:\n{out}"
        );
    }

    #[test]
    fn host_button_with_part_style_emits_button_style() {
        let style = StyleDef {
            component_name: "X".into(),
            parts: vec![PartStyle {
                name: "danger".into(),
                base: vec![
                    StyleProp {
                        name: "background".into(),
                        value: "#f87171".into(),
                    },
                    StyleProp {
                        name: "color".into(),
                        value: "#1a1a2e".into(),
                    },
                    StyleProp {
                        name: "padding".into(),
                        value: "10px".into(),
                    },
                    StyleProp {
                        name: "border-color".into(),
                        value: "#7f1d1d".into(),
                    },
                    StyleProp {
                        name: "border-width".into(),
                        value: "2px".into(),
                    },
                    StyleProp {
                        name: "border-radius".into(),
                        value: "7px".into(),
                    },
                ],
                transitions: vec![],
                states: Vec::new(),
            }],
        };
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            LayoutNode {
                tag: "HostButton".into(),
                part_name: Some("danger".into()),
                props: vec![LayoutProp {
                    name: "label".into(),
                    value: LayoutPropValue::String("Again".into()),
                }],
                children: vec![],
            },
        );
        let r = from_pipeline(&m, &l, &style).unwrap();
        let out = &r.output;
        assert!(
            out.contains("style: ButtonStyle("),
            "missing ButtonStyle:\n{out}"
        );
        assert!(
            out.contains("backgroundColor: WidgetStatePropertyAll(const Color(0xFFF87171))"),
            "missing background style:\n{out}"
        );
        assert!(
            out.contains("foregroundColor: WidgetStatePropertyAll(const Color(0xFF1A1A2E))"),
            "missing foreground style:\n{out}"
        );
        assert!(
            out.contains("padding: WidgetStatePropertyAll(const EdgeInsets.all(10))"),
            "missing padding style:\n{out}"
        );
        assert!(
            out.contains("borderRadius: BorderRadius.circular(7)"),
            "missing border radius style:\n{out}"
        );
        assert!(
            out.contains("side: BorderSide(color: const Color(0xFF7F1D1D), width: 2)"),
            "missing border side style:\n{out}"
        );
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

    #[test]
    fn host_button_disabled_slot_controls_onpressed() {
        let m = component(
            "BrowserChrome",
            vec![slot("back-disabled", SlotType::Bool, true)],
            vec![emit("onBack", vec![])],
        );
        let l = layout(
            "BrowserChrome",
            node_with(
                "HostButton",
                vec![
                    LayoutProp {
                        name: "disabled".into(),
                        value: LayoutPropValue::SlotRef("back-disabled".into()),
                    },
                    LayoutProp {
                        name: "onClick".into(),
                        value: LayoutPropValue::EmitRef("onBack".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("BrowserChrome")).unwrap();
        assert!(
            r.output.contains(
                "onPressed: _mosaicTruthy(backDisabled) ? null : () => dispatch(BrowserChromeEventBack())"
            ),
            "slot-backed disabled state must control the native button:\n{}",
            r.output
        );
    }

    // ----- HostInput with placeholder + slot value ---------------------

    #[test]
    fn host_input_with_placeholder_emits_input_decoration() {
        let m = component("X", vec![slot("formula", SlotType::Text, true)], vec![]);
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

    #[test]
    fn host_input_lowers_read_only_slot_and_commit_event() {
        let m = component(
            "BrowserChrome",
            vec![
                slot("address", SlotType::Text, true),
                slot("navigation-disabled", SlotType::Bool, true),
            ],
            vec![emit("onNavigate", vec![])],
        );
        let l = layout(
            "BrowserChrome",
            node_with(
                "HostInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("address".into()),
                    },
                    LayoutProp {
                        name: "read-only".into(),
                        value: LayoutPropValue::SlotRef("navigation-disabled".into()),
                    },
                    LayoutProp {
                        name: "onCommit".into(),
                        value: LayoutPropValue::EmitRef("onNavigate".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("BrowserChrome")).unwrap();
        assert!(r
            .output
            .contains("readOnly: _mosaicTruthy(navigationDisabled)"));
        assert!(r
            .output
            .contains("onSubmitted: (value) => dispatch(BrowserChromeEventNavigate())"));
    }

    #[test]
    fn host_input_commit_supplies_required_text_payload() {
        let m = component(
            "Editor",
            vec![slot("body", SlotType::Text, true)],
            vec![emit(
                "onCommit",
                vec![EmitParam {
                    name: "value".into(),
                    r#type: EmitPayloadType::Text,
                }],
            )],
        );
        let l = layout(
            "Editor",
            node_with(
                "HostInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("body".into()),
                    },
                    LayoutProp {
                        name: "onCommit".into(),
                        value: LayoutPropValue::EmitRef("onCommit".into()),
                    },
                ],
                vec![],
            ),
        );
        let output = from_pipeline(&m, &l, &empty_style("Editor"))
            .expect("emit input with commit payload")
            .output;
        assert!(
            output.contains("onSubmitted: (value) => dispatch(EditorEventCommit(value: value))")
        );
    }

    #[test]
    fn host_input_directly_inside_row_receives_finite_width() {
        let m = component(
            "BrowserChrome",
            vec![slot("address", SlotType::Text, true)],
            vec![],
        );
        let l = layout(
            "BrowserChrome",
            node_with(
                "Row",
                vec![],
                vec![node_with(
                    "HostInput",
                    vec![LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("address".into()),
                    }],
                    vec![],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("BrowserChrome")).unwrap();
        assert!(
            r.output.contains("Expanded(") && r.output.contains("child: TextField("),
            "a direct Row input must be flex-constrained:\n{}",
            r.output
        );
    }

    /// Regression: `HostInput { onChange: emit: onFormulaChange }`
    /// must lower to a real `dispatch(...)` call, not a literal
    /// `/* TODO: ... */` placeholder.  The dispatched event subclass
    /// is named `<Component>Event<Case>` to match the sealed-class
    /// hierarchy emitted at the top of the Dart file.  v0.1.0 of the
    /// emitter shipped the TODO directly in the output, breaking the
    /// VisiCalc Flutter demo at compile time.
    #[test]
    fn host_input_on_change_emits_real_dispatch_call() {
        let m = component(
            "FormulaBar",
            vec![slot("formula", SlotType::Text, true)],
            vec![],
        );
        let l = layout(
            "FormulaBar",
            node_with(
                "HostInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("formula".into()),
                    },
                    LayoutProp {
                        name: "onChange".into(),
                        value: LayoutPropValue::EmitRef("onFormulaChange".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("FormulaBar")).unwrap();
        let out = &r.output;
        assert!(
            out.contains(
                "onChanged: (value) => dispatch(FormulaBarEventFormulaChange(value: value))"
            ),
            "expected real dispatch call in:\n{out}"
        );
        assert!(
            !out.contains("/* TODO"),
            "regenerated output should not contain TODO placeholders:\n{out}"
        );
    }

    // ----- HostCheckbox + HostRadio scaffolds --------------------------

    #[test]
    fn host_checkbox_with_checked_slot_emits_checkbox_widget() {
        let m = component("X", vec![slot("agreed", SlotType::Bool, true)], vec![]);
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
    fn host_checkbox_uses_declared_event_field_without_component_name_collision() {
        let m = component(
            "Checkbox",
            vec![slot("checked", SlotType::Bool, true)],
            vec![emit(
                "onChange",
                vec![EmitParam {
                    name: "checked".into(),
                    r#type: EmitPayloadType::Bool,
                }],
            )],
        );
        let l = layout(
            "Checkbox",
            node_with(
                "HostCheckbox",
                vec![
                    LayoutProp {
                        name: "checked".into(),
                        value: LayoutPropValue::SlotRef("checked".into()),
                    },
                    LayoutProp {
                        name: "onToggle".into(),
                        value: LayoutPropValue::EmitRef("onChange".into()),
                    },
                ],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Checkbox"))
            .expect("emit Checkbox toolkit component")
            .output;
        assert!(out.contains("as material show Checkbox;"));
        assert!(out.contains("material.Checkbox(value: checked"));
        assert!(out.contains("dispatch(CheckboxEventChange(checked: v ?? false))"));
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

    #[test]
    fn host_slider_lowers_native_range_steps_disabled_and_events() {
        let m = component(
            "Slider",
            vec![
                slot("value", SlotType::Number, true),
                slot("disabled", SlotType::Bool, true),
                slot("label", SlotType::Text, true),
                slot("step", SlotType::Number, true),
            ],
            vec![
                emit(
                    "onChange",
                    vec![EmitParam {
                        name: "value".into(),
                        r#type: EmitPayloadType::Number,
                    }],
                ),
                emit(
                    "onCommit",
                    vec![EmitParam {
                        name: "value".into(),
                        r#type: EmitPayloadType::Number,
                    }],
                ),
            ],
        );
        let l = layout(
            "Slider",
            node_with(
                "HostSlider",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("value".into()),
                    },
                    LayoutProp {
                        name: "min".into(),
                        value: LayoutPropValue::Number(0.0),
                    },
                    LayoutProp {
                        name: "max".into(),
                        value: LayoutPropValue::Number(100.0),
                    },
                    LayoutProp {
                        name: "step".into(),
                        value: LayoutPropValue::SlotRef("step".into()),
                    },
                    LayoutProp {
                        name: "disabled".into(),
                        value: LayoutPropValue::SlotRef("disabled".into()),
                    },
                    LayoutProp {
                        name: "a11y-label".into(),
                        value: LayoutPropValue::SlotRef("label".into()),
                    },
                    LayoutProp {
                        name: "onChange".into(),
                        value: LayoutPropValue::EmitRef("onChange".into()),
                    },
                    LayoutProp {
                        name: "onCommit".into(),
                        value: LayoutPropValue::EmitRef("onCommit".into()),
                    },
                ],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Slider"))
            .expect("emit native Flutter slider")
            .output;

        assert!(out.contains("hide Checkbox, Radio, Slider, Tooltip"));
        assert!(out.contains("as material show Slider;"));
        assert!(out.contains("Semantics(label: label, child: material.Slider("));
        assert!(out.contains("value: (value).toDouble()"));
        assert!(out.contains("min: (0).toDouble()"));
        assert!(out.contains("max: (100).toDouble()"));
        assert!(out.contains("divisions: (step).toDouble() > 0 ?"));
        assert!(out.contains("/ (step).toDouble()).round()"));
        assert!(out.contains(
            "onChanged: _mosaicTruthy(disabled) ? null : (value) { dispatch(SliderEventChange(value: value)); }"
        ));
        assert!(out.contains(
            "onChangeEnd: _mosaicTruthy(disabled) ? null : (value) { dispatch(SliderEventCommit(value: value)); }"
        ));
    }

    #[test]
    fn host_slider_step_zero_is_continuous() {
        let m = component("Opacity", vec![], vec![]);
        let l = layout(
            "Opacity",
            node_with(
                "HostSlider",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::Number(0.5),
                    },
                    LayoutProp {
                        name: "min".into(),
                        value: LayoutPropValue::Number(0.0),
                    },
                    LayoutProp {
                        name: "max".into(),
                        value: LayoutPropValue::Number(1.0),
                    },
                    LayoutProp {
                        name: "step".into(),
                        value: LayoutPropValue::Number(0.0),
                    },
                    LayoutProp {
                        name: "a11y-label".into(),
                        value: LayoutPropValue::String("Opacity".into()),
                    },
                ],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Opacity"))
            .expect("emit continuous Flutter slider")
            .output;
        let slider = out
            .lines()
            .find(|line| line.contains("material.Slider("))
            .expect("slider line");
        assert!(slider.contains("Semantics(label: \"Opacity\", child:"));
        assert!(slider.contains("onChanged: false ? null"));
        assert!(!slider.contains("divisions:"));
        assert!(!slider.contains("onChangeEnd:"));
    }

    #[test]
    fn host_drag_drop_emits_native_pointer_keyboard_and_semantics_runtime() {
        let m = component(
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
                slot("drop-disabled", SlotType::Bool, true),
            ],
            vec![
                emit(
                    "onDragStart",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                    ],
                ),
                emit(
                    "onDragEnd",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "dropped".into(),
                            r#type: EmitPayloadType::Bool,
                        },
                    ],
                ),
                emit(
                    "onDragEnter",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                    ],
                ),
                emit(
                    "onDragLeave",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                    ],
                ),
                emit(
                    "onDropHover",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "target-key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "position".into(),
                            r#type: EmitPayloadType::Text,
                        },
                    ],
                ),
                emit(
                    "onDrop",
                    vec![
                        EmitParam {
                            name: "key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "kind".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "target-key".into(),
                            r#type: EmitPayloadType::Text,
                        },
                        EmitParam {
                            name: "position".into(),
                            r#type: EmitPayloadType::Text,
                        },
                    ],
                ),
            ],
        );
        let draggable = node_with(
            "HostDraggable",
            vec![
                LayoutProp {
                    name: "drag-key".into(),
                    value: LayoutPropValue::SlotRef("drag-key".into()),
                },
                LayoutProp {
                    name: "drag-kind".into(),
                    value: LayoutPropValue::SlotRef("drag-kind".into()),
                },
                LayoutProp {
                    name: "drag-label".into(),
                    value: LayoutPropValue::SlotRef("drag-label".into()),
                },
                LayoutProp {
                    name: "drag-disabled".into(),
                    value: LayoutPropValue::SlotRef("drag-disabled".into()),
                },
                LayoutProp {
                    name: "onDragStart".into(),
                    value: LayoutPropValue::EmitRef("onDragStart".into()),
                },
                LayoutProp {
                    name: "onDragEnd".into(),
                    value: LayoutPropValue::EmitRef("onDragEnd".into()),
                },
            ],
            vec![node_with(
                "Text",
                vec![LayoutProp {
                    name: "content".into(),
                    value: LayoutPropValue::String("Card".into()),
                }],
                vec![],
            )],
        );
        let drop_target = node_with(
            "HostDropTarget",
            vec![
                LayoutProp {
                    name: "drop-key".into(),
                    value: LayoutPropValue::String("lane-a".into()),
                },
                LayoutProp {
                    name: "accepts".into(),
                    value: LayoutPropValue::SlotRef("accepted-kinds".into()),
                },
                LayoutProp {
                    name: "drop-disabled".into(),
                    value: LayoutPropValue::SlotRef("drop-disabled".into()),
                },
                LayoutProp {
                    name: "onDragEnter".into(),
                    value: LayoutPropValue::EmitRef("onDragEnter".into()),
                },
                LayoutProp {
                    name: "onDragLeave".into(),
                    value: LayoutPropValue::EmitRef("onDragLeave".into()),
                },
                LayoutProp {
                    name: "onDropHover".into(),
                    value: LayoutPropValue::EmitRef("onDropHover".into()),
                },
                LayoutProp {
                    name: "onDrop".into(),
                    value: LayoutPropValue::EmitRef("onDrop".into()),
                },
            ],
            vec![draggable],
        );
        let out = from_pipeline(&m, &layout("Board", drop_target), &empty_style("Board"))
            .expect("emit native Flutter drag/drop")
            .output;

        for expected in [
            "import 'package:flutter/services.dart';",
            "import 'package:flutter/semantics.dart';",
            "class _MosaicDragScope extends StatefulWidget",
            "final List<_MosaicDropTargetState> _targets",
            "Draggable<_MosaicDragData>(",
            "DragTarget<_MosaicDragData>(",
            "LogicalKeyboardKey.space",
            "LogicalKeyboardKey.enter",
            "LogicalKeyboardKey.escape",
            "LogicalKeyboardKey.arrowDown",
            "SemanticsService.announce",
            "onTap: disabled && scope._active == null ? null : () => _toggle(context)",
            "previous?.leave(data);",
            "next.enter(data);",
            "next.hover(data, 'into');",
            "_MosaicDragScope(",
            "acceptsKinds: acceptedKinds",
            "disabled: _mosaicTruthy(dragDisabled)",
            "disabled: _mosaicTruthy(dropDisabled)",
            "BoardEventDragStart(key: dragKey.toString(), kind: dragKind.toString())",
            "BoardEventDragEnd(key: dragKey.toString(), kind: dragKind.toString(), dropped: dropped)",
            "BoardEventDragEnter(key: data.key, kind: data.kind)",
            "BoardEventDragLeave(key: data.key, kind: data.kind)",
            "BoardEventDropHover(key: data.key, kind: data.kind, targetKey: \"lane-a\", position: position)",
            "BoardEventDrop(key: data.key, kind: data.kind, targetKey: \"lane-a\", position: position)",
        ] {
            assert!(out.contains(expected), "missing `{expected}`:\n{out}");
        }
    }

    #[test]
    fn non_drag_component_omits_drag_runtime_and_services_import() {
        let out = from_pipeline(
            &component("Plain", vec![], vec![]),
            &layout("Plain", node("Box")),
            &empty_style("Plain"),
        )
        .expect("emit non-drag component")
        .output;
        assert!(!out.contains("package:flutter/services.dart"));
        assert!(!out.contains("_MosaicDragScope"));
        assert!(!out.contains("Draggable<_MosaicDragData>"));
    }

    #[test]
    fn empty_drop_target_preserves_authored_minimum_hit_area() {
        let style = StyleDef {
            component_name: "Board".into(),
            parts: vec![PartStyle {
                name: "lane".into(),
                base: vec![StyleProp {
                    name: "min-height".into(),
                    value: "60".into(),
                }],
                transitions: vec![],
                states: vec![],
            }],
        };
        let mut target = node_with(
            "HostDropTarget",
            vec![LayoutProp {
                name: "drop-key".into(),
                value: LayoutPropValue::String("empty-lane".into()),
            }],
            vec![],
        );
        target.part_name = Some("lane".into());
        let out = from_pipeline(
            &component("Board", vec![], vec![]),
            &layout("Board", target),
            &style,
        )
        .expect("emit styled empty drop target")
        .output;
        assert!(out.contains("constraints: const BoxConstraints(minHeight: 60)"));
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
        assert!(matches!(
            err,
            PipelineEmitError::ComponentNameMismatch { .. }
        ));
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
        let m = component("X", vec![slot("class", SlotType::Text, true)], vec![]);
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
        let l = layout("Host", node("Foo*/dispatch(evil());/*"));
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
            r.output
                .contains("/* TODO: component reference 'UserCard' not yet resolved */"),
            "expected labelled placeholder, got:\n{}",
            r.output
        );
        assert!(r.output.contains("const SizedBox.shrink()"));
    }

    // =====================================================================
    // UI29-4 — HostLink / HostTooltip / HostNumberInput (Flutter)
    // =====================================================================

    /// UI29-4 Flutter test 1 — bare `HostLink` with literal href +
    /// label lowers to an `InkWell` wrapping a `Text(label)`, with
    /// the href interpolated into the `launchUrl` TODO comment.
    #[test]
    fn host_link_with_literal_href_and_label_emits_inkwell_with_launchurl_todo() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![
                    LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String("https://anthropic.com".into()),
                    },
                    LayoutProp {
                        name: "label".into(),
                        value: LayoutPropValue::String("Anthropic".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("InkWell("), "expected InkWell, got:\n{out}");
        assert!(
            out.contains(
                "/* TODO: launchUrl(Uri.parse(\"https://anthropic.com\")) — target=same */"
            ),
            "expected launchUrl TODO with href, got:\n{out}"
        );
        assert!(
            out.contains("Text(\"Anthropic\")"),
            "expected `Text(\"Anthropic\")`, got:\n{out}"
        );
    }

    /// #13052: a literal `href` carrying an explicit disallowed URI
    /// scheme is rejected at compile time when `external` is not
    /// `false` -- the path a real `launchUrl` call will eventually
    /// use (currently still a TODO comment, checked preventatively).
    #[test]
    fn host_link_disallowed_scheme_href_is_rejected() {
        for hostile in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "intent:#Intent;action=android.intent.action.VIEW;end",
            "file:///etc/passwd",
        ] {
            let m = component("X", vec![], vec![]);
            let l = layout(
                "X",
                node_with(
                    "HostLink",
                    vec![LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String(hostile.to_string()),
                    }],
                    vec![],
                ),
            );
            let err = from_pipeline(&m, &l, &empty_style("X")).unwrap_err();
            assert!(
                matches!(err, PipelineEmitError::UnsafeUriScheme(ref h) if h == hostile),
                "expected UnsafeUriScheme for {hostile:?}, got: {err:?}"
            );
        }
    }

    /// #13052 security-review finding: a scheme hidden behind leading
    /// whitespace or an embedded tab/CR/LF must still be rejected --
    /// a naive scan sees no alphabetic first character and
    /// misclassifies it as "no scheme, therefore safe," but a real
    /// consumer normalizes that whitespace away before parsing the
    /// scheme, so it's really a disallowed scheme.
    #[test]
    fn host_link_scheme_hidden_by_whitespace_is_still_rejected() {
        for hostile in [
            " javascript:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "\tjavascript:alert(1)",
        ] {
            let m = component("X", vec![], vec![]);
            let l = layout(
                "X",
                node_with(
                    "HostLink",
                    vec![LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String(hostile.to_string()),
                    }],
                    vec![],
                ),
            );
            let err = from_pipeline(&m, &l, &empty_style("X")).unwrap_err();
            assert!(
                matches!(err, PipelineEmitError::UnsafeUriScheme(ref h) if h == hostile),
                "expected UnsafeUriScheme for whitespace-obscured {hostile:?}, got: {err:?}"
            );
        }
    }

    /// #13052: allowed schemes and relative references (no scheme at
    /// all -- the common in-app-routing shape) stay valid.
    #[test]
    fn host_link_allowed_or_relative_href_still_compiles() {
        for safe in [
            "http://example.com",
            "https://example.com",
            "HTTPS://example.com",
            "mailto:hello@example.com",
            "#",
            "/about",
        ] {
            let m = component("X", vec![], vec![]);
            let l = layout(
                "X",
                node_with(
                    "HostLink",
                    vec![LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String(safe.to_string()),
                    }],
                    vec![],
                ),
            );
            let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
            assert!(
                out.contains(&format!("Uri.parse(\"{safe}\")")),
                "expected {safe:?} to still compile, got:\n{out}"
            );
        }
    }

    /// #13052: a disallowed scheme with `external: false` stays valid
    /// -- that path never reaches the eventual `launchUrl` call (it's
    /// dispatch-only), matching the native-widget backends' scoping.
    #[test]
    fn host_link_disallowed_scheme_href_stays_valid_with_external_false() {
        let m = component("X", vec![], vec![emit("onNavigate", vec![])]);
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![
                    LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String("javascript:alert(1)".into()),
                    },
                    LayoutProp {
                        name: "external".into(),
                        value: LayoutPropValue::Keyword("false".into()),
                    },
                    LayoutProp {
                        name: "onActivate".into(),
                        value: LayoutPropValue::EmitRef("onNavigate".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X"));
        assert!(
            r.is_ok(),
            "expected a disallowed scheme with external:false to compile, got: {r:?}"
        );
    }

    /// UI29-4 Flutter test 2 — `HostLink` with `external: false`
    /// suppresses the `launchUrl` TODO (host handles in-app routing
    /// via the `onActivate` dispatch instead).
    #[test]
    fn host_link_external_false_suppresses_launchurl_todo() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![
                    LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String("/about".into()),
                    },
                    LayoutProp {
                        name: "external".into(),
                        value: LayoutPropValue::Keyword("false".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(
            !out.contains("launchUrl"),
            "external: false must NOT emit launchUrl TODO; got:\n{out}"
        );
        assert!(
            out.contains("InkWell("),
            "still expected InkWell, got:\n{out}"
        );
    }

    /// UI29-4 Flutter test 3 — `HostLink` with onActivate emits a
    /// real dispatch call inside the onTap closure.  v0.1.0 of this
    /// emitter shipped a `/* TODO: ... */` placeholder; the
    /// flutter-emit-other-hosts cycle replaces it with the real
    /// `<Component>Event<Case>(href: ...)` call.
    #[test]
    fn host_link_with_on_activate_emits_real_dispatch() {
        let m = component(
            "X",
            vec![],
            vec![emit(
                "onLinkActivated",
                vec![EmitParam {
                    name: "href".into(),
                    r#type: EmitPayloadType::Text,
                }],
            )],
        );
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![
                    LayoutProp {
                        name: "href".into(),
                        value: LayoutPropValue::String("https://example.org".into()),
                    },
                    LayoutProp {
                        name: "onActivate".into(),
                        value: LayoutPropValue::EmitRef("onLinkActivated".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("dispatch(XEventLinkActivated(href:"),
            "expected real dispatch call, got:\n{}",
            r.output
        );
        // And no leftover TODO at the dispatch site.
        assert!(
            !r.output.contains("/* TODO: dispatch LinkActivated"),
            "expected no TODO placeholder, got:\n{}",
            r.output
        );
    }

    #[test]
    fn host_link_inside_indexed_for_dispatches_declared_number_payload() {
        let m = component(
            "Nav",
            vec![slot(
                "items",
                SlotType::List(Box::new(ListInnerType::Text)),
                true,
            )],
            vec![emit(
                "onSelect",
                vec![EmitParam {
                    name: "index".into(),
                    r#type: EmitPayloadType::Number,
                }],
            )],
        );
        let link = node_with(
            "HostLink",
            vec![
                LayoutProp {
                    name: "href".into(),
                    value: LayoutPropValue::String("#".into()),
                },
                LayoutProp {
                    name: "label".into(),
                    value: LayoutPropValue::Keyword("item".into()),
                },
                LayoutProp {
                    name: "external".into(),
                    value: LayoutPropValue::Keyword("false".into()),
                },
                LayoutProp {
                    name: "onActivate".into(),
                    value: LayoutPropValue::EmitRef("onSelect".into()),
                },
            ],
            vec![],
        );
        let l = layout(
            "Nav",
            node_with(
                "Row",
                vec![],
                vec![for_node(
                    LayoutPropValue::SlotRef("items".into()),
                    "item",
                    Some("i"),
                    vec![link],
                )],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Nav"))
            .expect("emit indexed links")
            .output;
        assert!(out.contains("final i = entry.key;"));
        assert!(out.contains("child: Text(item)"));
        assert!(out.contains("dispatch(NavEventSelect(index: i));"));
    }

    /// UI29-4 Flutter test 4 — `HostLink` injection regression. A
    /// malicious href containing `*/` should NOT terminate the
    /// `/* ... */` block comment early; `escape_dart_string` does
    /// not strip `*/` (it's not a Dart escape concern), so this
    /// test confirms the literal is escaped as a string AND the
    /// comment delimiter survives intact. Critically: the `$`
    /// interpolation char must be escaped to `\$` so a slot like
    /// `$cmd` can't trigger Dart string interpolation.
    #[test]
    fn host_link_with_special_chars_in_href_is_escaped() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![LayoutProp {
                    name: "href".into(),
                    value: LayoutPropValue::String("https://e.com?q=$cmd\"oops".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        // `$` becomes `\$`; `"` becomes `\"` — both verified.
        assert!(
            r.output.contains(r#"\$cmd\"oops"#),
            "expected escaped `\\$cmd\\\"oops`, got:\n{}",
            r.output
        );
    }

    /// UI29-4 Flutter test 4b — security regression: an `href` value
    /// containing a `*/` sequence must NOT terminate the surrounding
    /// `/* TODO: launchUrl(...) */` block comment in the generated
    /// Dart. Dart's comment tokenizer is greedy and ignores
    /// string-literal quotes, so without the `*/` → `*\\u002f` rewrite
    /// inside the comment, an `href = "x*/exit(0);/*"` would let the
    /// injected `exit(0)` run inside the onTap closure. The fix
    /// substitutes `\\u002f` (which decodes to `/` inside the string
    /// at runtime) so the URL is unchanged but the source-level
    /// comment terminator is broken.
    #[test]
    fn host_link_with_comment_terminator_in_href_is_neutralised() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostLink",
                vec![LayoutProp {
                    name: "href".into(),
                    value: LayoutPropValue::String("x*/exit(0);/*".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;

        // The launchUrl TODO comment must terminate exactly once,
        // at its intended `*/`. Find the opening `/* TODO: launchUrl`
        // and scan forward — the first `*/` we see must come AFTER
        // the closing `"))`, not in the middle of the href.
        let open_pos = out
            .find("/* TODO: launchUrl(Uri.parse(")
            .expect("expected launchUrl TODO opener");
        let after_open = &out[open_pos..];
        let close_pos = after_open.find("*/").expect("expected comment closer");
        let comment_body = &after_open[..close_pos];
        // The comment body must contain the neutralised sequence,
        // never the raw `*/` that would close the comment early.
        assert!(
            comment_body.contains("*\\u002f"),
            "expected `*/` to be neutralised to `*\\u002f` inside the comment; got body:\n{comment_body}"
        );
        // Sanity: `exit(0)` must appear ONLY inside the comment body,
        // never as live Dart between the closer and the next token.
        assert!(comment_body.contains("exit(0)"));
        let after_close = &after_open[close_pos + 2..];
        assert!(
            !after_close.contains("exit(0)"),
            "injection: `exit(0)` appears OUTSIDE the comment in:\n{after_close}"
        );
    }

    /// UI29-4 Flutter test 5 — `HostTooltip` wraps its single child
    /// in `Tooltip(message:, child:)`.
    #[test]
    fn host_tooltip_with_text_and_child_emits_tooltip_widget() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostTooltip",
                vec![LayoutProp {
                    name: "text".into(),
                    value: LayoutPropValue::String("Click to save".into()),
                }],
                vec![node_with(
                    "HostButton",
                    vec![LayoutProp {
                        name: "label".into(),
                        value: LayoutPropValue::String("Save".into()),
                    }],
                    vec![],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(out.contains("Tooltip("), "expected Tooltip, got:\n{out}");
        assert!(
            out.contains("message: \"Click to save\""),
            "expected `message:` arg, got:\n{out}"
        );
        assert!(
            out.contains("ElevatedButton"),
            "expected child ElevatedButton, got:\n{out}"
        );
    }

    /// UI29-4 Flutter test 6 — `HostTooltip` with slot-ref text uses
    /// the bare identifier (no `"..."` quoting).
    #[test]
    fn host_tooltip_with_slot_text_uses_bare_identifier() {
        let m = component("X", vec![slot("hint", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostTooltip",
                vec![LayoutProp {
                    name: "text".into(),
                    value: LayoutPropValue::SlotRef("hint".into()),
                }],
                vec![node("Box")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("message: hint,"),
            "expected `message: hint,`, got:\n{}",
            r.output
        );
    }

    /// UI29-4 Flutter test 7 — `HostNumberInput` lowers to a
    /// `TextField` with `keyboardType: TextInputType.number` (the
    /// primary mobile-keypad win) and a `TextEditingController`
    /// initialised from the bound slot's `.toString()`.
    #[test]
    fn host_number_input_emits_textfield_with_number_keyboard() {
        let m = component("X", vec![slot("quantity", SlotType::Number, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostNumberInput",
                vec![LayoutProp {
                    name: "value".into(),
                    value: LayoutPropValue::SlotRef("quantity".into()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(
            out.contains("TextField("),
            "expected TextField, got:\n{out}"
        );
        assert!(
            out.contains("keyboardType: TextInputType.number"),
            "expected `TextInputType.number`, got:\n{out}"
        );
        assert!(
            out.contains("TextEditingController(text: quantity.toString())"),
            "expected `.toString()` on the slot, got:\n{out}"
        );
    }

    /// UI29-4 Flutter test 8 — `HostNumberInput` with `min`/`max`/
    /// `step` numeric literals emits them in the range hint
    /// comment. These come from `LayoutPropValue::Number(f64)` so
    /// they're injection-safe by construction.
    #[test]
    fn host_number_input_with_min_max_step_emits_range_hint() {
        let m = component("X", vec![slot("n", SlotType::Number, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostNumberInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("n".into()),
                    },
                    LayoutProp {
                        name: "min".into(),
                        value: LayoutPropValue::Number(0.0),
                    },
                    LayoutProp {
                        name: "max".into(),
                        value: LayoutPropValue::Number(100.0),
                    },
                    LayoutProp {
                        name: "step".into(),
                        value: LayoutPropValue::Number(5.0),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(
            out.contains("min: 0"),
            "expected `min: 0` in range hint, got:\n{out}"
        );
        assert!(
            out.contains("max: 100"),
            "expected `max: 100` in range hint, got:\n{out}"
        );
        assert!(
            out.contains("step: 5"),
            "expected `step: 5` in range hint, got:\n{out}"
        );
    }

    /// UI29-4 Flutter test 9 — `HostNumberInput` with `onChange`
    /// wires `onSubmitted` (commit semantics — spec §3.3 explicitly
    /// rejects per-keystroke dispatch for numeric fields).
    #[test]
    fn host_number_input_with_on_change_wires_on_submitted() {
        let m = component(
            "X",
            vec![slot("n", SlotType::Number, true)],
            vec![emit(
                "onValueChange",
                vec![EmitParam {
                    name: "value".into(),
                    r#type: EmitPayloadType::Number,
                }],
            )],
        );
        let l = layout(
            "X",
            node_with(
                "HostNumberInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("n".into()),
                    },
                    LayoutProp {
                        name: "onChange".into(),
                        value: LayoutPropValue::EmitRef("onValueChange".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        let out = &r.output;
        assert!(
            out.contains("onSubmitted: (v) {"),
            "expected `onSubmitted:` (commit semantics), got:\n{out}"
        );
        // After flutter-emit-other-hosts: the dispatch is real, not
        // a TODO.  The event subclass is `<Component>Event<Case>`
        // with the parsed `value` payload.
        assert!(
            out.contains("dispatch(XEventValueChange(value: double.tryParse(v) ?? 0))"),
            "expected real dispatch call, got:\n{out}"
        );
        assert!(
            !out.contains("/* TODO: dispatch ValueChange"),
            "expected no TODO placeholder, got:\n{out}"
        );
    }

    /// UI29-4 Flutter test 10 — `HostNumberInput` with `disabled:
    /// true` keyword sets `enabled: false`.
    #[test]
    fn host_number_input_disabled_true_emits_enabled_false() {
        let m = component("X", vec![slot("n", SlotType::Number, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostNumberInput",
                vec![
                    LayoutProp {
                        name: "value".into(),
                        value: LayoutPropValue::SlotRef("n".into()),
                    },
                    LayoutProp {
                        name: "disabled".into(),
                        value: LayoutPropValue::Keyword("true".into()),
                    },
                ],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).unwrap();
        assert!(
            r.output.contains("enabled: false"),
            "expected `enabled: false`, got:\n{}",
            r.output
        );
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
                transitions: vec![],
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

    // ================================================================
    // UI31 — HostTable a11y gate + RTL contract (Flutter backend)
    //
    // Mirrors the React (#4143) and HTML (#4156) + WebComponent
    // (#4162) precedents:
    //
    // - **A11y gate**: the lowering must produce Flutter's native
    //   `DataTable` widget (not a hand-rolled `Container`/`Row` mess
    //   that loses screen-reader semantics). The grep test is the
    //   gate.
    // - **RTL gate**: when `dir:` is authored, the `DataTable` is
    //   wrapped in `Directionality(textDirection: ..., child: ...)`.
    //   Allow-list is `ltr|rtl|auto`; unknown keywords drop silently
    //   so an attacker-controlled keyword can't break out of the
    //   format string.
    // ================================================================

    /// Unsupported/empty HostTable shapes retain the visual fallback rather
    /// than making a false native-semantics claim.
    #[test]
    fn ui31_unsupported_host_table_retains_visual_fallback() {
        let m = component("T", vec![], vec![]);
        let l = layout("T", node("HostTable"));
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            r.output.contains("Column("),
            "HostTable must lower to Column(children: [...]) (UI28-1 / U29-D1), got:\n{}",
            r.output
        );
    }

    /// UI31 §3.1 a11y gate — the canonical dynamic Grid structure lowers to
    /// Flutter's native DataTable/DataColumn/DataRow/DataCell family.
    #[test]
    fn ui31_canonical_dynamic_host_table_uses_native_datatable_widget() {
        let for_prop = |name: &str, value: LayoutPropValue| LayoutProp {
            name: name.into(),
            value,
        };
        let header_cells = node_with(
            "For",
            vec![
                for_prop("each", LayoutPropValue::Expr("headers".into())),
                for_prop("as", LayoutPropValue::Keyword("h".into())),
                for_prop("index", LayoutPropValue::Keyword("ch".into())),
            ],
            vec![node_with(
                "Box",
                vec![],
                vec![node_with(
                    "Text",
                    vec![for_prop("content", LayoutPropValue::Expr("h".into()))],
                    vec![],
                )],
            )],
        );
        let body_cells = node_with(
            "For",
            vec![
                for_prop("each", LayoutPropValue::Keyword("row".into())),
                for_prop("as", LayoutPropValue::Keyword("v".into())),
                for_prop("index", LayoutPropValue::Keyword("c".into())),
            ],
            vec![node_with(
                "Text",
                vec![for_prop("content", LayoutPropValue::Expr("v".into()))],
                vec![],
            )],
        );
        let body_rows = node_with(
            "For",
            vec![
                for_prop("each", LayoutPropValue::Expr("rows".into())),
                for_prop("as", LayoutPropValue::Keyword("row".into())),
                for_prop("index", LayoutPropValue::Keyword("r".into())),
            ],
            vec![node_with("Row", vec![], vec![body_cells])],
        );
        let table = node_with(
            "HostTable",
            vec![],
            vec![
                node_with(
                    "HostTableHead",
                    vec![],
                    vec![node_with("Row", vec![], vec![header_cells])],
                ),
                node_with("HostTableBody", vec![], vec![body_rows]),
            ],
        );
        let r = from_pipeline(
            &component("T", vec![], vec![]),
            &layout("T", table),
            &empty_style("T"),
        )
        .unwrap();

        assert!(r.output.contains("DataTable("), "{}", r.output);
        assert!(
            r.output
                .contains("(headers).isEmpty ? const SizedBox.shrink()"),
            "{}",
            r.output
        );
        assert!(
            r.output.contains("DataColumn(label: Text(h))"),
            "{}",
            r.output
        );
        assert!(r.output.contains("return DataRow("), "{}", r.output);
        assert!(r.output.contains("DataCell(Text(v))"), "{}", r.output);
        assert!(r.output.contains("key: ValueKey(r)"), "{}", r.output);
        assert!(
            !r.output.contains("final ch ="),
            "unused header indices must not be emitted: {}",
            r.output
        );
        assert!(
            !r.output.contains("final c ="),
            "unused cell indices must not be emitted: {}",
            r.output
        );
    }

    /// UI31 §3.2 RTL contract — `dir: rtl` wraps the `DataTable` in
    /// `Directionality(textDirection: TextDirection.rtl, child: ...)`.
    /// Flutter's `Directionality` is the canonical RTL knob; column
    /// ordering inside `DataTable` flips when the ambient text
    /// direction is RTL.
    #[test]
    fn ui31_rtl_host_table_dir_rtl_keyword_wraps_in_directionality() {
        let m = component("T", vec![], vec![]);
        let table = node_with(
            "HostTable",
            vec![LayoutProp {
                name: "dir".into(),
                value: LayoutPropValue::Keyword("rtl".into()),
            }],
            vec![],
        );
        let l = layout("T", table);
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            r.output.contains("Directionality("),
            "expected Directionality(...) wrapper, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("textDirection: TextDirection.rtl"),
            "expected textDirection: TextDirection.rtl, got:\n{}",
            r.output
        );
    }

    /// `dir: ltr` wraps with `TextDirection.ltr`. Symmetry with the
    /// `rtl` test; this exists to lock the keyword→enum mapping so
    /// a future refactor can't accidentally invert the polarity.
    #[test]
    fn ui31_rtl_host_table_dir_ltr_keyword_wraps_with_ltr_text_direction() {
        let m = component("T", vec![], vec![]);
        let table = node_with(
            "HostTable",
            vec![LayoutProp {
                name: "dir".into(),
                value: LayoutPropValue::Keyword("ltr".into()),
            }],
            vec![],
        );
        let l = layout("T", table);
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            r.output.contains("textDirection: TextDirection.ltr"),
            "expected textDirection: TextDirection.ltr, got:\n{}",
            r.output
        );
    }

    /// `dir: auto` is the spec-mandated "let the host decide"
    /// keyword. Flutter has no `TextDirection.auto` enum value —
    /// the right behaviour is to NOT wrap so the ambient
    /// `Directionality` (typically supplied by `MaterialApp`) flows
    /// through unchanged. A wrap with an invented enum value would
    /// not compile.
    #[test]
    fn ui31_rtl_host_table_dir_auto_keyword_does_not_wrap() {
        let m = component("T", vec![], vec![]);
        let table = node_with(
            "HostTable",
            vec![LayoutProp {
                name: "dir".into(),
                value: LayoutPropValue::Keyword("auto".into()),
            }],
            vec![],
        );
        let l = layout("T", table);
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            !r.output.contains("Directionality("),
            "auto must NOT emit a Directionality wrapper, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("Column("),
            "HostTable body should render as Column (UI28-1 / U29-D1 — DataTable doesn't fit the For-driven Grid shape), got:\n{}",
            r.output
        );
    }

    /// `dir: slot: layout-direction` interpolates the bound slot
    /// (camel-cased to `layoutDirection`) into the `textDirection:`
    /// position. The slot is expected to be a Dart expression that
    /// evaluates to a `TextDirection`; this is the contract the host
    /// must honour. Slot name goes through `is_safe_dart_identifier`
    /// so it can't smuggle malicious characters into the source.
    #[test]
    fn ui31_rtl_host_table_dir_slot_ref_interpolates_camel_case_identifier() {
        let m = component(
            "T",
            vec![slot("layout-direction", SlotType::Text, true)],
            vec![],
        );
        let table = node_with(
            "HostTable",
            vec![LayoutProp {
                name: "dir".into(),
                value: LayoutPropValue::SlotRef("layout-direction".into()),
            }],
            vec![],
        );
        let l = layout("T", table);
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            r.output.contains("textDirection: layoutDirection"),
            "expected textDirection: layoutDirection, got:\n{}",
            r.output
        );
    }

    /// Unknown `dir:` keywords (anything outside the `ltr|rtl|auto`
    /// allow-list) MUST drop silently. This is the security gate: an
    /// attacker-controlled keyword can't sneak `child: pwn(),` style
    /// payloads into the generated Dart source because it never
    /// reaches the format string. The bare `DataTable` still renders.
    #[test]
    fn ui31_rtl_host_table_unknown_dir_keyword_drops_silently() {
        let m = component("T", vec![], vec![]);
        let table = node_with(
            "HostTable",
            vec![LayoutProp {
                name: "dir".into(),
                value: LayoutPropValue::Keyword("rtl, child: pwn()".into()),
            }],
            vec![],
        );
        let l = layout("T", table);
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            !r.output.contains("pwn()"),
            "unknown keyword payload must not appear, got:\n{}",
            r.output
        );
        assert!(
            !r.output.contains("Directionality("),
            "unknown keyword must NOT wrap, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("Column("),
            "HostTable body should render as Column (UI28-1 / U29-D1 — DataTable doesn't fit the For-driven Grid shape), got:\n{}",
            r.output
        );
    }

    /// Regression guard — `HostTable` with no `dir:` prop emits the
    /// bare `DataTable` without a `Directionality` wrapper. A future
    /// refactor that always-wraps would break authors who rely on
    /// inheriting from an ancestor `Directionality` (typically
    /// supplied by `MaterialApp`).
    #[test]
    fn ui31_rtl_host_table_without_dir_prop_does_not_wrap() {
        let m = component("T", vec![], vec![]);
        let l = layout("T", node("HostTable"));
        let r = from_pipeline(&m, &l, &empty_style("T")).unwrap();
        assert!(
            !r.output.contains("Directionality("),
            "no Directionality wrapper expected when dir is absent, got:\n{}",
            r.output
        );
        assert!(
            r.output.contains("Column("),
            "HostTable should render as Column (UI28-1 / U29-D1), got:\n{}",
            r.output
        );
    }

    // =================================================================
    // UI32-K-flutter — `--emit-project` Flutter shell tests
    //
    // Covers UI32 spec §3.1-§3.8 per-PR gates:
    //   §3.4 Composable     : default options = no project shell.
    //   §3.5 Banner          : every emitted file starts with banner.
    //   §3.1 Reproducible    : two runs produce byte-identical output.
    //   §3.6.1 Validation    : invalid pub name → fail-loud error.
    //   §3.6.2 Flutter row   : derived name satisfies Dart pub RFC.
    //   §3.6.3 Pinning       : pubspec.yaml contains pinned SDKs.
    //   §3.7 Output paths    : only the spec §2.2 enumeration.
    //   §3.8 No env reads    : no /Users/, $HOME, etc. in output.
    // =================================================================

    #[test]
    fn ui32_emit_project_false_is_backward_compatible_with_from_pipeline() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let s = empty_style("X");

        let legacy = from_pipeline(&m, &l, &s).unwrap();
        let extended = from_pipeline_with_options(&m, &l, &s, &EmitOptions::default()).unwrap();

        assert_eq!(legacy.output, extended.output, ".dart bytes diverged");
        assert_eq!(legacy.component_name, extended.component_name);
        assert!(
            extended.project.is_none(),
            "default options must NOT emit a project shell"
        );
    }

    #[test]
    fn ui32_emit_project_true_returns_project_files() {
        let m = component("Hello", vec![], vec![]);
        let l = layout("Hello", node("Box"));
        let s = empty_style("Hello");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let r = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert!(
            r.project.is_some(),
            "emit_project: true must produce a shell"
        );
    }

    #[test]
    fn native_complete_shell_requires_runtime_props_without_samples() {
        let mut count = slot("count", SlotType::Number, false);
        count.default = Some(SlotDefault::Number(3.0));
        let m = component(
            "Card",
            vec![
                slot("label", SlotType::Text, true),
                slot("subtitle", SlotType::Text, false),
                count,
            ],
            vec![],
        );
        let l = layout("Card", node("Box"));
        let opts = EmitOptions {
            emit_project: true,
            require_runtime: true,
            ..EmitOptions::default()
        };
        let project = from_pipeline_with_options(&m, &l, &empty_style("Card"), &opts)
            .unwrap()
            .project
            .expect("strict project shell");

        assert!(project.main_dart.contains("MosaicHost.loadRequired()"));
        assert!(project.main_dart.contains("required this.mosaicHost"));
        assert!(project.main_dart.contains("bool _hostReady = false"));
        assert!(project.main_dart.contains("response.containsKey('props')"));
        assert!(project
            .main_dart
            .contains("mosaicRequiredString(_hostProps, \"label\")"));
        assert!(project
            .main_dart
            .contains("mosaicOptionalString(_hostProps, \"subtitle\")"));
        assert!(project
            .main_dart
            .contains("mosaicOptionalDouble(_hostProps, \"count\") ?? 3.0"));
        assert!(project.main_dart.contains("Starting Card"));
        assert!(project
            .main_dart
            .contains("child: const CircularProgressIndicator()"));
        assert!(!project.main_dart.contains(": const Semantics("));
        assert!(project.main_dart.contains("return value;"));
        assert!(!project.main_dart.contains("MosaicHost?"));
        assert!(!project.main_dart.contains("_mosaicHost?."));
        assert!(!project.main_dart.contains("debugPrint(\"event:"));
        assert!(!project.main_dart.contains("Sample Label"));
        assert!(project
            .mosaic_host_dart
            .contains("static MosaicHost loadRequired()"));
        assert!(project
            .readme
            .contains("requires Mosaic's standard Rust application runtime"));
        assert!(project
            .readme
            .contains("never substitutes preview/sample values"));
    }

    #[test]
    fn ui32_every_emitted_side_file_carries_auto_generated_banner() {
        let m = component("Hello", vec![], vec![]);
        let l = layout("Hello", node("Box"));
        let s = empty_style("Hello");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .expect("project shell expected");

        // YAML uses `#` for comments.
        assert!(
            proj.pubspec_yaml.starts_with("# AUTO-GENERATED"),
            "pubspec.yaml must START with `# AUTO-GENERATED`, got:\n{}",
            &proj.pubspec_yaml[..80.min(proj.pubspec_yaml.len())]
        );
        // Dart uses `//` for line comments.
        assert!(
            proj.main_dart.starts_with("// AUTO-GENERATED"),
            "lib/main.dart must START with `// AUTO-GENERATED`"
        );
        assert!(
            proj.mosaic_host_dart.starts_with("// AUTO-GENERATED"),
            "lib/mosaic_host.dart must START with `// AUTO-GENERATED`"
        );
        assert!(
            proj.analysis_options_yaml.starts_with("# AUTO-GENERATED"),
            "analysis_options.yaml must START with `# AUTO-GENERATED`"
        );
        assert!(
            proj.widget_test_dart.starts_with("// AUTO-GENERATED"),
            "test/widget_test.dart must START with `// AUTO-GENERATED`"
        );
        // README uses HTML-comment syntax.
        assert!(
            proj.readme.starts_with("<!-- AUTO-GENERATED"),
            "README.md must START with banner"
        );
    }

    #[test]
    fn ui32_emit_project_is_byte_deterministic() {
        let m = component("Deterministic", vec![], vec![]);
        let l = layout("Deterministic", node("Box"));
        let s = empty_style("Deterministic");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };

        let a = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        let b = from_pipeline_with_options(&m, &l, &s, &opts).unwrap();
        assert_eq!(a.output, b.output, ".dart is not deterministic");
        assert_eq!(a.project, b.project, "project shell is not deterministic");
    }

    /// §3.6.1 + §3.6.2 Flutter row. Invalid Dart pub name (uppercase,
    /// hyphen, leading underscore/digit) MUST fail-loud, not silently
    /// substitute.
    #[test]
    fn ui32_invalid_explicit_pub_name_returns_error() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            package_name: Some("Mosaic-Grid".to_string()), // uppercase + hyphen
            ..EmitOptions::default()
        };

        let err =
            from_pipeline_with_options(&m, &l, &s, &opts).expect_err("invalid pub name must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("Mosaic-Grid") && msg.contains("pub naming convention"),
            "expected Dart pub RFC violation error, got: {msg}"
        );
    }

    /// §3.6.2 Flutter row: PascalCase component name → snake_case pub
    /// name + `mosaic_` prefix.
    #[test]
    fn ui32_derived_pub_name_snake_cases_pascalcase() {
        let m = component("ProfileCard", vec![], vec![]);
        let l = layout("ProfileCard", node("Box"));
        let s = empty_style("ProfileCard");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        assert!(
            proj.pubspec_yaml.contains("name: mosaic_profile_card"),
            "expected `mosaic_profile_card` pub name, got:\n{}",
            proj.pubspec_yaml
        );
    }

    /// §3.6.3 Pinning. pubspec.yaml carries the default pinned
    /// SDK ranges verbatim.
    #[test]
    fn ui32_pubspec_carries_pinned_default_sdks_exactly() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        assert!(
            proj.pubspec_yaml.contains("sdk: '>=3.5.0 <4.0.0'"),
            "expected Dart SDK pin, got:\n{}",
            proj.pubspec_yaml
        );
        assert!(
            proj.pubspec_yaml.contains("flutter: '>=3.32.0 <4.0.0'"),
            "expected Flutter SDK pin, got:\n{}",
            proj.pubspec_yaml
        );
        assert!(
            proj.pubspec_yaml
                .contains("flutter_lints: '>=6.0.0 <7.0.0'"),
            "expected analyzer dependency, got:\n{}",
            proj.pubspec_yaml
        );
        assert!(proj.readme.contains("Flutter SDK 3.32+"));
    }

    /// §3.7 Output paths tripwire.
    #[test]
    fn ui32_project_files_struct_exposes_complete_flutter_bootstrap_files() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let ProjectFiles {
            pubspec_yaml,
            analysis_options_yaml,
            main_dart,
            mosaic_host_dart,
            widget_test_dart,
            readme,
        } = proj;
        assert!(!pubspec_yaml.is_empty(), "pubspec.yaml empty");
        assert!(
            !analysis_options_yaml.is_empty(),
            "analysis_options.yaml empty"
        );
        assert!(!main_dart.is_empty(), "lib/main.dart empty");
        assert!(!mosaic_host_dart.is_empty(), "lib/mosaic_host.dart empty");
        assert!(!widget_test_dart.is_empty(), "test/widget_test.dart empty");
        assert!(!readme.is_empty(), "README.md empty");
    }

    #[test]
    fn generated_widget_test_uses_actual_pub_name_and_mosaic_app() {
        let m = component("ProfileCard", vec![], vec![]);
        let project = from_pipeline_with_options(
            &m,
            &layout("ProfileCard", node("Box")),
            &empty_style("ProfileCard"),
            &EmitOptions {
                emit_project: true,
                ..EmitOptions::default()
            },
        )
        .unwrap()
        .project
        .unwrap();

        assert!(project
            .widget_test_dart
            .contains("package:mosaic_profile_card/main.dart"));
        assert!(project
            .widget_test_dart
            .contains("tester.pumpWidget(const MosaicApp())"));
        assert!(!project.widget_test_dart.contains("MyApp"));
        assert!(!project.main_dart.contains("_hostProps"));
        assert!(project
            .analysis_options_yaml
            .contains("include: package:flutter_lints/flutter.yaml"));
    }

    #[test]
    fn runtime_required_widget_test_does_not_load_native_runtime() {
        let m = component("ProfileCard", vec![], vec![]);
        let project = from_pipeline_with_options(
            &m,
            &layout("ProfileCard", node("Box")),
            &empty_style("ProfileCard"),
            &EmitOptions {
                emit_project: true,
                require_runtime: true,
                ..EmitOptions::default()
            },
        )
        .unwrap()
        .project
        .unwrap();

        assert!(project
            .widget_test_dart
            .contains("expect(MosaicApp, isNotNull)"));
        assert!(!project.widget_test_dart.contains("loadRequired"));
        assert!(!project.widget_test_dart.contains("pumpWidget"));
        assert!(!project.main_dart.contains("_hostProps"));
    }

    #[test]
    fn ui32_emitted_files_contain_no_environment_specific_strings() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node("Box"));
        let s = empty_style("X");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        let all = format!("{}\n{}\n{}", proj.pubspec_yaml, proj.main_dart, proj.readme);
        for banned in ["/Users/", "/home/", "C:\\Users\\", "$HOME"] {
            assert!(
                !all.contains(banned),
                "emitted shell contains environment-specific fragment `{banned}`"
            );
        }
    }

    /// lib/main.dart mounts the component as the MaterialApp's
    /// home widget. Verify the package-local import + constructor
    /// invocation.
    #[test]
    fn ui32_main_dart_mounts_component_in_material_app_home() {
        let m = component("MyWidget", vec![], vec![]);
        let l = layout("MyWidget", node("Box"));
        let s = empty_style("MyWidget");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();
        assert!(
            proj.main_dart.contains("import 'MyWidget.dart';"),
            "main.dart must import the component from lib/, got:\n{}",
            proj.main_dart
        );
        assert!(
            proj.main_dart.contains("MyWidget("),
            "main.dart must invoke MyWidget constructor"
        );
        assert!(
            proj.main_dart.contains("import 'mosaic_host.dart';"),
            "main.dart must import the optional Mosaic host hook"
        );
        assert!(
            proj.main_dart.contains("MosaicHost.load()"),
            "main.dart must load the optional Mosaic host"
        );
        assert!(
            proj.main_dart
                .contains("class MosaicApp extends StatefulWidget")
                && proj
                    .main_dart
                    .contains("const MosaicApp({super.key, this.mosaicHost})")
                && proj
                    .main_dart
                    .contains("widget.mosaicHost ?? MosaicHost.load()"),
            "main.dart must expose host injection for direct shell acceptance"
        );
        assert!(
            proj.readme
                .contains("flutter create --platforms=macos,windows,linux ."),
            "README must document the platform-runner bootstrap"
        );
        assert!(
            proj.main_dart
                .contains("_queueMosaicResponse(_mosaicHost?.props())"),
            "main.dart must hydrate initial props through the Mosaic host"
        );
        assert!(
            proj.main_dart.contains(
                "_mosaicHost?.setPropsChangedHandler(() =>\n        _queueMosaicResponse(_mosaicHost.props()))"
            ),
            "main.dart must subscribe to host-owned page interaction updates"
        );
        assert!(
            proj.main_dart
                .contains("final response = _mosaicHost?.handleEvent(event.mosaicEnvelope);"),
            "main.dart must forward Mosaic event envelopes to the host"
        );
        assert!(
            proj.main_dart
                .contains("debugPrint(\"event: ${event.mosaicEnvelope}\");"),
            "main.dart must keep a sample fallback when no host is installed"
        );
        assert!(
            proj.main_dart.contains("MaterialApp("),
            "main.dart must wrap in MaterialApp"
        );
        assert!(
            proj.mosaic_host_dart
                .contains("static MosaicHost? load() => null;"),
            "default mosaic_host.dart must be a no-op hook"
        );
        assert!(
            proj.mosaic_host_dart
                .contains("FutureOr<Map<String, Object?>?> handleEvent"),
            "default mosaic_host.dart must allow async host responses"
        );
        assert!(
            proj.mosaic_host_dart
                .contains("void setPropsChangedHandler(void Function()? handler) {}"),
            "default mosaic_host.dart must expose optional prop-change notifications"
        );
    }

    #[test]
    fn ui32_main_dart_passes_sample_slot_values_to_component() {
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
                slot(
                    "tags",
                    SlotType::List(Box::new(mosmodel_compiler::ListInnerType::Text)),
                    true,
                ),
            ],
            vec![],
        );
        let l = layout("ProfileCard", node("Box"));
        let s = empty_style("ProfileCard");
        let opts = EmitOptions {
            emit_project: true,
            ..EmitOptions::default()
        };
        let proj = from_pipeline_with_options(&m, &l, &s, &opts)
            .unwrap()
            .project
            .unwrap();

        assert!(proj.main_dart.contains("ProfileCard("));
        assert!(proj
            .main_dart
            .contains("displayName: mosaicString(_hostProps, \"display-name\", \"Ada\"),"));
        assert!(proj
            .main_dart
            .contains("age: mosaicDouble(_hostProps, \"age\", 0.0),"));
        assert!(proj
            .main_dart
            .contains("isActive: mosaicBoolean(_hostProps, \"is-active\", false),"));
        assert!(proj
            .main_dart
            .contains("avatarUrl: mosaicString(_hostProps, \"avatar-url\", \"sample-image\"),"));
        assert!(proj
            .main_dart
            .contains("accent: mosaicString(_hostProps, \"accent\", \"#808080\"),"));
        assert!(proj
            .main_dart
            .contains("tags: mosaicStringList(_hostProps, \"tags\"),"));
        assert!(proj.main_dart.contains("_queueMosaicResponse(response);"));
    }

    /// Truth table for is_valid_dart_pub_name.
    #[test]
    fn ui32_is_valid_dart_pub_name_truth_table() {
        // Accepts (lowercase + underscores + digits, starts with letter)
        assert!(is_valid_dart_pub_name("foo"));
        assert!(is_valid_dart_pub_name("foo_bar"));
        assert!(is_valid_dart_pub_name("mosaic_grid"));
        assert!(is_valid_dart_pub_name("foo123"));
        assert!(is_valid_dart_pub_name("a"));
        // Rejects
        assert!(!is_valid_dart_pub_name(""));
        assert!(!is_valid_dart_pub_name("Foo")); // uppercase
        assert!(!is_valid_dart_pub_name("foo-bar")); // hyphen
        assert!(!is_valid_dart_pub_name("_foo")); // leading underscore
        assert!(!is_valid_dart_pub_name("1foo")); // leading digit
        assert!(!is_valid_dart_pub_name("foo bar")); // space
        assert!(!is_valid_dart_pub_name("foo.bar")); // dot (Dart pub rejects)
        assert!(!is_valid_dart_pub_name(&"a".repeat(65))); // over 64 char limit
    }

    // =====================================================================
    // UI29-FU / Phase 2 — Flutter For / If / Else lowering
    //
    // These tests pin the Dart shapes emitted for the three control-flow
    // primitives. The grammar already accepts For / If / Else (UI29 §3.1
    // and §3.2); UI28-1 §6.2 calls out that the Flutter emitter was a
    // placeholder until this PR, blocking mosaic-pkg-grid v0.2.0 on the
    // Flutter backend.
    // =====================================================================

    // ----- Helpers (control-flow specific) -----------------------------------

    fn for_node(
        each_value: LayoutPropValue,
        as_name: &str,
        index_name: Option<&str>,
        body: Vec<LayoutNode>,
    ) -> LayoutNode {
        let mut props = vec![
            LayoutProp {
                name: "each".into(),
                value: each_value,
            },
            LayoutProp {
                name: "as".into(),
                value: LayoutPropValue::Keyword(as_name.into()),
            },
        ];
        if let Some(idx) = index_name {
            props.push(LayoutProp {
                name: "index".into(),
                value: LayoutPropValue::Keyword(idx.into()),
            });
        }
        LayoutNode {
            tag: "For".into(),
            part_name: None,
            props,
            children: body,
        }
    }

    fn if_node(when_value: LayoutPropValue, body: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "If".into(),
            part_name: None,
            props: vec![LayoutProp {
                name: "when".into(),
                value: when_value,
            }],
            children: body,
        }
    }

    fn else_node(body: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "Else".into(),
            part_name: None,
            props: vec![],
            children: body,
        }
    }

    fn text_node(content: &str) -> LayoutNode {
        LayoutNode {
            tag: "Text".into(),
            part_name: None,
            props: vec![LayoutProp {
                name: "content".into(),
                value: LayoutPropValue::String(content.into()),
            }],
            children: vec![],
        }
    }

    // ----- For: slot-ref each, as-only (no index) ---------------------------

    #[test]
    fn for_with_slot_ref_each_and_as_only_emits_simple_map_to_list() {
        // For ( each: slot: rows , as: r ) { Text(content: "x") }
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::SlotRef("rows".into()),
                "r",
                None,
                vec![text_node("x")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        // SlotRef lowers via to_camel_case_first_lower; "rows" stays "rows".
        assert!(
            out.contains("Column(children: rows.map((_) =>"),
            "expected unindexed map form, got:\n{}",
            out
        );
        // No ValueKey when index is absent.
        assert!(
            !out.contains("ValueKey"),
            "did not expect ValueKey without index: binding, got:\n{}",
            out
        );
    }

    // ----- For: with index — emits KeyedSubtree(ValueKey(i), child: ...) ----

    #[test]
    fn for_with_index_binding_emits_keyed_subtree_for_stable_keys() {
        // For ( each: slot: rows , as: row , index: r ) { Text(content: "x") }
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::SlotRef("rows".into()),
                "row",
                Some("r"),
                vec![text_node("x")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(
            out.contains("rows.asMap().entries.map((entry)"),
            "expected enumerated map form, got:\n{}",
            out
        );
        assert!(
            out.contains("final r = entry.key;"),
            "expected `final r = entry.key;`, got:\n{}",
            out
        );
        assert!(!out.contains("final row = entry.value;"));
        assert!(
            out.contains("KeyedSubtree(key: ValueKey(r)"),
            "expected stable KeyedSubtree per UI28-1 §5, got:\n{}",
            out
        );
    }

    // ----- For: Expr each — passes through verbatim -------------------------

    #[test]
    fn for_with_expr_each_passes_through_expression_text() {
        // For ( each: <expr 'cols.visible'> , as: c ) { Text(...) }
        // The validator accepts Expr; the emitter passes it through.
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::Expr("cols.visible".into()),
                "c",
                None,
                vec![text_node("col")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output
                .contains("Column(children: cols.visible.map((_) =>"),
            "expected the Expr text verbatim as the collection, got:\n{}",
            r.output
        );
    }

    // ----- For: kebab-case binding names lower to camelCase ---------------

    #[test]
    fn for_with_kebab_case_index_name_lowers_to_camel_case() {
        // For ( each: ... , as: row-data , index: row-idx ) { Text(...) }
        // The Dart-side bindings must be camelCase identifiers — not raw
        // kebab-case (which is illegal Dart syntax).
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::SlotRef("rows".into()),
                "row-data",
                Some("row-idx"),
                vec![text_node("x")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(out.contains("final rowIdx = entry.key;"));
        assert!(!out.contains("final rowData = entry.value;"));
        assert!(out.contains("ValueKey(rowIdx)"));
    }

    // ----- If: standalone (no Else) -----------------------------------------

    #[test]
    fn if_standalone_emits_ternary_with_sizedbox_else() {
        // If ( when: slot: editing ) { Text("yes") }   — no Else
        let m = component("X", vec![slot("editing", SlotType::Bool, true)], vec![]);
        let l = layout(
            "X",
            if_node(
                LayoutPropValue::SlotRef("editing".into()),
                vec![text_node("yes")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(
            out.contains("((_mosaicTruthy(editing)) ?"),
            "expected ternary on the camelCased slot ref, got:\n{}",
            out
        );
        assert!(
            out.contains(": const SizedBox.shrink())"),
            "expected SizedBox.shrink() in the empty-else branch, got:\n{}",
            out
        );
    }

    // ----- If + Else inside a Box: sibling pairing fires --------------------

    #[test]
    fn if_else_pair_inside_container_emits_full_ternary() {
        // Box {
        //   If ( when: slot: editing ) { Text("editor") }
        //   Else                       { Text("display") }
        // }
        // This shape mirrors Cell.mll exactly.
        let m = component("X", vec![slot("editing", SlotType::Bool, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "Box",
                vec![],
                vec![
                    if_node(
                        LayoutPropValue::SlotRef("editing".into()),
                        vec![text_node("editor")],
                    ),
                    else_node(vec![text_node("display")]),
                ],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(
            out.contains("((_mosaicTruthy(editing)) ?"),
            "expected the conditional ternary, got:\n{}",
            out
        );
        assert!(
            out.contains("Text(\"editor\")"),
            "expected the then-branch Text widget, got:\n{}",
            out
        );
        assert!(
            out.contains("Text(\"display\")"),
            "expected the else-branch Text widget instead of SizedBox, got:\n{}",
            out
        );
        // The post-Else SizedBox fallback must NOT appear when an Else is paired.
        assert!(
            !out.contains("SizedBox.shrink"),
            "did not expect a fallback SizedBox.shrink when Else was paired, got:\n{}",
            out
        );
    }

    // ----- If when: is an Expr — passes through verbatim --------------------

    #[test]
    fn if_with_expr_when_passes_through_expression_text() {
        // If ( when: <expr 'cellRow == editRow && cellCol == editCol'> )
        // This is the Cell.mll predicate shape — verifies the Cell case
        // works after this PR even without UI29 §3.4 (since the names
        // resolve to Cell's own slots).
        let m = component(
            "X",
            vec![
                slot("cellRow", SlotType::Number, true),
                slot("editRow", SlotType::Number, true),
                slot("cellCol", SlotType::Number, true),
                slot("editCol", SlotType::Number, true),
            ],
            vec![],
        );
        let l = layout(
            "X",
            if_node(
                LayoutPropValue::Expr("cellRow == editRow && cellCol == editCol".into()),
                vec![text_node("editing")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output
                .contains("((_mosaicTruthy(cellRow == editRow && cellCol == editCol)) ?"),
            "expected the Expr source threaded into the ternary, got:\n{}",
            r.output
        );
    }

    // ----- Empty For body — yields a SizedBox.shrink in the closure -------

    #[test]
    fn for_with_empty_body_emits_sizedbox_shrink_in_closure() {
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(LayoutPropValue::SlotRef("rows".into()), "r", None, vec![]),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output
                .contains("Column(children: rows.map((_) => const SizedBox.shrink())"),
            "expected SizedBox.shrink() body for empty For, got:\n{}",
            r.output
        );
    }

    // ----- Nested For (Grid.mll shape) compiles end-to-end ----------------
    //
    // This is the v0.2.0 Grid composition: outer For over rows,
    // inner For over a hardcoded Expr (we can't reference outer
    // bindings until UI29 §3.4 lands). The Expr-as-each form
    // sidesteps §3.4 — useful as a sanity check that the inner
    // emit_for_dart recursion works.

    // ── X5 Flutter analog: semantic-glyph lowering ──────────────

    /// `Icon (glyph: "spinner")` lowers to
    /// `CircularProgressIndicator()` — Material's `Icons.spinner`
    /// doesn't exist (would compile to `Icons.star` via the
    /// `unwrap_or("star")` default, which is the wrong visual
    /// semantic for a spinner).  The semantic table fires before
    /// the `Icons.<name>` path.
    #[test]
    fn x5_icon_with_glyph_spinner_lowers_to_circular_progress_indicator() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Icon",
                vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("spinner".to_string()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(
            out.contains("CircularProgressIndicator()"),
            "expected CircularProgressIndicator for `spinner`, got:\n{out}"
        );
        assert!(
            !out.contains("Icons.spinner"),
            "Icons.spinner doesn't exist in Material — must NOT appear, got:\n{out}"
        );
        assert!(
            !out.contains("Icons.star"),
            "default fallback Icons.star must NOT fire when semantic match succeeds, got:\n{out}"
        );
    }

    /// X5 Flutter analog: `source` prop also flows through the
    /// semantic table.  `Icon (source: "spinner")` is the same
    /// declaration in mosaic-pkg-toolkit's idiom for emitters that
    /// historically expected `source` instead of `glyph`.
    #[test]
    fn x5_icon_with_source_spinner_also_lowers_to_circular_progress_indicator() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Icon",
                vec![LayoutProp {
                    name: "source".to_string(),
                    value: LayoutPropValue::String("spinner".to_string()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output.contains("CircularProgressIndicator()"),
            "got:\n{}",
            r.output
        );
    }

    /// X5 scope: non-semantic glyph names still lower to
    /// `Icon(Icons.<name>)`.  `Save`, `home`, `settings` all stay
    /// on the FontIcon-equivalent (Icons.x) path.
    #[test]
    fn x5_icon_with_non_semantic_glyph_still_emits_icons_dot_name() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Icon",
                vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("home".to_string()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output.contains("Icon(Icons.home)"),
            "non-semantic glyph must lower to Icons.<name>, got:\n{}",
            r.output
        );
    }

    /// X5 Flutter analog: prop-name compatibility — the toolkit's
    /// preferred `glyph` name flows through the legacy `source`
    /// path so the same `.mll` source renders correctly on both
    /// XAML and Flutter.
    #[test]
    fn icon_glyph_prop_is_accepted_as_synonym_for_source() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "Icon",
                vec![LayoutProp {
                    name: "glyph".to_string(),
                    value: LayoutPropValue::String("settings".to_string()),
                }],
                vec![],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        assert!(
            r.output.contains("Icon(Icons.settings)"),
            "glyph prop must be accepted as a synonym for source, got:\n{}",
            r.output
        );
    }

    #[test]
    fn nested_for_inside_row_compiles_with_expr_inner_each() {
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::SlotRef("rows".into()),
                "row",
                Some("r"),
                vec![node_with(
                    "Row",
                    vec![],
                    vec![for_node(
                        // §3.4 placeholder: pretend `row` is an Expr.
                        // The real fix lands when scoping enables NAME refs.
                        LayoutPropValue::Expr("row".into()),
                        "cell",
                        Some("c"),
                        vec![text_node("cell")],
                    )],
                )],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        // Outer For shape.
        assert!(out.contains("rows.asMap().entries.map((entry)"));
        assert!(out.contains("final r = entry.key;"));
        assert!(out.contains("final row = entry.value;"));
        // Inner For shape — uses the Expr text 'row' as the collection.
        assert!(out.contains("...row.map((cell)"));
        assert!(!out.contains("final c = entry.key;"));
    }

    // ====================================================================
    // Spreadsheet-grid lowering (Bug A: For-spread; Bug B: cell styling)
    // ====================================================================

    /// A `Box` carrying a `part_name`, so the styled-cell path fires.
    fn box_part(part: &str, props: Vec<LayoutProp>, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            tag: "Box".into(),
            part_name: Some(part.to_string()),
            props,
            children,
        }
    }

    /// `state-when-<state>: ( expr )` prop on a Box, matching the
    /// post-resolution Cell.mll shape.
    fn state_when(state: &str, expr: &str) -> LayoutProp {
        LayoutProp {
            name: format!("state-when-{state}"),
            value: LayoutPropValue::Expr(expr.into()),
        }
    }

    /// The Grid.dark.msl `cell` part (border + padding + height +
    /// right-align + selected/editing state blocks) as a StyleDef.
    fn grid_cell_style() -> StyleDef {
        StyleDef {
            component_name: "Grid".into(),
            parts: vec![
                PartStyle {
                    name: "cell".into(),
                    base: vec![
                        StyleProp {
                            name: "border-width".into(),
                            value: "1px".into(),
                        },
                        StyleProp {
                            name: "border-color".into(),
                            value: "#3f3f46".into(),
                        },
                        StyleProp {
                            name: "padding".into(),
                            value: "2px".into(),
                        },
                        StyleProp {
                            name: "height".into(),
                            value: "22px".into(),
                        },
                        StyleProp {
                            name: "text-align".into(),
                            value: "right".into(),
                        },
                    ],
                    transitions: vec![],
                    states: vec![
                        mosstyle_compiler::StateStyle {
                            state: "selected".into(),
                            transitions: vec![],
                            props: vec![
                                StyleProp {
                                    name: "background".into(),
                                    value: "#264f78".into(),
                                },
                                StyleProp {
                                    name: "color".into(),
                                    value: "#ffffff".into(),
                                },
                            ],
                        },
                        mosstyle_compiler::StateStyle {
                            state: "editing".into(),
                            transitions: vec![],
                            props: vec![StyleProp {
                                name: "background".into(),
                                value: "#1f4f3f".into(),
                            }],
                        },
                    ],
                },
                PartStyle {
                    name: "header-cell".into(),
                    base: vec![
                        StyleProp {
                            name: "background".into(),
                            value: "#2d2d30".into(),
                        },
                        StyleProp {
                            name: "color".into(),
                            value: "#9d9d9d".into(),
                        },
                        StyleProp {
                            name: "text-align".into(),
                            value: "center".into(),
                        },
                        StyleProp {
                            name: "border-width".into(),
                            value: "1px".into(),
                        },
                        StyleProp {
                            name: "border-color".into(),
                            value: "#3f3f46".into(),
                        },
                    ],
                    transitions: vec![],
                    states: vec![],
                },
            ],
        }
    }

    // ----- Bug A: a For inside a Row SPREADS its cells (no Column) ----------

    #[test]
    fn for_inside_row_spreads_into_parent_children_no_nested_column() {
        // Row { For ( each: slot: cells , as: v , index: c ) { Text(( v )) } }
        // The header / data-row shape. The For must SPREAD with `...` so
        // the Row lays the cells out HORIZONTALLY; a nested `Column`
        // (the old bug) would stack them vertically.
        let m = component("X", vec![slot("cells", SlotType::Text, true)], vec![]);
        let inner_for = for_node(
            LayoutPropValue::SlotRef("cells".into()),
            "v",
            Some("c"),
            vec![text_node("x")],
        );
        let l = layout("X", node_with("Row", vec![], vec![inner_for]));
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        // The For lowered to a collection spread directly in the Row.
        assert!(
            out.contains("...cells.map((_)"),
            "expected a spread with an anonymous unused item binding, got:\n{out}"
        );
        // The Row owns the cells; the For must NOT wrap them in its own
        // Column (that was the vertical-stack bug).
        assert!(
            !out.contains("Column(children: cells"),
            "For inside a Row must NOT nest a Column, got:\n{out}"
        );
        assert!(
            out.contains("Row("),
            "expected the enclosing Row, got:\n{out}"
        );
    }

    // ----- A top-level For keeps the standalone Column fallback ------------

    #[test]
    fn for_at_root_keeps_standalone_column_fallback() {
        // A `For` that is NOT a direct child of a Row/Column children-list
        // (here it's the layout root) keeps the self-contained
        // `Column(children: …map().toList())` form.
        let m = component("X", vec![slot("rows", SlotType::Text, true)], vec![]);
        let l = layout(
            "X",
            for_node(
                LayoutPropValue::SlotRef("rows".into()),
                "row",
                Some("r"),
                vec![text_node("x")],
            ),
        );
        let r = from_pipeline(&m, &l, &empty_style("X")).expect("ok");
        let out = &r.output;
        assert!(
            out.contains("Column(children: rows.asMap().entries.map((entry)"),
            "root For must keep the standalone Column form, got:\n{out}"
        );
        assert!(
            out.contains(".toList())"),
            "standalone form ends in `.toList())`, got:\n{out}"
        );
        assert!(
            !out.contains("final row = entry.value;"),
            "unused authored item bindings must not become analyzer warnings:\n{out}"
        );
        assert!(
            !out.trim_start().starts_with("...") && !out.contains("return\n      ...rows"),
            "root For must not be a bare spread, got:\n{out}"
        );
    }

    // ----- Bug B: cell Container has decoration.border / width / alignment --

    #[test]
    fn styled_cell_box_emits_decorated_container() {
        // Row { For ( each: slot: cells , index: c ) {
        //   Box [cell] ( state-when-selected, state-when-editing ) {
        //     If (when: slot: is-editing) { HostInput } Else { Text }
        //   }
        // } }  inside a HostTable that carries the columnWidths colgroup.
        let m = component(
            "Grid",
            vec![
                slot("cells", SlotType::Text, true),
                slot(
                    "column-widths",
                    SlotType::List(Box::new(ListInnerType::Number)),
                    true,
                ),
                slot("is-editing", SlotType::Bool, true),
            ],
            vec![],
        );

        let cell = box_part(
            "cell",
            vec![
                state_when("selected", "( r == selectedRow && c == selectedCol )"),
                state_when("editing", "( r == editRow && c == editCol )"),
            ],
            vec![
                if_node(
                    LayoutPropValue::SlotRef("is-editing".into()),
                    vec![text_node("e")],
                ),
                else_node(vec![text_node("d")]),
            ],
        );
        let row = node_with(
            "Row",
            vec![],
            vec![for_node(
                LayoutPropValue::SlotRef("cells".into()),
                "v",
                Some("c"),
                vec![cell],
            )],
        );
        // colgroup so columnWidths threading fires.
        let colgroup = node_with(
            "HostTableColGroup",
            vec![],
            vec![for_node(
                LayoutPropValue::SlotRef("column-widths".into()),
                "w",
                Some("cw"),
                vec![node("Col")],
            )],
        );
        let table = node_with("HostTable", vec![], vec![colgroup, row]);
        let l = layout("Grid", table);

        let r = from_pipeline(&m, &l, &grid_cell_style()).expect("ok");
        let out = &r.output;

        // Border, width, height, alignment present on the cell Container.
        assert!(
            out.contains("border: Border.all(color: const Color(0xFF3F3F46), width: 1)"),
            "cell must draw the 1px #3f3f46 border, got:\n{out}"
        );
        assert!(
            out.contains("width: columnWidths[c]"),
            "cell width must index the columnWidths slot by the For index, got:\n{out}"
        );
        assert!(
            out.contains("height: 22"),
            "cell height must be 22, got:\n{out}"
        );
        assert!(
            out.contains("alignment: Alignment.centerRight"),
            "text-align:right must lower to Alignment.centerRight, got:\n{out}"
        );
        // Background rides inside the decoration (NOT a Container `color:`
        // alongside `decoration:` — that's a Flutter assertion failure).
        assert!(
            out.contains("decoration: BoxDecoration(color:"),
            "background must live inside BoxDecoration, got:\n{out}"
        );
        assert!(
            !out.contains("Container(\n")
                || !out.contains(", color: const Color")
                || out.contains("decoration: BoxDecoration"),
            "a Container must not carry both color: and decoration:, got:\n{out}"
        );
        // Selected → blue fill + white text; editing → green fill.
        assert!(
            out.contains("? const Color(0xFF264F78)"),
            "selected state must fill #264f78, got:\n{out}"
        );
        assert!(
            out.contains("? const Color(0xFF1F4F3F)"),
            "editing state must fill #1f4f3f, got:\n{out}"
        );
        assert!(
            out.contains("? const Color(0xFFFFFFFF)"),
            "selected state must whiten text, got:\n{out}"
        );
        // The state predicate text threads through verbatim.
        assert!(
            out.contains("r == selectedRow && c == selectedCol"),
            "selected predicate must reach the generated Dart, got:\n{out}"
        );
    }

    // ----- Bug B: header cell background is #2d2d30, not the text color ----

    #[test]
    fn header_cell_background_is_panel_color_not_text_color() {
        // Row { For ( each: slot: headers , index: ch ) {
        //   Box [header-cell] { Text(( h )) }
        // } }
        let m = component("Grid", vec![slot("headers", SlotType::Text, true)], vec![]);
        let header = box_part("header-cell", vec![], vec![text_node("A")]);
        let row = node_with(
            "Row",
            vec![],
            vec![for_node(
                LayoutPropValue::SlotRef("headers".into()),
                "h",
                Some("ch"),
                vec![header],
            )],
        );
        let l = layout("Grid", node_with("HostTableHead", vec![], vec![row]));

        let r = from_pipeline(&m, &l, &grid_cell_style()).expect("ok");
        let out = &r.output;

        // The Container BACKGROUND must be the panel color #2d2d30 …
        assert!(
            out.contains("decoration: BoxDecoration(color: const Color(0xFF2D2D30)"),
            "header background must be #2d2d30 inside the decoration, got:\n{out}"
        );
        // … and the TEXT color #9d9d9d must land on the child Text style,
        // NOT on the Container background (the original bug).
        assert!(
            out.contains("color: const Color(0xFF9D9D9D)"),
            "header text color #9d9d9d must reach the TextStyle, got:\n{out}"
        );
        assert!(
            !out.contains("color: const Color(0xFF9D9D9D), border"),
            "the #9d9d9d TEXT color must not be used as the box background, got:\n{out}"
        );
        // Header is center-aligned.
        assert!(
            out.contains("alignment: Alignment.center"),
            "header text-align:center must lower to Alignment.center, got:\n{out}"
        );
    }

    // ---------------------------------------------------------------------
    // #13010 — HostDialog: real showDialog wiring, not a placeholder
    // ---------------------------------------------------------------------

    /// A `HostDialog` with `open: slot: x`, a literal `title`, an
    /// `onClose` emit, and a single child lowers to a real
    /// `_MosaicDialogHost` + `AlertDialog`, not the old placeholder.
    #[test]
    fn host_dialog_with_open_slot_emits_real_dialog_host() {
        let m = component(
            "Modal",
            vec![slot("open", SlotType::Bool, true)],
            vec![emit("onClose", vec![])],
        );
        let l = layout(
            "Modal",
            node_with(
                "HostDialog",
                vec![
                    LayoutProp {
                        name: "open".into(),
                        value: LayoutPropValue::SlotRef("open".into()),
                    },
                    LayoutProp {
                        name: "title".into(),
                        value: LayoutPropValue::String("Save changes?".into()),
                    },
                    LayoutProp {
                        name: "onClose".into(),
                        value: LayoutPropValue::EmitRef("onClose".into()),
                    },
                ],
                vec![node_with(
                    "Text",
                    vec![LayoutProp {
                        name: "content".into(),
                        value: LayoutPropValue::String("Body".into()),
                    }],
                    vec![],
                )],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("Modal")).unwrap().output;
        assert!(
            !out.contains("TODO: HostDialog"),
            "expected a real dialog, not the placeholder, got:\n{out}"
        );
        assert!(
            out.contains("_MosaicDialogHost("),
            "expected the shared dialog host widget, got:\n{out}"
        );
        assert!(
            out.contains("open: open,"),
            "expected the open slot's live value, got:\n{out}"
        );
        assert!(
            out.contains("onClose: () { dispatch(const ModalEventClose()); },"),
            "expected the onClose dispatch closure, got:\n{out}"
        );
        assert!(
            out.contains("title: Text(\"Save changes?\"),"),
            "expected the literal title, got:\n{out}"
        );
        assert!(
            out.contains("content:") && out.contains("Text(\"Body\")"),
            "expected the single child as content, got:\n{out}"
        );
        assert!(
            out.contains("class _MosaicDialogHost extends StatefulWidget"),
            "expected the shared dialog helper class to be emitted, got:\n{out}"
        );
    }

    /// `dismiss-on-backdrop: false` maps to `barrierDismissible: false`.
    /// Unset (the default) emits no attribute at all, relying on
    /// Flutter's own `true` default.
    #[test]
    fn host_dialog_dismiss_on_backdrop_false_sets_barrier_dismissible_false() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostDialog",
                vec![LayoutProp {
                    name: "dismiss-on-backdrop".into(),
                    value: LayoutPropValue::Keyword("false".into()),
                }],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            out.contains("barrierDismissible: false,"),
            "expected barrierDismissible: false, got:\n{out}"
        );
    }

    #[test]
    fn host_dialog_default_dismiss_on_backdrop_emits_no_barrier_attr() {
        let m = component("X", vec![], vec![]);
        let l = layout("X", node_with("HostDialog", vec![], vec![]));
        let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        // The shared helper class itself always mentions
        // `barrierDismissible` (it's a field/parameter there) --
        // check the *call site* doesn't pass one, not that the word
        // never appears anywhere in the file.
        assert!(
            !out.contains("barrierDismissible: false"),
            "expected no barrierDismissible: false at the call site when unset, got:\n{out}"
        );
    }

    /// Multiple children are wrapped in a `Column` (via the paired
    /// walker, matching `emit_host_scroll`'s identical shape) rather
    /// than only the first child surviving.
    #[test]
    fn host_dialog_multiple_children_wrapped_in_column() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostDialog",
                vec![],
                vec![
                    node_with(
                        "Text",
                        vec![LayoutProp {
                            name: "content".into(),
                            value: LayoutPropValue::String("One".into()),
                        }],
                        vec![],
                    ),
                    node_with(
                        "Text",
                        vec![LayoutProp {
                            name: "content".into(),
                            value: LayoutPropValue::String("Two".into()),
                        }],
                        vec![],
                    ),
                ],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            out.contains("content: Column("),
            "expected multiple children wrapped in a Column, got:\n{out}"
        );
        assert!(out.contains("\"One\""), "expected first child, got:\n{out}");
        assert!(out.contains("\"Two\""), "expected second child, got:\n{out}");
    }

    /// #13010's documented scope decision: `modal: false` is NOT
    /// implemented (Flutter's `showDialog` is inherently modal) --
    /// it must keep the old placeholder rather than emit a
    /// wrong-shaped (still-modal) dialog silently.
    #[test]
    fn host_dialog_modal_false_keeps_placeholder() {
        let m = component("X", vec![], vec![]);
        let l = layout(
            "X",
            node_with(
                "HostDialog",
                vec![LayoutProp {
                    name: "modal".into(),
                    value: LayoutPropValue::Keyword("false".into()),
                }],
                vec![],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        assert!(
            out.contains("SizedBox.shrink()"),
            "expected the placeholder for modal:false, got:\n{out}"
        );
        assert!(
            !out.contains("_MosaicDialogHost("),
            "modal:false must not emit the real dialog host, got:\n{out}"
        );
        assert!(
            !out.contains("class _MosaicDialogHost"),
            "the shared helper class must not be emitted when no HostDialog uses it, got:\n{out}"
        );
    }

    /// The shared `_MosaicDialogHost` helper class is emitted exactly
    /// once even with multiple `HostDialog` nodes in the same file,
    /// and not at all when no `HostDialog` is present (matches the
    /// existing `uses_drag`-style gating pattern).
    #[test]
    fn host_dialog_helper_emitted_once_when_used_not_at_all_otherwise() {
        let m = component("X", vec![slot("open", SlotType::Bool, true)], vec![]);
        let l = layout(
            "X",
            node_with(
                "Column",
                vec![],
                vec![
                    node_with(
                        "HostDialog",
                        vec![LayoutProp {
                            name: "open".into(),
                            value: LayoutPropValue::SlotRef("open".into()),
                        }],
                        vec![],
                    ),
                    node_with(
                        "HostDialog",
                        vec![LayoutProp {
                            name: "open".into(),
                            value: LayoutPropValue::SlotRef("open".into()),
                        }],
                        vec![],
                    ),
                ],
            ),
        );
        let out = from_pipeline(&m, &l, &empty_style("X")).unwrap().output;
        let helper_count = out.matches("class _MosaicDialogHost extends StatefulWidget").count();
        assert_eq!(
            helper_count, 1,
            "expected the shared helper class exactly once, got {helper_count}:\n{out}"
        );

        let m2 = component("Y", vec![], vec![]);
        let l2 = layout("Y", node_with("Box", vec![], vec![]));
        let out2 = from_pipeline(&m2, &l2, &empty_style("Y")).unwrap().output;
        assert!(
            !out2.contains("_MosaicDialogHost"),
            "a layout with no HostDialog must not pay for the helper, got:\n{out2}"
        );
    }
}
