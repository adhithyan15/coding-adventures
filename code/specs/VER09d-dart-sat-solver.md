# VER09d — Dart Level 3 CDCL SAT Solver & Tseitin Bit-Blaster

**Parent:** [VER00 §7.1](VER00-verified-shipping-program.md), [VER09](VER09-cdcl-sat.md), [VER09a](VER09a-typescript-sat-solver.md).

This specification defines the Dart implementation of the Level 3 CDCL SAT
solver and Tseitin bit-blaster in `code/packages/dart/sat-solver`. It is a
behavioral port of VER09a **including the VER-P043 repair semantics**: same
literal packing, 2WL propagation with blockers, 1-UIP learning, VSIDS with
phase saving and Luby restarts, assumption solving with sound failed cores,
conflict/step budgets with sticky root conflicts, strict DIMACS transport,
and the Tseitin gate/bit-vector builders.

---

## 1. Solver Architecture

- **Literals**: `lit = (v << 1) | sign`, `varOf = lit >> 1`,
  `signOf = lit & 1` (1 = negated), `negate = lit ^ 1`. Three-valued logic
  `True = 1`, `False = -1`, `Undef = 0`.
- **2WL propagation**: two watched literals per clause with blocker fast
  path; unit clauses propagate onto the trail; conflicts return the clause
  index, `-1` when clean, `-2` on step-budget exhaustion. The budget is
  checked *before* dequeuing trail work, so UNKNOWN preserves pending root
  work for a later solve or public `propagate()`. A level-0 conflict makes
  the contradiction sticky (`ok = false`).
- **1-UIP learning** with non-chronological backjumping; unit learnts assign
  at level 0; learned database halves past `maxLearned` (locked clauses and
  binary learnts survive).
- **VSIDS** with binary heap, phase saving (default negative), Luby restarts
  (`100 * luby(n)`, clauses and phases preserved).
- **Assumptions**: decided in order at successive levels; already-true
  assumptions take dummy levels; contradictions during the assumption prefix
  extract a sound duplicate-free failed core and stay local (`ok` untouched).
- **Budgets**: `maxConflicts` / `maxSteps` bound one solve call; exhaustion
  returns `unknown` after backjumping to level 0. A fractional `maxSteps`
  permits its whole-number portion. Budgets reset to unbounded after each
  call. A solve that reaches the known-root-UNSAT fast path clears any older
  failed core; malformed-assumption validation keeps precedence.

## 2. Tseitin & Bit-Blaster

`CnfBuilder` over an owned `Solver`: `and`/`or`/`xor` (4-clause),
`not` (literal negation, no clauses), `ite`/`equiv` (4-clause),
`fullAdder` (XOR/XOR sum, AND/OR carry), `bvEq` (empty vectors are true),
`bvAdd` (ripple-carry from a false carry-in), `trueLit`/`falseLit` unit
literals. Width mismatches throw.

## 3. DIMACS Transport

`parseDimacs` is strict: lines split on CR/LF, comments (`c…`) and blanks
skipped, `%` line terminates input (SATLIB), header exactly four tokens
`p cnf <vars> <clauses>` with digit-only counts, variable ceiling
`maxVars` (default 2^20), integer-only tokens, `0`-terminated clauses,
literals in `[1, numVars]` (`-0` terminates), exact clause-count match.
`writeDimacs` emits `p cnf` plus `lit… 0` lines. Malformed input throws
`FormatException`.

## 4. Conformance Verification

- The shared `sat-cnf-v1` stateful corpus: exact 14-case id list, per-step
  `solve`/`propagate`/`add_clause`/`reset` execution, SAT models validated
  against the independently decoded signed CNF (width, clauses,
  assumptions; core cleared), UNSAT cores checked duplicate-free, subset of
  assumptions, and brute-force sound (plus pinned `expect_core` rows),
  UNKNOWN clears the core.
- 100 seeded 3-SAT differential rounds vs brute force at m/n ≈ 4.26 with
  both outcomes exercised; pigeonhole UNSAT; 4-queens model; full-adder and
  4-bit ripple-carry equivalence with sabotage checks; assumption/core
  soundness; budget isolation across successive solves; strict DIMACS
  rejection battery.
- 64 seeded stateful differential traces (seed `0x51a7e5eed`, budgets
  0/1/1.5 cycling) against the independent oracle with exact operation
  accounting.
- A bounded 2,500-iteration mutation campaign through the Dart `fuzz`
  lane (seed `0x504043`) with positive/negative controls, full iteration
  budget execution, zero crashes, and semantic-edge exploration.
- Zero runtime dependencies; 95% branch+function coverage gate.

## 5. Dart-Specific Notes

- Typed arrays are `dart:typed_data` (`Int8List`/`Int32List`/`Float64List`);
  growth, heap rebuild, and rescale thresholds mirror VER09a exactly.
- Results are the strings `sat`/`unsat`/`unknown` (matching corpus rows).
- No `dart:io`: corpus bytes load in tests; the library is pure compute.
