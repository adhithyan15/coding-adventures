// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- Ls00
// ============================================================================
//
// Generic LSP framework for Swift. Handles all protocol boilerplate so
// language authors only need to implement the LanguageBridge protocol
// and optional provider protocols.
//
// Depends on the JsonRpc package for Content-Length-framed JSON-RPC 2.0
// communication, which itself depends on the generic Rpc package.
//
// ============================================================================
import PackageDescription

let package = Package(
    name: "Ls00",
    products: [
        .library(name: "Ls00", targets: ["Ls00"]),
    ],
    dependencies: [
        .package(path: "../json-rpc"),
    ],
    targets: [
        .target(
            name: "Ls00",
            dependencies: [
                .product(name: "JsonRpc", package: "json-rpc"),
            ]
        ),
        .testTarget(
            name: "Ls00Tests",
            dependencies: ["Ls00"]
        ),
    ]
)
