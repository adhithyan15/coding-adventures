- **iOS and iPadOS (UI89 step 7, §2.4).** Journal now runs on the iPhone and
  iPad simulators with no source changes. CI links `journal-mosaic-app`
  statically from an `.xcframework` and builds the generated iOS app target.
  The app must export `mosaic_app_create` and carry
  `dev.codingadventures.journalapp` / `JournalApp`. On the iPhone simulator,
  `mosaic-ios-simulator-gate.sh` launches it three times: fresh state must be
  written in its container, restored, and a refused `{}` quarantined. The app
  is then launched on an iPad simulator.
