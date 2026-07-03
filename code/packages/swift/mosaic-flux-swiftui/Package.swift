// swift-tools-version: 5.9
//
// Package manifest for @coding-adventures/mosaic-flux-swiftui.
//
// Per UI33-rewrite §6, this is the Apple-platform runtime library
// that the Mosaic UI SwiftUI emitter targets.  It implements the
// MosaicAction Command Pattern, MosaicStore with fine-grained
// subscriptions, middleware, selectors, and a DevTools-protocol
// middleware stub.
//
// Platform versions are pinned to the latest stable Apple OS major
// releases that ship modern Swift features (primary associated
// types, the Observation framework, etc.).  Downstream Mosaic apps
// targeting older OS versions can lower these bounds; we ship the
// upper-bound defaults because v0.1.0 is greenfield code.

import PackageDescription

let package = Package(
    name: "MosaicFlux",
    platforms: [
        .macOS(.v14),
        .iOS(.v17),
        .watchOS(.v10),
        .tvOS(.v17),
        .visionOS(.v1),
    ],
    products: [
        .library(name: "MosaicFlux", targets: ["MosaicFlux"]),
    ],
    targets: [
        .target(
            name: "MosaicFlux",
            path: "Sources/MosaicFlux"
        ),
        .testTarget(
            name: "MosaicFluxTests",
            dependencies: ["MosaicFlux"],
            path: "Tests/MosaicFluxTests"
        ),
    ]
)
