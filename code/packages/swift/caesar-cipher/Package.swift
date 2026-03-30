// swift-tools-version: 6.0
// ============================================================================
// Package.swift — CaesarCipher
// ============================================================================
//
// Swift Package Manager manifest for the CaesarCipher library.
//
// This package implements the Caesar cipher — one of the oldest and simplest
// encryption techniques. It includes encryption/decryption, ROT13, brute-force
// attack, and frequency analysis. Part of the coding-adventures educational
// computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "CaesarCipher",
    products: [
        .library(name: "CaesarCipher", targets: ["CaesarCipher"]),
    ],
    targets: [
        .target(name: "CaesarCipher"),
        .testTarget(name: "CaesarCipherTests", dependencies: ["CaesarCipher"]),
    ]
)
