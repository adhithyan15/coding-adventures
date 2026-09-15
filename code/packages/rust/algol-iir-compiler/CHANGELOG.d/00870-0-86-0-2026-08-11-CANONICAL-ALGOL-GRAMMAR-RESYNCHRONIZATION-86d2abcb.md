## 0.86.0 — 2026-08-11 — canonical ALGOL grammar resynchronization

The compiler now consumes the canonical generated parser shapes for scalar
`own` declarations and recursively nested conditional expression and
designational branches. This removes the parser artifact's historical drift
without introducing a dynamic closure or thunk ABI.

