// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "BarcodeLayout1D",
    products: [
        .library(name: "BarcodeLayout1D", targets: ["BarcodeLayout1D"]),
    ],
    dependencies: [
        .package(path: "../PaintInstructions"),
        .package(path: "../sha256"),
    ],
    targets: [
        .target(
            name: "BarcodeLayout1D",
            dependencies: ["PaintInstructions"],
            path: "Sources/BarcodeLayout1D"
        ),
        .testTarget(
            name: "BarcodeLayout1DTests",
            dependencies: [
                "BarcodeLayout1D",
                "PaintInstructions",
                .product(name: "SHA256", package: "sha256"),
            ],
            path: "Tests/BarcodeLayout1DTests"
        ),
    ]
)
