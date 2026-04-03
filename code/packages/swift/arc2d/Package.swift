// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "arc2d",
    products: [
        .library(name: "Arc2D", targets: ["Arc2D"]),
    ],
    dependencies: [
        .package(path: "../trig"),
        .package(path: "../point2d"),
        .package(path: "../bezier2d"),
    ],
    targets: [
        .target(
            name: "Arc2D",
            dependencies: [
                .product(name: "Trig", package: "trig"),
                .product(name: "Point2D", package: "point2d"),
                .product(name: "Bezier2D", package: "bezier2d"),
            ]
        ),
        .testTarget(
            name: "Arc2DTests",
            dependencies: ["Arc2D"]
        ),
    ]
)
