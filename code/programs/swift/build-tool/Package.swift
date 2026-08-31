// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "build-tool",
    products: [
        .library(name: "BuildToolCore", targets: ["BuildToolCore"]),
        .executable(name: "build-tool", targets: ["build-tool"]),
    ],
    dependencies: [
        .package(path: "../../../packages/swift/sha256"),
    ],
    targets: [
        .target(
            name: "BuildToolCore",
            dependencies: [
                .product(name: "SHA256", package: "sha256"),
            ]
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
