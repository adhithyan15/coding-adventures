// swift-tools-version: 5.9
// ============================================================================
// Package.swift — FPGA
// ============================================================================

import PackageDescription

let package = Package(
    name: "fpga",
    products: [
        .library(name: "FPGA", targets: ["FPGA"]),
    ],
    dependencies: [
        .package(path: "../logic_gates"),
        .package(path: "../block-ram")
    ],
    targets: [
        .target(
            name: "FPGA",
            dependencies: [
                .product(name: "LogicGates", package: "logic_gates"),
                .product(name: "BlockRAM", package: "block-ram")
            ]
        ),
        .testTarget(
            name: "FPGATests",
            dependencies: ["FPGA"]
        ),
    ]
)
