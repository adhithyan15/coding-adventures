// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ITF",
    products: [
        .library(name: "ITF", targets: ["ITF"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "ITF",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/ITF"
        ),
        .testTarget(
            name: "ITFTests",
            dependencies: ["ITF", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/ITFTests"
        ),
    ]
)
