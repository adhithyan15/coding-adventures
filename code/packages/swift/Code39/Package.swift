// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Code39",
    products: [
        .library(name: "Code39", targets: ["Code39"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "Code39",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/Code39"
        ),
        .testTarget(
            name: "Code39Tests",
            dependencies: ["Code39", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/Code39Tests"
        ),
    ]
)
