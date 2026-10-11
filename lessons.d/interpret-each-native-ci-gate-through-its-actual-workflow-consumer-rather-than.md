---
category: CI & GitHub Actions
---

# Interpret each native CI gate through its actual workflow consumer rather than requiring all OS-suite flags

A local receipt incorrectly asserted that Linux, macOS and Windows OS-suite flags must all be true for Closure native test execution. The Linux flag selects Forme cgroup tests and the macOS flag selects separate OS-only suites; ordinary affected-package tests run on both regardless. Windows needs its explicit OS-suite flag to enter general package testing. Read ci.yml dispatch and step conditions with the production plan: require actual compiler selection on all platforms, Rust toolchain and the Windows general-test gate, then verify actual CI commands/results. Do not broaden unrelated gates to satisfy a mistaken proxy assertion.
