// swift-tools-version: 5.9
// ============================================================================
// Package.swift — AsciidocParser: AsciiDoc Block + Inline Parser
// ============================================================================
//
// Parses AsciiDoc source text into the format-agnostic Document AST.
//
// # Architecture
//
//   AsciiDoc text
//        │
//        ▼  BlockParser.parseBlocks(_:) — Phase 1
//   [IntermediateBlock]  (headings, paragraphs, code blocks, lists, …)
//        │
//        ▼  InlineParser.parse(_:) — Phase 2
//   BlockNode.document(DocumentNode(...))
//
// # Dependency Chain
//
//   document-ast       (Layer 0)
//   asciidoc-parser    (Layer 1) — depends on document-ast
//
import PackageDescription

let package = Package(
    name: "AsciidocParser",
    products: [
        .library(name: "AsciidocParser", targets: ["AsciidocParser"]),
    ],
    dependencies: [
        .package(path: "../document-ast"),
    ],
    targets: [
        .target(
            name: "AsciidocParser",
            dependencies: [
                .product(name: "DocumentAst", package: "document-ast"),
            ]
        ),
        .testTarget(
            name: "AsciidocParserTests",
            dependencies: [
                "AsciidocParser",
                .product(name: "DocumentAst", package: "document-ast"),
            ]
        ),
    ]
)
