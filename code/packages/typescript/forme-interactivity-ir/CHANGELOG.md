# Changelog — @coding-adventures/forme-interactivity-ir

## 0.1.0 — 2026-10-02

### Added

- FM05 Interactivity IR v1 types for state, bindings, predicates, handlers,
  declarative effects, and progressive-enhancement islands.
- Hostile-input validation with immutable snapshots, stable structured errors,
  non-trapping Proxy rejection, reference and type checks, safe-navigation
  policy, and bounded collections, recursion, node counts, strings, and exact
  incrementally checked canonical bytes.
- Deterministic canonical JSON serialization and the frozen
  `EMPTY_INTERACTIVITY` zero-JavaScript document.
- Conformance tests covering accepted documents and adversarial objects,
  accessors, cycles, sparse arrays, invalid limits, and unsafe effects.

### Security

- The data-only IR has no raw-code, module-path, host-handle, network,
  filesystem, environment, or shell surface. Island execution remains behind
  the Forme plugin/product authorization boundary.
- Canonicalization accepts only validator-branded snapshots, uses Unicode
  scalar key ordering, and rejects ill-formed surrogate sequences.
