// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Codabar",
    products: [
        .library(name: "Codabar", targets: ["Codabar"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "Codabar",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/Codabar"
        ),
        .testTarget(
            name: "CodabarTests",
            dependencies: ["Codabar", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/CodabarTests"
        ),
    ]
)
