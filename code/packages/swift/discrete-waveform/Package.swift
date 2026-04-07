// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Discrete Waveform
// ============================================================================

import PackageDescription

let package = Package(
    name: "discrete-waveform",
    products: [
        .library(name: "DiscreteWaveform", targets: ["DiscreteWaveform"]),
    ],
    dependencies: [
        .package(path: "../analog-waveform"),
    ],
    targets: [
        .target(
            name: "DiscreteWaveform",
            dependencies: [
                .product(name: "AnalogWaveform", package: "analog-waveform"),
            ]
        ),
        .testTarget(
            name: "DiscreteWaveformTests",
            dependencies: ["DiscreteWaveform"]
        ),
    ]
)
