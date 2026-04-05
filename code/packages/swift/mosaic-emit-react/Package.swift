// swift-tools-version: 5.9
// ============================================================================
// Package.swift — React backend: emits TSX functional components from MosaicIR
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "mosaic-emit-react",
    products: [
        .library(name: "MosaicEmitReact", targets: ["MosaicEmitReact"]),
    ],
    dependencies: [
        .package(path: "../mosaic-vm"),
        .package(path: "../mosaic-analyzer"),
        .package(path: "../mosaic-parser"),
        .package(path: "../mosaic-lexer"),
    ],
    targets: [
        .target(
            name: "MosaicEmitReact",
            dependencies: [
                .product(name: "MosaicVm", package: "mosaic-vm"),
                .product(name: "MosaicAnalyzer", package: "mosaic-analyzer"),
                .product(name: "MosaicParser", package: "mosaic-parser"),
                .product(name: "MosaicLexer", package: "mosaic-lexer"),
            ]
        ),
        .testTarget(
            name: "MosaicEmitReactTests",
            dependencies: ["MosaicEmitReact"]
        ),
    ]
)
