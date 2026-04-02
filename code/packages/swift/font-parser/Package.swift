// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "font-parser",
    products: [
        .library(name: "FontParser", targets: ["FontParser"]),
    ],
    targets: [
        .target(name: "FontParser"),
        .testTarget(
            name: "FontParserTests",
            dependencies: ["FontParser"]
            // Test fixture is resolved at runtime using #filePath,
            // so no SPM resource bundle is needed.
        ),
    ]
)
