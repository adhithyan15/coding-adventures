// swift-tools-version: 5.9
// ============================================================================
// Package.swift — WebAssembly LEB128 encoding
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
    name: "wasm-leb128",
    products: [
        .library(name: "WasmLeb128", targets: ["WasmLeb128"]),
    ],
    targets: [
        .target(
            name: "WasmLeb128"
        ),
        .testTarget(
            name: "WasmLeb128Tests",
            dependencies: ["WasmLeb128"]
        ),
    ]
)
