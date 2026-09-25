# mosaic-ios-project

Generates the Xcode project that turns a Mosaic SwiftUI package into an
installable iOS and iPadOS app (spec [UI89](../../../specs/UI89-mobile-targets.md)
§2.2).

SwiftPM compiles SwiftUI for iOS but cannot produce an `.app`; an Xcode app
target can. This crate is a pure function from a small description of the app
(its name, identity, sources, include paths and `.xcframework`s) to the text
of `project.pbxproj`. No XcodeGen, no Tuist, no Xcode needed to generate or to
test it.

## Where it sits

```text
mosaic-compile pkg --backend swiftui --emit-project --runtime-library <X>.xcframework
  └── mosaic-package-artifact-builder
        ├── Package.swift, Sources/…            (the Swift package, as before)
        └── mosaic-ios-project ──▶ App.xcodeproj/project.pbxproj
```

It is shared Mosaic infrastructure: every app built with a static runtime —
Trestle today, Journal, Engram and Venture next — gets the same project from
the same generator.

## What the project contains

- One application target for iPhone and iPad (`TARGETED_DEVICE_FAMILY = "1,2"`).
- The generated Swift sources and the C runtime loader, compiled with
  `MOSAIC_RUNTIME_STATIC`, importable through a module map (`module_map`).
- The runtime `.xcframework`, linked; Xcode picks the device or simulator slice.
- `Info.plist` from build settings: display name, launch screen, scene
  manifest (multiple windows on iPadOS), every orientation.
- Deterministic object identifiers: regenerating changes nothing.

Every string is quoted and escaped, and absolute paths, `..`, `$(…)` and
control characters are refused, so package data cannot inject build settings
or reach outside the project.

## Usage

```rust
use mosaic_ios_project::{default_bundle_identifier, project_pbxproj, IosApp};

let app = IosApp {
    product_name: "App".into(),
    display_name: "Trestle".into(),
    bundle_identifier: default_bundle_identifier("task-app"),
    marketing_version: "0.1.0".into(),
    deployment_target: "16.0".into(),
    swift_sources: vec!["Sources/App/App.swift".into()],
    c_sources: vec!["Sources/CMosaicRuntime/CMosaicRuntime.c".into()],
    headers: vec![],
    header_search_paths: vec!["Sources/CMosaicRuntime/include".into()],
    swift_include_paths: vec!["Sources/CMosaicRuntime/include".into()],
    preprocessor_definitions: vec!["MOSAIC_RUNTIME_STATIC=1".into()],
    xcframeworks: vec!["Runtime/MosaicAppRuntime.xcframework".into()],
};
let text = project_pbxproj(&app)?;
```

Then build and run it:

```bash
xcodebuild -project App.xcodeproj -target App -sdk iphonesimulator CODE_SIGNING_ALLOWED=NO build
xcrun simctl install booted build/Debug-iphonesimulator/App.app
```
