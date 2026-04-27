// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "feature-normalization",
    products: [
        .library(name: "FeatureNormalization", targets: ["FeatureNormalization"]),
    ],
    targets: [
        .target(name: "FeatureNormalization"),
        .testTarget(
            name: "FeatureNormalizationTests",
            dependencies: ["FeatureNormalization"]
        ),
    ]
)
