// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Stats -- descriptive statistics, frequency analysis,
// and cryptanalysis helpers
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "Stats",
    products: [
        .library(name: "Stats", targets: ["Stats"]),
    ],
    targets: [
        .target(
            name: "Stats"
        ),
        .testTarget(
            name: "StatsTests",
            dependencies: ["Stats"]
        ),
    ]
)
