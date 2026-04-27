// swift-tools-version: 6.0
// ============================================================================
// Package.swift — PDF417
// ============================================================================
//
// Swift Package Manager manifest for the PDF417 library.
//
// This package implements a PDF417 stacked linear barcode encoder compliant
// with ISO/IEC 15438:2015. PDF417 is widely used for AAMVA driver's licences,
// IATA boarding passes, USPS labels, and US immigration forms.
//
// Dependencies:
//   - Barcode2D:        ModuleGrid type and layout() function for rendering.
//   - PaintInstructions: PaintScene type used by `encodeAndLayout()`.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "PDF417",
    products: [
        .library(name: "PDF417", targets: ["PDF417"]),
    ],
    dependencies: [
        .package(path: "../Barcode2D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "PDF417",
            dependencies: [
                .product(name: "Barcode2D", package: "Barcode2D"),
                .product(name: "PaintInstructions", package: "PaintInstructions"),
            ],
            path: "Sources/PDF417"
        ),
        .testTarget(
            name: "PDF417Tests",
            dependencies: [
                "PDF417",
                .product(name: "Barcode2D", package: "Barcode2D"),
            ],
            path: "Tests/PDF417Tests"
        ),
    ]
)
