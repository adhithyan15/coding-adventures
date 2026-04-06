// swift-tools-version: 5.9
// ============================================================================
// Package.swift — PixelContainer
// ============================================================================
//
// Swift Package Manager manifest for the PixelContainer library.
//
// PixelContainer is IC00 in the coding-adventures image codec stack.
// It defines the fundamental RGBA8 pixel buffer type used by every image
// format encoder and decoder in this stack (BMP, PPM, QOI, PNG, …).
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "PixelContainer",
    products: [
        .library(name: "PixelContainer", targets: ["PixelContainer"]),
    ],
    targets: [
        .target(name: "PixelContainer", path: "Sources/PixelContainer"),
        .testTarget(
            name: "PixelContainerTests",
            dependencies: ["PixelContainer"],
            path: "Tests/PixelContainerTests"
        ),
    ]
)
