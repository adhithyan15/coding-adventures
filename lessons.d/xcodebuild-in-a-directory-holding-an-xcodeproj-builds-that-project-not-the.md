---
category: CI & GitHub Actions
---

# xcodebuild in a directory holding an .xcodeproj builds that project, not the Swift package beside it

**What went wrong.** UI89 step 2 (#16041) wrote the generated iOS app
project as `swiftui/App.xcodeproj`, beside the Swift package's
`Package.swift`. The existing CI step builds that package for iOS with
`cd swiftui && xcodebuild -scheme App …` and then runs `nm` on
`Build/Products/Debug-iphonesimulator/App`. With an `.xcodeproj` in the
directory, `xcodebuild` builds the project instead of the package: the build
still succeeded, but produced `App.app`, and `nm` failed with "No such file".
A local check had passed for the same reason: "BUILD SUCCEEDED" came from the
wrong target.

**Fix.** The generator writes the project to `swiftui/iOS/App.xcodeproj` with
`projectDirPath = ".."`, so its paths are still the package's, and the package
directory holds no `.xcodeproj`.

**Do differently.**
- Never put an `.xcodeproj` (or `.xcworkspace`) beside a `Package.swift` that
  anything builds with `xcodebuild -scheme`: the project silently wins.
- When verifying an `xcodebuild` step locally, check the **product path** the
  step consumes, not only "BUILD SUCCEEDED".
