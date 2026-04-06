// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "CodingAdventuresRepl",
    products: [
        .library(name: "CodingAdventuresRepl", targets: ["CodingAdventuresRepl"]),
    ],
    targets: [
        .target(
            name: "CodingAdventuresRepl",
            path: "Sources/CodingAdventuresRepl"
        ),
        .testTarget(
            name: "CodingAdventuresReplTests",
            dependencies: ["CodingAdventuresRepl"],
            path: "Tests/CodingAdventuresReplTests"
        ),
    ]
)
