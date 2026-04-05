// swift-tools-version: 5.9
// ============================================================================
// Package.swift — WebAssembly 1.0 execution engine
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
    name: "wasm-execution",
    products: [
        .library(name: "WasmExecution", targets: ["WasmExecution"]),
    ],
    dependencies: [
        .package(path: "../wasm-leb128"),
        .package(path: "../wasm-types"),
        .package(path: "../wasm-opcodes"),
        .package(path: "../virtual-machine"),
    ],
    targets: [
        .target(
            name: "WasmExecution",
            dependencies: [
                .product(name: "WasmLeb128", package: "wasm-leb128"),
                .product(name: "WasmTypes", package: "wasm-types"),
                .product(name: "WasmOpcodes", package: "wasm-opcodes"),
                .product(name: "VirtualMachine", package: "virtual-machine"),
            ]
        ),
        .testTarget(
            name: "WasmExecutionTests",
            dependencies: ["WasmExecution"]
        ),
    ]
)
