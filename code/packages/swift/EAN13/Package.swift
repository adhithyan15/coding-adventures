// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "EAN13",
    products: [
        .library(name: "EAN13", targets: ["EAN13"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "EAN13",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/EAN13"
        ),
        .testTarget(
            name: "EAN13Tests",
            dependencies: ["EAN13", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/EAN13Tests"
        ),
    ]
)
