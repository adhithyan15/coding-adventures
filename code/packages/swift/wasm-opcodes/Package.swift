// swift-tools-version: 5.9
// ============================================================================
// Package.swift — WebAssembly opcode table
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
    name: "wasm-opcodes",
    products: [
        .library(name: "WasmOpcodes", targets: ["WasmOpcodes"]),
    ],
    dependencies: [
        .package(path: "../wasm-types"),
    ],
    targets: [
        .target(
            name: "WasmOpcodes",
            dependencies: [
                .product(name: "WasmTypes", package: "wasm-types"),
            ]
        ),
        .testTarget(
            name: "WasmOpcodesTests",
            dependencies: ["WasmOpcodes"]
        ),
    ]
)
