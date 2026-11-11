# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Security

- Memory safety / DoS: decode indexed table[idx] for a dangling dictIndex (trap) and expanded hostile token chains quadratically; it now stops on a dangling index and once the declared length is reached. deserialiseTokens iterates only over the tokens actually present. NOT RUN locally (toolchain unavailable in the review environment).
- Added Tests/*/HostileHeadersTests.swift (XCTest): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (toolchain unavailable in the review environment).

