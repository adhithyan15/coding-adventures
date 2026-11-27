// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "HistoricalUnixFixtureAdapter",
    dependencies: [
        .package(path: "../../../../../packages/swift/unix-history-simulator"),
    ],
    products: [
        .executable(name: "fixture-cli", targets: ["FixtureCLI"]),
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
