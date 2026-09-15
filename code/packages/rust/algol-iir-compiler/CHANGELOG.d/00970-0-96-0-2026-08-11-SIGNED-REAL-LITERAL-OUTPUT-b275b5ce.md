## 0.96.0 — 2026-08-11 — signed real-literal output

The real-literal output fast path now accepts an exact unary `+` or `-` and
preserves that sign in stdout. Binary arithmetic and runtime real values still
bypass the literal recognizer and fail closed pending a runtime formatter ABI.

