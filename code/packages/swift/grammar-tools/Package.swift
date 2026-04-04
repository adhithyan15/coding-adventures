// swift-tools-version: 6.0
// ============================================================================
// Package.swift — GrammarTools
// ============================================================================
//
// Swift Package Manager manifest for the GrammarTools library.
//
// This package provides parsers and validators for two kinds of grammar files:
//
// 1. **.tokens files** — Declarative descriptions of a language's lexical
//    grammar. Each line defines a token pattern (regex or literal string)
//    that the lexer should recognize. The parser produces a `TokenGrammar`
//    struct that downstream tools (lexer generators, cross-validators)
//    consume.
//
// 2. **.grammar files** — EBNF descriptions of a language's syntactic
//    structure. Each rule defines how tokens combine into larger constructs
//    (expressions, statements, programs). The parser produces a
//    `ParserGrammar` struct that grammar-driven parsers consume.
//
// Together, these two files fully describe a programming language's surface
// syntax — the .tokens file says "these are the words" and the .grammar
// file says "these are the sentences."
//
// Part of the coding-adventures educational computing stack.
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "GrammarTools",
    products: [
        .library(name: "GrammarTools", targets: ["GrammarTools"]),
    ],
    targets: [
        .target(name: "GrammarTools"),
        .testTarget(name: "GrammarToolsTests", dependencies: ["GrammarTools"]),
    ]
)
