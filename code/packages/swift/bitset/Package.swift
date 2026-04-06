// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Bitset: a compact scalable boolean array
// ============================================================================

import PackageDescription

let package = Package(
    name: "bitset",
    products: [
        .library(name: "Bitset", targets: ["Bitset"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "Bitset",
            dependencies: []
        ),
        .testTarget(
            name: "BitsetTests",
            dependencies: ["Bitset"]
        ),
    ]
)
