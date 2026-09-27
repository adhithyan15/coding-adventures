### Fixed — SwiftUI native startup failures are visible and recoverable (#16092)

Strict generated SwiftUI projects now keep runtime and initial-props failures
in the first window and retry through a fresh runtime host. Bundled-runtime path
rewriting and package-owned effect installation follow every fresh attempt, so
Engram retains its import/export effects without replacing generated files.

