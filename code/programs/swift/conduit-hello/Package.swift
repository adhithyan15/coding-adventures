// swift-tools-version: 6.0
// conduit-hello — a full Sinatra-style demo built on the Swift Conduit package.
import PackageDescription

let package = Package(
    name: "ConduitHello",
    dependencies: [
        .package(path: "../../../packages/swift/conduit"),
    ],
    targets: [
        // The Conduit library's own linker flags use a path relative to ITS
        // package root, which is wrong when a downstream package does the linking.
        // So we re-add the search path to libconduit_capi.a relative to THIS
        // package (the demo BUILD stages the lib there).
        .executableTarget(
            name: "ConduitHello",
            dependencies: [.product(name: "Conduit", package: "conduit")],
            path: "Sources/ConduitHello",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "../../../packages/swift/conduit/Sources/CConduit",
                    "-l", "conduit_capi",
                ])
            ]
        ),
        .testTarget(
            name: "ConduitHelloTests",
            dependencies: [
                "ConduitHello",
                .product(name: "Conduit", package: "conduit"),
            ],
            path: "Tests/ConduitHelloTests",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "../../../packages/swift/conduit/Sources/CConduit",
                    "-l", "conduit_capi",
                ])
            ]
        ),
    ]
)
