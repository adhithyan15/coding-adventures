// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Block RAM
// ============================================================================

import PackageDescription

let package = Package(
    name: "block-ram",
    products: [
        .library(name: "BlockRAM", targets: ["BlockRAM"]),
    ],
    dependencies: [
        .package(path: "../logic_gates"),
    ],
    targets: [
        .target(
            name: "BlockRAM",
            dependencies: [
                .product(name: "LogicGates", package: "logic_gates"),
            ]
        ),
        .testTarget(
            name: "BlockRAMTests",
            dependencies: ["BlockRAM"]
        ),
    ]
)
