// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Parrot",
    dependencies: [
        .package(path: "../../../packages/swift/repl"),
    ],
    targets: [
        .executableTarget(
            name: "Parrot",
            dependencies: [
                .product(name: "CodingAdventuresRepl", package: "repl"),
            ],
            path: "Sources/Parrot"
        ),
        .testTarget(
            name: "ParrotTests",
            dependencies: [
                "Parrot",
                .product(name: "CodingAdventuresRepl", package: "repl"),
            ],
            path: "Tests/ParrotTests"
        ),
    ]
)
