// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CSVParser",
    products: [
        .library(name: "CSVParser", targets: ["CSVParser"]),
    ],
    targets: [
        .target(name: "CSVParser"),
        .testTarget(name: "CSVParserTests", dependencies: ["CSVParser"]),
    ]
)
