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
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
    )
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
        .replace(
            "__MOSAIC_PROTOCOL_VERSION__",
            &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
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
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
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
    .replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
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
        header: bind_application(include_str!("../templates/qt/MosaicHost.h"), application_id)
            .replace(
                "__MOSAIC_PROTOCOL_VERSION__",
                &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
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
        assert!(source.contains(&format!(
            "private const val MOSAIC_PROTOCOL_VERSION = {}",
            mosaic_app_runtime::PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
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
        let source = swift_runtime_binding().host_swift;
        assert!(source.contains(&format!(
            "private let mosaicProtocolVersion = {}",
            mosaic_app_runtime::PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
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
            "init() {\n    self.bridge = MosaicRuntimeHost.loadRequired()\n  }",
            true,
        );
        assert!(strict_app.contains(
            "MosaicRuntimeHost.loadRequired(libraryPath: Bundle.module.url(forResource: \"libmosaic_app\", withExtension: \"dylib\", subdirectory: \"Runtime\")?.path)"
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
        assert!(source.contains(&format!(
            "private const int ProtocolVersion = {};",
            mosaic_app_runtime::PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
        let dispatch = source.find("dispatch(app, input, out output)").unwrap();
        let commit = source.find("sequence = nextSequence").unwrap();
        assert!(
            dispatch < commit,
            "sequence must commit only after dispatch succeeds"
        );
        assert!(source.contains("public static void LoadRequired()"));
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
        assert!(source.contains(&format!(
            "static const int _protocolVersion = {};",
            mosaic_app_runtime::PROTOCOL_VERSION
        )));
        assert!(!source.contains("__MOSAIC_PROTOCOL_VERSION__"));
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
        assert!(binding.header.contains(&format!(
            "static constexpr quint32 ProtocolVersion = {};",
            mosaic_app_runtime::PROTOCOL_VERSION
        )));
        assert!(!binding.header.contains("__MOSAIC_PROTOCOL_VERSION__"));
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
    }
}
