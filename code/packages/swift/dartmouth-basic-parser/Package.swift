// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- DartmouthBasicParser
// ============================================================================
//
// Dartmouth BASIC (1964) parser for Swift. A grammar-driven parser built on
// the GrammarParser engine from the Parser package, parsing the token stream
// produced by DartmouthBasicLexer against dartmouth_basic.grammar.
//
// The parser takes a sequence of Tokens (from the lexer) and produces a
// generic ASTNode tree. Each node's `ruleName` field names the grammar rule
// that was matched — `program`, `let_stmt`, `expr`, etc.
//
// Dependency chain (leaf → root):
//
//   grammar-tools                (parses .grammar and .tokens files)
//   lexer                        (GrammarLexer engine)
//   parser                       (GrammarParser engine + ASTNode)
//   dartmouth-basic-lexer        (tokenizes BASIC source)
//   dartmouth-basic-parser  ←    (this package)
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "DartmouthBasicParser",
    products: [
        .library(
            name: "DartmouthBasicParser",
            targets: ["DartmouthBasicParser"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
        .package(path: "../parser"),
        .package(path: "../dartmouth-basic-lexer"),
    ],
    targets: [
        .target(
            name: "DartmouthBasicParser",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
                .product(name: "Parser", package: "parser"),
                .product(name: "DartmouthBasicLexer", package: "dartmouth-basic-lexer"),
            ]
        ),
        .testTarget(
            name: "DartmouthBasicParserTests",
            dependencies: [
                "DartmouthBasicParser",
                .product(name: "Lexer", package: "lexer"),
                .product(name: "Parser", package: "parser"),
            ]
        ),
    ]
)
