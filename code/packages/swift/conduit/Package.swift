// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Conduit (Swift)
// ============================================================================
//
// A Sinatra/Express-style web framework for Swift. It hosts the Rust web-core
// HTTP engine (WEB08 facade) through the reusable `conduit-capi` C ABI, linked
// at compile time via Swift's native C interop — no third-party FFI library.
//
// ARCHITECTURE
// ─────────────
//   rust/conduit-capi/         ← Rust crate → libconduit_capi.a (the C ABI)
//       include/conduit_capi.h ← the stable C header
//
//   swift/conduit/
//       Sources/CConduit/          ← SPM systemLibrary (header + module map)
//           include/conduit_capi.h
//           module.modulemap
//           libconduit_capi.a       ← staged here by BUILD (gitignored)
//       Sources/Conduit/           ← the Swift DSL
//       Tests/ConduitTests/
//
// BUILD (before `swift test`)
// ───────────────────────────
//   cd code/packages/rust/conduit-capi && cargo build --release
//   cp ../target/release/libconduit_capi.a ../../swift/conduit/Sources/CConduit/
//   cd ../../swift/conduit && swift build && swift test
//
// The `BUILD` script automates this.

import PackageDescription

let package = Package(
    name: "Conduit",
    products: [
        .library(name: "Conduit", targets: ["Conduit"]),
    ],
    targets: [
        // The C target: gives SPM a `CConduit` module from the header. No Swift
        // source — just the module map + header.
        .systemLibrary(
            name: "CConduit",
            path: "Sources/CConduit"
        ),

        // The Swift library. Links libconduit_capi.a (staged into the CConduit
        // dir by BUILD). The Rust static lib bundles web-core/tcp-runtime and the
        // Rust std; on macOS/Linux those resolve against the default system libs.
        .target(
            name: "Conduit",
            dependencies: ["CConduit"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "Sources/CConduit",
                    "-l", "conduit_capi",
                ])
            ]
        ),

        // NOTE: tests require libconduit_capi.a staged in Sources/CConduit/.
        .testTarget(
            name: "ConduitTests",
            dependencies: ["Conduit"]
        ),
    ]
)
