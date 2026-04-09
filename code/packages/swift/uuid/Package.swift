// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "UUID",
    products: [
        .library(name: "UUID", targets: ["UUID"]),
    ],
    dependencies: [
        .package(path: "../md5"),
        .package(path: "../sha1"),
    ],
    targets: [
        .target(
            name: "UUID",
            dependencies: [
                .product(name: "MD5", package: "md5"),
                .product(name: "SHA1", package: "sha1"),
            ]
        ),
        .testTarget(
            name: "UUIDTests",
            dependencies: ["UUID"]
        ),
    ]
)
