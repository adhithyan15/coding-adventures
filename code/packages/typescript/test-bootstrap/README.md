# @coding-adventures/test-bootstrap

Zero-dependency TypeScript/JavaScript Level 0 bootstrap harness, TFV v1 parser, and Level 1 unit test matchers (TF01 / TF01a).

## Principles

1. **Zero External Runtime Dependencies:** Relies purely on Node.js built-ins (`node:test`, `node:assert`, standard library). No `vitest`, no `jest`, no transitive dependency tree.
2. **Canonical TF01 Conformance:** Pinned against the `test-bootstrap-v1` cross-language self-corpus (1,256 cases).
3. **Pure Deep Equality:** Spec-compliant float bit pattern comparisons, arbitrary-precision integers up to 1000 digits, byte arrays, strings, lists, and maps.
4. **Deterministic Run Protocol:** Seeded SplitMix64 pseudo-random ordering with isolated sub-seeds and TAP version 14 output.
5. **Drop-in Matchers:** Familiar `describe`, `it`, and `expect` assertions to retire third-party test runners cleanly.
6. **Declarative CLI Runner:** Built with internal `@coding-adventures/cli-builder` supporting `--seed`, `--sabotage`, `--reporter`, `--filter`, and `--bail`.

## Fuzz campaign accounting

`fuzzTarget` validates its bounded options and copies the initial corpus before
calling the target. Its report keeps the legacy `iterations` field for mutation
calls and adds separate triage and mutation ledgers for `pass`, `crash`,
`timeout`, and `oom`. Callback exceptions still propagate. A clean report
requires a nonempty caller corpus, at least one mutation result, and only pass
outcomes; the compatibility fallback seed and a zero-iteration run are not
clean campaigns. These are returned-callback counts and caller-recorded edges,
not a process watchdog, enforced OOM/timeout, native coverage, or source-proof
claim. See `code/specs/VER06b-typescript-fuzz-engine.md` §8 for limits and
truncation behavior.
