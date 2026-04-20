// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ImagePointOps
// ============================================================================
//
// Swift Package Manager manifest for the ImagePointOps library.
//
// ImagePointOps is IMG03 in the coding-adventures image processing stack.
// It provides per-pixel point operations over PixelContainer (IC00):
// invert, threshold, gamma, exposure, greyscale, sepia, colour matrix,
// saturation, hue rotation, and 1D LUTs — all computed correctly in
// linear light where required.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "ImagePointOps",
    products: [
        .library(name: "ImagePointOps", targets: ["ImagePointOps"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "ImagePointOps",
            dependencies: ["PixelContainer"],
            path: "Sources/ImagePointOps"
        ),
        .testTarget(
            name: "ImagePointOpsTests",
            dependencies: ["ImagePointOps", "PixelContainer"],
            path: "Tests/ImagePointOpsTests"
        ),
    ]
)
