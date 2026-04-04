// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Commonmark: CommonMark → HTML Pipeline
// ============================================================================
//
// A thin convenience wrapper that chains `CommonmarkParser` and
// `DocumentAstToHtml` into a single `toHtml(_:)` function.
//
// # Architecture
//
//   CommonMark text
//        │
//        ▼
//   CommonmarkParser.parse(_:)   → BlockNode.document(...)
//        │
//        ▼
//   DocumentAstToHtml.render(_:) → HTML string
//
// # Dependency Chain
//
//   document-ast       (Layer 0)
//   commonmark-parser  (Layer 1) ─┐
//   document-ast-to-html (Layer 1) ─┤
//       └── commonmark (Layer 2) ←─┘
//
import PackageDescription

let package = Package(
    name: "Commonmark",
    products: [
        .library(name: "Commonmark", targets: ["Commonmark"]),
    ],
    dependencies: [
        .package(path: "../document-ast"),
        .package(path: "../commonmark-parser"),
        .package(path: "../document-ast-to-html"),
    ],
    targets: [
        .target(
            name: "Commonmark",
            dependencies: [
                .product(name: "DocumentAst", package: "document-ast"),
                .product(name: "CommonmarkParser", package: "commonmark-parser"),
                .product(name: "DocumentAstToHtml", package: "document-ast-to-html"),
            ]
        ),
        .testTarget(
            name: "CommonmarkTests",
            dependencies: ["Commonmark"]
        ),
    ]
)
