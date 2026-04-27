// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "PaintInstructions",
    products: [
        .library(name: "PaintInstructions", targets: ["PaintInstructions"]),
    ],
    targets: [
        .target(
            name: "PaintInstructions",
            path: "Sources/PaintInstructions"
        ),
        .testTarget(
            name: "PaintInstructionsTests",
            dependencies: ["PaintInstructions"],
            path: "Tests/PaintInstructionsTests"
        ),
    ]
)
