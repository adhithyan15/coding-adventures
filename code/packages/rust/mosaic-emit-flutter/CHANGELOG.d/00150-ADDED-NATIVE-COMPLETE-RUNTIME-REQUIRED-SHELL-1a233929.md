### Added - native-complete runtime-required shell

`EmitOptions::require_runtime` now selects a fail-loud Flutter application
shell that requires Mosaic's standard Rust host, waits for its first props
envelope, and omits nullable-host, event-print, and generated sample-value
fallbacks. The default remains byte-compatible permissive project emission.

