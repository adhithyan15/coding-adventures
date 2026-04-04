// swift-tools-version: 5.9
// ============================================================================
// Package.swift — CommonmarkParser: Markdown → Document AST Parser
// ============================================================================
//
// This package implements a CommonMark 0.31.2 subset parser. It converts
// Markdown text into a Document AST (from the `document-ast` package).
//
// # Two-Phase Architecture
//
// Phase 1 — BlockParser:
//   Line-by-line structural parsing. Identifies headings, code blocks,
//   blockquotes, lists, thematic breaks, and paragraphs. Stores raw inline
//   content as strings for phase 2.
//
// Phase 2 — InlineParser:
//   Transforms raw inline strings into InlineNode trees. Handles emphasis,
//   strong, code spans, links, images, autolinks, and hard/soft breaks.
//
// # Dependency Chain
//
//   document-ast (Layer 0) ← commonmark-parser (Layer 1)
//
import PackageDescription

let package = Package(
    name: "CommonmarkParser",
    products: [
        .library(name: "CommonmarkParser", targets: ["CommonmarkParser"]),
    ],
    dependencies: [
        .package(path: "../document-ast"),
    ],
    targets: [
        .target(
            name: "CommonmarkParser",
            dependencies: [
                .product(name: "DocumentAst", package: "document-ast"),
            ]
        ),
        .testTarget(
            name: "CommonmarkParserTests",
            dependencies: [
                "CommonmarkParser",
                .product(name: "DocumentAst", package: "document-ast"),
            ]
        ),
    ]
)
