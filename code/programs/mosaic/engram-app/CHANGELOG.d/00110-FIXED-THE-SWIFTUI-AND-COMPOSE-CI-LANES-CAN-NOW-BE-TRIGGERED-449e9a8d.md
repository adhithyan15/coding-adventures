### Fixed — the SwiftUI and Compose CI lanes can now be triggered by Engram

`mosaic/programs/engram-app` was absent from both acceptance sets, so a change
touching only `host/swiftui/EngramEffects.swift` did not fire the lane that
compiles it. The SwiftUI lane has built Engram since #13728 — but never on
Engram's own account, only when some other package dragged it in. The edits
most likely to break a handler were precisely the ones that skipped its only
compile check.

Found while adding the matching Compose lane. Both sets now list the package,
and both suites have a test that fails without it.

