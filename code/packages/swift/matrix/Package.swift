// swift-tools-version:6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "Matrix",
    products: [
        .library(name: "Matrix", targets: ["Matrix"]),
    ],
    targets: [
        .target(name: "Matrix"),
        .testTarget(name: "MatrixTests", dependencies: ["Matrix"]),
    ]
)
