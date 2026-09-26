### Fixed — Compose native startup failures are visible and recoverable (#15786)

Strict generated Compose Desktop shells now open their window before loading the
Rust runtime and initial snapshot. They show a system-theme-aware loading state,
turn initialization errors into an in-window message with diagnostic detail and
saved-data reassurance, and let the user retry initialization without relaunching.
The first host and props load run off the UI thread; a failed partial host is
closed before retry.

