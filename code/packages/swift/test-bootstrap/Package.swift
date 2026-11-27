// swift-tools-version: 5.9
// ============================================================================
// Package.swift -- TestBootstrap (VER05k)
// ============================================================================
//
// The Swift property-based testing engine. No dependencies: the library uses
// the standard library plus Foundation's NSLock and Thread (nothing else).
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "TestBootstrap",
    // TaskLocal (the async half of the shrink-budget scope) needs the concurrency runtime.
    platforms: [.macOS(.v10_15), .iOS(.v13), .tvOS(.v13), .watchOS(.v6)],
    products: [
        .library(name: "TestBootstrap", targets: ["TestBootstrap"]),
    ],
    targets: [
        .target(
            name: "TestBootstrap"
        ),
        .testTarget(
            name: "TestBootstrapTests",
            dependencies: ["TestBootstrap"]
        ),
    ]
)
