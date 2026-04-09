// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Analog Waveform: continuous-time signal fundamentals
// ============================================================================

import PackageDescription

let package = Package(
    name: "analog-waveform",
    products: [
        .library(name: "AnalogWaveform", targets: ["AnalogWaveform"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "AnalogWaveform",
            dependencies: []
        ),
        .testTarget(
            name: "AnalogWaveformTests",
            dependencies: ["AnalogWaveform"]
        ),
    ]
)
