### Added — Engram runs on the iOS and iPadOS simulators (UI89 step 7, §2.5)

CI now runs Engram on the iOS and iPadOS simulators:
- `engram-mosaic-app` is built as an `.xcframework`, and the generated iOS app
  target is built for the simulator. This is the first iOS compile of Engram's
  SwiftUI output, including the `#else` branch of `EngramEffects.swift` and
  the touch layout.
- CI checks that `_mosaic_app_create` is linked and that the app carries
  `dev.codingadventures.engramapp` / `EngramApp`.
- `mosaic-ios-simulator-gate.sh` launches the app three times on the iPhone
  simulator: state must be written, restored, and refused state quarantined.
  The app then runs on the iPad simulator.

No Engram source changes.
