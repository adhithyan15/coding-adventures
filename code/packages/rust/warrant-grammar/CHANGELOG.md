# Changelog

## 0.1.0 — Unreleased

- Evidence generator, second package (gf256, VER04h): the per-package binding
  table is now data. `examples/gen_evidence [--check] [<package> ...]` reads
  `code/packages/rust/<package>/evidence.bindings` (parsed by the new
  `binding::parse_evidence_bindings`, which rejects unknown keys and options so a
  typo cannot weaken evidence) instead of a hardcoded op-to-export table.
  ct-compare's generated harness is byte-identical through its new bindings file.
  - `EvidenceOptions` gains `exhaustive_budget` (default 400,000; gf256 uses 2^24
    for the `U8^3` field-law claims, measured at about 9 s per claim in a debug
    build) and `arg_types` (Rust parameter types of an export, e.g. `power`'s
    `u32` exponent against the Warrant `U64`).
  - New scalar adapter shape for operations over `U8`/`U64`/`Nat` returning `U8` or
    `Result<U8, E>` (a panic is normalised to `E` by the binding, as for
    `ct_select_bytes`), with a closed registry of field-arithmetic faults
    (`or-for-xor`, `wrapping-sub`, `mul-as-add`, `alt-poly`, `generator-shift`,
    `exp-mod-256`, `missing-error`). The generated `Fault` enum lists only the
    faults a package uses.
  - A negative control on a scalar operation attacks that operation alone
    (`attacking`/`fault_for`) and stops at the first claim that catches the fault;
    a fault applied to every operation of a field was invisible to every
    relational claim (lesson recorded).
  - Binder refinements may use models only; an operation inside one is refused
    (W0701). `U8` binders stay inside 0..=255 under relation-biased generation,
    `U64` binders draw small values often, and the fuzz coverage map sees `U8`
    values; these prelude regions are feature-gated so other packages' output
    does not change.
  - Tests: `tests/evidence_bindings.rs` (bindings reader, budget, fault and
    refinement rules, byte-for-byte regeneration of both checked-in harnesses).

- Add `evidence` (VER04h / VER-P030): compile a checked Warrant document into a
  Rust evidence harness (examples, bounded-exhaustive, property, fuzz and
  sabotage suites) whose oracle is the *translated claim body*, not a hand-copied
  `a == b`; supports any number of binders and `Bytes<N>` widths. New diagnostic
  `W0706` (exhaustive domain over budget). `examples/gen_evidence` regenerates or
  `--check`s `ct-compare/tests/generated/warrant_evidence.rs`.
- Stop emitting claim helpers from `compile_rust_l0_harness`: the old helper
  hard-coded `a == b`, accepted at most two binders and was never compiled.
  Removed the tests that string-matched it and the `a == b` vs `a == b`
  "sabotage" emulation; `tests/evidence.rs` compiles and runs the generated
  harness against the real `ct-compare` crate instead.
- Add `ct-compare/warrant` claims for `eq_fixed` (widths 0/1/2/4/16),
  `select_bytes` (pick + length mismatch) and `eq_u64`.

- Add VER04f/P011 Warrant Universal Claims to Property Tests Compiler:
  - Extract and validate universally quantified `claim` declarations (`ClaimCase`) calling bound host operations.
  - Compile claims into Level 2 property-based tests executed on `coding_adventures_test_bootstrap::pbt`.
  - Type-to-generator mappings for `Bytes`, `U64`, `U8`, `Bool` with deterministic PRNG replay and isolated sub-seeds.
  - Integrated shrinking minimizing counterexamples across component values.
  - Public host API call binding dispatch with panic isolation and fault injection.
  - Harness emission of claim sabotage negative controls (`claim_sabotage/{id}_broken_impl`) to detect broken implementations.
  - Privacy-preserving failure output in CI mode redacting raw inputs with FNV-1a reproducer digest.
  - Exceed strict ≥90% line and branch/region test coverage (93.19% lines, 94.51% regions) with zero third-party dependencies.
- Add VER04e/P008 Warrant code generation and L0 test vector compiler:
  - Bounded pure compile-time ground model evaluator with strict recursion depth (<= 64)
    and work quotas (<= 1,000,000 steps).
  - Canonical Portable Test Vector (TFV) compiler transforming Warrant `example` and `claim`
    declarations into structured portable records with logical work accounting.
  - Standalone, deterministic Rust L0 test harness generator executing on `test-bootstrap`,
    calling real public exports, verifying fresh-output non-aliasing, containing and
    normalizing declared panics via `catch_unwind`, and incorporating adapter negative controls.
  - Closed diagnostic error codes (`W0701`..`W0705`) and checked regeneration drift detection.
  - Pilot `code/specs/ct-compare.warrant` canonical specification.
  - Exceed strict ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04d/P007 Warrant type checker and Rust binding schema: static type checking,
  scope/symbol resolution, cycle detection on type aliases and pure model functions,
  minimal subtyping (`Nat <: Int`, `Bytes<N> <: Bytes`), exhaustive match verification,
  and contract validation (`labels`, `bound`, `input`, `assume`).
- Enforce closed diagnostic codes (`W0201`, `W0202`, `W0203`, `W0301`, `W0401`, `W0501`, `W0502`, `W0602`)
  and opaque error messages with zero input secret leakage.
- Add closed host binding schema (`BindingManifest`, `OpBinding`) validating complete operation
  coverage, fresh requirements, negative controls, and error normalizations.
- Achieve ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04c/P006b3 guarded Warrant source parser: typed AST construction conforming
  to `warrant.grammar`, strict recursion depth guard (`max_depth: 64`) preventing
  call-stack exhaustion (CWE-674), AST node quota (`max_nodes: 65,536`), and work limits.
- Enforce closed `W0001` diagnostics and verify zero source leakage in error Display/Debug.
- Validate all 57 canonical syntax cases from `cases.tfv` through admitted document,
  layout capture, and guarded AST parser.
- Add VER04b/P006b2 bounded significant-layout capture: space-only indentation,
  indentation stack tracking with configurable limits, tabs-in-indentation rejection,
  and exact zero-width synthetic token origins (INDENT/DEDENT/NEWLINE) without
  fictitious positive-width byte spans.
- Add VER04a/P006b1 source admission: bounded UTF-8/BOM/NUL/CR validation,
  borrowed LF views, fallible CRLF normalization and original-byte/one-based
  line/byte-column mapping through a source-bound line-start index.
- Enforce lowerable byte/capacity/work caps before scans/reservations; use
  closed W0001 errors and suppress source in error/document Debug output.
- Test authored Unicode/CRLF ranges, exhaustive small scalar ranges, malformed
  encodings, source/buffer/work boundaries and opaque diagnostics. Route the
  source contract to the Warrant consumer in the build tool.
- Add canonical significant-indentation Warrant token/EBNF grammars and
  deterministically generated Rust TokenGrammar/ParserGrammar constructors.
- Exercise 57 authored syntax cases through TFV/L0, exact subtree witnesses and
  dynamic/embedded grammar agreement. Add three adapter controls and a
  separately falsified precedence witness, source-shape guards and opaque errors.
- Check exact generated bytes and wire grammar/corpus/reference changes into
  repository input selection plus platform BUILD commands.
- Keep significant-layout origin capture, the guarded parser and its remaining
  quotas, typing/binding validation, proof and publishing work deferred.
