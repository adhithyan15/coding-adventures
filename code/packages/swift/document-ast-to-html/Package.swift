// swift-tools-version: 5.9
// ============================================================================
// Package.swift — DocumentAstToHtml: Document AST → HTML Renderer
// ============================================================================
//
// This package converts a Document AST node tree to an HTML string.
// It is the standard CommonMark HTML back-end in this project.
//
// # Dependency Chain
//
//   document-ast (Layer 0) ← document-ast-to-html (Layer 1)
//
// The renderer depends only on the IR type definitions from `document-ast`.
// It has no knowledge of Markdown syntax — that's the parser's job.
//
import PackageDescription

let package = Package(
    name: "DocumentAstToHtml",
    products: [
        .library(name: "DocumentAstToHtml", targets: ["DocumentAstToHtml"]),
    ],
    dependencies: [
        .package(path: "../document-ast"),
    ],
    targets: [
        .target(
            name: "DocumentAstToHtml",
            dependencies: [
                .product(name: "DocumentAst", package: "document-ast"),
            ]
        ),
        .testTarget(
            name: "DocumentAstToHtmlTests",
            dependencies: ["DocumentAstToHtml"]
        ),
    ]
)
