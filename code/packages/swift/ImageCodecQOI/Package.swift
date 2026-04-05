// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ImageCodecQOI
// ============================================================================
//
// Swift Package Manager manifest for the ImageCodecQOI library.
//
// ImageCodecQOI is IC03 in the coding-adventures image codec stack.
// It implements the QOI (Quite OK Image) format encoder and decoder,
// depending on PixelContainer (IC00) for the shared pixel buffer type.
//
// QOI (https://qoiformat.org) is a lossless image format designed for
// simplicity and speed. Its encoder and decoder each fit in ~300 lines of C.
// It achieves typical 2-4x compression on photographic images and near-PNG
// quality on pixel art — making it an excellent format for learning
// compression algorithms.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "ImageCodecQOI",
    products: [
        .library(name: "ImageCodecQOI", targets: ["ImageCodecQOI"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "ImageCodecQOI",
            dependencies: ["PixelContainer"],
            path: "Sources/ImageCodecQOI"
        ),
        .testTarget(
            name: "ImageCodecQOITests",
            dependencies: ["ImageCodecQOI", "PixelContainer"],
            path: "Tests/ImageCodecQOITests"
        ),
    ]
)
