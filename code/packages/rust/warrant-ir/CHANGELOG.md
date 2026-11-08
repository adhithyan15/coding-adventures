# Changelog

## 0.1.0

- Initial implementation of WIR core AST, static type checker, step cost schedule, observable trace events, provenance-tracking memory model, and constant-time verification engine (VER25).
- Confined Rust subset grammar, AST, and recursive-descent parser (VER25a, VER-P022), parsing real ct-compare source, tracking volatile/unsafe compiler-observability assumptions, with zero external dependencies and >90% line and branch coverage.
- Rust-to-WIR lowering front end (VER25b, VER-P023), compiling algorithmic Rust and volatile barriers into strongly typed WIR control-flow graphs with differential execution and constant-time validation against native `ct-compare`.
- WIR symbolic execution and bit-precise proof engine (VER19, VER-P024), featuring fixed-width bit-vector semantics (`bit-blaster`), symbolic memory with multiplexed indexing, bounded path exploration, miter formula construction ($PC \land \neg \text{equiv}$), CDCL SAT solving (`cdcl-sat`), DRAT proof validation (`drat-check`), counterexample extraction, and CWE-125/CWE-787 bounds safety guards with >90% line and branch coverage.
