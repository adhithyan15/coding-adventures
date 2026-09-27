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

The macOS lane runs clean, restored, and incompatible-state launches against
one explicit state path.

## Platform library checks (UI87 §7)

`PlatformEffectsChecks.swift` drives the SwiftUI platform library
(`MosaicPlatformEffects.swift`: `files.open` / `files.save` and the router that
sends each effect to the app's handler or to the library by kind) with a fake
host and fake panels, so the real open/save logic, limits and routing run
without a display or a Rust runtime. It is compiled only when asked for, so the
harness still builds from the binding alone:

```sh
cp <generated>/Sources/App/MosaicPlatformEffects.swift Sources/Conformance/
swift run -Xswiftc -DMOSAIC_PLATFORM_EFFECTS Conformance --platform-effects
```

The macOS CI lane runs this after the runtime round trip.
