// swift-tools-version: 5.9
// ============================================================================
// Package.swift — TCP client with buffered I/O and configurable timeouts
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
    name: "tcp-client",
    products: [
        .library(name: "TcpClient", targets: ["TcpClient"]),
    ],
    targets: [
        .target(
            name: "TcpClient"
        ),
        .testTarget(
            name: "TcpClientTests",
            dependencies: ["TcpClient"]
        ),
    ]
)
