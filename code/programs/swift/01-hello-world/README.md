# 01 — Hello World

The first app in the Swift learning series. Two targets, zero logic —
just enough to prove the toolchain works end to end.

## What it does

- iPhone: displays a water drop icon and "Hello, World!"
- Apple Watch: displays the same icon and "Hello from your Watch!"

## What it teaches

- SwiftUI app structure (`@main`, `App`, `WindowGroup`)
- The `View` protocol and the `body` property
- SF Symbols
- How iOS and watchOS targets coexist in one Xcode project
- How to build and test with `xcodebuild` on the command line

## Running locally

```bash
# Generate the Xcode project
xcodegen generate

# Build for iPhone simulator
xcodebuild build \
  -scheme HelloWorld \
  -destination 'platform=iOS Simulator,name=iPhone 16'

# Build for Apple Watch simulator
xcodebuild build \
  -scheme HelloWorldWatch \
  -destination 'platform=watchOS Simulator,name=Apple Watch Ultra 2 (49mm)'
```

Then open `HelloWorld.xcodeproj` in Xcode and press Run (⌘R).

## Project structure

```
01-hello-world/
├── project.yml                        xcodegen spec (source of truth)
├── HelloWorld.xcodeproj/              generated — do not edit by hand
├── Sources/
│   ├── HelloWorld/
│   │   ├── HelloWorldApp.swift        iOS app entry point
│   │   └── ContentView.swift          the one iPhone screen
│   └── HelloWorldWatch/
│       ├── HelloWorldWatchApp.swift   watchOS app entry point
│       └── WatchContentView.swift     the one Watch screen
└── README.md
```

## Part of the Foveo series

This app is the foundation of [Foveo](../../foveo/), a self-care companion
app with water reminders, a plant mascot, and Apple Watch Ultra integration.
Each app in this series introduces one layer of complexity.
