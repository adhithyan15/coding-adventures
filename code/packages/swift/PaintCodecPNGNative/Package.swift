// swift-tools-version: 5.9
import Foundation
import PackageDescription

let packageDirectory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path

let package = Package(
    name: "PaintCodecPNGNative",
    products: [
        .library(name: "PaintCodecPNGNative", targets: ["PaintCodecPNGNative"]),
    ],
    dependencies: [
        .package(path: "../PixelContainer"),
    ],
    targets: [
        .systemLibrary(
            name: "CPaintCodecPNGNative",
            path: "Sources/CPaintCodecPNGNative"
        ),
        .target(
            name: "PaintCodecPNGNative",
            dependencies: [
                "PixelContainer",
                .target(
                    name: "CPaintCodecPNGNative",
                    condition: .when(platforms: [.macOS, .linux])
                ),
            ],
            path: "Sources/PaintCodecPNGNative",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "\(packageDirectory)/Sources/CPaintCodecPNGNative",
                    "-l", "paint_codec_png_c",
                ], .when(platforms: [.macOS, .linux])),
            ]
        ),
        .testTarget(
            name: "PaintCodecPNGNativeTests",
            dependencies: ["PaintCodecPNGNative", "PixelContainer"],
            path: "Tests/PaintCodecPNGNativeTests"
        ),
    ]
)
