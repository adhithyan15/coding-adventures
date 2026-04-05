// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "build-tool",
    products: [
        .library(name: "BuildToolCore", targets: ["BuildToolCore"]),
        .executable(name: "build-tool", targets: ["build-tool"]),
    ],
    targets: [
        .target(
            name: "BuildToolCore"
        ),
        .executableTarget(
            name: "build-tool",
            dependencies: ["BuildToolCore"]
        ),
        .testTarget(
            name: "BuildToolCoreTests",
            dependencies: ["BuildToolCore"],
            path: "Tests/BuildToolCoreTests"
        ),
    ]
)
