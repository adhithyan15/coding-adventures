// swift-tools-version: 5.9
//
// Package.swift — VisiCalc SwiftUI demo (VC2-swiftui), now computing on the
// shared Rust spreadsheet engine through its C ABI.
//
// Run with:
//   bash scripts/build.sh    # generates the views + builds & vendors the engine
//   swift run
//
// The engine is the `spreadsheet-capi` Rust crate built to a static library
// (libspreadsheet_capi.a) and vendored into Vendor/ by scripts/build.sh. The
// `CSpreadsheetEngine` target exposes its C header (spreadsheet.h) as a
// Swift-importable module; the executable links the static library.

import PackageDescription
import Foundation

// Absolute path to this package, derived from the manifest's own location, so
// the `-L` search path is correct regardless of the build's working directory.
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "VisiCalc",
    platforms: [
        .macOS(.v14),
        .iOS(.v17),
    ],
    products: [
        .executable(name: "visicalc", targets: ["VisiCalc"]),
    ],
    targets: [
        // C module exposing the engine's C ABI header to Swift.
        .target(name: "CSpreadsheetEngine"),
        .executableTarget(
            name: "VisiCalc",
            dependencies: ["CSpreadsheetEngine"],
            path: "Sources/VisiCalc",
            linkerSettings: [
                // Link the Rust static library vendored by scripts/build.sh,
                // per platform (the engine is cross-compiled for each target):
                //   macOS → Vendor/macos     (cargo --target <host>)
                //   iOS   → Vendor/ios-sim   (cargo --target aarch64-apple-ios-sim)
                .unsafeFlags(["-L\(packageDir)/Vendor/macos", "-lspreadsheet_capi"], .when(platforms: [.macOS])),
                .unsafeFlags(["-L\(packageDir)/Vendor/ios-sim", "-lspreadsheet_capi"], .when(platforms: [.iOS])),
            ]
        ),
        .testTarget(
            name: "VisiCalcTests",
            dependencies: ["VisiCalc"],
            path: "Tests/VisiCalcTests",
            linkerSettings: [
                // Tests run on the host (macOS).
                .unsafeFlags(["-L\(packageDir)/Vendor/macos", "-lspreadsheet_capi"], .when(platforms: [.macOS])),
            ]
        ),
    ]
)
