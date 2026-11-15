// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Hamurabi Simulation Core for Swift
// ============================================================================
import PackageDescription

let package = Package(
    name: "hamurabi",
    products: [
        .library(name: "Hamurabi", targets: ["Hamurabi"]),
    ],
    targets: [
        .target(
            name: "Hamurabi"
        ),
        .testTarget(
            name: "HamurabiTests",
            dependencies: ["Hamurabi"]
        ),
    ]
)
