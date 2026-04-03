// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "affine2d",
    products: [
        .library(name: "Affine2D", targets: ["Affine2D"]),
    ],
    dependencies: [
        .package(path: "../trig"),
        .package(path: "../point2d"),
    ],
    targets: [
        .target(
            name: "Affine2D",
            dependencies: [
                .product(name: "Trig", package: "trig"),
                .product(name: "Point2D", package: "point2d"),
            ]
        ),
        .testTarget(
            name: "Affine2DTests",
            dependencies: ["Affine2D"]
        ),
    ]
)
