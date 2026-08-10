//! Package-independent native bindings for the Mosaic application C ABI.
//!
//! Each function returns deterministic source installed by the artifact builder.
//! Keeping bindings here prevents backend shells and applications from growing
//! separate FFI implementations of the same runtime protocol.

/// Generate the standard Compose/JVM JNA host binding.
pub fn compose_jna_binding() -> String {
    include_str!("../templates/compose/MosaicRuntimeHost.kt").replace(
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
    SwiftRuntimeBinding {
        host_swift: include_str!("../templates/swiftui/MosaicRuntimeHost.swift").replace(
            "__MOSAIC_PROTOCOL_VERSION__",
            &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
        ),
        header: include_str!("../templates/swiftui/CMosaicRuntime.h").to_string(),
        loader_c: include_str!("../templates/swiftui/CMosaicRuntime.c").to_string(),
    }
}

/// Connect an emitted SwiftUI shell to the standard runtime before its legacy
/// reflection-based host fallback.
pub fn swift_app_with_runtime_binding(app_swift: &str) -> String {
    app_swift.replacen(
        "self.bridge = MosaicHostBridge.load()",
        "self.bridge = MosaicRuntimeHost.load() ?? MosaicHostBridge.load()",
        1,
    )
}

/// Add the generated C loader target to an emitted Swift package manifest.
pub fn swift_package_with_runtime_binding(package_swift: &str) -> String {
    let with_target = package_swift.replacen(
        "  targets: [\n    .executableTarget(",
        "  targets: [\n    .target(\n      name: \"CMosaicRuntime\",\n      path: \"Sources/CMosaicRuntime\",\n      publicHeadersPath: \"include\"\n    ),\n    .executableTarget(",
        1,
    );
    with_target.replacen(
        "      name: \"App\",\n      path: \"Sources/App\"",
        "      name: \"App\",\n      dependencies: [\"CMosaicRuntime\"],\n      path: \"Sources/App\"",
        1,
    )
}

/// Generate the standard XAML/.NET host binding for the requested C# namespace.
pub fn xaml_runtime_binding(namespace: &str) -> String {
    include_str!("../templates/xaml/MosaicRuntimeHost.cs")
        .replace(
            "__MOSAIC_PROTOCOL_VERSION__",
            &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
        )
        .replace("__MOSAIC_NAMESPACE__", namespace)
}

/// Generate the standard Flutter/Dart FFI host binding.
pub fn flutter_runtime_binding() -> String {
    include_str!("../templates/flutter/mosaic_host.dart").replace(
        "__MOSAIC_PROTOCOL_VERSION__",
        &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
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

/// Files that expose the fixed Mosaic application C ABI as a QML host object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QtRuntimeBinding {
    pub header: String,
    pub source: String,
}

/// Generate the standard Qt/QML host using Qt Core's dynamic loading and JSON
/// APIs, with no application-specific C++ adapter.
pub fn qt_runtime_binding() -> QtRuntimeBinding {
    QtRuntimeBinding {
        header: include_str!("../templates/qt/MosaicHost.h").replace(
            "__MOSAIC_PROTOCOL_VERSION__",
            &mosaic_app_runtime::PROTOCOL_VERSION.to_string(),
        ),
        source: include_str!("../templates/qt/MosaicHost.cpp").to_string(),
    }
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
    fn compose_binding_has_cross_platform_library_and_startup_context() {
        let source = compose_jna_binding();
        assert!(source.contains("System.getProperty(\"mosaic.app.library\")"));
        assert!(source.contains("System.getenv(\"MOSAIC_APP_LIBRARY\")"));
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
        assert!(source.contains("static func loadRequired() -> MosaicRuntimeHost"));
        assert!(source.contains("native-complete requires the Mosaic Rust application runtime"));
    }

    #[test]
    fn swift_shell_patches_install_the_runtime_before_legacy_fallback() {
        let app = swift_app_with_runtime_binding(
            "init() {\n    self.bridge = MosaicHostBridge.load()\n  }",
        );
        assert!(app.contains("MosaicRuntimeHost.load() ?? MosaicHostBridge.load()"));

        let package = swift_package_with_runtime_binding(
            "  targets: [\n    .executableTarget(\n      name: \"App\",\n      path: \"Sources/App\"\n    ),\n  ]",
        );
        assert!(package.contains("name: \"CMosaicRuntime\""));
        assert!(package.contains("dependencies: [\"CMosaicRuntime\"]"));
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
    }
}
