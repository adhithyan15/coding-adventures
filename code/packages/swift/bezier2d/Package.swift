// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "bezier2d",
    products: [
        .library(name: "Bezier2D", targets: ["Bezier2D"]),
    ],
    dependencies: [
        .package(path: "../trig"),
        .package(path: "../point2d"),
    ],
    targets: [
        .target(
            name: "Bezier2D",
            dependencies: [
                .product(name: "Trig", package: "trig"),
                .product(name: "Point2D", package: "point2d"),
            ]
        ),
        .testTarget(
            name: "Bezier2DTests",
            dependencies: ["Bezier2D"]
        ),
    ]
)
