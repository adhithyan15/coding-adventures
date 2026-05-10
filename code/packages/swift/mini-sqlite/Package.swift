// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "MiniSqlite",
    products: [
        .library(name: "MiniSqlite", targets: ["MiniSqlite"]),
    ],
    targets: [
        .target(name: "MiniSqlite"),
        .testTarget(name: "MiniSqliteTests", dependencies: ["MiniSqlite"]),
    ]
)
