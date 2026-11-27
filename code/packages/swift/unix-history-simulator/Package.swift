// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "UnixHistorySimulator",
    products: [
        .library(name: "UnixHistorySimulator", targets: ["UnixHistorySimulator"]),
    ],
    targets: [
        .target(name: "UnixHistorySimulator"),
        .testTarget(name: "UnixHistorySimulatorTests", dependencies: ["UnixHistorySimulator"]),
    ]
)
