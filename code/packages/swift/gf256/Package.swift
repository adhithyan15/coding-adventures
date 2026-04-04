// swift-tools-version: 6.0
// ============================================================================
// Package.swift — GF256
// ============================================================================
//
// Swift Package Manager manifest for the GF256 library.
//
// This package implements arithmetic in GF(2^8), the Galois field with
// 256 elements. It uses the primitive polynomial x^8 + x^4 + x^3 + x^2 + 1
// (0x11D). GF(2^8) arithmetic underpins Reed-Solomon error correction
// (used in QR codes, CDs, and hard drives) and the AES cipher.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "GF256",
    products: [
        .library(name: "GF256", targets: ["GF256"]),
    ],
    targets: [
        .target(name: "GF256"),
        .testTarget(name: "GF256Tests", dependencies: ["GF256"]),
    ]
)
