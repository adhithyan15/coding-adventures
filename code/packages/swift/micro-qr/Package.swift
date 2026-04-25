// swift-tools-version: 6.0
// ============================================================================
// Package.swift — MicroQR
// ============================================================================
//
// Swift Package Manager manifest for the MicroQR library.
//
// This package implements a Micro QR Code encoder compliant with
// ISO/IEC 18004:2015 Annex E. Micro QR Code is the compact variant designed
// for surface-mount component labels, circuit board markings, and other
// applications where the smallest standard QR (21×21) is too large.
//
// Dependencies:
//   - GF256:    Galois Field GF(2^8) arithmetic for Reed-Solomon ECC
//   - Barcode2D: ModuleGrid type and layout() function for rendering
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "MicroQR",
    products: [
        .library(name: "MicroQR", targets: ["MicroQR"]),
    ],
    dependencies: [
        .package(path: "../gf256"),
        .package(path: "../Barcode2D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "MicroQR",
            dependencies: [
                .product(name: "GF256", package: "gf256"),
                .product(name: "Barcode2D", package: "Barcode2D"),
                .product(name: "PaintInstructions", package: "PaintInstructions"),
            ],
            path: "Sources/MicroQR"
        ),
        .testTarget(
            name: "MicroQRTests",
            dependencies: [
                "MicroQR",
                .product(name: "Barcode2D", package: "Barcode2D"),
            ],
            path: "Tests/MicroQRTests"
        ),
    ]
)
