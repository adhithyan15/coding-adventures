// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Register-based VM, V8 Ignition-style
// ============================================================================
//
// This is the Swift Package Manager manifest for this package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
import PackageDescription

let package = Package(
    name: "register-vm",
    products: [
        .library(name: "RegisterVM", targets: ["RegisterVM"]),
    ],
    targets: [
        .target(name: "RegisterVM"),
        .testTarget(name: "RegisterVMTests", dependencies: ["RegisterVM"]),
    ]
)
