// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Code128",
    products: [
        .library(name: "Code128", targets: ["Code128"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "Code128",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/Code128"
        ),
        .testTarget(
            name: "Code128Tests",
            dependencies: ["Code128", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/Code128Tests"
        ),
    ]
)
