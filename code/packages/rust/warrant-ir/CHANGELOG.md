# Changelog

## 0.1.0

- VER04h / VER-P030: add `inline` (WIR call inlining) so the symbolic executor and provers analyse a public export itself (`ct_eq`, `ct_eq_fixed`) rather than only its helper; differential-tested against the un-inlined program and the real Rust function.
- `ConstantTimeProver` and `BoundProver` keep every DRAT-checked refutation (`ProofCertificate`, with a `trivial` flag for miter-collapsed proofs) in `certificates`.
- Lowering fidelity: `assert!`/`assert_eq!` now lower to a branch into a trap (previously no-ops, which made `ct_select_bytes`'s length-mismatch path an out-of-bounds scan); `vec![0; n]` no longer emits a first-element store (wrote past a zero-length buffer). `debug_assert*` unchanged.
- Add `tests/warrant_proofs.rs`: Warrant `labels`/`bound` rows drive 2-safety and step-bound obligations over the real lowered `ct-compare` source for every length shape up to 128 (`bounded(128)`; `eq_u64` is `proved`), negative controls on mutated sources, and a proof manifest with content-addressed CNF/DRAT files (`WARRANT_PROOF_OUT`).

- Initial implementation of WIR core AST, static type checker, step cost schedule, observable trace events, provenance-tracking memory model, and constant-time verification engine (VER25).
- Confined Rust subset grammar, AST, and recursive-descent parser (VER25a, VER-P022), parsing real ct-compare source, tracking volatile/unsafe compiler-observability assumptions, with zero external dependencies and >90% line and branch coverage.
- Rust-to-WIR lowering front end (VER25b, VER-P023), compiling algorithmic Rust and volatile barriers into strongly typed WIR control-flow graphs with differential execution and constant-time validation against native `ct-compare`.
- WIR symbolic execution and bit-precise proof engine (VER19, VER-P024), featuring fixed-width bit-vector semantics (`bit-blaster`), symbolic memory with multiplexed indexing, bounded path exploration, miter formula construction ($PC \land \neg \text{equiv}$), CDCL SAT solving (`cdcl-sat`), DRAT proof validation (`drat-check`), counterexample extraction, and CWE-125/CWE-787 bounds safety guards with >90% line and branch coverage.
- WIR resource-bound and constant-time trace proofs (VER20, VER-P025), delivering self-composition 2-safety hyperproperty miters ($\sim_{\text{obs}}$), public/secret input partitioning, observable trace event divergence detection (memory address, branch decision, step count), resource bound verification ($steps \le B(n)$), vacuity guards (`VacuousProof`), and verified DRAT certificates (`drat-check`) with >90% line and branch coverage.
