// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "UPCA",
    products: [
        .library(name: "UPCA", targets: ["UPCA"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../PaintInstructions"),
    ],
    targets: [
        .target(
            name: "UPCA",
            dependencies: ["BarcodeLayout1D", "PaintInstructions"],
            path: "Sources/UPCA"
        ),
        .testTarget(
            name: "UPCATests",
            dependencies: ["UPCA", "BarcodeLayout1D", "PaintInstructions"],
            path: "Tests/UPCATests"
        ),
    ]
)
