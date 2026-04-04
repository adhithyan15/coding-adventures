// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Parser
// ============================================================================
//
// Swift Package Manager manifest for the Parser library.
//
// This package provides a grammar-driven parser that reads grammar rules
// from a ParserGrammar (parsed from a .grammar file by grammar-tools) and
// interprets them at runtime. The same Swift code can parse Python, Ruby,
// or any language -- just swap the .grammar file.
//
// Features:
// - Packrat memoization to avoid exponential backtracking
// - Warth left-recursion support for left-recursive grammars
// - Furthest-failure error reporting for clear error messages
// - Pre/post parse hooks for token and AST transforms
// - Support for all EBNF elements plus extensions (lookahead, one-or-more,
//   separated repetition)
//
// Dependencies:
// - Lexer: provides the Token type.
// - GrammarTools: provides ParserGrammar, GrammarElement, GrammarRule types.
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "Parser",
    products: [
        .library(name: "Parser", targets: ["Parser"]),
    ],
    dependencies: [
        .package(path: "../lexer"),
        .package(path: "../grammar-tools"),
    ],
    targets: [
        .target(
            name: "Parser",
            dependencies: [
                .product(name: "Lexer", package: "lexer"),
                .product(name: "GrammarTools", package: "grammar-tools"),
            ]
        ),
        .testTarget(
            name: "ParserTests",
            dependencies: ["Parser"]
        ),
    ]
)
