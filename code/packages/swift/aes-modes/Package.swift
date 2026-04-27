// swift-tools-version: 6.0
// ============================================================================
// Package.swift — AES Modes of Operation
// ============================================================================
//
// Swift Package Manager manifest for the AES Modes library.
//
// This package implements four AES modes of operation (ECB, CBC, CTR, GCM)
// wrapping the AES block cipher from the sibling `aes` package.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "aes-modes",
    products: [.library(name: "AESModes", targets: ["AESModes"])],
    dependencies: [.package(path: "../aes")],
    targets: [
        .target(name: "AESModes", dependencies: [.product(name: "AES", package: "AES")]),
        .testTarget(name: "AESModesTests", dependencies: ["AESModes"]),
    ]
)
