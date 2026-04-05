// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "ReedSolomon",
    products: [
        .library(name: "ReedSolomon", targets: ["ReedSolomon"]),
    ],
    dependencies: [
        .package(path: "../gf256"),
    ],
    targets: [
        .target(
            name: "ReedSolomon",
            dependencies: [.product(name: "GF256", package: "gf256")]
        ),
        .testTarget(
            name: "ReedSolomonTests",
            dependencies: ["ReedSolomon"]
        ),
    ]
)
