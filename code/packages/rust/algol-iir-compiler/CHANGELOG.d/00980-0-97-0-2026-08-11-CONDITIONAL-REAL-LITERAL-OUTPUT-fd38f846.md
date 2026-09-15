## 0.97.0 — 2026-08-11 — conditional real-literal output

Implementation-defined output now accepts an arithmetic conditional when every
leaf is a direct real literal. The condition branches directly between each
leaf's source-spelled string output, so runtime selection works without a
partial f64 formatter ABI; any computed or runtime real leaf still fails closed.

