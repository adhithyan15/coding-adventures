// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Generic tree walker that drives Mosaic compiler backends
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "mosaic-vm",
    products: [
        .library(name: "MosaicVm", targets: ["MosaicVm"]),
    ],
    dependencies: [
        .package(path: "../mosaic-analyzer"),
        .package(path: "../mosaic-parser"),
        .package(path: "../mosaic-lexer"),
    ],
    targets: [
        .target(
            name: "MosaicVm",
            dependencies: [
                .product(name: "MosaicAnalyzer", package: "mosaic-analyzer"),
                .product(name: "MosaicParser", package: "mosaic-parser"),
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
            ]
        ),
        .testTarget(
            name: "MosaicVmTests",
            dependencies: ["MosaicVm"]
        ),
    ]
)
