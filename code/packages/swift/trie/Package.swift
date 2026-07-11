// swift-tools-version: 6.0
// ============================================================================
// Package.swift — Trie
// ============================================================================
//
// Swift Package Manager manifest for the Trie library: a prefix tree for string
// keys with insert/search/delete and prefix operations (words-with-prefix,
// longest-prefix-match). Part of the coding-adventures stack.
// ============================================================================

import PackageDescription

let package = Package(
    name: "Trie",
    products: [
        .library(name: "Trie", targets: ["Trie"]),
    ],
    targets: [
        .target(name: "Trie"),
        .testTarget(name: "TrieTests", dependencies: ["Trie"]),
    ]
)
