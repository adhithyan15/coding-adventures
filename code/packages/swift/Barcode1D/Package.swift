// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Barcode1D",
    products: [
        .library(name: "Barcode1D", targets: ["Barcode1D"]),
        .executable(name: "Barcode1DExample", targets: ["Barcode1DExample"]),
    ],
    dependencies: [
        .package(path: "../BarcodeLayout1D"),
        .package(path: "../Code39"),
        .package(path: "../PaintInstructions"),
        .package(path: "../PaintVmMetalNative"),
        .package(path: "../PaintCodecPNGNative"),
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .target(
            name: "Barcode1D",
            dependencies: [
                "BarcodeLayout1D",
                "Code39",
                "PaintInstructions",
                "PaintVmMetalNative",
                "PaintCodecPNGNative",
                "PixelContainer",
            ],
            path: "Sources/Barcode1D"
        ),
        .executableTarget(
            name: "Barcode1DExample",
            dependencies: ["Barcode1D", "PaintVmMetalNative", "PaintCodecPNGNative"],
            path: "Sources/Barcode1DExample"
        ),
        .testTarget(
            name: "Barcode1DTests",
            dependencies: ["Barcode1D", "PaintVmMetalNative", "PaintCodecPNGNative"],
            path: "Tests/Barcode1DTests"
        ),
    ]
)
