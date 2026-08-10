# SwiftUI runtime conformance

This macOS console harness compiles the exact `MosaicRuntimeHost.swift`, C loader,
and C header emitted into a generated SwiftUI project. It loads the shared
`mosaic-app-conformance` dylib and verifies startup, revisions, prop projection,
semantic dispatch, snapshot/restore, prop-change notification, and teardown.

CI generates the complete TaskApp project, copies its generated binding sources
into this harness in a temporary workspace, and runs the result. The binding and
loader are deliberately not duplicated here. The harness declares only the
source-compatible host protocol that the generated TaskApp normally owns, so it
can exercise the unchanged runtime host without launching a SwiftUI window.
