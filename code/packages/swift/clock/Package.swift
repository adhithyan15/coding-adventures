// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Clock
// ============================================================================

import PackageDescription

let package = Package(
    name: "clock",
    products: [
        .library(name: "Clock", targets: ["Clock"]),
    ],
    dependencies: [
    ],
    targets: [
        .target(
            name: "Clock",
            dependencies: [
            ]
        ),
        .testTarget(
            name: "ClockTests",
            dependencies: ["Clock"]
        ),
    ]
)
