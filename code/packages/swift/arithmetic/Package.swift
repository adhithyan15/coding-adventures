// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Arithmetic: Layer 2, built on Logic Gates
// ============================================================================

import PackageDescription

let package = Package(
    name: "arithmetic",
    products: [
        .library(name: "Arithmetic", targets: ["Arithmetic"]),
    ],
    dependencies: [
        .package(path: "../logic_gates"),
    ],
    targets: [
        .target(
            name: "Arithmetic",
            dependencies: [
                .product(name: "LogicGates", package: "logic_gates"),
            ]
        ),
        .testTarget(
            name: "ArithmeticTests",
            dependencies: ["Arithmetic"]
        ),
    ]
)
