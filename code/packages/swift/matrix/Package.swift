// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "matrix",
    products: [
        .library(name: "Matrix", targets: ["Matrix"]),
    ],
    targets: [
        .target(name: "Matrix"),
        .testTarget(name: "MatrixTests", dependencies: ["Matrix"]),
    ]
)
