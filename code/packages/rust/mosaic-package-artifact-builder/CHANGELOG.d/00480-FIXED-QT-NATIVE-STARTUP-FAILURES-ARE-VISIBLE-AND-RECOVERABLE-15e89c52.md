### Fixed — Qt native startup failures are visible and recoverable (#15818)

Strict packaged Qt shells now paint a system-theme-aware loading surface before
initializing the standard Rust host. A runtime, snapshot, props, or generated-QML
failure becomes an in-window diagnostic with saved-data reassurance and a retry
control; retry destroys any partial host and reruns initialization with a fresh
one. A compile-time-only acceptance seam drives that real emitted control in CI.

