// swift-tools-version: 6.0
// ============================================================================
// Package.swift — ChaCha20-Poly1305
// ============================================================================
//
// Swift Package Manager manifest for the ChaCha20-Poly1305 library.
//
// This package implements ChaCha20-Poly1305 authenticated encryption
// (RFC 8439) from scratch. It is entirely self-contained — only ARX
// (Add, Rotate, XOR) operations and big-integer arithmetic are needed.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "ChaCha20Poly1305",
    products: [
        .library(name: "ChaCha20Poly1305", targets: ["ChaCha20Poly1305"]),
    ],
    targets: [
        .target(
            name: "ChaCha20Poly1305"
        ),
        .testTarget(
            name: "ChaCha20Poly1305Tests",
            dependencies: ["ChaCha20Poly1305"]
        ),
    ]
)
