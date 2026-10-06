---
category: Swift
---

# A shared Xcode scheme for a project whose projectDirPath is .. belongs in an explicit .xcworkspace, not the project

`mosaic-ios-project` puts `App.xcodeproj` in `iOS/`, with `projectDirPath =
".."`, so the Swift package next to it still builds. UI89 §4.3 needed
`xcodebuild test`, which needs a scheme. Three CI rounds failed with the
scheme in `App.xcodeproj/xcshareddata/xcschemes/App.xcscheme`:

1. "Supported platforms for the buildables in the current scheme is empty"
   and "Unable to find a destination".
2. Adding `SDKROOT = iphoneos` on every target and passing `-sdk
   iphonesimulator` changed nothing. `xcodebuild -list` printed
   "Schemes: App App" from one file on disk, and then reported "Scheme App
   is not currently configured for the build action".
3. The fix was an explicit `iOS/App.xcworkspace` (`contents.xcworkspacedata`
   with `FileRef location = "group:App.xcodeproj"`). The shared scheme went
   in the workspace's `xcshareddata/xcschemes/AppUITests.xcscheme`, named
   differently from the app target, and CI ran
   `xcodebuild test -workspace App.xcworkspace -scheme AppUITests`. The
   Trestle, Journal and Engram XCUITests then all passed on the first run.

The likely cause is that the project's implicit workspace resolves the
scheme's `container:App.xcodeproj` reference against the `..` project
directory, so its buildables never resolve. It also lists an auto-created
scheme with the same name.

What to do: for a generated project with a non-default `projectDirPath`,
generate an explicit workspace beside it and put shared schemes there, under
a name no target has. Log `xcodebuild -list` and `-showdestinations` before
`xcodebuild test`, because a broken scheme is undiagnosable from the test's
own error. Do not iterate on build settings while the scheme listing looks
wrong.
