// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Logic Gates: the foundation of all digital circuits
// ============================================================================
//
// This is the Swift Package Manager manifest for the logic-gates package.
// It is part of the coding-adventures project, an educational computing stack
// built from logic gates up through interpreters and compilers.
//
// # Dependency Chain
//
//   transistors (Layer 0) ← logic-gates (Layer 1) ← arithmetic (Layer 2) ← ...
//
// Logic gates delegate their physical evaluation to the transistors package.
// Every gate is ultimately implemented by CMOS transistor pairs — the NAND
// and NOR gates are the "natural" CMOS primitives, and all others are derived
// by combining them with inverters.
//
import PackageDescription

let package = Package(
    name: "logic-gates",
    products: [
        .library(name: "LogicGates", targets: ["LogicGates"]),
    ],
    dependencies: [
        // Local monorepo dependency: transistors provides the CMOS gate
        // implementations that logic gates delegate to.
        .package(path: "../transistors"),
    ],
    targets: [
        .target(
            name: "LogicGates",
            dependencies: [
                .product(name: "Transistors", package: "transistors"),
            ]
        ),
        .testTarget(
            name: "LogicGatesTests",
            dependencies: ["LogicGates"]
        ),
    ]
)
