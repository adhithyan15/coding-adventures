// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ImageGeometricTransforms
// ============================================================================
//
// Swift Package Manager manifest for the ImageGeometricTransforms library.
//
// ImageGeometricTransforms is IMG04 in the coding-adventures image processing
// stack. It provides spatial (geometric) transforms over PixelContainer (IC00):
// flip, rotate (90/180/arbitrary), crop, pad, scale, affine warp, and
// perspective warp — with nearest, bilinear, and bicubic (Catmull-Rom)
// interpolation modes and configurable out-of-bounds handling.
//
// Part of the coding-adventures educational computing stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "ImageGeometricTransforms",
    products: [
        .library(name: "ImageGeometricTransforms", targets: ["ImageGeometricTransforms"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "ImageGeometricTransforms",
            dependencies: ["PixelContainer"],
            path: "Sources/ImageGeometricTransforms"
        ),
        .testTarget(
            name: "ImageGeometricTransformsTests",
            dependencies: ["ImageGeometricTransforms", "PixelContainer"],
            path: "Tests/ImageGeometricTransformsTests"
        ),
    ]
)
