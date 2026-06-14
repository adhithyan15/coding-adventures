// swift-tools-version: 5.9
//
// Package.swift — VisiCalc SwiftUI demo (VC2-swiftui).
//
// Executable target. Run with:
//   swift run
// or open in Xcode:
//   open Package.swift

import PackageDescription

let package = Package(
    name: "VisiCalc",
    platforms: [
        // SwiftUI's TextField + onSubmit + onExitCommand all need
        // macOS 11+ / iOS 15+. mosaic-emit-swiftui emits the
        // `.onChange(of: x) { handler }` form which Apple revised
        // in macOS 14 / iOS 17 (the old single-closure signature
        // is now deprecated, and the new two-or-three-arg form is
        // required). Pick macOS 14 / iOS 17 to match the emitter's
        // current output without an `if #available` workaround.
        .macOS(.v14),
        .iOS(.v17),
    ],
    products: [
        .executable(name: "visicalc", targets: ["VisiCalc"]),
    ],
    targets: [
        .executableTarget(
            name: "VisiCalc",
            path: "Sources/VisiCalc"
        ),
    ]
)
