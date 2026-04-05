// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Web Components backend: emits Custom Element classes from MosaicIR
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
// Local monorepo dependencies are declared via relative path references so
// that SPM resolves them from the local filesystem.
//
import PackageDescription

let package = Package(
    name: "mosaic-emit-webcomponent",
    products: [
        .library(name: "MosaicEmitWebcomponent", targets: ["MosaicEmitWebcomponent"]),
    ],
    dependencies: [
        .package(path: "../mosaic-vm"),
        .package(path: "../mosaic-analyzer"),
        .package(path: "../mosaic-parser"),
        .package(path: "../mosaic-lexer"),
    ],
    targets: [
        .target(
            name: "MosaicEmitWebcomponent",
            dependencies: [
                .product(name: "MosaicVm", package: "mosaic-vm"),
                .product(name: "MosaicAnalyzer", package: "mosaic-analyzer"),
                .product(name: "MosaicParser", package: "mosaic-parser"),
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
            ]
        ),
        .testTarget(
            name: "MosaicEmitWebcomponentTests",
            dependencies: ["MosaicEmitWebcomponent"]
        ),
    ]
)
