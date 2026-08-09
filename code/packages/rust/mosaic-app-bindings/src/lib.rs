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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_binding_owns_the_full_c_abi_lifecycle() {
        let source = compose_jna_binding();
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
}
