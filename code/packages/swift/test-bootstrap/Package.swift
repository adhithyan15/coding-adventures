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
