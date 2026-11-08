# Changelog

## 0.1.0

- Initial implementation of WIR core AST, static type checker, step cost schedule, observable trace events, provenance-tracking memory model, and constant-time verification engine (VER25).
- Confined Rust subset grammar, AST, and recursive-descent parser (VER25a, VER-P022), parsing real ct-compare source, tracking volatile/unsafe compiler-observability assumptions, with zero external dependencies and >90% line and branch coverage.
- Rust-to-WIR lowering front end (VER25b, VER-P023), compiling algorithmic Rust and volatile barriers into strongly typed WIR control-flow graphs with differential execution and constant-time validation against native `ct-compare`.
