// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ImageCodecPPM
// ============================================================================
//
// Swift Package Manager manifest for the ImageCodecPPM library.
//
// ImageCodecPPM is IC02 in the coding-adventures image codec stack.
// It implements the PPM (Portable Pixmap) image format encoder and decoder,
// depending on PixelContainer (IC00) for the shared pixel buffer type.
//
// PPM is a plain-text image format from the Netpbm toolkit. Its simplicity
// makes it excellent for learning text-based binary I/O: parsing ASCII
// headers mixed with binary pixel data.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "ImageCodecPPM",
    products: [
        .library(name: "ImageCodecPPM", targets: ["ImageCodecPPM"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "ImageCodecPPM",
            dependencies: ["PixelContainer"],
            path: "Sources/ImageCodecPPM"
        ),
        .testTarget(
            name: "ImageCodecPPMTests",
            dependencies: ["ImageCodecPPM", "PixelContainer"],
            path: "Tests/ImageCodecPPMTests"
        ),
    ]
)
