// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Generic stack-based VM with pluggable opcode handlers
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
    name: "virtual-machine",
    products: [
        .library(name: "VirtualMachine", targets: ["VirtualMachine"]),
    ],
    targets: [
        .target(
            name: "VirtualMachine"
        ),
        .testTarget(
            name: "VirtualMachineTests",
            dependencies: ["VirtualMachine"]
        ),
    ]
)
