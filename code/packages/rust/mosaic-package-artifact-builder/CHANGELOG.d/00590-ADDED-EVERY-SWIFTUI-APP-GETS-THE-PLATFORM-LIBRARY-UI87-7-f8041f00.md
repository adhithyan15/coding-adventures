### Added — every SwiftUI app gets the platform library (UI87 §7)

- `Sources/App/MosaicPlatformEffects.swift` is written beside
  `MosaicRuntimeHost.swift` in every SwiftUI project (and so is listed in the
  iOS app target too).
- `App.swift` installs `installMosaicPlatformEffects` right after the host is
  assigned in `MosaicHostState`, after the package's own `[host_effects]`
  handler if there is one, with the handler's `kinds` as a Swift set (or
  `nil`). The same class-scoped, line-anchored install point as the handler.
- A package with no SwiftUI handler now gets the platform line alone, where it
  used to be left untouched; an app without the anchor is still left as it is.
  Two `MosaicHostState` declarations are refused either way, and the message
  names the platform library when no handler was declared.

