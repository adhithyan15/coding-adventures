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
        .package(path: "../Codabar"),
        .package(path: "../Code128"),
        .package(path: "../Code39"),
        .package(path: "../EAN13"),
        .package(path: "../ITF"),
        .package(path: "../PaintInstructions"),
        .package(path: "../PaintVmMetalNative"),
        .package(path: "../PaintCodecPNGNative"),
        .package(path: "../PixelContainer"),
        .package(path: "../UPCA"),
    ],
    targets: [
        .target(
            name: "Barcode1D",
            dependencies: [
                "BarcodeLayout1D",
                "Codabar",
                "Code128",
                "Code39",
                "EAN13",
                "ITF",
                "PaintInstructions",
                "PaintVmMetalNative",
                "PaintCodecPNGNative",
                "PixelContainer",
                "UPCA",
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
