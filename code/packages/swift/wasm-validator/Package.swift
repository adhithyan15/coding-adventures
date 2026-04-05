// swift-tools-version: 5.9
// ============================================================================
// Package.swift — WebAssembly 1.0 validator
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
    name: "wasm-validator",
    products: [
        .library(name: "WasmValidator", targets: ["WasmValidator"]),
    ],
    dependencies: [
        .package(path: "../wasm-leb128"),
        .package(path: "../wasm-types"),
        .package(path: "../wasm-opcodes"),
        .package(path: "../wasm-module-parser"),
    ],
    targets: [
        .target(
            name: "WasmValidator",
            dependencies: [
                .product(name: "WasmLeb128", package: "wasm-leb128"),
                .product(name: "WasmTypes", package: "wasm-types"),
                .product(name: "WasmOpcodes", package: "wasm-opcodes"),
                .product(name: "WasmModuleParser", package: "wasm-module-parser"),
            ]
        ),
        .testTarget(
            name: "WasmValidatorTests",
            dependencies: ["WasmValidator"]
        ),
    ]
)
