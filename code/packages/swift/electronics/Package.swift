// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Electronics
// ============================================================================

import PackageDescription

let package = Package(
    name: "electronics",
    products: [
        .library(name: "Electronics", targets: ["Electronics"]),
    ],
    dependencies: [
        .package(path: "../power-supply"),
        .package(path: "../analog-waveform"),
    ],
    targets: [
        .target(
            name: "Electronics",
            dependencies: [
                .product(name: "PowerSupply", package: "power-supply"),
                .product(name: "AnalogWaveform", package: "analog-waveform"),
            ]
        ),
        .testTarget(
            name: "ElectronicsTests",
            dependencies: ["Electronics"]
        ),
    ]
)
