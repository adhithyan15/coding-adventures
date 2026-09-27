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
        .package(path: "../sha256"),
    ],
    targets: [
        .target(
            name: "ITF",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/ITF"
        ),
        .testTarget(
            name: "ITFTests",
            dependencies: [
                "ITF",
                "BarcodeLayout1D",
                "PaintInstructions",
                .product(name: "SHA256", package: "sha256"),
            ],
            path: "Tests/ITFTests"
        ),
    ]
)
