// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Asciidoc: AsciiDoc → HTML Convenience Pipeline
// ============================================================================
//
// A thin convenience wrapper that chains `AsciidocParser` and
// `DocumentAstToHtml` into a single `toHtml(_:)` function.
//
// # Architecture
//
//   AsciiDoc text
//        │
//        ▼
//   AsciidocParser.parse(_:)      → BlockNode.document(...)
//        │
//        ▼
//   DocumentAstToHtml.render(_:)  → HTML string
//
// # Dependency Chain
//
//   document-ast           (Layer 0)
//   asciidoc-parser        (Layer 1) ─┐
//   document-ast-to-html   (Layer 1) ─┤
//       └── asciidoc       (Layer 2) ←─┘
//
import PackageDescription

let package = Package(
    name: "Asciidoc",
    products: [
        .library(name: "Asciidoc", targets: ["Asciidoc"]),
    ],
    dependencies: [
        .package(path: "../document-ast"),
        .package(path: "../asciidoc-parser"),
        .package(path: "../document-ast-to-html"),
    ],
    targets: [
        .target(
            name: "Asciidoc",
            dependencies: [
                .product(name: "DocumentAst", package: "document-ast"),
                .product(name: "AsciidocParser", package: "asciidoc-parser"),
                .product(name: "DocumentAstToHtml", package: "document-ast-to-html"),
            ]
        ),
        .testTarget(
            name: "AsciidocTests",
            dependencies: ["Asciidoc"]
        ),
    ]
)
