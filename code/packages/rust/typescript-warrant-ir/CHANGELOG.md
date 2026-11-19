# Changelog

## Unreleased

- Add a reusable source-bound proof example that validates caller-supplied
  source path and digest, proves healthy helper equivalence, and runs guarded
  source-mutant queries with explicit provenance and machine-readable results.
- Add symbolic precondition support to the WIR prover and expose proof-only
  dependencies to the repository build graph.

## 0.1.0

- Add the bounded `ts5.8` AST-to-WIR bridge for three typed scalar bit helpers.
- Bind the proof fixture bytes by path and SHA-256; type-check the lowered WIR.
- Add independent 64-output-bit SAT references, source-operator mutants, decoded
  witnesses, and clean-environment Node replay.
- Refuse unsupported syntax, unsafe shift/BigInt forms, unbounded BigInt NOT, and
  JavaScript `return` line terminators missing from the shared grammar.
