// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Rpc
// ============================================================================
//
// Codec-agnostic RPC primitives for Swift. This package is the protocol layer
// that future JSON-RPC and LSP implementations can build on top of.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "Rpc",
    products: [
        .library(name: "Rpc", targets: ["Rpc"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "Rpc"
        ),
        .testTarget(
            name: "RpcTests",
            dependencies: ["Rpc"]
        ),
    ]
)
