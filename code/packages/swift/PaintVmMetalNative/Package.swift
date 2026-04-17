// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDirectory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "PaintVmMetalNative",
    products: [
        .library(name: "PaintVmMetalNative", targets: ["PaintVmMetalNative"]),
    ],
    dependencies: [
        .package(path: "../PaintInstructions"),
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .systemLibrary(
            name: "CPaintVmMetalNative",
            path: "Sources/CPaintVmMetalNative"
        ),
        .target(
            name: "PaintVmMetalNative",
            dependencies: [
                "PaintInstructions",
                "PixelContainer",
                .target(
                    name: "CPaintVmMetalNative",
                    condition: .when(platforms: [.macOS])
                ),
            ],
            path: "Sources/PaintVmMetalNative",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "\(packageDirectory)/Sources/CPaintVmMetalNative",
                    "-l", "paint_vm_metal_c",
                ], .when(platforms: [.macOS])),
                .linkedFramework("Metal", .when(platforms: [.macOS])),
                .linkedFramework("CoreGraphics", .when(platforms: [.macOS])),
                .linkedFramework("CoreText", .when(platforms: [.macOS])),
                .linkedFramework("CoreFoundation", .when(platforms: [.macOS])),
                .linkedFramework("AppKit", .when(platforms: [.macOS])),
                .linkedLibrary("objc", .when(platforms: [.macOS])),
            ]
        ),
        .testTarget(
            name: "PaintVmMetalNativeTests",
            dependencies: ["PaintVmMetalNative", "PaintInstructions", "PixelContainer"],
            path: "Tests/PaintVmMetalNativeTests"
        ),
    ]
)
