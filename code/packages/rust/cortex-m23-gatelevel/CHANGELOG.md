# Changelog

## 0.1.0 — 2026-10-26

- Add an independent Armv8-M Baseline Cortex-M23 gate transition engine.
- Store the complete architectural state in exactly 524,898 repository D
  flip-flops and derive every production transition through repository gates.
- Add typed atomic lifecycle faults, fallible bounded complete-state traces,
  deterministic allocation-failure seams, and exact functional-oracle
  differential coverage for every implemented instruction subfamily.
- Add AST-based topology and module-routing checks with hostile native-host,
  oracle-substitution, helper-indirection, post-dispatch overwrite,
  direct-Q-write, macro, string, and attribute confusers, plus exact locks for
  every critical decode-to-commit body.
- Exercise every condition and shift-boundary outcome, arithmetic and
  multiply edges, and the register/immediate memory-addressing overlap matrix.
- Reach 93.64% combined Rust line coverage (1,825 of 1,949 lines) across 34
  release tests.
