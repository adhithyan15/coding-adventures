// swift-tools-version: 5.9
// cowsay — routed through PaintVmAscii (Swift port). See
// code/specs/cowsay-paintvm-pipeline.md for the full design rationale.
import PackageDescription

let package = Package(
    name: "Cowsay",
    products: [
        .executable(name: "Cowsay", targets: ["Cowsay"]),
    ],
    dependencies: [
        .package(path: "../../../packages/swift/cli-builder"),
        .package(path: "../../../packages/swift/PaintInstructions"),
        .package(path: "../../../packages/swift/PaintVmAscii"),
    ],
    targets: [
        .executableTarget(
            name: "Cowsay",
            dependencies: [
                .product(name: "CliBuilder", package: "cli-builder"),
                "PaintInstructions",
                "PaintVmAscii",
            ],
            path: "Sources/Cowsay"
        ),
        .testTarget(
            name: "CowsayTests",
            dependencies: ["Cowsay", "PaintInstructions", "PaintVmAscii"],
            path: "Tests/CowsayTests"
        ),
    ]
)
