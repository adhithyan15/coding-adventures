// swift-tools-version: 6.0
// ============================================================================
// Package.swift — AztecCode
// ============================================================================
//
// Swift Package Manager manifest for the AztecCode library.
//
// This package implements an Aztec Code encoder compliant with
// ISO/IEC 24778:2008. Aztec Code was invented by Andrew Longacre Jr. at
// Welch Allyn in 1995 and published as a patent-free format. It is named
// after the central bullseye finder pattern, which resembles the stepped
// pyramid on the Aztec calendar.
//
// ## Where Aztec Code is used today
//
//   - IATA boarding passes — every airline boarding pass barcode
//   - Eurostar and Amtrak rail tickets — printed and digital tickets
//   - PostNL, Deutsche Post, La Poste — European postal routing
//   - US military ID cards
//
// ## Symbol variants
//
//   Compact: 1–4 layers,  size = 11 + 4*layers  (15×15 to 27×27)
//   Full:    1–32 layers, size = 15 + 4*layers  (19×19 to 143×143)
//
// ## Dependencies
//
//   - Barcode2D:  `ModuleGrid` type and `layout()` function for rendering.
//                 The GF(256)/0x12D arithmetic is implemented inline because
//                 the repo's `GF256` package uses polynomial 0x11D (QR Code),
//                 while Aztec requires 0x12D (same as Data Matrix).
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "AztecCode",
    products: [
        .library(name: "AztecCode", targets: ["AztecCode"]),
    ],
    dependencies: [
        .package(path: "../Barcode2D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "AztecCode",
            dependencies: [
                .product(name: "Barcode2D", package: "Barcode2D"),
                .product(name: "PaintInstructions", package: "PaintInstructions"),
            ],
            path: "Sources/AztecCode"
        ),
        .testTarget(
            name: "AztecCodeTests",
            dependencies: [
                "AztecCode",
                .product(name: "Barcode2D", package: "Barcode2D"),
            ],
            path: "Tests/AztecCodeTests"
        ),
    ]
)
