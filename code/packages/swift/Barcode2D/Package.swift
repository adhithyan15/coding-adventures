// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Barcode2D",
    products: [.library(name: "Barcode2D", targets: ["Barcode2D"])],
    dependencies: [.package(path: "../PaintInstructions")],
    targets: [
        .target(name: "Barcode2D", dependencies: ["PaintInstructions"], path: "Sources/Barcode2D"),
        .testTarget(name: "Barcode2DTests", dependencies: ["Barcode2D", "PaintInstructions"], path: "Tests/Barcode2DTests"),
    ]
)
