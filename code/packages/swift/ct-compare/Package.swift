// swift-tools-version: 5.9
// ============================================================================
// Package.swift -- CtCompare (CT01)
// ============================================================================
//
// Swift Package Manager manifest for constant-time byte comparison and
// selection. No dependencies: the library is pure computation on [UInt8].
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "CtCompare",
    products: [
        .library(name: "CtCompare", targets: ["CtCompare"]),
    ],
    targets: [
        .target(
            name: "CtCompare"
        ),
        .testTarget(
            name: "CtCompareTests",
            dependencies: ["CtCompare"]
        ),
    ]
)
