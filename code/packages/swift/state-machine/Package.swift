// swift-tools-version: 6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.
//
// === State Machine Package ===
//
// This package provides two fundamental automata types from the theory of
// computation: Deterministic Finite Automata (DFA) and Non-deterministic
// Finite Automata (NFA). These are the building blocks underlying regular
// expressions, lexical analyzers, protocol validators, and hardware
// state machines.
//
// No external dependencies — this is a pure-computation library that uses
// only the Swift standard library and XCTest for testing.

import PackageDescription

let package = Package(
    name: "StateMachine",
    products: [
        .library(
            name: "StateMachine",
            targets: ["StateMachine"]
        ),
    ],
    targets: [
        .target(
            name: "StateMachine"
        ),
        .testTarget(
            name: "StateMachineTests",
            dependencies: ["StateMachine"]
        ),
    ]
)
