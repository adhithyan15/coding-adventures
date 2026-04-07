// swift-tools-version: 5.9
// ============================================================================
// Package.swift — ImageCodecBMP
// ============================================================================
//
// Swift Package Manager manifest for the ImageCodecBMP library.
//
// ImageCodecBMP is IC01 in the coding-adventures image codec stack.
// It implements the Windows BMP (Bitmap) image format encoder and decoder,
// depending on PixelContainer (IC00) for the shared pixel buffer type.
//
// BMP is the simplest well-documented binary image format: a small fixed
// header followed by uncompressed pixel rows. It is an ideal first format
// to learn binary file I/O and little-endian byte layout.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "ImageCodecBMP",
    products: [
        .library(name: "ImageCodecBMP", targets: ["ImageCodecBMP"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "ImageCodecBMP",
            dependencies: ["PixelContainer"],
            path: "Sources/ImageCodecBMP"
        ),
        .testTarget(
            name: "ImageCodecBMPTests",
            dependencies: ["ImageCodecBMP", "PixelContainer"],
            path: "Tests/ImageCodecBMPTests"
        ),
    ]
)
