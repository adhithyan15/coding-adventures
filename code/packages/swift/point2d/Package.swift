// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "point2d",
    products: [
        .library(name: "Point2D", targets: ["Point2D"]),
    ],
    dependencies: [
        .package(path: "../trig"),
    ],
    targets: [
        .target(
            name: "Point2D",
            dependencies: [
                .product(name: "Trig", package: "trig"),
            ]
        ),
        .testTarget(
            name: "Point2DTests",
            dependencies: ["Point2D"]
        ),
    ]
)
