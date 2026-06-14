// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SqlBackend",
    products: [
        .library(name: "SqlBackend", targets: ["SqlBackend"]),
    ],
    targets: [
        .target(name: "SqlBackend"),
        .testTarget(name: "SqlBackendTests", dependencies: ["SqlBackend"]),
    ]
)
