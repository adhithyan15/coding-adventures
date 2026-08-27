// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "PaintVmAscii",
    products: [
        .library(name: "PaintVmAscii", targets: ["PaintVmAscii"]),
    ],
    dependencies: [
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(name: "PaintVmAscii", dependencies: ["PaintInstructions"], path: "Sources/PaintVmAscii"),
        .testTarget(
            name: "PaintVmAsciiTests",
            dependencies: ["PaintVmAscii", "PaintInstructions"],
            path: "Tests/PaintVmAsciiTests"
        ),
    ]
)
