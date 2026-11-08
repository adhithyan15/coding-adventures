# @coding-adventures/test-bootstrap

Zero-dependency TypeScript/JavaScript Level 0 bootstrap harness, TFV v1 parser, and Level 1 unit test matchers (TF01 / TF01a).

## Principles

1. **Zero External Runtime Dependencies:** Relies purely on Node.js built-ins (`node:test`, `node:assert`, standard library). No `vitest`, no `jest`, no transitive dependency tree.
2. **Canonical TF01 Conformance:** Pinned against the `test-bootstrap-v1` cross-language self-corpus (1,256 cases).
3. **Pure Deep Equality:** Spec-compliant float bit pattern comparisons, arbitrary-precision integers up to 1000 digits, byte arrays, strings, lists, and maps.
4. **Deterministic Run Protocol:** Seeded SplitMix64 pseudo-random ordering with isolated sub-seeds and TAP version 14 output.
5. **Drop-in Matchers:** Familiar `describe`, `it`, and `expect` assertions to retire third-party test runners cleanly.
6. **Declarative CLI Runner:** Built with internal `@coding-adventures/cli-builder` supporting `--seed`, `--sabotage`, `--reporter`, `--filter`, and `--bail`.
