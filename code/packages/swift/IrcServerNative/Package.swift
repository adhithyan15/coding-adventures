// swift-tools-version: 6.0
// ============================================================================
// Package.swift — IrcServerNative (Swift)
// ============================================================================
//
// A high-performance IRC server for Swift. Every line of IRC and TCP logic runs
// in Rust (the `irc-net-reactor` engine on the home-grown kqueue/epoll reactor);
// Swift only launches and controls the server. It binds the engine through the
// reusable `irc-server-capi` C ABI, linked at compile time via Swift's native C
// interop — no third-party FFI library.
//
// ARCHITECTURE
// ─────────────
//   rust/irc-server-capi/          ← Rust crate → libirc_server_capi.a (the C ABI)
//       include/irc_server_capi.h  ← the stable C header
//
//   swift/IrcServerNative/
//       Sources/CIrcServer/            ← SPM systemLibrary (header + module map)
//           include/irc_server_capi.h
//           module.modulemap
//           libirc_server_capi.a        ← staged here by BUILD (gitignored)
//       Sources/IrcServerNative/       ← the Swift facade (IrcServer)
//       Tests/IrcServerNativeTests/
//
// BUILD (before `swift test`)
// ───────────────────────────
//   cd code/packages/rust/irc-server-capi && cargo build --release
//   cp ../target/release/libirc_server_capi.a \
//      ../../swift/IrcServerNative/Sources/CIrcServer/
//   cd ../../swift/IrcServerNative && swift build && swift test
//
// The `BUILD` script automates this.

import PackageDescription

let package = Package(
    name: "IrcServerNative",
    products: [
        .library(name: "IrcServerNative", targets: ["IrcServerNative"]),
    ],
    targets: [
        // The C target: gives SPM a `CIrcServer` module from the header. No Swift
        // source — just the module map + header.
        .systemLibrary(
            name: "CIrcServer",
            path: "Sources/CIrcServer"
        ),

        // The Swift library. Links libirc_server_capi.a (staged into the
        // CIrcServer dir by BUILD). The Rust static lib bundles irc-net-reactor /
        // tcp-runtime and the Rust std; on macOS/Linux those resolve against the
        // default system libs.
        .target(
            name: "IrcServerNative",
            dependencies: ["CIrcServer"],
            linkerSettings: [
                .unsafeFlags([
                    "-L", "Sources/CIrcServer",
                    "-l", "irc_server_capi",
                ])
            ]
        ),

        // NOTE: tests require libirc_server_capi.a staged in Sources/CIrcServer/.
        .testTarget(
            name: "IrcServerNativeTests",
            dependencies: ["IrcServerNative"]
        ),
    ]
)
