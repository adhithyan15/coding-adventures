// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- JsonRpc
// ============================================================================
//
// JSON-RPC 2.0 implementation for Swift, providing Content-Length-framed
// message reading/writing and a server with method dispatch. Built on top
// of the generic Rpc package.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "JsonRpc",
    products: [
        .library(name: "JsonRpc", targets: ["JsonRpc"]),
    ],
    dependencies: [
        .package(path: "../rpc"),
    ],
    targets: [
        .target(
            name: "JsonRpc",
            dependencies: [
                .product(name: "Rpc", package: "rpc"),
            ]
        ),
        .testTarget(
            name: "JsonRpcTests",
            dependencies: ["JsonRpc"]
        ),
    ]
)
