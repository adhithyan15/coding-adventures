// swift-tools-version: 5.9
// ============================================================================
// Package.swift — Document AST: Format-Agnostic Document IR
// ============================================================================
//
// This package defines the Document AST — the "LLVM IR of documents". It is
// the shared intermediate representation (IR) between all document front-end
// parsers (Markdown, RST, HTML) and all back-end renderers (HTML, PDF,
// plain text, LaTeX).
//
// With a shared IR, N parsers × M renderers requires only N + M implementations
// instead of N × M:
//
//   Markdown ─────────────────────────────────► HTML
//   reStructuredText ──► Document AST (IR) ──► PDF
//   HTML input ───────────────────────────────► Plain text
//   DOCX ─────────────────────────────────────► LaTeX
//
// # Dependency Chain
//
//   document-ast (Layer 0, types only)
//       ▲
//       └── document-ast-to-html, commonmark-parser, gfm-parser, ...
//
import PackageDescription

let package = Package(
    name: "DocumentAst",
    products: [
        .library(name: "DocumentAst", targets: ["DocumentAst"]),
    ],
    targets: [
        .target(name: "DocumentAst"),
        .testTarget(
            name: "DocumentAstTests",
            dependencies: ["DocumentAst"]
        ),
    ]
)
