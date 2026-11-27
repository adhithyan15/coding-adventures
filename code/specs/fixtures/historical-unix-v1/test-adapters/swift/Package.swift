// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "HistoricalUnixFixtureAdapter",
    products: [
        .executable(name: "fixture-cli", targets: ["FixtureCLI"]),
    ],
    dependencies: [
        .package(path: "../../../../../packages/swift/unix-history-simulator"),
    ],
    targets: [
        .executableTarget(
            name: "FixtureCLI",
            dependencies: [
                .product(name: "UnixHistorySimulator", package: "unix-history-simulator"),
            ]
        ),
    ]
)
