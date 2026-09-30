//! Package-independent native bindings for the Mosaic application C ABI.
//!
//! Each function returns deterministic source installed by the artifact builder.
//! Keeping bindings here prevents backend shells and applications from growing
//! separate FFI implementations of the same runtime protocol.

/// Generate the standard Compose/JVM JNA host binding.
pub fn compose_jna_binding() -> String {
    compose_jna_binding_source(None)
}

/// Generate a persistent Compose/JVM host for an emitted application.
pub fn compose_jna_binding_for_application(application_id: &str) -> String {
    compose_jna_binding_source(Some(application_id))
}

fn compose_jna_binding_source(application_id: Option<&str>) -> String {
    bind_application(
        include_str!("../templates/compose/MosaicRuntimeHost.kt"),
        application_id,
    )
    // Protocol 2, for the same reason as Qt and SwiftUI: this host implements
    // effect completion. Declaring 2 without it is the harmful direction, so
    // the tests pin the claim and the capability together.
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::EFFECT_PROTOCOL_VERSION.to_string(),
    )
}

/// The Compose platform library (UI87 §7): the operating-system capabilities
/// Mosaic answers for every generated app -- `files.open` and `files.save`
/// through the native file dialog -- and the router that sends each effect to
/// the app's own handler or to this library by kind. Written beside
/// `MosaicRuntimeHost.kt` in every Compose project; installed by `Main.kt`.
pub fn compose_platform_effects() -> String {
    include_str!("../templates/compose/MosaicPlatformEffects.kt").to_string()
}

/// The SwiftUI platform library (UI87 §7): `files.open` and `files.save`
/// through `NSOpenPanel` / `NSSavePanel` on macOS, a clear failure on iOS and
/// iPadOS until UI89 step 6, and the router that sends each effect to the
/// app's own handler or to this library by kind -- the same contract the
/// Compose library answers. Written beside `MosaicRuntimeHost.swift` in every
/// SwiftUI project; installed by the generated `App.swift`.
pub fn swift_platform_effects() -> String {
    include_str!("../templates/swiftui/MosaicPlatformEffects.swift").to_string()
}

/// The Qt platform library (UI87 §7, §7.4a).
pub struct QtPlatformEffects {
    pub header: String,
    pub source: String,
}

/// The Qt platform library: `files.open` and `files.save` through
/// `QFileDialog`, answered inline, and the router `installMosaicPlatformEffects`
/// sets as the host's one effect handler -- the same contract as the Compose
/// and SwiftUI libraries. Written beside `MosaicHost.{h,cpp}` in every Qt
/// project; installed by the generated `main.cpp`.
pub fn qt_platform_effects() -> QtPlatformEffects {
    QtPlatformEffects {
        header: include_str!("../templates/qt/MosaicPlatformEffects.h").to_string(),
        source: include_str!("../templates/qt/MosaicPlatformEffects.cpp").to_string(),
    }
}

/// The XAML platform library (UI87 §7, §7.6): `files.open` and `files.save`
/// through WinUI 3's `FileOpenPicker` / `FileSavePicker`, deferred and answered
/// from the window's `DispatcherQueue`, and the router that sends each effect
/// to the app's own handler or to this library by kind -- the same contract
/// the Compose, SwiftUI and Qt libraries answer. Written beside
/// `MosaicRuntimeHost.cs` in every XAML project, in the same C# namespace;
/// installed by the generated `MainWindow.xaml.cs`.
///
/// Everything WinUI sits behind `#if !MOSAIC_HEADLESS_TEST`, which no
/// generated project defines, so the headless conformance harness can run the
/// rest on plain .NET with a fake picker.
pub fn xaml_platform_effects(namespace: &str) -> String {
    include_str!("../templates/xaml/MosaicPlatformEffects.cs")
        .replace("__MOSAIC_NAMESPACE__", namespace)
}

/// Files that make the fixed Mosaic application C ABI available to SwiftUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftRuntimeBinding {
    pub host_swift: String,
    pub header: String,
    pub loader_c: String,
}

/// Generate the standard SwiftUI/Foundation host and its C dynamic loader.
pub fn swift_runtime_binding() -> SwiftRuntimeBinding {
    swift_runtime_binding_source(None)
}

/// Generate a persistent SwiftUI/Foundation host for an emitted application.
pub fn swift_runtime_binding_for_application(application_id: &str) -> SwiftRuntimeBinding {
    swift_runtime_binding_source(Some(application_id))
}

fn swift_runtime_binding_source(application_id: Option<&str>) -> SwiftRuntimeBinding {
    SwiftRuntimeBinding {
        host_swift: bind_application(
            include_str!("../templates/swiftui/MosaicRuntimeHost.swift"),
            application_id,
        )
        // Protocol 2, for the same reason as Qt: this host implements effect
        // completion. Declaring 2 without it is the harmful direction -- the
        // app emits into a void -- so the tests pin the claim and the
        // capability together.
        .replace(
            "__MOSAIC_PROTOCOL_VERSION__",
            &mosaic_app_runtime::EFFECT_PROTOCOL_VERSION.to_string(),
        ),
        header: include_str!("../templates/swiftui/CMosaicRuntime.h").to_string(),
        loader_c: include_str!("../templates/swiftui/CMosaicRuntime.c").to_string(),
    }
}

/// Connect an emitted SwiftUI shell to the standard runtime before its legacy
/// reflection-based host fallback.
pub fn swift_app_with_runtime_binding(app_swift: &str, bundle_runtime: bool) -> String {
    let with_binding = app_swift.replacen(
        "self.bridge = MosaicHostBridge.load()",
        "self.bridge = MosaicRuntimeHost.load() ?? MosaicHostBridge.load()",
        1,
    );
    if !bundle_runtime {
        return with_binding;
    }
    let runtime_path = "Bundle.module.url(forResource: \"libmosaic_app\", withExtension: \"dylib\", subdirectory: \"Runtime\")?.path";
    with_binding
        .replace(
            "MosaicRuntimeHost.loadRecoverable()",
            &format!("MosaicRuntimeHost.loadRecoverable(libraryPath: {runtime_path})"),
        )
        .replace(
            "MosaicRuntimeHost.loadRequired()",
            &format!("MosaicRuntimeHost.loadRequired(libraryPath: {runtime_path})"),
        )
        .replace(
            "MosaicRuntimeHost.load()",
            &format!("MosaicRuntimeHost.load(libraryPath: {runtime_path})"),
        )
}

/// Add the generated C loader target to an emitted Swift package manifest.
pub fn swift_package_with_runtime_binding(package_swift: &str, bundle_runtime: bool) -> String {
    let with_target = package_swift.replacen(
        "  targets: [\n    .executableTarget(",
        "  targets: [\n    .target(\n      name: \"CMosaicRuntime\",\n      path: \"Sources/CMosaicRuntime\",\n      publicHeadersPath: \"include\"\n    ),\n    .executableTarget(",
        1,
    );
    let with_binding = with_target.replacen(
        "      name: \"App\",\n      path: \"Sources/App\"",
        "      name: \"App\",\n      dependencies: [\"CMosaicRuntime\"],\n      path: \"Sources/App\"",
        1,
    );
    if bundle_runtime {
        with_binding.replacen(
            "      path: \"Sources/App\"",
            "      path: \"Sources/App\",\n      resources: [.copy(\"Runtime\")]",
            1,
        )
    } else {
        with_binding
    }
}

/// The directory, inside an emitted SwiftUI package, where a statically linked
/// runtime's `.xcframework` is placed (UI89 §2.1).
pub const SWIFT_STATIC_RUNTIME_PATH: &str = "Runtime/MosaicAppRuntime.xcframework";

/// Link a statically built runtime into an emitted Swift package (UI89 §2.1).
///
/// iOS and iPadOS do not let an app load its own `.dylib` at run time, so the
/// runtime is a static library in an `.xcframework` -- one slice per platform
/// (device, simulator, and macOS when it was built) -- that the package links:
///
/// ```text
///   .binaryTarget(name: "MosaicAppRuntime", path: "Runtime/MosaicAppRuntime.xcframework")
///   .target(name: "CMosaicRuntime", dependencies: ["MosaicAppRuntime"],
///           cSettings: [.define("MOSAIC_RUNTIME_STATIC")], ...)
/// ```
///
/// `MOSAIC_RUNTIME_STATIC` switches the C loader to name the runtime's
/// functions directly instead of looking them up with `dlsym`. Apply after
/// [`swift_package_with_runtime_binding`] with `bundle_runtime = false`: a
/// linked runtime is not a resource.
pub fn swift_package_with_static_runtime(package_swift: &str) -> String {
    let with_loader = package_swift.replacen(
        "      name: \"CMosaicRuntime\",\n      path: \"Sources/CMosaicRuntime\",\n      publicHeadersPath: \"include\"\n    ),",
        "      name: \"CMosaicRuntime\",\n      dependencies: [\"MosaicAppRuntime\"],\n      path: \"Sources/CMosaicRuntime\",\n      publicHeadersPath: \"include\",\n      cSettings: [.define(\"MOSAIC_RUNTIME_STATIC\")]\n    ),",
        1,
    );
    with_loader.replacen(
        "  targets: [\n",
        &format!(
            "  targets: [\n    .binaryTarget(\n      name: \"MosaicAppRuntime\",\n      path: \"{SWIFT_STATIC_RUNTIME_PATH}\"\n    ),\n"
        ),
        1,
    )
}

/// Generate the standard XAML/.NET host binding for the requested C# namespace.
pub fn xaml_runtime_binding(namespace: &str) -> String {
    xaml_runtime_binding_source(namespace, None)
}

/// Generate a persistent XAML/.NET host for an emitted application.
pub fn xaml_runtime_binding_for_application(namespace: &str, application_id: &str) -> String {
    xaml_runtime_binding_source(namespace, Some(application_id))
}

fn xaml_runtime_binding_source(namespace: &str, application_id: Option<&str>) -> String {
    bind_application(
        include_str!("../templates/xaml/MosaicRuntimeHost.cs"),
        application_id,
    )
    // Protocol 2, for the same reason as the other four hosts: this one
    // implements effect completion. Declaring 2 without it is the harmful
    // direction, so the tests pin the claim and the capability together.
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::EFFECT_PROTOCOL_VERSION.to_string(),
    )
    .replace("__MOSAIC_NAMESPACE__", namespace)
}

/// Generate the standard Flutter/Dart FFI host binding.
pub fn flutter_runtime_binding() -> String {
    flutter_runtime_binding_source(false, None)
}

/// Generate the standard Flutter/Dart FFI host binding for a project whose
/// selected Rust engine is registered as a bundled Dart code asset.
pub fn flutter_runtime_binding_with_bundled_asset() -> String {
    flutter_runtime_binding_source(true, None)
}

/// Generate a persistent Flutter host for an emitted application.
pub fn flutter_runtime_binding_for_application(
    application_id: &str,
    bundle_runtime: bool,
) -> String {
    flutter_runtime_binding_source(bundle_runtime, Some(application_id))
}

fn flutter_runtime_binding_source(bundle_runtime: bool, application_id: Option<&str>) -> String {
    bind_application(
        include_str!("../templates/flutter/mosaic_host.dart"),
        application_id,
    )
    // Protocol 2, for the same reason as Qt, SwiftUI and Compose: this host
    // implements effect completion. Declaring 2 without it is the harmful
    // direction, so the tests pin the claim and the capability together.
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::EFFECT_PROTOCOL_VERSION.to_string(),
    )
    .replace(
        "__MOSAIC_BUNDLED_RUNTIME__",
        if bundle_runtime { "true" } else { "false" },
    )
}

/// Add the small allocation helper used by Dart's native FFI to a generated
/// Flutter package manifest.
pub fn flutter_pubspec_with_runtime_binding(pubspec_yaml: &str) -> String {
    pubspec_yaml.replacen(
        "dependencies:\n",
        "dependencies:\n  ffi: '>=2.1.0 <3.0.0'\n",
        1,
    )
}

/// Add dependencies a package's `[host_assets]` declared for Flutter.
///
/// Each coordinate is a pubspec dependency line written verbatim -- `ffi:
/// ^2.1.3`, `file_selector: ^1.0.3` -- because the manifest carries the string
/// the package manager expects rather than trying to model every ecosystem's
/// version syntax.
///
/// This exists for the same reason the Compose equivalent does: `[host_assets]`
/// lets a package replace a generated host file, and a replacement that imports
/// something the emitter has no reason to know about needs a way to say so.
/// Engram's `mosaic_host.dart` imports `package:file_selector`, which no
/// generated Dart uses.
pub fn flutter_pubspec_with_host_asset_dependencies(
    pubspec_yaml: &str,
    coordinates: &[String],
) -> String {
    if coordinates.is_empty() {
        return pubspec_yaml.to_string();
    }
    let block: String = coordinates
        .iter()
        .map(|coordinate| format!("  {coordinate}\n"))
        .collect();
    pubspec_yaml.replacen("dependencies:\n", &format!("dependencies:\n{block}"), 1)
}

/// Add stable Dart build-hook dependencies and SDK floors to a Flutter
/// project that bundles a selected precompiled Rust code asset.
pub fn flutter_pubspec_with_bundled_runtime(pubspec_yaml: &str) -> String {
    flutter_pubspec_with_runtime_binding(pubspec_yaml)
        .replace("sdk: '>=3.5.0 <4.0.0'", "sdk: '>=3.10.0 <4.0.0'")
        .replace("flutter: '>=3.32.0 <4.0.0'", "flutter: '>=3.38.0 <4.0.0'")
        .replacen(
            "dependencies:\n",
            "dependencies:\n  code_assets: '>=1.0.0 <2.0.0'\n  hooks: '>=1.0.0 <3.0.0'\n",
            1,
        )
}

/// Files that expose the fixed Mosaic application C ABI as a QML host object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtRuntimeBinding {
    pub header: String,
    pub source: String,
}

/// Generate the standard Qt/QML host using Qt Core's dynamic loading and JSON
/// APIs, with no application-specific C++ adapter.
pub fn qt_runtime_binding() -> QtRuntimeBinding {
    qt_runtime_binding_source(None)
}

/// Generate a persistent Qt/QML host for an emitted application.
pub fn qt_runtime_binding_for_application(application_id: &str) -> QtRuntimeBinding {
    qt_runtime_binding_source(Some(application_id))
}

fn qt_runtime_binding_source(application_id: Option<&str>) -> QtRuntimeBinding {
    QtRuntimeBinding {
        // The Qt host declares protocol 2 because it implements the effect
        // completion path -- it resolves `mosaic_app_complete_effect`, answers
        // awaited effects, and fails the ones no handler took.
        //
        // UI47 §5.3 makes this an opt-in per host, and it is safe in both
        // directions: a v2 host running a v1 app never sees an `Await`, so the
        // completion path simply goes unused. Declaring 2 without implementing
        // completion would be the harmful direction -- the app would emit
        // effects into a void, which is the failure the protocol field exists
        // to make impossible.
        header: bind_application(include_str!("../templates/qt/MosaicHost.h"), application_id)
            .replace(
                "__MOSAIC_PROTOCOL_VERSION__",
                &mosaic_app_runtime::EFFECT_PROTOCOL_VERSION.to_string(),
            ),
        source: bind_application(
            include_str!("../templates/qt/MosaicHost.cpp"),
            application_id,
        ),
    }
}

fn bind_application(template: &str, application_id: Option<&str>) -> String {
    let escaped = application_id
        .unwrap_or_default()
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect::<String>();
    template
        .replace(
            "__MOSAIC_PERSISTENCE_ENABLED__",
            if application_id.is_some() {
                "true"
            } else {
                "false"
            },
        )
        .replace("__MOSAIC_APPLICATION_ID__", &escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI48 ENV4 on Compose (§7.4): the host reports the environment, keeps
    /// the showing props for a no-reaction update at the same revision only,
    /// and seeds the start context with what the platform knows.
    #[test]
    fn compose_host_reports_the_environment() {
        let host = compose_jna_binding_for_application("probe");
        assert!(host.contains("fun reportEnvironment(environment: Map<String, String>): Map<String, Any?>?"));
        assert!(host.contains("if (environment == lastReportedEnvironment) return null"));
        assert!(host.contains("mapOf(\"name\" to \"environmentChanged\", \"payload\" to environment)"));
        // Remembered only after the runtime took it: the catch returns first.
        let refused = host.find("} catch (error: MosaicRuntimeException) {").expect("refusal");
        let remembered = host.find("lastReportedEnvironment = environment.toMap()").expect("remember");
        assert!(refused < remembered);
        // Props kept only at the revision already showing.
        assert!(host.contains("if (revision != shownRevision) return update"));
        assert!(host.contains("val settled = keepShowingProps(settleEffects(update))"));
        // The start context carries pointer, hover and reduced motion.
        assert!(host.contains("for ((axis, value) in initialEnvironment()) put(axis, value)"));
        assert!(host.contains("mapOf(\"pointer\" to \"fine\", \"hover\" to \"hover\", \"reducedMotion\" to \"no-preference\")"));
        assert!(host.contains("mapOf(\"pointer\" to \"coarse\", \"hover\" to \"none\", \"reducedMotion\" to \"no-preference\")"));
    }

    /// UI48 ENV4 on XAML (§7.7): the same contract as Compose's, plus the
    /// reducer the WinUI shell calls, and no re-apply for an ignored report.
    #[test]
    fn xaml_host_reports_the_environment() {
        let host = xaml_runtime_binding_for_application("Acme.App", "probe");
        assert!(host.contains("public static string? ReportEnvironment("));
        assert!(host.contains("Dispatch(\"environmentChanged\", report);"));
        // Dropped when equal to the last one taken or the last one refused;
        // a refusal is recorded as refused; the one taken is remembered before
        // the apply (the runtime has it), and a tripped guard is reported.
        let held_back = host
            .find("if (SameEnvironment(lastRefusedEnvironment, report)) return null;")
            .expect("hold back refused");
        let sent = host.find("Dispatch(\"environmentChanged\", report);").unwrap();
        let refused = host.find("lastRefusedEnvironment = report;").expect("refused");
        let remembered = host.find("lastReportedEnvironment = report;").expect("remember");
        let guard = host
            .find("if (settleError is not null)\n                    return Status(")
            .expect("guard");
        let applied = host
            .find("ApplyProps(component, requiredProps, strict: requiredProps.Count > 0);")
            .expect("apply");
        assert!(held_back < sent && sent < refused && refused < remembered);
        assert!(remembered < guard && guard < applied);
        assert!(host.contains("if (settling > 0) return null;"));
        // Re-applied only when something newer than the last apply is showing.
        assert!(host.contains("if (Revision(latestUpdate) != appliedRevision)"));
        assert!(host.contains("appliedRevision = Revision(latestUpdate);"));
        // Props kept only at the revision already showing, on every dispatch.
        assert!(host.contains("|| revision != shownRevision)"));
        assert_eq!(
            host.matches("latestUpdate = KeepShowingProps(SettleEffects(update));").count(),
            2,
            "dispatch and effect completion"
        );
        // The thresholds every host uses, and the desktop's pointer.
        assert!(host.contains(
            "[\"sizeClass\"] = width < 600 ? \"compact\" : width < 1024 ? \"regular\" : \"expanded\","
        ));
        assert!(host.contains("[\"orientation\"] = height > width ? \"portrait\" : \"landscape\","));
        assert!(host.contains("[\"pointer\"] = \"fine\","));
        assert!(host.contains("[\"hover\"] = \"hover\","));
        assert!(host.contains("[\"reducedMotion\"] = \"no-preference\","));
        // The start context carries them too.
        assert!(host.contains("foreach (var (axis, value) in InitialEnvironment()) start[axis] = value;"));
    }

    /// UI48 ENV4 on Flutter (§7.8): the XAML contract in Dart. The reducer
    /// lives in the host beside the wire names; the shell only observes.
    #[test]
    fn flutter_host_reports_the_environment() {
        let host = flutter_runtime_binding_for_application("probe", false);
        assert!(host.contains(
            "  Map<String, Object?>? reportEnvironment(Map<String, String> environment) {\n    final runtime = _runtime;"
        ));
        assert!(host.contains("static const String _environmentChanged = 'environmentChanged';"));
        assert!(host.contains("'name': _environmentChanged,\n        'payload': report,"));
        // Held back when equal to the last one taken or the last one refused
        // (and inside a settle); a refusal is recorded as refused; the report
        // taken is remembered once the dispatch returned.
        let runtime_half = host
            .find("Map<String, Object?>? reportEnvironment(Map<String, String> environment) {\n    _ensureOpen();")
            .expect("runtime half");
        let settling = host.find("    if (_settling > 0) return null;\n    final report").expect("settle backstop");
        let taken = host
            .find("if (_sameEnvironment(_lastReportedEnvironment, report)) return null;")
            .expect("hold back taken");
        let held_back = host
            .find("if (_sameEnvironment(_lastRefusedEnvironment, report)) return null;")
            .expect("hold back refused");
        let sent = host.find("final answer = _dispatchEnvironment(report);").expect("send");
        let remembered = host.find("_lastReportedEnvironment = report;").expect("remember");
        assert!(runtime_half < settling && settling < taken && taken < held_back);
        assert!(held_back < sent && sent < remembered);
        assert!(host.contains("    } on Object {\n      _lastRefusedEnvironment = report;\n      rethrow;\n    }"));
        // An ignored report has nothing to show; a tripped guard does.
        assert!(host.contains("if (answer['error'] != null) return answer;"));
        assert!(host.contains("return _revision(answer) == shownRevision ? null : answer;"));
        // The public half never throws: a failure is an `error` answer.
        assert!(host.contains("'error': 'Mosaic environment report failed: $error',"));
        // Props kept only at the revision already showing, on every dispatch
        // and effect completion, against the runtime's own (undecorated) props.
        assert!(host.contains("if (revision == null || revision != _revision(_runtimeUpdate)) {"));
        assert_eq!(
            host.matches("final settled = _keepShowingProps(_settleEffects(update));\n    _runtimeUpdate = settled;").count(),
            2,
            "dispatch and effect completion"
        );
        assert_eq!(host.matches("_runtimeUpdate = ").count(), 5, "create, dispatch, completion, restore + field");
        // The thresholds every host uses, and the platform's pointer.
        assert!(host.contains(
            "'sizeClass': width < 600\n        ? 'compact'\n        : width < 1024\n        ? 'regular'\n        : 'expanded',"
        ));
        assert!(host.contains("'orientation': height > width ? 'portrait' : 'landscape',"));
        assert!(host.contains("'colorScheme': dark ? 'dark' : 'light',"));
        assert!(host.contains("'reducedMotion': reduceMotion ? 'reduce' : 'no-preference',"));
        assert!(host.contains("final touch = Platform.isAndroid || Platform.isIOS;"));
        assert!(host.contains("'pointer': touch ? 'coarse' : 'fine',"));
        assert!(host.contains("'hover': touch ? 'none' : 'hover',"));
        // The start context carries them too.
        assert!(host.contains("          ...MosaicHost.initialEnvironment(),\n          'restoredSnapshot'"));
        // The binding still runs under the plain Dart VM (the conformance
        // harness has no Flutter engine), so the reducer imports no Flutter.
        assert!(!host.contains("package:flutter"));
        assert!(!host.contains("dart:ui"));
    }

    /// The SwiftUI and Compose libraries answer one contract (UI87 §7.1): the
    /// same kinds, limits and MIME table, so an app sees the same outcome on
    /// either host. A drift in one template fails here, not on a Mac.
    #[test]
    fn swift_platform_effects_match_the_compose_contract() {
        let swift = swift_platform_effects();
        let kotlin = compose_platform_effects();
        for kind in ["\"files.open\"", "\"files.save\""] {
            assert!(swift.contains(kind) && kotlin.contains(kind), "{kind}");
        }
        assert!(swift.contains("let mosaicMaxOpenBytes = 50 * 1024 * 1024"));
        assert!(kotlin.contains("MOSAIC_MAX_OPEN_BYTES: Long = 50L * 1024 * 1024"));
        assert!(swift.contains("let mosaicMaxSaveBytes = 16 * 1024 * 1024"));
        assert!(kotlin.contains("MOSAIC_MAX_SAVE_BYTES: Int = 16 * 1024 * 1024"));
        // Every MIME -> extensions row, in the same order, in both tables.
        let kotlin_rows: Vec<(String, String)> = kotlin
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (mime, rest) = line.strip_prefix('"')?.split_once("\" to listOf(")?;
                Some((mime.to_string(), rest.trim_end_matches("),").to_string()))
            })
            .collect();
        let swift_rows: Vec<(String, String)> = swift
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (mime, rest) = line.strip_prefix("(\"")?.split_once("\", [")?;
                Some((mime.to_string(), rest.trim_end_matches("]),").to_string()))
            })
            .collect();
        assert_eq!(kotlin_rows.len(), 13, "{kotlin_rows:?}");
        assert_eq!(swift_rows, kotlin_rows);
    }

    /// The executable-extension denylist is one set on both hosts, so an
    /// app's `files.save` without a type is refused or allowed the same way.
    #[test]
    fn swift_and_compose_refuse_the_same_executable_extensions() {
        fn listed(source: &str, start: &str, end: &str) -> Vec<String> {
            let from = source.find(start).expect("list start") + start.len();
            let to = from + source[from..].find(end).expect("list end");
            let mut names: Vec<String> = source[from..to]
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .flat_map(|line| line.split(','))
                .map(|item| item.trim().trim_matches('"').to_string())
                .filter(|item| !item.is_empty())
                .collect();
            names.sort();
            names
        }
        let swift = listed(
            &swift_platform_effects(),
            "let mosaicExecutableExtensions: Set<String> = [",
            "]",
        );
        let kotlin = listed(
            &compose_platform_effects(),
            "val MOSAIC_EXECUTABLE_EXTENSIONS: Set<String> = setOf(",
            ")\n",
        );
        assert!(swift.len() >= 30, "{swift:?}");
        assert_eq!(swift, kotlin);
        for must in ["command", "terminal", "webloc", "exe", "desktop"] {
            assert!(swift.iter().any(|name| name == must), "{must}");
        }
    }

    /// The Qt library answers the same contract (UI87 §7.4a): the same
    /// executable list, MIME rows in the same order, and the same limits as
    /// the Compose library. Pinned here because the Qt C++ is only compiled
    /// where Qt is installed.
    #[test]
    fn qt_platform_effects_match_the_compose_contract() {
        let qt = qt_platform_effects().source;
        let header = qt_platform_effects().header;
        let kotlin = compose_platform_effects();
        let qt_list: std::collections::BTreeSet<String> = {
            let from = qt.find("static const QSet<QString> extensions{").expect("qt list");
            let to = from + qt[from..].find("};").expect("end");
            qt[from..to]
                .split("QStringLiteral(\"")
                .skip(1)
                .map(|item| item.split('"').next().unwrap().to_string())
                .collect()
        };
        let kotlin_list: std::collections::BTreeSet<String> = {
            let from = kotlin.find("val MOSAIC_EXECUTABLE_EXTENSIONS").expect("kotlin list");
            let to = from + kotlin[from..].find(")\n").expect("end");
            kotlin[from..to]
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .flat_map(|line| line.split(','))
                .filter_map(|item| item.trim().strip_prefix('"').map(|rest| rest.trim_end_matches('"').to_string()))
                .collect()
        };
        assert!(qt_list.len() >= 60, "{qt_list:?}");
        assert_eq!(qt_list, kotlin_list);
        let qt_mimes: Vec<&str> = qt
            .lines()
            .filter_map(|line| line.trim().strip_prefix("{QStringLiteral(\""))
            .filter(|rest| rest.contains('/'))
            .map(|rest| rest.split('"').next().unwrap())
            .collect();
        let kotlin_mimes: Vec<&str> = kotlin
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter(|rest| rest.contains("\" to listOf("))
            .map(|rest| rest.split('"').next().unwrap())
            .collect();
        assert_eq!(qt_mimes, kotlin_mimes);
        assert!(header.contains("MosaicMaxOpenBytes = 50LL * 1024 * 1024"));
        assert!(header.contains("MosaicMaxSaveBytes = 16LL * 1024 * 1024"));
    }

    /// The XAML library answers the same contract (UI87 §7.6): the same kinds,
    /// limits, MIME rows in the same order, executable list and failure
    /// messages as the Compose library. Pinned here because the C# is only
    /// compiled where .NET is installed, and the WinUI half only on Windows.
    #[test]
    fn xaml_platform_effects_match_the_compose_contract() {
        let xaml = xaml_platform_effects("Mosaic.Generated");
        let kotlin = compose_platform_effects();
        assert!(xaml.contains(
            "new HashSet<string>(StringComparer.Ordinal) { \"files.open\", \"files.save\" };"
        ));
        assert!(kotlin.contains("setOf(\"files.open\", \"files.save\")"));
        assert!(xaml.contains("public const long MaxOpenBytes = 50L * 1024 * 1024;"));
        assert!(kotlin.contains("MOSAIC_MAX_OPEN_BYTES: Long = 50L * 1024 * 1024"));
        assert!(xaml.contains("public const int MaxSaveBytes = 16 * 1024 * 1024;"));
        assert!(kotlin.contains("MOSAIC_MAX_SAVE_BYTES: Int = 16 * 1024 * 1024"));
        // Every MIME -> extensions row, in the same order, in both tables.
        let kotlin_rows: Vec<(String, String)> = kotlin
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (mime, rest) = line.strip_prefix('"')?.split_once("\" to listOf(")?;
                Some((mime.to_string(), rest.trim_end_matches("),").to_string()))
            })
            .collect();
        let xaml_rows: Vec<(String, String)> = xaml
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (mime, rest) = line.strip_prefix("(\"")?.split_once("\", new[] { ")?;
                Some((mime.to_string(), rest.trim_end_matches(" }),").to_string()))
            })
            .collect();
        assert_eq!(kotlin_rows.len(), 13, "{kotlin_rows:?}");
        assert_eq!(xaml_rows, kotlin_rows);
        // The executable-extension denylist is one set.
        fn listed(source: &str, start: &str, end: &str) -> std::collections::BTreeSet<String> {
            let from = source.find(start).expect("list start") + start.len();
            let to = from + source[from..].find(end).expect("list end");
            source[from..to]
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .flat_map(|line| line.split(','))
                .map(|item| item.trim().trim_matches('"').to_string())
                .filter(|item| !item.is_empty())
                .collect()
        }
        let xaml_list = listed(
            &xaml,
            "ExecutableExtensions = new HashSet<string>(StringComparer.Ordinal)\n    {",
            "};",
        );
        let kotlin_list = listed(
            &kotlin,
            "val MOSAIC_EXECUTABLE_EXTENSIONS: Set<String> = setOf(",
            ")\n",
        );
        assert!(xaml_list.len() >= 60, "{xaml_list:?}");
        assert_eq!(xaml_list, kotlin_list);
        // The failure messages an app can see are the ones Compose sends.
        for message in [
            "that is not a regular file",
            "couldn't read the selected file",
            "suggestedName must be a plain file name",
            "bytes must be base64 text",
            "suggestedName must end in an extension of an accepted type",
            "suggestedName must not end in an executable extension",
            "couldn't save the file",
            "another file operation is in progress",
            "the file dialog failed",
        ] {
            let quoted = format!("\"{message}\"");
            assert!(xaml.contains(&quoted), "XAML: {message}");
            assert!(kotlin.contains(&quoted), "Compose: {message}");
        }
        assert!(xaml.contains("$\"the selected file is larger than {MaxOpenBytes} bytes\""));
        assert!(xaml.contains("$\"the file is larger than {MaxSaveBytes} bytes\""));
        // Routing: the same three outcomes, null meaning nobody.
        assert!(xaml.contains("if (appKinds is not null && appKinds.Contains(kind)) return false;"));
        assert!(xaml.contains("if (StandardEffectKinds.Contains(kind)) return true;"));
        assert!(xaml.contains("if (appKinds is null) return false;\n        return null;"));
        // The namespace placeholder is bound, as MosaicRuntimeHost.cs's is.
        assert!(xaml.contains("namespace Mosaic.Generated;"));
        assert!(!xaml.contains("__MOSAIC_NAMESPACE__"));
    }

    /// WinUI exists only in the generated project; the headless harness
    /// compiles this file with `MOSAIC_HEADLESS_TEST`. So every WinUI use must
    /// sit inside `#if !MOSAIC_HEADLESS_TEST`, and the fence must be the only
    /// conditional -- a second symbol could switch the real pickers off in a
    /// generated project.
    #[test]
    fn xaml_platform_effects_fence_winui_from_the_headless_test() {
        let xaml = xaml_platform_effects("Mosaic.Generated");
        let mut inside = false;
        let mut fences = 0;
        for line in xaml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#if") {
                assert_eq!(trimmed, "#if !MOSAIC_HEADLESS_TEST", "{line}");
                assert!(!inside, "nested fence");
                inside = true;
                fences += 1;
            } else if trimmed.starts_with("#else") || trimmed.starts_with("#elif") {
                panic!("the fence has no alternative branch: {line}");
            } else if trimmed == "#endif" {
                inside = false;
            } else if !trimmed.starts_with("//") && !trimmed.starts_with("///") {
                for winui in [
                    "Microsoft.UI",
                    "Windows.Storage",
                    "WinRT.Interop",
                    "DispatcherQueue",
                    "FileOpenPicker",
                    "FileSavePicker",
                ] {
                    assert!(
                        !trimmed.contains(winui) || inside,
                        "`{winui}` outside the fence: {line}"
                    );
                }
            }
        }
        assert!(!inside, "unclosed fence");
        assert_eq!(fences, 2, "the WinUI Install overload and the picker class");
        // The window's handle owns the pickers (unpackaged WinUI 3), and its
        // queue runs them after the settle.
        assert!(xaml.contains("WinRT.Interop.WindowNative.GetWindowHandle(window)"));
        assert_eq!(
            xaml.matches("WinRT.Interop.InitializeWithWindow.Initialize(picker, window);")
                .count(),
            2
        );
        assert!(xaml.contains("work => queue.TryEnqueue(() => work())"));
        // Deferred before any picker; a refused queue is answered, not lost.
        let deferred = xaml.find("owned = host.DeferEffect(id);").expect("defer");
        let queued = xaml.find("queued = runOnUi(").expect("queue");
        let refused = xaml
            .find("TryComplete(id, MosaicPlatformEffects.Failed(\"the file dialog failed\"));")
            .expect("refused");
        assert!(deferred < queued && queued < refused);
        // Copied before the router is marked busy, so a throw cannot strand it.
        let clone = xaml.find("payload.Clone();").expect("clone");
        let busy = xaml
            .find("Interlocked.CompareExchange(ref busy, 1, 0)")
            .expect("busy");
        assert!(clone < busy);
        // File I/O runs off the UI thread.
        assert!(xaml.contains("return await Task.Run(() => ReadOpened(chosen));"));
        assert!(xaml.contains("return await Task.Run(() => WriteReplacing(target, bytes));"));
        // Saving over a file on Windows keeps its ACL and attributes.
        assert!(xaml.contains(
            "File.Replace(temporary, full, destinationBackupFileName: null, ignoreMetadataErrors: true);"
        ));
    }

    /// A picker left open across a retried start (`Close`, then
    /// `LoadRequired`) must not answer the new runtime, whose effect ids
    /// restart (UI87 §7.6). So the library never answers through the static
    /// host, which forwards to whichever runtime is loaded; it answers through
    /// an `EffectScope` bound to the runtime loaded at install.
    #[test]
    fn xaml_platform_effects_answer_the_runtime_that_asked() {
        let xaml = xaml_platform_effects("Mosaic.Generated");
        let code: String = xaml
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for static_call in [
            "MosaicRuntimeHost.CompleteEffect(",
            "MosaicRuntimeHost.DeferEffect(",
            "MosaicRuntimeHost.EffectHandler",
        ] {
            assert!(!code.contains(static_call), "{static_call}");
        }
        assert!(xaml.contains("MosaicRuntimeHost.EffectScope.Current() is { } scope"));
        assert!(xaml.contains("public void CompleteEffect(ulong id, object result) => scope.CompleteEffect(id, result);"));
        assert!(xaml.contains("if (MosaicRuntimeHostEffects.Current() is not { } host) return;"));

        let host = xaml_runtime_binding("Mosaic.Generated");
        assert!(host.contains("public sealed class EffectScope"));
        assert!(host.contains("public bool IsCurrent => ReferenceEquals(State, runtime);"));
        assert!(host.contains(
            "public bool DeferEffect(ulong id) => IsCurrent && runtime.DeferEffect(id);"
        ));
        assert!(host.contains(
            "public void CompleteEffect(ulong id, object result) => runtime.CompleteEffect(id, result);"
        ));
    }

    /// AppKit exists only on macOS; the iOS app target compiles this file too
    /// (UI89 §2.2 lists every `Sources/App/*.swift`), so every AppKit use must
    /// sit behind `#if os(macOS)`.
    #[test]
    fn swift_platform_effects_fence_appkit_to_macos() {
        let swift = swift_platform_effects();
        let mut depth_macos = false;
        for line in swift.lines() {
            let trimmed = line.trim();
            if trimmed == "#if os(macOS)" {
                depth_macos = true;
            } else if trimmed == "#else" || trimmed == "#endif" {
                depth_macos = false;
            } else if !trimmed.starts_with("//") && !trimmed.starts_with("///") {
                for appkit in ["NSOpenPanel", "NSSavePanel", "import AppKit", "UTType("] {
                    assert!(
                        !trimmed.contains(appkit) || depth_macos,
                        "`{appkit}` outside #if os(macOS): {line}"
                    );
                }
            }
        }
    }

    #[test]
    fn compose_binding_owns_the_full_c_abi_lifecycle() {
        let source = compose_jna_binding();
        assert!(source.contains("class MosaicSizeT(value: Long = 0)"));
        assert!(source.contains("open class MosaicBytes : Structure()"));
        assert!(source.contains("open class MosaicBuffer : Structure()"));
        assert!(source.contains("interface MosaicNativeApi : Library"));
        assert!(!source.contains("private class MosaicSizeT"));
        assert!(!source.contains("private open class MosaicBytes"));
        assert!(!source.contains("private open class MosaicBuffer"));
        assert!(!source.contains("private interface MosaicNativeApi"));
        for symbol in [
            "mosaic_app_create",
            "mosaic_app_dispatch",
            "mosaic_app_snapshot",
            "mosaic_app_restore",
            "mosaic_buffer_free",
            "mosaic_app_destroy",
        ] {
            assert!(source.contains(symbol), "missing {symbol}");
        }
        assert!(source.contains("finally {\n            api.mosaic_buffer_free"));
        assert!(source.contains("override fun close()"));
        assert!(source.contains("fun snapshot(): Map<String, Any?>?"));
        assert!(source.contains("fun restore(snapshot: Map<String, Any?>)"));
    }

    #[test]
    fn compose_binding_uses_the_shared_protocol_and_sequences_successes() {
        let source = compose_jna_binding();
        // Protocol 2 because this host implements effect completion. Still from
        // the shared constant rather than a literal, and the capability is
        // asserted beside the claim -- declaring 2 without it makes the app
        // emit into a void.
        assert!(source.contains(&format!(
            "private const val MOSAIC_PROTOCOL_VERSION = {}",
            mosaic_app_runtime::EFFECT_PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
        assert!(
            source.contains("fun mosaic_app_complete_effect"),
            "a protocol 2 host must bind the completion symbol"
        );
        assert!(
            source.contains("fun completeEffect") && source.contains("fun deferEffect"),
            "a protocol 2 host must expose ways to answer and to defer"
        );
        assert!(source.contains("val nextSequence = Math.addExact(sequence, 1L)"));
        let dispatch = source.find("api.mosaic_app_dispatch").unwrap();
        let commit = source.find("sequence = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
    }

    #[test]
    fn compose_binding_accepts_generated_and_explicit_event_envelopes() {
        let source = compose_jna_binding();
        assert!(source.contains("(event[\"name\"] ?: event[\"event\"]) as? String"));
        assert!(source.contains("require(!name.isNullOrEmpty())"));
        assert!(source.contains("if (explicitPayload is Map<*, *>)"));
        assert!(source.contains(
            "event.filterKeys { key -> key !in setOf(\"name\", \"event\", \"payload\") }"
        ));
        assert!(source.contains("put(\"payload\", payload.toJsonElement())"));
    }

    #[test]
    fn compose_binding_preserves_json_primitive_types() {
        let source = compose_jna_binding();
        assert!(source.contains("isString -> content"));
        assert!(source.contains("else -> booleanOrNull ?: longOrNull ?: doubleOrNull ?: content"));
    }

    #[test]
    fn compose_binding_has_cross_platform_library_and_startup_context() {
        let source = compose_jna_binding();
        assert!(source.contains("System.getProperty(\"mosaic.app.library\")"));
        assert!(source.contains("System.getenv(\"MOSAIC_APP_LIBRARY\")"));
        assert!(source.contains("System.getProperty(\"compose.application.resources.dir\")"));
        assert!(source.contains("?: bundledMosaicLibrary()"));
        assert!(source.contains("\"libmosaic_app.dylib\""));
        assert!(source.contains("\"mosaic_app.dll\""));
        assert!(source.contains("\"libmosaic_app.so\""));
        assert!(source.contains("Locale.getDefault().toLanguageTag()"));
        assert!(source.contains("put(\"textScale\", 1.0)"));
        assert!(source.contains("-> \"apple\""));
        assert!(source.contains("-> \"windows\""));
        assert!(source.contains("-> \"linux\""));
    }

    #[test]
    fn swift_binding_owns_the_full_c_abi_lifecycle() {
        let binding = swift_runtime_binding();
        for symbol in [
            "mosaic_app_create",
            "mosaic_app_dispatch",
            "mosaic_app_snapshot",
            "mosaic_app_restore",
            "mosaic_buffer_free",
            "mosaic_app_destroy",
        ] {
            assert!(binding.loader_c.contains(symbol), "missing {symbol}");
        }
        assert!(binding.host_swift.contains("defer { free(&buffer) }"));
        assert!(binding.host_swift.contains("deinit { close() }"));
        assert!(binding.host_swift.contains("func snapshot()"));
        assert!(binding.host_swift.contains("func restore("));
    }

    #[test]
    fn qt_binding_prefers_the_application_relative_runtime() {
        let source = qt_runtime_binding().source;
        assert!(source.contains("QCoreApplication::applicationDirPath()"));
        assert!(source.contains("QDir(QCoreApplication::applicationDirPath()).filePath(fileName)"));
        let bundled = source.find("return {bundled, fileName").unwrap();
        let global = source.find("QStringLiteral(\"mosaic_app\")").unwrap();
        assert!(
            bundled < global,
            "the app-relative path must precede global lookup"
        );
    }

    #[test]
    fn swift_binding_uses_the_shared_protocol_and_commits_successful_sequences() {
        let binding = swift_runtime_binding();
        let source = binding.host_swift.clone();
        // Protocol 2 because this host implements effect completion. Still from
        // the shared constant rather than a literal -- the number moved, the
        // rule did not -- and the capability is asserted beside the claim,
        // since declaring 2 without it makes the app emit into a void.
        assert!(source.contains(&format!(
            "private let mosaicProtocolVersion = {}",
            mosaic_app_runtime::EFFECT_PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
        assert!(
            binding.loader_c.contains("mosaic_app_complete_effect"),
            "a protocol 2 host must resolve the completion symbol"
        );
        assert!(
            source.contains("public func completeEffect"),
            "a protocol 2 host must expose a way to answer an effect"
        );
        assert!(
            source.contains("let (nextSequence, overflow) = sequence.addingReportingOverflow(1)")
        );
        let dispatch = source.find("mosaic_binding_dispatch").unwrap();
        let commit = source.find("sequence = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
        assert!(source
            .contains("static func loadRequired(libraryPath: String? = nil) -> MosaicRuntimeHost"));
        assert!(source.contains("static func loadRecoverable("));
        assert!(source.contains("Result<MosaicRuntimeHost, Error>"));
        assert!(source.contains("native-complete requires the Mosaic Rust application runtime"));
    }

    #[test]
    fn swift_shell_patches_install_the_runtime_before_legacy_fallback() {
        let app = swift_app_with_runtime_binding(
            "init() {\n    self.bridge = MosaicHostBridge.load()\n  }",
            false,
        );
        assert!(app.contains("MosaicRuntimeHost.load() ?? MosaicHostBridge.load()"));

        let package = swift_package_with_runtime_binding(
            "  targets: [\n    .executableTarget(\n      name: \"App\",\n      path: \"Sources/App\"\n    ),\n  ]",
            false,
        );
        assert!(package.contains("name: \"CMosaicRuntime\""));
        assert!(package.contains("dependencies: [\"CMosaicRuntime\"]"));
    }

    #[test]
    fn swift_shell_patches_resolve_a_bundled_runtime_resource() {
        let strict_app = swift_app_with_runtime_binding(
            "init() {\n    self.loader = { MosaicRuntimeHost.loadRecoverable() }\n  }",
            true,
        );
        assert!(strict_app.contains(
            "MosaicRuntimeHost.loadRecoverable(libraryPath: Bundle.module.url(forResource: \"libmosaic_app\", withExtension: \"dylib\", subdirectory: \"Runtime\")?.path)"
        ));

        let package = swift_package_with_runtime_binding(
            "  targets: [\n    .executableTarget(\n      name: \"App\",\n      path: \"Sources/App\"\n    ),\n  ]",
            true,
        );
        assert!(package.contains("resources: [.copy(\"Runtime\")]"));
    }

    #[test]
    fn xaml_binding_owns_the_full_c_abi_lifecycle() {
        let source = xaml_runtime_binding("Mosaic.Generated");
        for symbol in [
            "mosaic_app_create",
            "mosaic_app_dispatch",
            "mosaic_app_snapshot",
            "mosaic_app_restore",
            "mosaic_buffer_free",
            "mosaic_app_destroy",
        ] {
            assert!(source.contains(symbol), "missing {symbol}");
        }
        assert!(source.contains("NativeLibrary.GetExport"));
        assert!(source.contains("finally { bufferFree(buffer); }"));
        assert!(source.contains("public void Dispose()"));
    }

    #[test]
    fn xaml_binding_prefers_the_application_relative_runtime() {
        let source = xaml_runtime_binding("Mosaic.Generated");
        assert!(source.contains("Path.Combine(AppContext.BaseDirectory, \"mosaic_app.dll\")"));
        let bundled = source
            .find("Path.Combine(AppContext.BaseDirectory")
            .unwrap();
        let global = source.find("\"mosaic_app\",").unwrap();
        assert!(
            bundled < global,
            "the app-relative path must precede global lookup"
        );
    }

    #[test]
    fn xaml_binding_uses_shared_protocol_and_successful_sequences() {
        let source = xaml_runtime_binding("Acme.App");
        assert!(source.contains("namespace Acme.App;"));
        // Protocol 2 because this host implements effect completion. Still
        // from the shared constant rather than a literal, and the capability is
        // asserted beside the claim -- declaring 2 without it makes the app
        // emit into a void.
        assert!(source.contains(&format!(
            "private const int ProtocolVersion = {};",
            mosaic_app_runtime::EFFECT_PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
        assert!(
            source.contains("\"mosaic_app_complete_effect\""),
            "a protocol 2 host must bind the completion symbol"
        );
        assert!(
            source.contains("public static void CompleteEffect(")
                && source.contains("public static bool DeferEffect("),
            "a protocol 2 host must expose ways to answer and to defer"
        );
        assert!(
            source.contains("NativeLibrary.TryGetExport"),
            "the completion symbol must resolve leniently, or a protocol 1 \
             runtime stops loading at all"
        );
        let dispatch = source.find("dispatch(app, input, out output)").unwrap();
        let commit = source.find("sequence = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
        assert!(source.contains("public static void LoadRequired()"));
        assert!(source.contains("State ??= Load();"));
        assert!(source.contains("Loader detail:"));
        assert!(source.contains("State = null;"));
        assert!(source.contains("ProcessExit += (_, _) => State?.Dispose();"));
        assert!(source.contains("public static string ApplyRequiredProps("));
        assert!(source.contains("public static Task<MosaicRuntimeResult> HandleRequiredEvent("));
        assert!(source.contains("native-complete requires the Mosaic Rust application runtime"));
        assert!(source.contains("Mosaic runtime props are missing required value"));
        assert!(source.contains("Mosaic runtime response did not include a props object"));
    }

    #[test]
    fn flutter_binding_owns_the_full_c_abi_lifecycle() {
        let source = flutter_runtime_binding();
        for symbol in [
            "mosaic_app_create",
            "mosaic_app_dispatch",
            "mosaic_app_snapshot",
            "mosaic_app_restore",
            "mosaic_buffer_free",
            "mosaic_app_destroy",
        ] {
            assert!(source.contains(symbol), "missing {symbol}");
        }
        assert!(source.contains("DynamicLibrary.open"));
        assert!(source.contains("finally {\n      _bufferFree(buffer);"));
        assert!(source.contains("void dispose()"));
        assert!(source.contains("const MosaicHost()"));
        assert!(source.contains("static MosaicHost loadRequired()"));
        assert!(source.contains("native-complete requires the Mosaic Rust application runtime"));
        assert!(source.contains("static const bool _hasBundledRuntime = false;"));
        assert!(!source.contains("__MOSAIC_BUNDLED_RUNTIME__"));
    }

    #[test]
    fn flutter_binding_can_resolve_a_bundled_code_asset() {
        let source = flutter_runtime_binding_with_bundled_asset();
        assert!(source.contains("@Native<_CreateNative>(symbol: 'mosaic_app_create')"));
        assert!(source.contains("static const bool _hasBundledRuntime = true;"));
        assert!(source.contains("if (_hasBundledRuntime) return _MosaicRuntime.bundled();"));
        let environment = source.find("MOSAIC_APP_LIBRARY").unwrap();
        let bundled = source
            .find("if (_hasBundledRuntime) return _MosaicRuntime.bundled();")
            .unwrap();
        assert!(
            environment < bundled,
            "the explicit development override wins"
        );
    }

    #[test]
    fn flutter_binding_uses_shared_protocol_and_successful_sequences() {
        let source = flutter_runtime_binding();
        // Protocol 2 because this host implements effect completion. Still
        // from the shared constant rather than a literal, and the capability
        // is asserted beside the claim -- declaring 2 without it makes the app
        // emit into a void.
        assert!(source.contains(&format!(
            "static const int _protocolVersion = {};",
            mosaic_app_runtime::EFFECT_PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
        assert!(
            source.contains("symbol: 'mosaic_app_complete_effect'"),
            "a protocol 2 host must bind the completion symbol"
        );
        assert!(
            source.contains("Map<String, Object?> completeEffect(")
                && source.contains("bool deferEffect("),
            "a protocol 2 host must expose ways to answer and to defer"
        );
        let dispatch = source.find("_dispatch(_app, input, output)").unwrap();
        let commit = source.find("_sequence = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
    }

    #[test]
    fn flutter_pubspec_installs_the_ffi_allocator() {
        let pubspec =
            flutter_pubspec_with_runtime_binding("dependencies:\n  flutter:\n    sdk: flutter\n");
        assert!(pubspec.contains("ffi: '>=2.1.0 <3.0.0'"));
        assert!(pubspec.contains("flutter:\n    sdk: flutter"));
    }

    #[test]
    fn flutter_pubspec_installs_stable_code_asset_support() {
        let pubspec = flutter_pubspec_with_bundled_runtime(
            "environment:\n  sdk: '>=3.5.0 <4.0.0'\n  flutter: '>=3.32.0 <4.0.0'\ndependencies:\n  flutter:\n    sdk: flutter\n",
        );
        assert!(pubspec.contains("sdk: '>=3.10.0 <4.0.0'"));
        assert!(pubspec.contains("flutter: '>=3.38.0 <4.0.0'"));
        assert!(pubspec.contains("code_assets: '>=1.0.0 <2.0.0'"));
        assert!(pubspec.contains("hooks: '>=1.0.0 <3.0.0'"));
        assert!(pubspec.contains("ffi: '>=2.1.0 <3.0.0'"));
    }

    #[test]
    fn qt_binding_owns_the_full_c_abi_lifecycle() {
        let binding = qt_runtime_binding();
        for symbol in [
            "mosaic_app_create",
            "mosaic_app_dispatch",
            "mosaic_app_snapshot",
            "mosaic_app_restore",
            "mosaic_buffer_free",
            "mosaic_app_destroy",
        ] {
            assert!(binding.source.contains(symbol), "missing {symbol}");
        }
        assert!(binding
            .header
            .contains("Q_INVOKABLE QVariantMap handleEvent"));
        assert!(binding.header.contains("~MosaicHost() override"));
        assert!(binding.source.contains("bufferFree_(buffer)"));
        assert!(binding.source.contains("destroy_(app_)"));
    }

    #[test]
    fn qt_binding_uses_shared_protocol_and_successful_sequences() {
        let binding = qt_runtime_binding();
        // The Qt host declares protocol 2 because it IMPLEMENTS effect
        // completion. Still taken from the shared constant rather than written
        // as a literal, which is what this assertion has always been about --
        // the number moved, the rule did not.
        assert!(binding.header.contains(&format!(
            "static constexpr quint32 ProtocolVersion = {};",
            mosaic_app_runtime::EFFECT_PROTOCOL_VERSION
        )));
        assert!(!binding.header.contains("__MOSAIC_PROTOCOL_VERSION__"));
        // Declaring 2 without being able to answer an effect is the harmful
        // direction: the app emits into a void, which is the exact failure the
        // protocol field exists to prevent. So the claim and the capability are
        // asserted together.
        assert!(
            binding.source.contains("mosaic_app_complete_effect"),
            "a protocol 2 host must resolve the completion symbol"
        );
        assert!(
            binding
                .header
                .contains("Q_INVOKABLE QVariantMap completeEffect"),
            "a protocol 2 host must expose a way to answer an effect"
        );
        let dispatch = binding
            .source
            .find("dispatch_(app_, input, &output)")
            .unwrap();
        let commit = binding.source.find("sequence_ = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
        assert!(binding.header.contains("void requireRuntime() const"));
        assert!(binding.header.contains("QVariantMap propsRequired() const"));
        assert!(binding
            .header
            .contains("Q_INVOKABLE QVariantMap handleRequiredEvent"));
        assert!(binding
            .source
            .contains("native-complete requires the Mosaic Rust application runtime"));
        assert!(binding.source.contains("missing required MIL prop"));
    }

    #[test]
    fn emitted_native_bindings_restore_and_atomically_persist_application_state() {
        let application_id = "com.example.tasks";
        let qt = qt_runtime_binding_for_application(application_id);
        let bindings = [
            (
                compose_jna_binding_for_application(application_id),
                "val restoredSnapshot = loadPersistedSnapshot()",
                "api.mosaic_app_create(input, app, output)",
                "api.mosaic_app_dispatch(app, input, output)",
                "persistSnapshot()",
            ),
            (
                swift_runtime_binding_for_application(application_id).host_swift,
                "let persisted = loadPersistedSnapshot()",
                "mosaic_binding_create(runtime, bytes, &app, output)",
                "mosaic_binding_dispatch(runtime, app, bytes, output)",
                "persistSnapshot()",
            ),
            (
                xaml_runtime_binding_for_application("Example.Tasks", application_id),
                "var persisted = LoadPersistedSnapshot()",
                "return create(input, out app, out output)",
                "return dispatch(app, input, out output)",
                "PersistSnapshot()",
            ),
            (
                flutter_runtime_binding_for_application(application_id, true),
                "final persisted = _loadPersistedSnapshot()",
                "_create(input, appOut, output)",
                "_dispatch(_app, input, output)",
                "_persistSnapshot()",
            ),
            (
                format!("{}\n{}", qt.header, qt.source),
                "const auto restoredSnapshot = loadPersistedSnapshot()",
                "create_(input, &app_, &output)",
                "dispatch_(app_, input, &output)",
                "persistSnapshot()",
            ),
        ];
        for (source, restore_marker, create_marker, dispatch_marker, persist_marker) in &bindings {
            assert!(source.contains(application_id));
            assert!(!source.contains("__MOSAIC_PERSISTENCE_ENABLED__"));
            assert!(!source.contains("__MOSAIC_APPLICATION_ID__"));
            assert!(source.contains("MOSAIC_APP_STATE_PATH"));
            assert!(source.contains("mosaic-state.v1.json"));
            assert!(source.contains("corrupt"));
            assert!(source.contains("rejected persisted state"));
            assert!(source.contains("restoredSnapshot"));
            assert!(source.contains("persistenceWarning") || source.contains("PersistenceWarning"));
            assert!(source.contains("storage-warning"));

            let restored = source.find(restore_marker).unwrap();
            let created = source.find(create_marker).unwrap();
            assert!(
                restored < created,
                "state must be restored before app creation"
            );

            let dispatched = source.find(dispatch_marker).unwrap();
            let persisted = source.rfind(persist_marker).unwrap();
            assert!(dispatched < persisted, "state must persist after dispatch");
        }

        assert!(bindings[0].0.contains("StandardCopyOption.ATOMIC_MOVE"));
        assert!(bindings[1].0.contains("options: [.atomic]"));
        assert!(bindings[2].0.contains("File.Move(temporary, path, true)"));
        assert!(bindings[3].0.contains("MoveFileExW"));
        assert!(bindings[3].0.contains("temporary.renameSync(target.path)"));
        assert!(bindings[4].0.contains("QSaveFile file(path)"));

        // Every native host tells the app its UTC offset (UI38 "Local time"),
        // read from the platform's own time zone API, in minutes EAST of UTC,
        // and leaves it out when it is out of range (the runtime would refuse
        // it and the app would not start).
        let offsets = [
            "if (utcOffsetMinutes in -840..840) put(\"utcOffsetMinutes\", utcOffsetMinutes)",
            "if (-840...840).contains(utcOffsetMinutes) { start[\"utcOffsetMinutes\"] = utcOffsetMinutes }",
            "if (utcOffsetMinutes >= -840 && utcOffsetMinutes <= 840) start[\"utcOffsetMinutes\"] = utcOffsetMinutes;",
            "..._utcOffsetEntry(),",
            "context.insert(QStringLiteral(\"utcOffsetMinutes\"), utcOffsetMinutes);",
        ];
        for ((source, ..), offset) in bindings.iter().zip(offsets) {
            assert!(source.contains(offset), "missing {offset}");
        }
        assert!(bindings[4].0.contains("#include <QDateTime>"));
    }

    /// UI89 §2.1: a static runtime is a linked binary target the loader
    /// depends on, with the macro that selects the direct-call path.
    #[test]
    fn a_static_runtime_links_the_xcframework_into_the_loader() {
        let generated = "// swift-tools-version:5.9\nlet package = Package(\n  name: \"App\",\n  targets: [\n    .executableTarget(\n      name: \"App\",\n      path: \"Sources/App\"\n    )\n  ]\n)\n";
        let bound = swift_package_with_runtime_binding(generated, false);
        let linked = swift_package_with_static_runtime(&bound);
        assert!(linked.contains(
            ".binaryTarget(\n      name: \"MosaicAppRuntime\",\n      path: \"Runtime/MosaicAppRuntime.xcframework\"\n    )"
        ), "{linked}");
        assert!(linked.contains("dependencies: [\"MosaicAppRuntime\"]"), "{linked}");
        assert!(linked.contains("cSettings: [.define(\"MOSAIC_RUNTIME_STATIC\")]"), "{linked}");
        assert!(!linked.contains("resources:"), "a linked runtime is not a resource: {linked}");
        let loader = swift_runtime_binding().loader_c;
        assert!(loader.contains("#if defined(MOSAIC_RUNTIME_STATIC)"));
        assert!(loader.contains("runtime->create = mosaic_app_create;"));
    }

}
