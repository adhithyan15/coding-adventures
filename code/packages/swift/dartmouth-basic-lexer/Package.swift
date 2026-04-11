// swift-tools-version: 6.0
// ============================================================================
// Package.swift -- DartmouthBasicLexer
// ============================================================================
//
// Dartmouth BASIC (1964) lexer for Swift. A thin wrapper around the
// grammar-driven GrammarLexer from the Lexer package, configured by
// dartmouth_basic.tokens.
//
// Dartmouth BASIC was designed by John Kemeny and Thomas Kurtz in 1964 at
// Dartmouth College to give non-science students access to computing. It ran
// on a GE-225 mainframe connected to teletype terminals. Its key features:
//
//   - Line-numbered: every statement has an integer line number (10, 20, ...)
//   - Case-insensitive: teletypes printed only uppercase; modern parsers
//     normalise input to uppercase before matching
//   - Pre-initialised: all variables start at 0; no declaration needed
//   - Simple: only 17 statement types; easily learned in an afternoon
//
// The lexer adds two post-tokenization passes specific to BASIC's structure:
//
//   1. relabelLineNumbers  — Promotes the first NUMBER on each line to LINE_NUM
//   2. suppressRemContent  — Removes tokens between REM and NEWLINE (comments)
//
// ============================================================================

import PackageDescription

let package = Package(
    name: "DartmouthBasicLexer",
    products: [
        .library(
            name: "DartmouthBasicLexer",
            targets: ["DartmouthBasicLexer"]
        ),
    ],
    dependencies: [
        .package(path: "../grammar-tools"),
        .package(path: "../lexer"),
    ],
    targets: [
        .target(
            name: "DartmouthBasicLexer",
            dependencies: [
                .product(name: "GrammarTools", package: "grammar-tools"),
                .product(name: "Lexer", package: "lexer"),
            ]
        ),
        .testTarget(
            name: "DartmouthBasicLexerTests",
            dependencies: ["DartmouthBasicLexer"]
        ),
    ]
)
