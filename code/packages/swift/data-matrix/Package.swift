// swift-tools-version: 6.0
// ============================================================================
// Package.swift — DataMatrix
// ============================================================================
//
// Swift Package Manager manifest for the DataMatrix library.
//
// This package implements a Data Matrix ECC200 encoder compliant with
// ISO/IEC 16022:2006. Data Matrix is a high-density 2D barcode used on PCBs,
// pharmaceutical unit-dose packages, aerospace parts, and surgical instruments.
//
// Dependencies:
//   - GF256:             Galois Field GF(2^8) arithmetic. We use the
//                        `GF256Field` factory with primitive polynomial
//                        0x12D (NOT 0x11D — Data Matrix uses a different
//                        irreducible polynomial than QR Code).
//   - Barcode2D:         `ModuleGrid` type and `layout()` function.
//   - PaintInstructions: `PaintScene` type used by the rendering layer.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "DataMatrix",
    products: [
        .library(name: "DataMatrix", targets: ["DataMatrix"]),
    ],
    dependencies: [
        .package(path: "../gf256"),
        .package(path: "../Barcode2D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "DataMatrix",
            dependencies: [
                .product(name: "GF256", package: "gf256"),
                .product(name: "Barcode2D", package: "Barcode2D"),
                .product(name: "PaintInstructions", package: "PaintInstructions"),
            ],
            path: "Sources/DataMatrix"
        ),
        .testTarget(
            name: "DataMatrixTests",
            dependencies: [
                "DataMatrix",
                .product(name: "Barcode2D", package: "Barcode2D"),
            ],
            path: "Tests/DataMatrixTests"
        ),
    ]
)
