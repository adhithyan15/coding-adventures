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
        // macOS 11+ / iOS 15+. We pick macOS 12 to also get
        // ToolbarItem(placement:) parity with iOS 15.
        .macOS(.v12),
        .iOS(.v15),
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
