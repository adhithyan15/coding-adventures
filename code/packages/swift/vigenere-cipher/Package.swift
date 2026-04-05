// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Vigenere cipher -- polyalphabetic substitution cipher
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "vigenere-cipher",
    products: [
        .library(name: "VigenereCipher", targets: ["VigenereCipher"]),
    ],
    targets: [
        .target(
            name: "VigenereCipher"
        ),
        .testTarget(
            name: "VigenereCipherTests",
            dependencies: ["VigenereCipher"]
        ),
    ]
)
