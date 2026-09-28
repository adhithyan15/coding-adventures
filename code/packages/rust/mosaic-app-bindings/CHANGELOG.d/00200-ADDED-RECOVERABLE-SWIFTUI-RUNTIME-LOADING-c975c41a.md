### Added — recoverable SwiftUI runtime loading

`MosaicRuntimeHost.loadRecoverable()` returns the strict runtime load as a
`Result`, allowing generated SwiftUI windows to report a loader failure and
retry instead of terminating through `loadRequired()`.

