// swift-tools-version: 6.0
// ============================================================================
// Package.swift — QrCode
// ============================================================================
//
// Swift Package Manager manifest for the QrCode library.
//
// This package implements a complete QR Code encoder (ISO/IEC 18004:2015).
// It encodes any UTF-8 string into a scannable QR Code using numeric,
// alphanumeric, or byte mode, with all four error correction levels and
// all 40 symbol versions.
//
// ## Dependency Stack
//
//   QrCode            ← this package
//     └─ Barcode2D    ← ModuleGrid type + layout() function
//     └─ GF256        ← GF(2^8) arithmetic for RS encoding
//
// Note: We do NOT depend on ReedSolomon (which uses the b=1 root convention).
// QR Code uses the b=0 convention (first root is α^0 = 1). We embed our own
// LFSR-based RS encoder with the correct QR generator polynomials.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "QrCode",
    products: [
        .library(name: "QrCode", targets: ["QrCode"]),
    ],
    dependencies: [
        .package(path: "../Barcode2D"),
        .package(path: "../gf256"),
    ],
    targets: [
        .target(
            name: "QrCode",
            dependencies: [
                .product(name: "Barcode2D", package: "Barcode2D"),
                .product(name: "GF256", package: "gf256"),
            ],
            path: "Sources/QrCode"
        ),
        .testTarget(
            name: "QrCodeTests",
            dependencies: ["QrCode"],
            path: "Tests/QrCodeTests"
        ),
    ]
)
