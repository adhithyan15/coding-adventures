// swift-tools-version: 5.9
// ============================================================================
// Package.swift — WebAssembly type definitions
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
    name: "wasm-types",
    products: [
        .library(name: "WasmTypes", targets: ["WasmTypes"]),
    ],
    dependencies: [
        .package(path: "../wasm-leb128"),
    ],
    targets: [
        .target(
            name: "WasmTypes",
            dependencies: [
                .product(name: "WasmLeb128", package: "wasm-leb128"),
            ]
        ),
        .testTarget(
            name: "WasmTypesTests",
            dependencies: ["WasmTypes"]
        ),
    ]
)
