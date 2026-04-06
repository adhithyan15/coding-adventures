// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Power Supply
// ============================================================================

import PackageDescription

let package = Package(
    name: "power-supply",
    products: [
        .library(name: "PowerSupply", targets: ["PowerSupply"]),
    ],
    dependencies: [
        .package(path: "../analog-waveform"),
    ],
    targets: [
        .target(
            name: "PowerSupply",
            dependencies: [
                .product(name: "AnalogWaveform", package: "analog-waveform"),
            ]
        ),
        .testTarget(
            name: "PowerSupplyTests",
            dependencies: ["PowerSupply"]
        ),
    ]
)
